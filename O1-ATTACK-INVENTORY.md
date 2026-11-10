# O1-ATTACK-INVENTORY — every cost claim found, by crate and document

Companion to `O1-ATTACK-REPORT.md` (cloud lane `cloud/o1p99-attack-20261110`,
10 Nov 2026, from `origin/feat/db-fix` at `beaf2e3c4`).

**What this file is.** Step 1 of the attack: every place the code or the
documents claim O(1), constant, flat, a p99 bound, or a time/cost bound or
budget, one row per (function or route, claim), with the file:line of the
claim, the size the claim says the cost is flat in, the bench that measures it
(or `none`), and whether the implementation, READ, does something that grows
with that size (`suspect`). It was compiled by five read-only research passes
over `crates/*/src`, `crates/*/benches`, `crates/*/tests` and `docs/`, by grep
and by reading the function bodies. Nothing in it was compiled or timed.

**How to read the `suspect` column.** It is a reading of the source, not a
measurement: `no`, `yes: why (file:line)`, `maybe: why`, and `ack` /
`yes (admitted)` where the code or `docs/06-limits.md` already says the cost
is not O(1). Every row the attack MEASURED is in the report's Table 1 with
its p50 and p99; a row here that is not in Table 1 was not timed by this lane
and keeps whatever status its own bench gives it. Line numbers are on
`beaf2e3c4`.

**Counts.** `api` 188 rows (101 routes, 31 benched); `pull` 234; `cli` 165;
the ten smaller crates 189 (store 63, runner 30, indicators 20, telemetry 19,
engine 17, core 14, costs 12, vocab 7, lake 4, greeks 3); documents 211 claim
rows. The admitted-limit extraction (653 `docs/06-limits.md` items, 138
decisions, 475 gate-11 allowlist entries) is summarised by its curated
exclusion list at the end; the attack used it to tell a known limit from a
new one.


## `api` crate — inventory of cost-bound claims (O(1) / constant / flat / p99 / budget)

Branch `cloud/o1p99-attack-20261110`. Read-only research; nothing in the repo was changed, nothing was compiled or run.

Method: `grep` over `crates/api/src/**` (production lines only: inline `mod tests {}` blocks and
server.rs test ranges 8677-8693, 18816-19585, 21166-36221 excluded) for `O(1)`, `O(log`, `flat`, `p99`,
`constant`, `costs the same`, `whatever the`, `however`, `grow`, plus the `# Cost` sections of every route
handler; each hit mapped to its function, deduped per (function, claim), and the body (and what it calls)
read for growth. Benches: `crates/api/benches/ratio.rs` (R) and `crates/api/benches/ratio/<x>.rs` (R/x);
tests: `crates/api/tests/*.rs` and in-file unit tests.

Notes that apply to every row:
* No custom/weak hasher exists in `api` (no `BuildHasher`/FNV map); every `HashMap`/`HashSet` is std SipHash
  `RandomState`, so "all keys collide" attacks on in-memory maps are not available. Collision-style attacks
  only apply to on-disk/compile-time tables in other crates (`store::quarantine` bucket runs, `core::alias`
  `MemberIndex`), which are fixed-size.
* Request target is capped at 8 KiB (`MAX_REQUEST_TARGET_BYTES`, server.rs:16280; 60 KiB for
  `/bars/window.json`, server.rs:16291), headers at 64 KiB: every per-request parse of the query string
  (`param`, `repeated_query_key`, `paging_sent`, `selection_of`) is O(target) <= that cap.
* "flat in" = the size dimension the claim says the cost does not track.
* suspect: `no` / `maybe: why` / `yes: why (file:line)`.

| # | where | what (fn / route) | claim (quote, <=15 words) | flat in | bench / test | suspect |
|---|---|---|---|---|---|---|
| 1 | server.rs:1028 | `/dashboard` `home` -> `dashboard_html` (server.rs:1038) | "costs the same whether the store holds two instruments or two hundred thousand" | instruments in universe | R C-16 (2->2,787->50,000, gated 3x), R C-27 (floor budget 2,100) | no (catalog counters precomputed at load; `notes_view` built once, server.rs:513) |
| 2 | render.rs:944 | `render::dashboard_page` | "the page costs the same whether the store holds two instruments or two" | instruments | R C-16/C-27 | no |
| 3 | catalog.rs:342, 475 | `/instruments` `page` -> `Catalog::page` | "One slice index and at most PAGE_ROWS row copies, whatever the size" | instruments, page number | R C-14/C-15/C-17 every sort x pill, `page=9999` | no (precomputed orders; descending = slice from far end) |
| 4 | catalog.rs:360, 531, 539 | `/instruments?q=` `Catalog::search` | "over-long search is answered in constant time instead of scanned" | instruments (needle > longest only) | R `search_is_measured_and_named` (printed, NOT gated); unit `a_needle_longer_than_the_longest_name_is_answered_without_a_scan` | yes for every other needle: <3-byte needle scans `0..rows` (catalog.rs:581), common trigram scans most rows, and `hits.sort_unstable_by_key` sorts all hits per request (catalog.rs:547). Stated in limits §24 |
| 5 | instruments_route.rs:75 | `/instruments.json` `instruments_json` | "a probe of the kept answer, whatever the census's size" | census entries | R/reads "instrument list, revalidated" (304, gated) | maybe: body+BLAKE3 rebuilt O(census) on store-read pool for the first ask after every census read; bench census never changes between asks |
| 6 | store_route.rs:731 | `/store.json` `census_etag`/`store_response` | "O(body) once per immutable census snapshot, vendor and wire format" | census entries (per request) | R/reads "store body, revalidated" (304); R/census `deltas_are_flat`, `read_backs_are_flat` (gated) | maybe: every census generation (Zerodha pull commits every 0.5-2 s, census_cache.rs:65) makes the next GET/304 re-encode + hash the whole body (21-53 MB for GDFL) |
| 7 | store_route.rs:404-411 | `store_route::behind` (census headers of `/census/facets.json`, `/census/rollup.json`, `/chain.json`, `/sources.json`, `/audit.json`) | "a request with neither changed is two pointer comparisons" | journal length | none | yes: holds `Arc<Parsed>` in `site.census_wire.behind` (store_route.rs:456) so every journal append makes `import_journal::snapshot` deep-clone the whole parsed journal (`Arc::make_mut`, import_journal.rs:754) |
| 8 | feeds_route.rs:230 | `/feeds.json` `feeds_json` -> `feeds_body` | "O(1) in the store: one cached census read for the whole answer" | census keys | R/db_page "the feed picker" (gated); tests/pool_load.rs (feed-list p99 < 10 ms beside 32 clients) | yes: `held_spans`->`part_span` walks every key of a part re-read whole (feeds_route.rs:808-811, "0.66 to 2 s" cold), and `imported_to`->`journal_reach` walks journal rows after every journal change (feeds_route.rs:919-950); bench census static |
| 9 | feeds_route.rs:775 | `part_span` | "the same generation as last time: one map probe, O(1)" | census keys | unit `feeds_route::tests::held_span_growth_bench` (hit and 1-key append at 1x/10x/100x) | yes: O(keys) walk on first request and after any whole re-read (epoch, compaction, rename, tail past floor) |
| 10 | feeds_route.rs:890 | `journal_reach` | "while the journal does not grow, one map probe" | journal rows | none (L-R305-3) | yes: after each growth walks rows newest-first until every kind's closed day is found: O(rows since the most-lagging kind) — a kind that stopped closing => whole journal per change (feeds_route.rs:919-949) |
| 11 | feeds_route.rs:1025 | `imported_to` (used by `/gaps.json`, `/feeds.json`) | "O(1) while the journal does not move" | journal length | none (L-GF1-33 UNVERIFIED) | yes: snapshot make_mut clone (import_journal.rs:754) + `journal_reach` walk |
| 12 | feeds_route.rs:465 | `readiness` | "O(1): a few field reads and no" | store | none | no |
| 13 | feeds_route.rs:569 | `held_ready` | "O(1): the census is already read by the caller" | census | none | no |
| 14 | feeds_route.rs:993 | `ImportReach::not_yet` | "O(1): one comparison with the day read once" | days | none | no |
| 15 | feeds_route.rs:1002 | `ImportReach::not_imported` | "O(log open)" | open days | none | no (`binary_search` on open-day Vec; log, labelled as log) |
| 16 | settings_route.rs:45 | `/settings.json` `settings_json` | "A GET is one `stat` ... O(1). ?history=N reads N + 1 files" | settings generations | R/settings (2/20/200 generations, `history=5`, gated) | maybe: `history` up to `MAX_HISTORY`=50 => 51 file reads per request (bench only times 5); first request lists the settings directory |
| 17 | calendar_route.rs:24-49 | `/calendar.json` `calendar_json` -> `calendar_json_reading` (calendar_route.rs:1031) | "a request that finds it is one map probe" (kept p99 0.59 ms) | months held, census | R/calendar (feed, NIFTY, record; gated) | maybe: kept only under the feed manifest stamp; every census commit forces a rebuild (p50 8.5 / p99 20 ms; first build 0.12-3.6 s => 202 after budget); kept body cloned per request (calendar_route.rs:1075) |
| 18 | server.rs:37583-37600 | stale doc before `vocab_json` (about `/calendar.json`) | "collect, sort and dedup the asked feed's census entries ... on every request" | census | none | stale/contradicts calendar_route.rs:1020 — flag as doc defect |
| 19 | calendar_record.rs:966 | `widen_to_record` | "O(1) per request in the store's size" | store | R/calendar `span=record` | no (bounded by the record's 3,199 days) |
| 20 | calendar_record.rs:1071 | `not_widened` | "One parse of the kept answer: O(1) per request" | store | R/calendar | maybe: re-parses the kept JSON body (~19 KB) every request: O(body), constant only in store size |
| 21 | calendar_record.rs:166 | `record_code` | "table and one of the fifteen-entry special-session table: O(1)" | days | none | no |
| 22 | calendar_record.rs:557 | `derivatives_owed` | "O(log days) table walks of a three-row table" | days | none | no |
| 23 | calendar_build.rs:12 | calendar build budget (`build::REQUEST_WAIT`) | "A route whose maximum is a whole build is not O(1)" | — | none | stated: a request waits up to the budget, then 202 |
| 24 | calendar_of.rs:41 | `calendar_of` counter check | "`BarFile::records()` is one header read, O(1) per month" | bars per month | none | maybe: a month whose count != sessions x 375 is walked record by record (14 of 81 on the operator's store) |
| 25 | gaps_route.rs:79 | `/gaps.json` `gaps_json` | "O(months asked), each month O(1) in its bars" | bars per month | R/gaps kept 1s month p99 (gated), 240-month span vs tiles (gated 1.5x/2x), classification per day; R/reads "gaps of the month" per bar | maybe: `inferred` reads the census per request when exchange/segment omitted (gaps_route.rs:288-318); `audit_one` takes an import-journal snapshot (item 26); `summary_fallback` reads all bars (printed only) |
| 26 | gaps_route.rs:1652 | `audit_one` import-journal probe | "a cached probe, O(1) while the journal does not move" | journal length | none | yes: same make_mut deep copy on every append (import_journal.rs:754) |
| 27 | gaps_route.rs:687 | `audit_span` -> `pull::gaps::not_owed` | "asks `pull::gaps::not_owed` in constant time" | days | R/gaps `not_owed per call` p99 (gated) | no |
| 28 | gaps_route.rs:1329 | `faults` | "O(1) in the store: no read and no walk" | store | none | no |
| 29 | gaps_route.rs:2361 | `second_audit` | "seconds, 86,400 a day, whatever the bars per month" | bars per month | R/gaps | no (per day bound) |
| 30 | gaps_route.rs:885-907 | `calendar=peers` vote `peer_vote_reading` | kept "while none has moved: one stat per vendor and one probe" | census | none | maybe: rebuild = `held_entries` sort of each vendor's census, O(E log E), on any manifest move (off-request, 202 meanwhile) |
| 31 | bars_route.rs:203 | `/bars.json` `bars_json` -> `month_answer` | "the claim is the shape, not the nanoseconds" (formerly "O(1) per bar") | bars before window | R/reads "bars, its first day" per row (gated); `refusals_are_flat` BBENCH-03; tests/bench_paged_month.rs | no (O(rows asked) by design, <= `MAX_MONTH_ANSWER_ROWS` 60,000) |
| 32 | bars_route.rs:358 | `month_answer` -> `quarantine_answer` (bars_route.rs:559) | "O(1) whatever the store holds" (two positional reads) | store / quarantine size | R/reads (indirect) | no (store::quarantine::probe_at reads one fixed `BUCKET_BYTES` run) |
| 33 | bars_route.rs:423, 664 | `past_the_last_bar` | "decided by reading ONE record: the last" | bars in month | unit tests count the read | no |
| 34 | bars_route.rs:446-481 | `/bars.json` too_many_rows refusal | "THE CAP IS KEPT, AND ITS REFUSAL IS O(1)" (bench doc) | month rows | R/reads `refusals_are_flat` 60,001 -> 102,400 -> 2,592,000 (gated) | no |
| 35 | bars.rs:394 | `note_absent` | "O(1): one atomic add and one or two hash probes" | requests | none (UNVERIFIED L-BRED23-01) | no (PathBuf clone O(path)) |
| 36 | bars.rs:1312 | `rekey_status` | "O(1): one ledger probe" | lineage ledger | none | maybe: the probe is on `Lineage::read`, which re-reads+re-parses the whole lineage ledger whenever its (len,mtime) stamp moves (bars_change.rs:487-509) |
| 37 | bars.rs:1574-1598 | `/bars/window.json` `bars::window` (pub) | "page 12,470 costing what page 1 costs" (O(months) stats + O(limit)) | bars per month, page offset | R/window seek/merge k=2/at=/sort first+last page (gated); R/months O(months) printed only; R/reads paged per row | yes in series count: `query.series` up to `MAX_SERIES`=1,024 (bars_series.rs:74) x 240 months => ~245k `stat`+quarantine probes per request; bench k <= 2 |
| 38 | bars.rs:1615-1625 | `CHEAP_READ_BUDGET` 65,536 | "About 3.5 ms of one thread" | — | none | budget: requests past it go to the scan pool |
| 39 | bars.rs:965-975 | `MAX_SCAN_BARS` 24,000,000 | "a little over a second of one scan-pool thread" | — | none | yes: a value sort over rows no `.srt` covers / any time-of-day sort reads up to 24M bars per request (bars_sort.rs:40-48) |
| 40 | bars_sort.rs:56-68 | value sort via `.srt` (`bars_sort`) | "p50 2.1-3.3 ms, p99 under 5 ms" | bars (indexed months) | R/window sort=c asc/desc, sort=v desc (gated, every month indexed) | yes: months < `INDEX_MIN_ROWS`=1,024 have no `.srt`, appended tails and failed indexes too: all their rows read per request, `MAX_READ_HELD` 1M ordered whole (bars_sort.rs:109) |
| 41 | bars_series.rs:42 | rank via `.tix` | "O(1) with the month's `.tix`" | bars per month | R/window `at=` k=2 (gated) | maybe: months without `.tix` bisect (142 of 194 GDFL months on the operator's store, calendar_route.rs:32) |
| 42 | bars_series.rs:83, 89 | `series::Memo` (MEMO_CAP 65,536) | "a lookup or an insert stays O(1): two hash probes" | memo size | none (UNVERIFIED) | no (two-generation; old half freed off-lock) |
| 43 | bars_series.rs:914 | `month_of_text` | "O(1): a fixed seven-byte pattern" | — | none | no |
| 44 | bars_series.rs:1219 | `SeriesPlan::admit` quarantine probe | "O(1) whatever the store holds" | store | none | no per month; O(months) per series |
| 45 | bars_series.rs:1537 | `nth` | "`O(log runs)`, at most thirteen" | runs | none | no |
| 46 | bars_series.rs:1616 | `previous_held` | "`O(log held)`" | held months | none | no |
| 47 | bars_series.rs:1721 | `read_stretch` unread months | "the answer stays bounded whatever the stretch holds" | stretch rows | none | no |
| 48 | bars_change.rs:43 | per-bar change rules | "Per bar, O(1) and no allocation" | bars | none (UNVERIFIED) | maybe: a refused master load is re-read once per day per request (bars_change.rs:372-399); lineage ledger re-parse on stamp move |
| 49 | bars_change.rs:367 | `Rules::master` | "Expected O(1) per bar: a hit in either memo" | bars | unit CSL-05 (once-per-request) | maybe (as 48) |
| 50 | bars_change.rs:445 | `Lineage::read` | "once, whatever the number of series" | series | none | yes: `read_to_string` + `parse_ledger` of the whole ledger whenever its stamp moves (bars_change.rs:487-509), on the request thread (window, chain via `lineage_of`) |
| 51 | bars_export.rs (bench R/export.rs:1-9) | `/bars/export.csv` `bars_export_csv` | "one export of the same rows costs the same however large the month" | bars per month | R/export (1,000 rows newest-first and by close, gated) | maybe: by-close export of a window with unindexed rows scans (as 40) |
| 52 | spot.rs:48 | `/spot.json` `spot_json` / `spot_at` (pub) | "O(1) through the month's `.tix` time index when it has one" | bars per month, months held | R/spot held/between/as-of prev (gated); R/months `spot.json last month` (gated) | maybe: <= `MAX_SPOT_TIMES`=400 instants per request (spot.rs:72); un-indexed months bisect |
| 53 | optiongreeks.rs:56-65 | `/greeks.json` `greeks_json` / `respond` (pub) | "one solve bounded by `greeks::solver::MAX_ITERATIONS`" ; per instant O(1) lookups | bars per month | R/greeks 1 and 10 instants, rate table UNSET so no solve is ever timed (gated) | yes: 400 instants x worst-case solver iterations per request; rate table reload O(rows+days) on stamp move |
| 54 | chain.rs:116-125 | `/chain.json` `chain_json` -> `answer` | "never a walk of the census"; O(S) strikes per instant | census, months | R/chain one instant / asof=prev (10 strikes, static census, gated) | yes: calls `facets::index` (chain.rs:933) which WAITS for / performs the FNO facet-index move or whole build on the request thread (facets.rs:592-598 "/chain.json still does"): O(appended) or O(FNO keys, ~2M) after any census change; plus `census_behind` journal clone |
| 55 | facets.rs:33 | `/census/facets.json` `facets_json` | "it costs the size of its answer, whatever the census holds" | census | R/options root, NIFTY expiries, strikes (gated) | maybe: answer size is the selection's (e.g. `segment=CASH` lists every cash underlying); journal clone via `behind` |
| 56 | facets.rs:51 | facet index move | "the appended entries since the index last moved (O(log) each)" | census | none | maybe: up to a quarter million appended entries per move; on request thread when Unattended (bench/one-shot) |
| 57 | facets.rs:754 | `stand` | "holds no part for, O(1)" | census | none | no |
| 58 | facets.rs:1057 | `emptied` | "O(1): one `mem::replace`" | index size | none | no (old index dropped by `retire` off-lock) |
| 59 | facets.rs:1508 | `root_answer` | "node read per segment ... whatever the census holds" | census | R/options "feed root" | no |
| 60 | facets.rs:1924 | `rows_answer` | "bounded by the two caps, whatever the store holds" | store | none | no |
| 61 | facets.rs:323 | `segment_cells` | "read, whatever the census holds" | census | R/rollup | no |
| 62 | census_rollup.rs:17 | `/census/rollup.json` `rollup_json` | "its size is the selection's months times its rungs, whatever the census holds" | census | R/rollup segment / underlying (all fixtures in ONE month) | maybe: answer grows with months held (decades x rungs); bench never grows months; journal clone via `behind` |
| 63 | census_cache.rs:7 | `VendorCensus::rows_of`/`rows_for` | "one O(1) probe of it" | census entries | R/census probe held/absent 1k->100k (gated) | no |
| 64 | census_cache.rs:58-80 | per-vendor census cache `Cached` | (was "p50 700 ms, p99 816 ms") now tail-extended | census entries | R/census `tails_are_flat` (gated) | maybe: in production refreshed in background; see 66 |
| 65 | census.rs:400 | `Part::held` | "One probe per tail level ... expected O(1)" | census | R/census | no (two levels) |
| 66 | census.rs:1661 | `census::extend` (tail read) | "O(appended entries), whatever the file holds, plus one copy" | census entries | R/census tail of 256 (gated) | yes: `Tail::settle` clones the whole settled level once per 4,096 appends (census.rs:262-271); tail > max(65,536, base/8) => whole re-read (census.rs:1743) |
| 67 | census.rs:433 | `Part::changed_since` | "O(log tail + answer)" | census | R/census delta + read-back (gated) | maybe: read-back up to `SINCE_READ_BACK_MOST`=262,144 entries per request |
| 68 | census.rs:415 | `Part::held_keys` | "O(1) to call, O(keys) to walk" | — | none | no (honest) |
| 69 | census.rs:1839 | `sized` | "O(1) calls; the read is O(manifest bytes), bounded" | — | unit test | no |
| 70 | census.rs:2032 | `unreadable_root` | "constant work with respect to store size" | store | none | no |
| 71 | census.rs:2048 | `month_back` | "both cost the same" | months back | none | no |
| 72 | census.rs:2289 | `held_series` | "O(keys log keys) ... Not O(1)" (per request in `/store?show=gaps`) | census | none | yes (stated): sort of every census key per request |
| 73 | census.rs:2341 | `held_entries` | "O(entries log entries), on FOUR PER-REQUEST PATHS" | census | none | yes (stated): still per request in `/bars` spot lookup (server.rs:15692), gaps peer vote, spot-calendar path (server.rs:3358) |
| 74 | census.rs:2422 | `StoreFilter::keeps` | "O(1) per entry rather than O(1) per character of the store" | — | unit `the_symbol_arm_searches_a_haystack_the_symbol_type_bounds` | no |
| 75 | census.rs:2486 | `filtered` | "O(entries) — one pass ... Not O(1)" | census | unit `the_unfiltered_page_borrows_the_table_and_copies_nothing` | yes (stated) for any filter |
| 76 | census.rs:2536 | `held_page` | "a request for page 5 touches the rows of page 5" | page number | none | no (`skip` on slice iter is O(1)) |
| 77 | server.rs:15490 | `/bars` `locate_contract` | "one hash probe per exchange ... does not grow with the 1.15 million contracts" | census | none | no for contracts |
| 78 | server.rs:15658-15692 | `/bars` `bars_get` -> `locate_spot` | "each walked once through `held_entries`, as before" | census | none | yes: when exchange or segment is omitted, every request sorts each vendor's whole census (O(E log E), 1.15M entries) |
| 79 | server.rs:1891 | `/verify.json` `verify_json` | "O(log length) in memory plus O(E_v) file opens" | — | R/months verify scrub (printed, not gated) | yes (stated O(entries)); one admitted at a time |
| 80 | verify.rs:36, 167, 234 | `verify::vendor` | "One hash probe per entry: O(1) per operation" | census | R/months printed | yes per request O(entries) (stated) |
| 81 | provenance.rs:61-85 | `/provenance.json` `provenance_json` | "This is O(days in the month), not O(1)" | bars per month, census | R/provenance index month / bar at t / option month (gated) | maybe: un-indexed months bisect (provenance.rs:664); import-journal snapshot (provenance.rs:905) clone on append |
| 82 | dayflags.rs:50-83 | `/dayflags.json` `dayflags_json` | "O(days in the page)" (each ledger a tail, BTreeMap probe a day) | ledgers, journals | R/dayflags (ledgers ABSENT, GDFL only, gated); R/logs_dirs day band L-IA-39 printed | yes: Zerodha spot page reads pull-journal index `runs` (O(runs)) + one read per refusal of the symbol each request (dayflags.rs:2954); day band keeps every `run` line (L-IA-39); ledger tails `Arc::make_mut` (dayflags.rs:1824, 2739) |
| 83 | dayflags.rs:1780-1846 | `read_ledger` (gdfl.dayflags, zerodha.gaps, quality ledger tails) | "a tail -- one `metadata` call and the bytes appended since" | ledger length | none | yes: `Arc::make_mut(&mut tail.parsed)` deep-copies the whole parsed ledger whenever any reader still holds the previous Arc (always under concurrency) — the exact defect scrub_route.rs:113-122 fixed for scrub only |
| 84 | dayflags.rs:898 | `LINEAGE` memo | "one map probe, whatever the ledger's length" | lineage ledger | none | maybe: whole re-parse on stamp move |
| 85 | dayflags.rs:1040 | `lineage_json` | "O(1) a request once the ledger is" read | ledger | none | maybe (as 84) |
| 86 | dayflags.rs:1129 | `LateCloses::get` | "O(1): one hash probe however many journal lines" | journal | unit test | no |
| 87 | dayflags.rs:2178 | quality `get` | "O(log n)" | ledger | none | no |
| 88 | dayflags.rs:2381 | gap line `get` | "O(log n)" | ledger | none | no |
| 89 | dayflags.rs:2555 | `derive_json` | "O(log n + lines in those months)" | ledger | none | no |
| 90 | dayflags.rs:2843 | `Via` (pull journal walk) | "long journal is the one non-constant cost of this route" | pull journal | none | yes (stated): no-index journal walked once per process (4.2M records, 1 GiB) |
| 91 | dayflags.rs:2954 | `read_pull_journal` | "O(runs) ... nothing grows with the journal" | pull journal | none | yes: O(runs) read per request grows with every pull run; refusal ordinals O(refusals of symbol), unbounded for a symbol refused every run |
| 92 | dayflags.rs:3784 | `counts` day scan | "the day's scan is kept while its month does not grow" | bars | none | maybe: a month being appended re-scans each request |
| 93 | dayflags.rs:4640 | `sampled_json` | "O(1): the counts the record already holds" | — | none | no |
| 94 | dayflags.rs:9732-9803 | test `the_real_days_are_labelled_and_the_answer_is_timed` | "TIMED n=200 ... p50 p99 max" | — | unit (asserts no time) | n/a |
| 95 | dayflags_rules.rs:22-26 | day-flag rules | "Every rule is O(1) a day ... master read is NOT O(1)" | days | none | maybe: master read per cold day (`LOADS_AT_ONCE`=4) |
| 96 | dayflags_rules.rs:262, 306, 372 | master loads | "master read once however many ask at once" | concurrent askers | none | no |
| 97 | sources.rs:52-62 | `/sources.json` `sources_json` | "`O(rungs x months held)` ... independent of how many instruments" | instruments | R/reads "sources of NIFTY" (gated) | maybe: owed-sessions = `/calendar.json` answer rebuilt per feed generation (0.7-0.9 s first read after a commit); `DIRS` memo cleared whole (item 100); `behind` journal clone |
| 98 | sources.rs:73-80 | `/sources/bars.json` `timeline` (pub) | "`O(offset + limit)` buckets ... bounded by the page, not by the history" | month history | R/sources page 1min asc/desc, offset 0, limit 10 (gated) | yes in offset/rung: `offset` <= `MAX_OFFSET`=1,000, `limit` <= 200, a GDFL fold reads (offset+limit) x width seconds, up to `MAX_ROWS_READ`=2,000,000 records per request (sources.rs:147-151) |
| 99 | sources.rs:81-95 | `/sources/day.json` `day_table` (pub) | "cached per (generations, instrument, day) ... an unchanged day costs three `stat`s" | month bars | R/sources "day table cached" (one key, gated); first read printed | yes: `DAYS` memo (sources.rs:2922) is a `Memo` that `clear()`s the whole map when it reaches 65,536 keys (sources.rs:483-488) under its mutex: a p99 cliff + every hot day cold again; any 1-24-byte symbol is accepted (sources.rs:301) so fake symbols fill it at 31 keys a request; plus `dayflags::answer` per request (sources.rs:3524) |
| 100 | sources.rs:463 | `sources::Memo` (DIRS, DAYS, MASTERS) | "emptied whole when full, so it never grows past MEMO_CAP" | memo keys | none | yes: the DB-B57 "p99 cliff nothing announced" that bars_series.rs:79-89 fixed with two generations is still here |
| 101 | sources.rs:3747 | `measured_run_json` | "O(1): one hash and one or two indexes" | basis table | none | no |
| 102 | scrub_route.rs:95-112 | `/scrub.json` `scrub_json` | "None of it grows with the ledger"; indexed incrementally | scrub ledger | R/db_page key + standing (ledger written once, gated) | maybe: first read after start and any rewrite re-index from byte 0 in `INDEX_STEP_BYTES` steps (truncated answers); bench never appends during timing |
| 103 | scrub_route.rs:364 | scrub `Index` | "a seek and a constant number of steps, whatever the ledger holds" | ledger | none | maybe: steps = standing passes ("a handful"), grows if many passes stand |
| 104 | scrub_route.rs:525, 598 | `Index::release`/`keep` | "O(log findings + findings of that month)" | findings | none | no |
| 105 | aliases.rs:28-33 | `/aliases.json` `aliases_json` | "The route is O(1) per request" | store | R/db_page table + `symbol=NIFTY` (gated) | no (rendered once; `symbol` capped at `MAX_NAME_BYTES`; compile-time tables) |
| 106 | basis_route.rs:31-42 | `/basis.json` `basis_json` | "a request costs those `stat`s, one hash ... O(1)" | basis/action tables | R/basis 20/200/2,000 shares (gated); unit `a_lookup_costs_the_same_at_1x_10x_and_100x_runs_measured` | maybe: no-`day` answer is O(share's runs+actions) (capped); new generation re-read O(rows) on request |
| 107 | basis_route.rs:128 | `tables` | "O(1) when kept: four `stat`s" | tables | unit `the_tables_are_read_again_only_when_a_generation_lands` | no |
| 108 | basis_route.rs:324 | `answer` | "O(1) with `day`; O(the share's runs and actions) without" | runs | unit + R/basis | no (stated) |
| 109 | feed_checks.rs:49-57 | `/feed-checks.json`, `/tick-checks.json`, `/calendar-days.json` | "one `stat` of the folder, one map probe per month ... O(limit) rows" | months, shares | R/feed_checks 6 targets (gated); unit `a_held_month_answers_in_time_that_does_not_grow_with_its_shares` | maybe: first ask of each (month, version) parses the whole CSV on the request; folder re-listed when its mtime moves (feed_checks_files.rs:200-255) |
| 110 | feed_checks.rs:1005 | `calendar_answer` | "the list of one year is a constant's work, whatever the store holds" | store | R/feed_checks | no |
| 111 | logs.rs:27-31, 51 | `/logs.json` `logs_json` (`SCAN_BYTES` 4 MiB) | "A request cannot make the server read the disk without a bound" | log size (until cap) | R/logs_dirs C-LG-01/C-LG-03 (KEYED requests only, 200 vs 2,000 lines/dir, gated); tests/logs_r3_24.rs; tests/pool_load.rs (1,000 concurrent log reads) | yes: an UNKEYED request whose filter admits fewer than `limit` lines (`level=`, `run=` without `durable=1`) falls back from the live copy to `telemetry::tail` over the files of EVERY directory, each to its scan cap (logs.rs:1318-1376, 510-516): up to ~8 dirs x 4 MiB decoded per request |
| 112 | logs.rs:1363 | `live_tail` | "a constant of this build, whatever the ring holds. Only the fallback reads the files" | ring size | R/logs_dirs live tail (printed only) | yes: the fallback (above) |
| 113 | logs.rs:4560 | `index_rows` keyed answer | "the same request costs the same at any size" | audit stream size | R/logs_dirs C-LG-01/03 (10x only, gated) | maybe: span-alone reads the audit files the span meets (`span_page`, logs.rs:3991) |
| 114 | logs.rs:4990 | `keyed_json_across` (`MAX_KEYED_READS`) | "whatever the streams hold" | streams | R/logs_dirs | no (capped) |
| 115 | logs.rs:3769, 3866 | `line_after`, `end_of_span` | "O(log(file size)) probes whatever" | file size | none | no (log, stated) |
| 116 | logs.rs:4379 | `place_in_span` | "p50 26.7 ms and p99 41.1 ms" | — | measured once | n/a |
| 117 | logs.rs:1520 | `contradicted` | "O(log days) each, at most the page's limit" | days | none | no |
| 118 | logs.rs:2229 | `sink_health` | "O(1), and it stays O(1) as the log" grows | log | none | no |
| 119 | logs.rs:3100 | `Ration` | "O(1) per request: three" counters | requests | none | no |
| 120 | logs.rs:5431, 5901 | `days` | "O(log n + days in the range)" | days | none | no |
| 121 | logs.rs:6050-6068 | `import_refusals` | ring "once per api process, then only the bytes appended since" | ring | R/logs_dirs evidence tails (gated) | maybe: per-call `DirState::of` metadata x 5 dirs; first read of a 64 MiB ring on request |
| 122 | logs.rs:6132 | `refusals_in` | "At most `slice` bytes read, whatever the ring holds" | ring | R/logs_dirs | no |
| 123 | logs.rs:2607 (+ speed.rs:16-20) | `note_request` middleware (every route) | "one read-locked hash probe for its row: O(1), and measured" | requests, routes | R `the_speed_table_costs_under_a_microsecond_a_request` (p99 <= 1 us gated, n=1e6) | no (row keyed by MatchedPath; `ROUTE_CAP` bounded) |
| 124 | import_journal.rs:37-46 | `import_journal::snapshot` (`/imports.json`, `/audit.json`, `/dayflags.json`, `/provenance.json`, `/gaps.json`, census headers) | "a tail, never a re-read ... O(log n + days in the range)" | journal length | R/logs_dirs C-LG-02 runs page 1k/10k/100k (gated, journal never appended between calls) | yes: `Arc::make_mut(&mut tail.parsed)` (import_journal.rs:754) deep-clones the WHOLE parsed journal under the global `tails()` mutex on every append, because long-lived holders keep the previous Arc (imports.rs:395-430 `KEPT`, store_route.rs:449-456 `behind`) — the code says so itself (imports.rs:410-412) |
| 125 | import_journal.rs:301 | `Parsed::range` | "O(log n + rows in range)" | journal | none | no |
| 126 | import_journal.rs:336 | `Parsed::runs_of` | "hash probe and the id's own lines, whatever the journal holds" | journal | R/logs_dirs `run=<id>` (gated) | no |
| 127 | import_journal.rs:841 | end-line fold | "AN END LINE IS FOLDED IN ONE PROBE, WHATEVER THE JOURNAL HOLDS" | journal | unit (prints p50/p99) | no |
| 128 | imports.rs:1451 | `/imports.json` `runs_page` | "page are decoded, whatever the journal holds" | journal | R/logs_dirs C-LG-02 | maybe: see 124 |
| 129 | imports.rs:383-392 | `imports::summary` (`/audit.json?feed=gdfl`) | "two tail reads that find nothing new, a lock and a clone" | journal | none | yes: keeps `Arc<Parsed>` => forces 124's clone on every append; kept `Value` cloned per request |
| 130 | ingest_refusals.rs:43 | `/ingest/refusals.json` `refusals_json` | "per (symbol, month), whatever the journal holds" | journal | none | maybe (journal tails, as 124) |
| 131 | audit_json.rs:406 | `/audit.json` `journal_block` last runs | "one positional read each: O(1), whatever follows them" | journal | none | no for last_runs |
| 132 | audit_json.rs:712 | `attribution` | "O(runs) ... whatever the journal's length" | journal records | none | yes: the cached `Attribution` holding every run is `.cloned()` on every request (audit_json.rs:722-732): O(runs) per request, growing with every pull run |
| 133 | audit.rs:283 | `Journal::append` | "O(1) and the file scannable at `ordinal * RECORD_LEN`" | journal | unit `append_synced n=400` p99 < 250 ms (audit.rs:5360-5388) | no |
| 134 | audit.rs:1397 | `APPEND_LOCK_WAIT` | "uncontended p99 3.7 to 12.5 ms" | — | audit_lock_wait_measure.rs (AG06 prints) | n/a |
| 135 | audit.rs:1943 | `Log::page` | "`take * RECORD_LEN` bytes however long the file is" | journal | none | no |
| 136 | audit.rs:2636 | `vendor_before` | "O(log runs)" | runs | none | no |
| 137 | audit.rs:3088 | `records_at` | "one positional read per record, whatever the journal's length" | journal | none | no |
| 138 | backtest.rs:49-55, 755; lib.rs:12 | `/backtest.json` `backtest_json` -> `respond` | "count O(1) ... read record i O(1) ... one request O(limit)" | ledger length | none (UNVERIFIED) | maybe: default `limit` 500 = "the whole file" in practice: O(min(ledger, 500)) per request |
| 139 | trades.rs:19-37 | `/trades.json` `trades_json` | "Only each in-memory identity probe is O(1)" | ledger, children | none | maybe (stated): cold index O(history), warm O(new records) |
| 140 | frontierjson.rs:65 | `/frontier.json` | "These refreshes and selected-row reads are not O(1) overall" | — | none | stated |
| 141 | indexstoprankingjson.rs:186 | `/index-stop-ranking.json` `render` | "NOT CONSTANT-TIME WHEN UNPINNED, WARM OR COLD" | — | none | stated |
| 142 | selectionv6json.rs:47 | `/selection-v6.json` (`PAGE_BLOCKS`) | "bounded whatever the files hold" | files | none | maybe |
| 143 | speed.rs:57, speed_route.rs:20-27 | `/speed.json` `speed_json` | "O(rows x buckets) ... does not grow with the requests served or the store" | requests, store | none (R3-14 report) | no (<= 256 x 272, x3) |
| 144 | speed.rs:1177, 1238 | `AccessWindows::account`/`cut` | "O(1) a read: one probe of a table of at most DB_READS" | routes | none | no |
| 145 | detail.rs:804-813 | `/pools.json` + every route `tally` | "O(log R) comparisons ... a constant per request" | routes | tests/pool_load.rs (p99 at 16/64/256 clients) | no (global `Mutex<BTreeMap>` contention only) |
| 146 | detail.rs:299 | pool permit `take` | "O(1) per wake-up" | waiters | none | no |
| 147 | detail.rs:1337 | `seek_window` | "O(1) arithmetic" | rows | unit `a_seeked_window_has_no_hold_cap_and_keeps_every_other_refusal` | no |
| 148 | detail.rs:1528, 1567, 1579 | `Cached` handle slot | "O(1), never" waits on a handle | — | unit `the_slot_lock_is_free_while_a_handle_opens_or_refreshes` | no |
| 149 | detail.rs:1058 | `being_written_answer` | "BOUNDED BY CONSTRUCTION, WHATEVER THE SENTENCES HOLD" | faults | none | no |
| 150 | page_report_route.rs:26 | `POST /page-report` | "O(1) a report — one parse of at most 1 KiB" | — | none | no |
| 151 | server.rs:16502 | `form_read_bound` | "O(1): five comparisons against literal paths" | — | none (UNVERIFIED) | no |
| 152 | server.rs:16598 | `repeated_query_key` | set "sized from the segment count the target cap already bounds" | query | none | no (SipHash; <= 8/60 KiB) |
| 153 | server.rs:17753 | `DeadlineBody` | "Each poll is O(1)" | body | none | no |
| 154 | server.rs:18050 | `exchange_watch` | "O(1) a request: one atomic add each way" | requests | none | no |
| 155 | server.rs:18121 | `HeadDeadline` | "O(1) per operation ... head scan is O(1) per byte read" | bytes | unit `a_progressing_write_touches_no_timer_and_a_stall_resets_it_once` | no |
| 156 | server.rs:2161 | `shared_governor` | "One lock, one index, one `Arc` clone. O(1)" | — | none | no |
| 157 | server.rs:2616 | `newest_record` | "One `metadata` call and one 256-byte read, whatever the file holds" | journal | none | no |
| 158 | server.rs:1316 | `timeframe_param` | "One comparison per known rung — two — so this is constant work" | — | none | no |
| 159 | server.rs:1493 | `Addressed::store_path_is_valid` | "Constant work: four" checks | — | none | no |
| 160 | server.rs:36489-36491 | `universe_tokens` (`/instruments.json`, `/universes.json` rows) | "O(1) per row by construction and UNVERIFIED as a measurement" | instruments | none | no |
| 161 | ladder.rs:48-61, 775 | `ladder::probing` | "The O(1) per probe is UNVERIFIED" (HashMap::get) | census | none | no (SipHash) |
| 162 | merge.rs:185, 212 | `Merged::native_id`, `native_lookup` | "One expected O(1) hash lookup" | universe | none | no (load-time maps, SipHash) |
| 163 | merge.rs:393-492 | `merge::merge` (load time) | "Probes have expected O(1) cost ... BTreeMap O(log n)" | universe | none | no (load-time; honest) |
| 164 | constituents.rs:18, 101, 577 | `constituents::Join` | "both questions are O(1) ... flat array" | tiers | unit | no |
| 165 | coverage.rs:56 | `Coverage::of` | "a single index into a flat" array | — | none | no |
| 166 | nse_identity.rs:39 | `Reference::confirm` | "one expected-O(1) hash probe" | — | none | no |
| 167 | credential_law.rs:33, 284 | credential fingerprint | "O(1) per instrument or cell: one fingerprint comparison" | — | none | no |
| 168 | ingest.rs:245 | `Target::names` | "TWO CONSTANT-TIME TESTS ... costs the same at 800 as at one" | target size | none | no |
| 169 | ingest.rs:1149 | `FnoRequest` lookup | "A hash probe is O(1)" | — | none | no |
| 170 | autopilot.rs:1185, 1426, 2187, 2807 | `/autopilot.json` state (`Place`, `enter`, `Control`, `refuse_drifted`) | "O(1) in the store's size" | store | none | no |
| 171 | pullrun.rs:40 | `POST /pull/run` legs | "O(1) per leg beyond the leg's own work: ... linear scan" | groups | none | maybe (linear scan of a small group list) |
| 172 | pullrun.rs:1494, 2049, 2793 | `/pull/run.json` `interrupted_in`, `restart_scan`, `older_runs` | "bounded whatever the journal's length" | journal | none | no |
| 173 | pullrun.rs:3202 | `rows_now` | "NOT O(1) on a miss. On a cache hit, five `stat`s" | census | none | maybe (stated) |
| 174 | sweeprun.rs:2493, 3272-3301 | `/backtest/run.json` `run_json` (`CliLogMemory`) | "p50 of about 21 ms, p99 63 ms and a cold 548 ms" (before) | cli log | none | maybe: remembered only "while it is unchanged" — during a sweep the log changes constantly |
| 175 | sweeprun.rs:1926 | `admit` | "each time for O(1) work" | — | none | no |
| 176 | sweeprun.rs:512 | `KNOBS` | "rule 4's constant cost, with fourteen as the bound" | — | none | no |
| 177 | fault_log.rs:43, 171 | `FaultLog` | "O(1) per refused record" | month rows | none | no |
| 178 | recovery.rs:8; recovery_journal.rs:41-44; recovery_control.rs:12 | `/pull/recovery*` | "indexed state costs expected O(1) per update" (collisions not claimed) | — | none | no (honest) |
| 179 | server.rs:9060 | `credentialed_source` | "O(1). One file read, one identity discovery" | — | none | no |
| 180 | server.rs:9772 | `recorded_with_failures` | "append O(1)" | journal | none | no |
| 181 | server.rs:10945, 11070 | `POST /pull/fno` `fno_land` | "one hash probe per contract-month. O(1) per" | contracts | none | no |
| 182 | server.rs:11405 | `SpotBooks` | "O(bars) on the first ask for a month, O(1) on every ask after" | bars | none (UNVERIFIED) | no (per-request map) |
| 183 | server.rs:11427, 11533-11535 | `chain_quotes`, `price_chain_month` | "then O(1) per row" | rows | none | no |
| 184 | server.rs:11654, 12509, 12590, 12673, 12752, 12856 | `/pull/fno` reporting helpers | "O(1) per request" / "O(1) per option row" | rows | none | no |
| 185 | server.rs:13325, 13453, 13660, 13992, 14117, 14832 | `/pull/fno` rolling helpers | "O(1) per request built and per row read" | — | none | no |
| 186 | server.rs:5603, 5673 | `spot_targets`, `unresolved_member` | "costs the same against 750 names as against one" | names | none | no |
| 187 | server.rs:7621, 7830, 8195, 8330 | pull chunk helpers | "an event O(1) in space" / "constant work" / "Not O(1)" | — | none | no |
| 188 | render.rs:537 | `vendor_cell` | "a renderer is not where a cost" is claimed | — | none | no |

### Routes and their benches (101 registered + fallback)

`route_table` (server.rs:16692-17141) registers 100 paths; `audited_router_serving` adds `/inspection.json`
(server.rs:16148). Benched = named in a bench `ROUTES` list or timed by call (`.github/gates_routes.rs`
`TIMED_BY_CALL`), or timed by `R/months`.

Benched (29 + 2 months-only): `/dashboard`, `/instruments`, `/instruments.json`(304), `/feeds.json`,
`/settings.json`, `/calendar.json`, `/bars.json`, `/gaps.json`, `/bars/window.json`, `/bars/export.csv`,
`/spot.json`, `/greeks.json`, `/census/facets.json`, `/census/rollup.json`, `/chain.json`, `/imports.json`,
`/basis.json`, `/scrub.json`, `/dayflags.json`, `/feed-checks.json`, `/tick-checks.json`, `/calendar-days.json`,
`/aliases.json`, `/sources.json`, `/sources/bars.json`, `/sources/day.json`, `/provenance.json`,
`/store.json`(304 + census_delta), `/logs.json` (keyed only); plus `/store` (`store_html`, R/months, one ratio
gated over census size) and `/verify.json` (`api::verify::vendor`, R/months, printed NOT gated).

NO bench at all (70): writes `POST /settings`, `/pull/spot`, `/pull/repair`, `/pull/quarantine`, `/pull/fno`,
`/pull/run`, `/pull/recovery`(POST), `/pull/run/stop`, `/universe/resolve`, `/autopilot/pause`,
`/autopilot/resume`, `/autopilot/control`, `/ingest/queue`, `/backtest/run`, `/backtest/descend`,
`/engine/command`, `/masters/refresh`, `/page-report`; reads `/universes.json`, `/vocab.json`, `/indexmap.json`,
`/folder.json`, `/census/warm.json`, `/disk.json`, `/ingest/refusals.json`, `/pools.json`, `/masters.js`,
`/typeahead.js`, `/pull`, `/pull/repair.json`, `/pull/quarantine.json`, `/pull/recovery`(GET),
`/pull/recovery.json`, `/pull/run.json`, `/autopilot.json`, `/ingest/status.json`, `/audit.json`, `/audit/page`,
`/bars`, `/logs`, `/backtest.json`, `/trades.json`, `/frontier.json`, `/sweep-evidence.json`,
`/candidate-trades.json`, `/expression-search.json`, `/boolean-candidates.json`, `/boolean-statistics.json`,
`/boolean-admission.json`, `/boolean-qualification.json`, `/boolean-campaign.json`,
`/boolean-qualified-campaign.json`, `/boolean-qualified-search.json`, `/boolean-oos.json`, `/selection-v6.json`,
`/live.json`, `/engine/boolean-launch.json`, `/index-stop.json`, `/index-stop-candles.json`,
`/engine/index-stop-launch.json`, `/index-stop-qualification.json`, `/index-stop-ranking.json`,
`/index-stop-vix.json`, `/engine/top.json`, `/masters`, `/masters/status.json`, `/backtest/run.json`,
`/backtest/audit.json`, `/health`, `/speed.json`, `/inspection.json`, and the front-end fallback.
(Every one is listed as owed in `.github/gates_routes.rs` `BENCHES_OWED`. `/disk.json` has only a unit
test, `the_call_at_its_p99_costs_less_than_the_df_child_at_its_p50`; `/pools.json`, `/feeds.json`, `/bars*`,
`/logs.json` are also load-tested in `tests/pool_load.rs`.)

### Bench fixture blind spots (why the flat rows pass)

* Served stores (`R/served.rs` `stores()`, `store_of(bars, census)`): census and month are written ONCE, before
  timing; nothing is appended during timing; the import journal, `gdfl.dayflags`, `zerodha.gaps`, lineage
  ledger and pull journal are ABSENT (`R/served.rs:31-35`). So every "kept/tail/cached" path is timed warm.
* Window/months benches use <= 2 series and indexed months only (`month_file` refuses an un-indexed month).
* `R/greeks.rs` leaves the rate table unset, so no IV solve is ever timed.
* `R/sources.rs` times one cached day key and offset 0 / limit 10.
* `R/logs_dirs.rs` times keyed searches and the live tail without a starving filter, at 200 vs 2,000 lines/dir.
* Every ratio gate is p50 (served family) or min-of-trials; p99 is printed, not gated (except gaps,
  speed table, window tod new/kept).

## `crates/pull` — inventory of cost-bound claims (O(1) / constant / flat / p99 / bound)

Branch `cloud/o1p99-attack-20261110`, read-only survey (no cargo/rustc run). Paths are repo-relative.
One row per (function, claim); several comment lines about one function are collapsed onto the first line.
Pure disclaimers ("this is NOT O(1)") are kept only where they sit beside a live claim.

Conventions
* **bench** = row of gate 8, `crates/pull/benches/ratio.rs` (+ `ratio/*.rs`), ceiling 3.0x, `harness=false`, `test=false`.
  Many rows take the MINIMUM over trials (`cost_ps`), so a periodic O(n) spike is invisible unless a `spread()` p99/max line is read (printed, never gated).
* **test** = a unit/integration test that times or counts; `[ign]` = `#[ignore]` (never runs in CI), `[assert]` = asserts a bound.
* Hashing: every map in `pull` is `std` `HashMap`/`HashSet` with `RandomState` (SipHash-1-3, per-process random keys); no FNV/Fx map anywhere in the crate
  (`brutex_core::universe::fnv1a` is only used to derive a 32-bit `symbol_id`). So adversarial *collision* attacks are not available; the
  realistic hash attacks are (a) key length (SipHash is O(bytes)), (b) un-presized maps rehashing at `7*2^k`, (c) `hashbrown` reserving one slot
  BEFORE probing, so even an update of an existing key rehashes a table with `growth_left == 0` (manifest.rs:3910-3916 documents this).
  `serde_json` is built without `preserve_order` (root Cargo.toml:117), so a JSON object is a `BTreeMap` (O(log keys) `get`).

| # | where | what | claim (short quote) | flat in | bench / test | suspect |
|---|---|---|---|---|---|---|
| 1 | crates/pull/src/alerts.rs:11 | `alerts::put_at` / `put_with` | "one positional read of the newest WINDOW records ... whatever the file's length" | records in `audit/alerts` | C-AL-01 `alert_put_is_flat` (1,024/10,240/102,400 records) | maybe: `fit_text(alert.text)` (alerts.rs:466) cleans the WHOLE caller text inside `put_with` under the exclusive lock, O(text bytes); `key_hash` is BLAKE3 over the whole key (outside lock); window bounded 1,024 |
| 2 | crates/pull/src/alerts.rs:14 | `alerts::window` / `page` | "one read of at most WINDOW records under the lock held shared" | records | C-AL-02 `alert_window_is_flat`; spread of `page(Some(50_000))` printed | no |
| 3 | crates/pull/src/alerts.rs:1018 | `alerts::Scope::Window` (ack) | "The newest WINDOW records only: O(1)" | records | C-AL-01 spread only (ack printed, not gated) | no (`Scope::Whole` is O(records), stated) |
| 4 | crates/pull/src/alerts.rs:1351 | `alerts::page` | "O(1) at any `before`" | records, `before` | C-AL-02 spread; test `the_operators_older_steps_timed` [ign] | no |
| 5 | crates/pull/src/audit_ledger.rs:34 | `ingest::audit_ledger` append | "O(1) per append in the ledger's size" | ledger lines | test `appends_cost_the_same_at_any_ledger_length` (C-GR-15) [assert p50<=4x+2ms] | no (one write is O(batch)) |
| 6 | crates/pull/src/calendar.rs:137 | `calendar` module (kind_of family) | "One subtraction, one array index and one bit test — O(1)" | days in calendar | none (UNVERIFIED) | no: plus two compile-time walks (5+4 entries) |
| 7 | crates/pull/src/calendar.rs:643 | `calendar::kind_of` / `measured_kind` | "at most nine comparisons ... O(1), no hash, no allocation" | days | none | no |
| 8 | crates/pull/src/calendar.rs:1183 | `calendar::sourced_kind` | "Two bounded walks over compile-time tables (3 and 38 entries)" | days | none | maybe: linear in `SOURCED_HOLIDAYS` (append-only, ~15 rows/yr); compile-time bound only |
| 9 | crates/pull/src/calendar.rs:1516 | `calendar::exchange_kind_of` | "O(1), no allocation ... p50 2 ns, p99 13 ns" | days | test `record_tests::calendar_of_record_costs` [ign][assert p99<50us] | no (one walk of SPECIAL_SESSIONS) |
| 10 | crates/pull/src/calendar.rs:1763 | `calendar::short_day_on` | "A walk of the two-entry SHORT_DAYS: O(1)" | days | same [ign] | no |
| 11 | crates/pull/src/calendar.rs:1775 | `calendar::halt_on` | "A walk of the three-entry HALTS: O(1)" | days | same [ign] | no |
| 12 | crates/pull/src/calendar.rs:1866 | `calendar::phase_at` | "hours lookup: O(1), no allocation" | days, minute | same [ign] | no |
| 13 | crates/pull/src/calendar.rs:1937 | `calendar::phase_at_second` | "O(1): one walk of the three-entry HALTS" | days | none | no |
| 14 | crates/pull/src/calendar.rs:2119 | `calendar::phases_of` | "at most sixteen cuts: O(1), no allocation, no sort" | days | same [ign] | no |
| 15 | crates/pull/src/calendar.rs:2320 | `calendar::holiday_note` | "A walk of the 38-entry table: O(1)" | days | same [ign] | maybe: linear in an append-only table (compile-time) |
| 16 | crates/pull/src/calendar.rs:2426 | `calendar::expiry_day` | "O(1): timed inside record_of" | days | same [ign] | no |
| 17 | crates/pull/src/calendar.rs:2529 | `calendar::settlement_on_or_before` | "At most MAX_SETTLEMENT_WALK + 1 calls of expiry_day: O(1)" | days | same [ign] | no (<=8 calls) |
| 18 | crates/pull/src/calendar.rs:2586 | `calendar::shift_reason` | "At most seven calls of expiry_day: O(1)" | days | none | no |
| 19 | crates/pull/src/calendar.rs:2640 | `calendar::record_of` | "O(1): the bounded walks above, once each" | days | same [ign] | no |
| 20 | crates/pull/src/calendar.rs:3327 | `calendar::DayFacts::at` | "four bounded walks of compile-time tables ... O(1)" | days | test `tradable_tests::tradable_day_and_every_bucket_are_bounded` [assert p99<50us] | no |
| 21 | crates/pull/src/calendar.rs:3398 | `calendar::tradable_day` | "bounded walks of compile-time tables, O(1), no allocation" | days | same [assert] | no |
| 22 | crates/pull/src/calendar.rs:3508 | `calendar::RunningSums::between` | "two lookups and a subtraction, O(1), whatever its length" | bucket length | same [assert] | no |
| 23 | crates/pull/src/calendar.rs:3606 | `calendar::excluded_between` | "O(1) after the first call builds RunningSums::newest" | bucket length | same (measured AFTER build) | maybe: first call per process builds sums, O(days of span) x tradable_day (cold-call p99/max) |
| 24 | crates/pull/src/calendar.rs:3670 | `calendar::year_exclusions` | "one RunningSums::between, O(1) after the sums are built" | years | same | maybe: same cold first call as #23 |
| 25 | crates/pull/src/calendar.rs:3731 | `calendar::year_record` | "at most the notice count squared ... the bound does not grow" | years | same [assert p99<100us] | maybe: QUADRATIC in `SESSION_CIRCULARS` (append-only; compile-time) |
| 26 | crates/pull/src/calendar.rs:4705 | `calendar::Calendar::kind_of` (5114) | "O(days) to build, once, and O(1) to query" | observed span | none | no |
| 27 | crates/pull/src/calendar.rs:4762 | `calendar::Runtime::kind_of` | "O(1), no allocation" | days | none | no |
| 28 | crates/pull/src/calendar.rs:5191 | `calendar::Calendar::sessions` | "O(1): the count is taken once" | span | test `sessions_is_counted_once_at_construction_not_per_call` | no |
| 29 | crates/pull/src/capture.rs:100 | `capture::NAME_ATTEMPTS` (record) | "collision handling O(1) even when a hostile directory pre-creates every candidate" | colliding names | none | no (<=16 `create_new`) |
| 30 | crates/pull/src/cash_auction.rs:11 | `cash_auction::DailyEligibility::{eligibility,lifecycle}` | "lookup uses a HashMap (expected O(1), plus key-byte cost)" | EQ identities | none | no (keys capped at 256 B by `identity_word`) |
| 31 | crates/pull/src/cash_session_cache.rs:78 | `VerifiedLifecycleMaster::eligibility` / `cas_eligibility` | "Expected O(1) plus bounded key bytes" | master rows | none | no |
| 32 | crates/pull/src/cash_session_cache.rs:130 | `VerifiedLifecycleMaster::inspect` | "Expected O(1) plus bounded key bytes/three-field assessment" | master rows | via C-ZI-01 | no |
| 33 | crates/pull/src/cash_session_cache.rs:519 | `cash_session_cache::hold` | "Expected O(1): one attempt when nobody holds the other side" | lock contention | test CSL-03 lock_tests | no (time-bounded wait) |
| 34 | crates/pull/src/config.rs:122 | `config` reader | "Reading is a bounded scan of a bounded file" | config size | none | no (all caps enforced) |
| 35 | crates/pull/src/corporate_actions.rs:83 | `corporate_actions` lookups (module) | "one hash of the symbol and one or two indexes ... O(1), no search" | rows per share, shares | tests below | maybe: SipHash of caller `symbol` (no cap); `excuse` walks the `labels` slice, O(rows in a caller-chosen window) |
| 36 | crates/pull/src/corporate_actions.rs:173 | `corporate_actions::session_between` | "At most MAX_WINDOW_WALK calendar lookups: O(1)" | window length | test `a_session_between_two_days_is_found_within_a_bounded_walk` | no |
| 37 | crates/pull/src/corporate_actions.rs:502 | `Action::day_known` | "O(1): one session_between" | days | same | no |
| 38 | crates/pull/src/corporate_actions.rs:888 | `CorporateActions::labels` | "One hash and two indexes ... O(1), no search, whatever the share holds" | rows per share | test `an_action_lookup_is_one_index_at_1_64_and_4096_rows_measured` [assert median<=4x] | no |
| 39 | crates/pull/src/corporate_actions.rs:1208 | `BasisRuns::run_on` | "One hash and one index into the share's day table: O(1)" | shares | test `a_lookup_costs_the_same_at_1x_10x_and_100x_the_shares_measured` [assert median<=4x] | no |
| 40 | crates/pull/src/corporate_actions.rs:1713 | `corporate_actions::crossed_back` | "at most the runs starting within REVERSAL_DAYS ... O(1) for one step" | runs per share | none (UNVERIFIED) | maybe: O(runs within +-90 days) per step, <=~181 |
| 41 | crates/pull/src/daycheck.rs:128 | `daycheck` `tally` | "O(1): a fixed number of compares" | days | none | no |
| 42 | crates/pull/src/disk_projection.rs:43 | `disk_projection::project` / module | "every projected unit ... one hash probe or one cached median, O(1)" | kinds, underlyings held | C-DP-01 `disk_projection_is_flat` (100/1k/10k underlyings) | no (per-unit name String clone) |
| 43 | crates/pull/src/disk_projection.rs:205 | `Totals::add_kind` | "O(1): one hash insert" | kinds measured | none (UNVERIFIED L-R407-11) | yes: `Totals` derives `Default` (disk_projection.rs:191-195), `kinds` never reserved; the insert at 7*2^k rehashes all n; plus `contains_key` then `insert` = 2 probes |
| 44 | crates/pull/src/disk_projection.rs:227 | `Totals::add_underlying` | "O(1): one hash insert" | underlyings measured | none (C-DP-01 builds through it, untimed) | yes: `underlyings: HashMap<String,..>` from `Default`, O(n) rehash at 7*2^k; SipHash + `to_owned` O(name bytes), name uncapped |
| 45 | crates/pull/src/disk_projection.rs:320 | `Sealed::per_day` | "O(1), timed through project by C-DP-01" | underlyings | C-DP-01 | no |
| 46 | crates/pull/src/disk_projection.rs:643 | `disk_projection::floor_check` | "O(1): integer arithmetic" | — | none | no |
| 47 | crates/pull/src/disk_projection.rs:689 | `disk_projection::check_plan` | "O(1) beyond the measurement: one floor_check" | plan units | none | no (clones `unmeasured` on refusal) |
| 48 | crates/pull/src/expiry_alias.rs:139 | `expiry_alias::settle` (module) | "O(1): one bounded walk over ALIASES" | ALIASES rows, days | test `the_decoders_costs_on_real_days_are_measured` [ign] | no |
| 49 | crates/pull/src/expiry_alias.rs:744 | `expiry_alias::alias_of` | "O(1): two walks bounded by ALIAS_ROWS" | ALIASES rows | none | no |
| 50 | crates/pull/src/fetch.rs:509 | `RawWindow` row vector | "the append stays worst-case constant" | rows | none | no (reserved from n <= MAX_ROWS) |
| 51 | crates/pull/src/fetch.rs:809 | `fetch::Refused::note` | "so this is O(1) per row" | rows refused | none | no (back-walk bounded by causes/day, rows in time order) |
| 52 | crates/pull/src/fetch.rs:867 | `fetch::note_refused` | "a walk of at most ... five, so O(1) per row" | refused days | none | no (only length-unmeasured days reach it) |
| 53 | crates/pull/src/fno_membership.rs:84 | `fno_membership::Membership` (members/pool_size/row/refused_through) | "one subtraction into a fixed-stride day index and one slice or row read" | held days | C-PIT-01 `fno_membership_lookup_is_flat` (1/10/100 months) | no |
| 54 | crates/pull/src/fno_membership.rs:2484 | `Membership::slot` | "Constant work: one subtraction and one bounds check" | held days | C-PIT-01 | no |
| 55 | crates/pull/src/fno_membership.rs:2515 | `Membership::row` | "O(1) (C-PIT-01)" | held days | C-PIT-01 | no |
| 56 | crates/pull/src/fno_membership.rs:2525 | `Membership::members` | "O(1): one subtraction into the fixed-stride day index, one slice" | held days | C-PIT-01 | no |
| 57 | crates/pull/src/fno_membership.rs:2539 | `Membership::pool_size` | "O(1): one subtraction, one row" | held days | C-PIT-01 | no |
| 58 | crates/pull/src/fno_membership.rs:2551 | `Membership::series_of` | "O(1): a bounded alias walk and one hash probe" | series, alias chain | C-PIT-01 via is_member (probe `ZZQAB` never in LINEAGE) | maybe: `LINEAGE.current` + `option_underlying` walk core tables (compile-time); bench never exercises a renamed share |
| 59 | crates/pull/src/fno_membership.rs:2578 | `Membership::is_member` | "O(1): one alias walk, one probe, one bit" | held days, series | C-PIT-01 | maybe: as #58 |
| 60 | crates/pull/src/fno_membership.rs:2598 | `Membership::refused_through` | "the running sum, O(1)" | held days | none direct | no |
| 61 | crates/pull/src/fnowork.rs:54 | `fnowork::gaps` | "O(cells), one hash probe each" | store size | none | maybe: generic `S: BuildHasher` — a caller may hand a weak hasher |
| 62 | crates/pull/src/fnowork.rs:373 | `fnowork::owed` | "held ... must be O(1) — a hash probe, not a search" | store size | none | no (contract on caller) |
| 63 | crates/pull/src/fnowork.rs:495 | `fnowork::gaps_by` | "held is called once per cell and must itself be O(1)" | store size | none | no |
| 64 | crates/pull/src/fnowork.rs:723 | `fnowork::futures_plan` | "no older snapshot is opened here: the cost does not grow with the snapshots held" | snapshots held | C-MS-02 `futures_plan_is_flat` (1/10/100 weekly snapshots); test `fnowork_futures_tests` MEASURED [ign] | **yes**: per call `first_listed::list_in` reads the WHOLE `absorbed.idx` (one line per snapshot ever taken, `read_log` first_listed.rs:482 via `read_lines`), walks `needed` (every stamp through `through`) twice (:949, :956), and the timed `Book::open` does `Catalog::read` = `read_dir` + sorts of the snapshot dir (master_snapshots.rs:411-528) — all O(snapshots) per plan |
| 65 | crates/pull/src/fold.rs:252 | `fold::fold_from_snapshots` | "O(1) arithmetic on two values already in hand ... amortised O(1) per push" | snapshots | C-FD-01 `fold_is_flat_per_bar` | no |
| 66 | crates/pull/src/gap_backfill.rs:95 | `ingest::gap_backfill::Have::holds` | "O(1) expected: one hash probe" | ledger lines | tests/zerodha_gaps.rs `a_ledger_probe_is_flat_in_the_ledgers_lines` [assert median<=3x] | maybe: `key.to_owned()` allocates per probe (O(key bytes)+malloc) |
| 67 | crates/pull/src/gaps.rs:57 | `gaps::classify` family | "A two-cursor merge, O(1) per minute and per bar" | minutes, bars | none | no |
| 68 | crates/pull/src/gaps.rs:250 | `gaps::Requested::holds` | "O(1) a probe, whatever the span" | ledger span | crates/api/benches/ratio.rs gap rows | no |
| 69 | crates/pull/src/gaps.rs:417 | `gaps::not_owed` | "O(1) and no allocation, no file, no lock" | months | crates/api/benches gap rows | no |
| 70 | crates/pull/src/gaps.rs:555 | `gaps::Ledger::not_owed` | "O(1) per run. Each run finds its answer's row by one hash probe" | distinct answers | test `not_owed_costs_the_same_with_one_answer_and_with_max_gaps_answers` | maybe: map `with_capacity(4)` grows by rehash to MAX_GAPS=4,096 answers (amortised) |
| 71 | crates/pull/src/gaps.rs:638 | `gaps::classify` | "One step per expected minute and one per stored bar, each O(1)" | bars | none | no |
| 72 | crates/pull/src/gaps.rs:791 | `gaps::classify_owed` | "two-cursor merge, plus one not_owed per session: O(1)" | bars | none | no |
| 73 | crates/pull/src/gaps.rs:1059 | `gaps::provable_holes` | "O(1) per minute per peer" | minutes | none | maybe: linear in `peers` per minute (stated) |
| 74 | crates/pull/src/gdfl_archive.rs:206 | `gdfl_archive::zip64_record_at` | "Two reads, O(1)" | archive size | none | no |
| 75 | crates/pull/src/gdfl_archive.rs:226 | `gdfl_archive::unprefixed` | "One 4-byte read, O(1)" | archive size | none | no |
| 76 | crates/pull/benches/ratio/day_table.rs:1 | `gdfl_archive::Archive` as `CmSource::day` (gdfl_archive.rs:770) | "one index into its dense day table ... flat at 1x, 10x and 100x the days" | days in outer archive | C-GI-04 `day_lookup_is_flat` (100/1k/10k index days of 2 entries) | maybe: every call re-walks that day zip's central directory and `push`es every entry (O(entries), `entries` Vec unreserved); for a STOCKS day >= 2026-08-03 with masters, `DayListing::scoped` -> `Cas::of` -> `read_local_lifecycle` gunzips+parses the whole NSE security master on EVERY call (gdfl_cm.rs:2006-2019) |
| 77 | crates/pull/src/gdfl_cm.rs:71 | `gdfl_cm::resolve` / `stem_slot` | "worst-case O(1) in its argument" | stem length | none | no (length cap before scan) |
| 78 | crates/pull/src/gdfl_cm.rs:373 | `gdfl_cm::RowWidths::note` | "O(1) per row" | rows | none | no |
| 79 | crates/pull/src/gdfl_cm.rs:1028 | `gdfl_cm::ticker_slot` | "worst-case O(1): two bounded probes of FNO_INDEX" | universe | core test bounds probe at 12 | no |
| 80 | crates/pull/src/gdfl_cm.rs:1126 | `CmScope::filing` / `extra_slot` | "one probe of a hash map ... (expected O(1), plus the key's bytes)" | stored shares | none | no (length capped) |
| 81 | crates/pull/src/gdfl_cm.rs:1474 | `CmScope::series_refusal` | "O(1): the name's length is refused ... before it is read" | name length | test gdfl_series_code_tests:208 | no |
| 82 | crates/pull/src/gdfl_cm.rs:1520 | `CmScope::member_of` | "one resolve, then ... for a share one hash probe" | scope | none | maybe: allocates the member String (`format!`) per call |
| 83 | crates/pull/src/gdfl_cm.rs:1781 | `SlotTable::file` | "O(1) per name beyond the name's own length" | names per day | test `day_folder_lookup_is_one_index_into_a_ticker_slot_array` (shape) | maybe: zip member names are uncapped (<=64 KiB): `rsplit_once` + `Box<str>` copy O(len) |
| 84 | crates/pull/src/gdfl_cm.rs:2023 | `Cas::auction` | "one probe of the master's parsed index (expected O(1))" | master rows | none | no |
| 85 | crates/pull/src/gdfl_cm.rs:2117 | `gdfl_cm::DayListing::push` | "O(1) per entry beyond the name's own length" | entries per day | none (bench pushes untimed) | yes: `entries: Vec::new()` in `DayListing::scoped` (gdfl_cm.rs:2074) is never reserved: doubling realloc copies every `ListedFile` at each power of two (worst O(entries)) |
| 86 | crates/pull/src/gdfl_cm.rs:2221 | `DayListing::entry_stem` | "O(1) beyond the name's own length" | entries | none | no |
| 87 | crates/pull/src/gdfl_cm.rs:2232 | `DayListing::locate` | "The file holding stem ...: one slot index" | entries per day | C-GI-02 `gdfl_stock_lookup_is_flat` (1k/10k/100k names, swept scope) | maybe: with a STORED scope (`series` on), `slot_of` -> `series_base` does `rsplit_once` O(stem len) + `format!` alloc, no cap (gdfl_cm.rs:1448-1455); bench only uses the swept scope |
| 88 | crates/pull/src/gdfl_cm.rs:3352 | `gdfl_cm::is_real_date` | "O(1): ten bytes" | — | none | no |
| 89 | crates/pull/src/gdfl_cm.rs:3466 | `gdfl_cm::session_of` | "One exchange_kind_of (O(1))" | days | none | no |
| 90 | crates/pull/src/gdfl_cm.rs:3506 | `gdfl_cm::landed` | "verdicts per window, each O(1)" | windows | none | no |
| 91 | crates/pull/src/gdfl_dayzips.rs:15 | `gdfl_dayzips` day | "a day is one hash probe" (+ O(its entries)) | day zips under root | test gdfl_dayzips_tests:539 [ign] | no (O(entries) stated) |
| 92 | crates/pull/src/gdfl_dropbox.rs:63 | `gdfl_dropbox::status` (1663) | "O(1) in the journal's length ... last 64 KiB" | journal lines | C-DP-02 (1k/10k/100k well-formed lines) | **yes**: if no whole PARSEABLE line ends in the last 64 KiB (lines a newer build wrote, garbage, one >64 KiB line) the window doubles and the tail is re-read up to MAX_STATUS_WINDOW = 4 MiB (gdfl_dropbox.rs:1683-1717): ~8 MiB read+split+parse per call, 64-128x the bench; `lines.remove(0)` is O(lines) |
| 93 | crates/pull/src/gdfl_dropbox.rs:2513 | `Names::verdict` (private) | "O(1): two hash probes and a fixed-width parse" | listed names | none (UNVERIFIED) | maybe: `to_ascii_lowercase`/`to_ascii_uppercase`/`contains` are O(name bytes) and `unsafe_name` caps no length |
| 94 | crates/pull/src/gdfl_format.rs:58 | `gdfl_format::note_format_changed` | "O(1): one lock, one probe" | shapes seen | none | maybe: `HashSet` from `Default` grows by rehash up to 1,024 (amortised) |
| 95 | crates/pull/src/gdfl_import.rs:58 | import skip of a done day | "skips every day done ... without reading the source (O(1) per day)" | journal size | none | no |
| 96 | crates/pull/src/gdfl_import.rs:349 | journal closing line | "the ledger is O(1) per day" | days | none | no |
| 97 | crates/pull/src/gdfl_import.rs:1712 | `gdfl_import::Report::name` | "O(1) amortised: failures grows to twice the bound and then drops" | failures named | test `progress_and_failure_costs` (C-GR-06) [ign], `the_report_drops_names_only_at_twice_the_bound` | maybe: every 1,024th call `drain(..1024)` memmoves + emits; `recent.remove(0)`; `failures_by_cause` is a BTreeMap (O(log causes)); `failure.clone()` O(text) |
| 98 | crates/pull/src/gdfl_import.rs:2566 | `gdfl_import::cm_present` | "one pass over the day's entries, a member name and one hash probe each" | entries | none | no |
| 99 | crates/pull/src/gdfl_import_day.rs:102 | `decisive` | "the pass is O(1) amortised per failure named" | failures | test ia16 ranked | no |
| 100 | crates/pull/src/gdfl_import_day.rs:324 | `Ctx::held` | "built once a run from the scope ledger, then one probe" | scope members | none | maybe: first call per scope builds a HashSet of every member (cold O(members)) |
| 101 | crates/pull/src/gdfl_import_day.rs:369 | `Ctx::owed` | "O(1) for a day that owes nothing (one compare of digests, two hash probes)" | journal days, members | test `what_a_done_day_owes_is_decided_without_walking...` (prints only) | maybe: examined branch walks `member_keys[key]` with `format!("{key}+{member}")` per member (gdfl_import_day.rs:401-415) — O(members of key) per call |
| 102 | crates/pull/src/gdfl_import_day.rs:676 | `KeyState::of` | "a key costs O(1) in the journal's size" | journal lines | none | no |
| 103 | crates/pull/src/gdfl_import_day.rs:771 | `admit_rule` | "O(1)" | — | none | no |
| 104 | crates/pull/src/gdfl_import_day.rs:889 | `restated_past_append` | "O(1) in the journal's size" | journal lines | test `the_auction_era_admission_check_costs_the_same_at_any_journal_size` | no |
| 105 | crates/pull/src/gdfl_import_day.rs:1729 | `reach_back` | "At most 30 days: one hash probe for a day no run closed" | journal | none | no |
| 106 | crates/pull/src/gdfl_import_journal.rs:354 | `Journal::load` per line | "the load stays O(1) per line and O(keys) in memory" | journal lines | none | yes(minor): `member_keys`/`closed_by` are `HashMap<String, BTreeSet<String>>` (O(log members) string compares per insert, :387/:394/:1184); maps not presized (rehash at 7*2^k) |
| 107 | crates/pull/src/gdfl_import_journal.rs:610 | `Journal::history` / `KeyHistory` | "Three hash probes, O(1)" | keys | test ia2 `the_journal_keeps_each_keys_...` | no |
| 108 | crates/pull/src/gdfl_import_journal.rs:771 | `Journal::derive_days` | "O(days x the open keys) and O(1) a day for the rest" | journal | none | maybe: `open.iter().any()` scans every open key per refreshed day (stated) |
| 109 | crates/pull/src/gdfl_import_journal.rs:946 | `Journal::is_done` (+ `gdfl_run::JournalReader::is_done` gdfl_run.rs:1992) | "one hash probe" | journal keys | none | no (`JournalReader` formats the key with the filter: O(filter bytes)) |
| 110 | crates/pull/src/gdfl_import_quality.rs:118 | quality module | "one difference array over the 86,400 seconds of the day, O(1) in the files" | files | none | no |
| 111 | crates/pull/src/gdfl_import_quality.rs:235 | `halt_window` | "O(1): one walk of the calendar's three-entry table" | — | none | no |
| 112 | crates/pull/src/gdfl_import_scope.rs:75 | scope ledger | "A digest's member list and a day's examined scope are each one hash probe" | ledger | none | no |
| 113 | crates/pull/src/gdfl_lineage.rs:172 | `gdfl_lineage::LotLedger::observe` (1222) | "one hash probe and one Euclid (O(log lot))" | contracts ever seen | test `the_unchanged_lot_arm_costs_one_pass_on_an_ordinary_day` [ign] | yes: `LotLedger { current: HashMap }` derives `Default` (gdfl_lineage.rs:1192-1195), never reserved; grows with every contract (~1.2M historic tickers): the insert at 7*2^k rehashes everything (tens of ms at 10^6) |
| 114 | crates/pull/src/gdfl_lineage.rs:1036 | `gdfl_lineage::is_contract_refused` | "O(1)" | — | none | no |
| 115 | crates/pull/src/gdfl_nfo.rs:94 | `gdfl_nfo::NfoDay::locate` (935) | "one hash probe (expected O(1), measured by C-GI-01)" | entries per day | C-GI-01 `gdfl_option_lookup_is_flat` (1k/10k/100k tickers) | maybe: two SipHash probes over the full ticker (`duplicates` then `by_ticker`); `locate` has no length cap (TICKER_CAP is only in `decode_ticker`) |
| 116 | crates/pull/src/gdfl_nfo.rs:99 | `NfoDay::strike_median` / `check_strike` | "computed ... on the first ask (O(entries log entries)); after it, one hash probe" | entries per day | none (C-NFO-01 listing has ONE entry) | yes: the first `read_file` of a day pays the whole median build (decode every ticker + sort per underlying); `push` drops the medians (gdfl_nfo.rs:893) so interleaved push/check rebuilds every time |
| 117 | crates/pull/src/gdfl_nfo.rs:182 | `gdfl_nfo::strike_implausible` | "One paisa more is not. O(1)" | — | none | no |
| 118 | crates/pull/src/gdfl_nfo.rs:1142 | `gdfl_nfo::NfoZips` day | "each day one hash probe for its day zip" | year zips | test `a_year_zip_is_indexed_once_and_each_day_is_one_lookup` | maybe: an absent/refusing year zip is not kept, so every day re-opens and re-walks it |
| 119 | crates/pull/benches/ratio.rs:1854 | `gdfl_nfo::read_file` | "costs the same PER ROW at 1,000, 10,000 and 100,000 rows" | rows per file | C-NFO-01 `nfo_read_is_flat_per_row` | maybe: per call it also pays `check_strike` (see #116) — not exercised with 1-entry listing |
| 120 | crates/pull/src/gdfl_rekey.rs:285 | `targets_continue` | "O(superseded records), no sort" | ledger | none | no |
| 121 | crates/pull/src/gdfl_rekey.rs:379 | `same_records` | "O(lines): one hash probe into fresh" | ledger | none | no |
| 122 | crates/pull/src/gdfl_rekey.rs:439 | `admit_superseded` | "O(1): three hash probes" | ledger | none | no |
| 123 | crates/pull/src/gdfl_rekey.rs:494 | `admit_retired` | "O(1)" | ledger | none | no |
| 124 | crates/pull/src/gdfl_rekey.rs:1250 | `gdfl_rekey::decoded_key` | "O(1): one settlement, a bounded walk over compile-time tables" | — | none | no |
| 125 | crates/pull/src/gdfl_rekey.rs:1306 | `gdfl_rekey::plan` | "O(1) per key for the ledger ... one search of the target's sorted month list" | keys, months | tests `plan_cost_per_key_is_flat_in_the_number_of_keys` [ign], `census_step_cost_...` [ign] | yes(minor): `month_of` is a binary search O(log months) (:1579); per-(underlying,day) ladders sorted (O(s log s)) |
| 126 | crates/pull/src/gdfl_rekey.rs:1465 | `against_ledger` | "O(1): two hash probes, and a third only for a key the ledger holds" | ledger | none | no |
| 127 | crates/pull/src/gdfl_rekey.rs:2274 | `ledger_lock_free` | "One lock wait, O(1) in the store" | store | none | maybe: `plan.retired.iter().any(...)` builds a line String per record (O(retired)) |
| 128 | crates/pull/src/gdfl_run_ledger.rs:20 | `gdfl_run` runs ledger `allocate` (332) | "The allocator: O(1) per run" | ledger records | test `gdfl_run::ia2_tests::ia2_timings` (C-GR-08) [ign] | maybe: a damaged last record triggers `Floor::read` of journal+claims, O(journal+claims) (stated) |
| 129 | crates/pull/src/gdfl_run_locks.rs:1308 | `gdfl_run::locks::lock_state` | "O(1) per file, at most four files" | — | ia2_timings [ign] | no (1+retry tries) |
| 130 | crates/pull/src/gdfl_tickstore.rs:792 | `TickStore` day (CmSource) | "one hash probe of the walked table" | day files | none | no (+O(entries) listing) |
| 131 | crates/pull/src/http.rs:120 | `http::pooled_client` | "One build per process; one Arc clone per call thereafter. O(1)" | calls | none | no (cold first call builds TLS client) |
| 132 | crates/pull/src/http.rs:642 | `HttpSource::wait_for_permit` | "O(1) per request and does not grow with how many requests" | requests | none | no |
| 133 | crates/pull/src/http.rs:1933 | `http::Refusals::note` | "O(1): a push into a vector capped at NAMED_ROWS" | refused rows | none | no |
| 134 | crates/pull/src/http.rs:2291 | `http::drop_impossible_bars` | "one pass, O(1) per row" | rows | none | no |
| 135 | crates/pull/src/identity_screen.rs:30 | identity screen per answer | "expected-O(1) listing probe ... each a binary search over those day bars" | minutes, day bars | C-ZI-01..03 | yes: per minute `DayBook::judge` is a binary search (see #172) |
| 136 | crates/pull/src/ingest.rs:17 | census read in `ingest` | "an O(1) read ... measured, C-12" | census entries | C-11, C-12, C-13 | see manifest rows |
| 137 | crates/pull/src/ingest.rs:51 | `closes_in_hand` (2229) | "two O(1) positional reads" | file rows | none | no |
| 138 | crates/pull/src/ingest.rs:87 | ingest per member | "one hash probe and one 128-byte entry per member" | census | C-13 | no |
| 139 | crates/pull/src/ingest.rs:609 | `note_fold` | "O(1) per RUN, not per bar" | bars | none | no |
| 140 | crates/pull/src/ingest.rs:837 | `derived_shortfall` | "Two integers compared. O(1) per member" | members | none | no |
| 141 | crates/pull/src/ingest.rs:1467 | `screen_candles` | "one hash probe per row: O(rows)" | rows | none | no |
| 142 | crates/pull/src/ingest.rs:5232 | `write_overlay` / `write_greeks` (5326) / `from_rows` (1741) | "not O(1) per call; corrected by D-2372" (O(log n_valid + rows + blocks)) | file rows | test `the_store_writing_doors_state_their_real_cost` | honest disclaimer (bisection in `BarFile::append`) |
| 143 | crates/pull/src/kite.rs:152 | `kite::KiteException::from_wire` | "Constant time: a match ... switches on the length first" | name length | none | no |
| 144 | crates/pull/src/landing_run.rs:43 | `landing_run::allocate` / hook | "the runs ledger's O(1)" | ledger | test `ia10c_timings` [ign] | maybe: first window per process reads the gap ledger whole (stated) |
| 145 | crates/pull/src/listing.rs:71 | `listing::Listing::span` | "one expected-O(1) hash probe ... plus bounded key bytes" | master rows | C-ZI-01 `listing_probe_is_flat` (300/3,000 rows) | no (note: `for_store` re-lists the master dir and re-parses on EVERY call while no master loads) |
| 146 | crates/pull/src/manifest.rs:2913 | `manifest::Manifest::entry` (3685) | "Lookup — expected O(1) ... one hash probe ... whatever the census holds" | census entries | C-12 `entry_lookup_is_flat` (1k/10k/100k, hit+miss), C-26 budget, C-14 | maybe: bench probes ONE hot key (`key(7)`) 20,000x; random keys over a 100k-entry (~15 MB) table add cache/TLB misses (doc calls this "residency, not measured") |
| 147 | crates/pull/src/manifest.rs:3815 | `Manifest::closes` / `held` | "Expected O(1), the same bound Manifest::entry carries" | census | via C-12 | as #146 |
| 148 | crates/pull/src/manifest.rs:2921 | `manifest::Manifest::record` (3938) | "no rehash for the first n_valid new keys ... amortised O(1) after that" | census entries | C-13 `append_after_load_is_flat` (256-key batches, round and 7*2^k sizes); unit `a_loaded_index_carries_headroom_for_the_appends_after_it` | **yes**: past `n_valid` new keys `index.insert` rehashes O(n_keys) and `log.push` reallocs O(n_valid) (walk reserves exactly 2x, manifest.rs:3283-3286); an UPDATE on a full table also rehashes; `genesis`/`partition_genesis` reserve nothing, so a census built from empty rehashes at every 7*2^k; C-13 batch (256) never leaves the headroom |
| 149 | crates/pull/src/manifest.rs:3585 | `Manifest::compacted` | "O(keys log keys) once ... amortised O(log keys) per append" | keys | C-14 `census_is_flat_across_a_compaction` | honest (log factor; per-entry ratio at 10x) |
| 150 | crates/pull/src/manifest.rs:3707 | `Manifest::all` | "O(1) to call and O(entries) to walk" | entries | none | no |
| 151 | crates/pull/src/manifest.rs:3771 | `Manifest::newest` | "One hash probe and one push per entry — O(1) per operation" | entries | none | no |
| 152 | crates/pull/benches/ratio.rs:291 | `Manifest::total_rows` | "the census is a counter read, not a walk" | entries | C-11 `census_beats_the_scan_it_replaces` | no |
| 153 | crates/pull/src/master_snapshots.rs:74 | `Catalog::{of_day,before,is_receipted}` | "one hash probe each" | snapshot dir entries | none | no (`Catalog::read` itself O(n log n), stated) |
| 154 | crates/pull/src/master_snapshots.rs:71 | `master_snapshots::Book::token_on` (1841) | "one day-table index and at most two hash probes" | snapshots held | C-MS-01 `token_on_is_flat` (1/10/100, `Book::of`) | maybe: bench books are pre-parsed (`Book::of`); an opened book (`Book::open`) reads+gunzips+parses a snapshot on its first probe (O(rows)) — a walk over days hits that once per snapshot |
| 155 | crates/pull/src/master_snapshots.rs:1420 | `Snapshot::lookup` | "one probe for the word, one for the token" | rows | via C-MS-01 | no |
| 156 | crates/pull/src/master_snapshots_first_listed.rs:76 | `master_snapshots::first_listed::listing` | "one directory listing ..., the absorbed log (one line per snapshot ...)" | snapshots | C-MS-02 | honest O(snapshots) — contradicts #64's "does not grow" |
| 157 | crates/pull/src/nse.rs:512 | `nse::index_links` | "O(n) with an O(1) per-link cost" | links | test `deduplicating_index_links_costs_one_probe_each_rather_than_a_scan` [assert ratio<3] | maybe: `HashSet::new()` not presized (rehash); SipHash over each href, href length uncapped (doc <= 8 MiB) |
| 158 | crates/pull/src/nse.rs:657 | `nse::constituents` | "the duplicate check is a HASH PROBE, O(1) per row" | rows | none | no (presized) |
| 159 | crates/pull/src/nseindex.rs:42 | `nseindex::Catalogue::resolve` | "O(1) when the symbol is published verbatim ... a bounded scan otherwise" | catalogue size | none | honest: non-verbatim symbols scan the whole catalogue, and production (`api::indexmap::join`) calls `resolve` per symbol |
| 160 | crates/pull/src/nseindex.rs:271 | `nseindex::Mapping::get` | "Built once; read O(1)" | mapping | none | no (`collapse` allocs O(len)) |
| 161 | crates/pull/src/padded.rs:46 | `padded::is_padded` | "one comparison of four prices and a volume: O(1) a bar" | minutes | C-PD-01/02 | no |
| 162 | crates/pull/src/padded.rs:144 | `padded::Walk::step` | "O(1): timed per minute by C-PD-01 and C-PD-02" | minutes | C-PD-01 `padded_fold_is_flat_per_minute` | no |
| 163 | crates/pull/src/pricing.rs:69 | `pricing::greeks_at` (822) | "O(1) time, O(1) space, no allocation" | — | none | no |
| 164 | crates/pull/src/pricing.rs:108 | `pricing::slot_of` | "O(1) — see costs::venue::swept_slot" | — | none | no |
| 165 | crates/pull/src/pricing.rs:124 | `pricing::trade_day_of` | "O(1): three field reads and two range checks" | — | none | no |
| 166 | crates/pull/src/pricing.rs:340 | `pricing::SpotBook::at` | "Probed in O(1), one hash lookup" | bars of month | none | no (presized, i64 keys) |
| 167 | crates/pull/src/pricing.rs:729 | `pricing::bar_greeks` / `solve_iv` (792) | "O(1): one bounded solve (at most MAX_ITERATIONS)" | — | none | no |
| 168 | crates/pull/src/pricing.rs:918 | `pricing::price` | "O(1) time ... One dated table lookup bounded by a compile-time constant" | — | none | no |
| 169 | crates/pull/src/rate.rs:131 | `rate::Window` | "A token bucket, because a sliding window is not O(1) space" | requests | unit P-34 `a_governor_holds_no_allocation_and_no_history` | no |
| 170 | crates/pull/src/rate.rs:973 | `rate::Governor::reserve` | "two walks of WINDOW_COUNT windows, a constant — O(1) per acquire" | requests | none | no |
| 171 | crates/pull/src/reference_data.rs:97 | reference-table lookups | "Every lookup a consumer makes after that is O(1)" | rows | test `the_loader_reads_every_table_inside_its_budget` | no |
| 172 | crates/pull/benches/ratio/identity.rs:7 | `series_identity::MinuteJudge::judge` -> `DayBook::judge` (series_identity.rs:418) | "its cost per minute must not grow with the answer" | day bars in the book | C-ZI-02 `minute_judge_is_flat` (books of 1/10/100 days) | **yes**: `self.days.binary_search_by_key(&day, ..)` (series_identity.rs:430) — O(log day bars) per minute, plus cache misses on a large book |
| 173 | crates/pull/src/reference_data.rs:892 | `CalendarTable::kind_of` (1151) | "One compare, one subtract, one index: O(1)" | days | none | no |
| 174 | crates/pull/src/reference_data.rs:1539 | `reference_data::Lineage` | "with O(1) probes by the old and by the new symbol" | rows | none | maybe: `forward()` hop walk bounded by rows |
| 175 | crates/pull/src/reference_data.rs:1848 | `FnoTable::member_on` | "one probe and at most MAX_SPANS_PER_SYMBOL comparisons" | rows | none | no |
| 176 | crates/pull/src/reference_data.rs:1932 | `ExpiryTable` | "one probe for an underlying's own row and one ..." | rows | none | no |
| 177 | crates/pull/src/reference_data.rs:2344 | `horizons` / `horizon_alerts` (2359) | "O(1)" | — | none | no |
| 178 | crates/pull/src/refusal.rs:80 | `refusal::{status_disposition, ErrorNames::read, classify}` | "Every function here is constant-time in the size of anything that grows" | body size | none | no |
| 179 | crates/pull/src/refusal.rs:544 | `refusal::named_error_of` / `disposition_of_value` | "the one linear function ... a constant number of key lookups" | body size, keys | none | maybe: `serde_json::Map` is a BTreeMap here, so each `get` is O(log keys) string compares over a vendor-chosen object |
| 180 | crates/pull/src/rehearsal.rs:61 | `rehearsal::Gate::admit` | "one hash lookup and a fixed number of comparisons: O(1) per start" | list | test `the_gate_is_one_lookup_per_start` | no |
| 181 | crates/pull/src/repair_plan.rs:220 | `ingest::repair_plan::owed` | "O(1): one calendar answer, at most two dated-table walks" | days | tests/zerodha_gaps.rs `what_a_day_owes_costs_the_same_on_every_date_of_the_window` [assert median<=3x] | no |
| 182 | crates/pull/src/request_minutes.rs:123 | `request_minutes::Cursor` (private) | "a membership question asked in time order is O(1) amortised" | refused seconds | none | no (O(lists) per ask) |
| 183 | crates/pull/src/response_keep.rs:68 | `response_keep::receipts` | "Nothing depends on how many packs the store holds" | packs; segments | C-RK-01 `keep_lookup_is_flat` (1/10/100 packs, ONE segment) | maybe: reads EVERY segment of the month (`(0..=newest)`, <= MAX_SEGMENTS = 8 x 128 KiB) — 8x the bench at a full month |
| 184 | crates/pull/src/response_keep.rs:70 | `response_keep::append` | "one read of the newest segment's receipts (at most MAX_RECEIPTS x 128 B)" | packs; receipts in segment | C-RK-02 `keep_append_is_flat` (appends into a near-empty 2030-01 month) | maybe: cost grows with receipts already in the newest segment: read+CRC of up to 1,024 receipts and a linear `holding` scan (response_keep.rs:1213); bench never appends to a full segment |
| 185 | crates/pull/src/response_keep.rs:1577 | `Parked::admit` | "one hash probe under one mutex per contended wait" | locks | none | no |
| 186 | crates/pull/src/response_keep.rs:1984 | `read_windows` | "One stat and at most two 64-byte reads, whatever the journal's length" | window journal | none | no |
| 187 | crates/pull/src/riskfree.rs:93 | `riskfree::Series::on` (771) | "O(1): one subtraction and two bounds-checked reads" | auctions in table | C-RF-01 `rate_lookup_is_flat` (52/520/5,200); test `a_carried_lookup_is_flat_in_the_table` [assert p99<=10x] | no |
| 188 | crates/pull/src/riskfree.rs:906 | `riskfree::replay` | "one O(1) Series::on a day asked" | days | same test | no |
| 189 | crates/pull/src/rolling.rs:42 | `rolling` module | "O(1) per request built and O(1) per row read" | rows | none | no |
| 190 | crates/pull/src/rolling.rs:85 | `rolling::expiry_of` | "O(1). One slot lookup and one calendar step; nothing scans" | — | none | maybe: `steps` calendar steps (1..=expiry_codes.len()) through `costs::expiry`, not one |
| 191 | crates/pull/src/rolling.rs:224 | `rolling::listing_of` | "O(1): one slot lookup and one dated-table step" | — | none | no |
| 192 | crates/pull/src/rolling.rs:526 | `rolling::url` | "One allocation, proportional to the URL. O(1) in the request" | — | none | no |
| 193 | crates/pull/src/scrub.rs:60 | `scrub::one` (199) | "O(1) per entry ... One open, one header read, two record reads" | file bars, store months | none (UNVERIFIED) | maybe: real cost is `store::BarFile::open_existing` + `read_record` (sealed months decode a frame) — not measured |
| 194 | crates/pull/src/scrub.rs:317 | `scrub::Tally::count` | "Folds one finding in. O(1)" | — | none | no |
| 195 | crates/pull/src/series_identity.rs:389 | `DayBook::unread_holds` | "calendar arithmetic and one hash probe, so the same cost with one torn month or a hundred" | torn months | C-ZI-03 `torn_month_judge_is_flat` (1/10/100 torn, EMPTY day list) | no for the probe; but `judge` binary-searches `days` first (#172) and the bench book holds no days |
| 196 | crates/pull/src/series_identity.rs:533 | `series_identity::Tally::count` | "O(1) for minutes in time order" | minutes | none | no |
| 197 | crates/pull/src/session.rs:73 | `session::Day::{days_from_epoch, from_days}` | "a timestamp in 1970 and one in 9999 cost the same" | dates | test (exhaustive correctness) | no |
| 198 | crates/pull/src/session.rs:1118 | `session::Window::days` | "a one-day window and a ten-year one cost the same" | window length | none | no |
| 199 | crates/pull/src/session.rs:1344 | `session::with_recorded_window` | "one walk of the fifteen-entry SPECIAL_SESSIONS ... O(1)" | — | none | no |
| 200 | crates/pull/src/ssm.rs:167 | `ssm::pooled_client` | "one Arc clone per call thereafter. O(1)" | calls | none | no |
| 201 | crates/pull/src/ssm.rs:736 | `ssm::refusal_detail` (private) | "Constant work in the body's length" | body bytes | none | yes: `body.contains(name)` for each `AWS_FAULTS` entry is O(body) each (ssm.rs:740) — linear, not constant, in the body |
| 202 | crates/pull/src/tenor.rs:71 | `tenor::Tenor::between` (262) | "O(1) time, O(1) space, no allocation, no calendar walk" | — | none | no |
| 203 | crates/pull/src/totp.rs:38 | `totp` | "Constant in everything that varies" | — | test `the_length_bound_is_checked_before_a_single_character_is_decoded` | no |
| 204 | crates/pull/src/universe.rs:34 | `universe::resolve` (549) / `MasterIndex` | "every exchange row is then one probe" | master rows | none | no |
| 205 | crates/pull/src/universe.rs:110 | `JoinKey::for_vendor` | "Constant time ... one jump" | vendors | none | no |
| 206 | crates/pull/src/upkeep.rs:29 | `upkeep::status` (884) | "reads a bounded tail (TAIL_BYTES) and nothing else" | journal length | none | no (parses every line in the tail, bounded) |
| 207 | crates/pull/src/vendor.rs:22 | `vendor` descriptor lookup | "the bound does not change when the table grows" | feeds | test `the_descriptor_table_is_indexed_by_the_discriminant` | no |
| 208 | crates/pull/src/vendor.rs:553 | `Granularity::is_finer_than` / `none_finer_than` (761) | "One u8 comparison — O(1)" / "One mask — O(1)" | ladder | none | no |
| 209 | crates/pull/src/vendor.rs:907 | `Venue::share_hours_on` | "The same constant work as hours_on" | — | none | no |
| 210 | crates/pull/src/vendor.rs:3304 | `route_for` / `granularity_token` / `listing_words` (3351) | "CONSTANT WORK, not a scan" | universe size | test `every_listing_table_holds_at_most_one_row_per_class` | no |
| 211 | crates/pull/src/vendor.rs:4612 | `granularity_verdict` | "O(1), and by construction rather than by measurement" | — | none | no |
| 212 | crates/pull/src/work.rs:27 | `work::gaps` | "one hash probe each against the held set" | store size | none | maybe: generic `S: BuildHasher` chosen by caller; `Cell` hashes an instrument String |
| 213 | crates/pull/src/work.rs:82 | `work::Selection::of` | "the set never grows at all" (lower size_hint) | names | none | maybe: a filtered iterator's size_hint().0 is 0 -> rehash growth (stated) |
| 214 | crates/pull/src/work.rs:159 | `Selection::cells` | "one probe per month: O(months), paid once" | months | none | no |
| 215 | crates/pull/src/zerodha_spool.rs:12 | `zerodha_verify::spool::SpoolWriter::append` | "a run appends here, O(1) per record" | records spooled | C-ZV-01 spool append (100/1k/10k records) | maybe: handle map `with_capacity(16)` grows with every (segment, tf, month) a run touches, each keeping an open fd (EMFILE ~1,024) |
| 216 | crates/pull/src/zerodha_spool.rs:99 | `spool::append_log` + `LogRead::last_of` (1476) | "A merge-log step is one hash probe ..., one 16-byte write and one sync_data" | records in merge log | C-ZV-03 `zerodha_merge_log_step_is_flat` (30/300/3,000 records; MIN of 12 steps) | **yes**: `read_log` sizes `records` to exactly the records read (zerodha_spool.rs:1563-1564), so the first `append_log` after it (`learn` -> `records.push`) reallocates+copies the whole log (<= 4,194,304 records, ~48 MB); `last` inserts rehash O(keys) at 7*2^k distinct (exchange,day) keys, even for an update; the bench builds logs from empty and keeps the minimum, hiding both |
| 217 | crates/pull/src/zerodha_spool.rs:226 | `spool::live_probe` | "O(1): at most two opens, two fstats and two flocks" | runs | test `a_live_probe_is_a_fixed_number_of_calls_in_every_layout` (prints) | no |
| 218 | crates/pull/src/zerodha_spool.rs:1182 | spool damage check / `resume_at` (1388) | "One read, O(1)" / "One read of at most MAX_DETAILS + 2 records" | spool | none | no |
| 219 | crates/pull/src/zerodha_spool.rs:1486 | `LogRead::refused` / `covered` (1508) | "O(1)" | log | none | no |
| 220 | crates/pull/src/zerodha_verify.rs:726 | `zerodha_verify::oracle` | "O(rows), one hash probe per row" | rows | none | no |
| 221 | crates/pull/src/zerodha_verify.rs:1455 | `refused_minutes` | "each membership test after it is one hash probe" | refused | none | no |
| 222 | crates/pull/src/zerodha_verify.rs:2278 | `peer_holds_day` (private) | "two first_at_or_after lookups, O(1) a day" | seconds in the GDFL month | none (UNVERIFIED) | maybe: `store::BarFile::first_at_or_after` is O(1) only on a month with a time index; a legacy/damaged-index month uses the D-1434 bisection, ceil(log2(n+1)) record reads each (crates/store/src/file.rs:3535-3593); first lookup per handle opens the index (cold) |
| 223 | crates/pull/src/zerodha_verify.rs:3898 | `not_live` | "O(runs), each O(1)" | runs | none | no |
| 224 | crates/pull/src/zerodha_verify.rs:4464 | `merge_day` | "O(1) per day besides the commit" | records, log | C-ZV-02, C-ZV-03 | maybe: each call builds a fresh `verdict::Reader` and reads the year index (`committed_generation`), may walk <= MAX_WALK_BACK image headers, and inherits #216's realloc on its first append |
| 225 | crates/pull/benches/ratio.rs:1357 | `zerodha_verify::verify_window` | "flat at 1x, 10x and 100x instruments held" | instruments in store | C-ZV-01 | no |
| 226 | crates/pull/benches/ratio.rs:1503 | `zerodha_verify::merge_all` | "per record ... Its sort is O(n log n): the ceiling allows it" | records per day spool | C-ZV-02 (343/3,430/27,440) | maybe: log factor per record (honest) |
| 227 | crates/pull/benches/ratio/tickstore_write.rs:1 | `tickstore_write::write_day` | "cost PER MEMBER ... flat" (marginal 10x->100x) | members per day | C-TSW-01 | maybe: per day `done.sort_by_key` O(n log n) (tickstore_write.rs:1248) and `leftovers` read_dir of the day folder |
| 228 | crates/pull/benches/ratio/tickstore_write.rs:12 | `NfoTickStore` day + `NfoDay::locate` | "one probe by ticker, which must cost the same at 100 times the members" | members | C-TSW-02 | as #115 |
| 229 | crates/pull/benches/ratio.rs:876 | `store::file::BarFile::read_record` (store crate) | "one positional read, flat at 1x, 10x and 100x the month" | bars | C-GI-03 | out of crate scope (store) |
| 230 | crates/pull/src/census_store.rs:1674 | `ingest::census_store::record_kept` | "reads one header region and probes only its own keys, whatever the census holds" | census entries | census_store_kept_tests [ign 8.4M]; tests/census_day_pace.rs | maybe: per cycle one dir listing O(epochs); the cycle that fills a partition compacts (O(keys log keys)) and the NEXT cycle re-reads the whole census (Kept dropped) — a p99/max spike |
| 231 | crates/pull/src/census_store.rs:745 | `census_store::probe` | "one probe per offered row" | census | none | no |
| 232 | crates/pull/src/day_universe.rs:214 | `day_universe` | "the per-name cost stays flat (O(inputs))" | names | test `the_universe_costs_one_insert_per_name_measured` | no |
| 233 | crates/pull/src/gdfl_cm_admission_tests.rs:1573 | `gdfl_cm` decode match band | "The match band costs O(1) a row and one pass a file" | rows | test `the_match_band_costs_one_pass_a_file` [ign] | no |
| 234 | crates/pull/src/census_store_link_tests.rs:717 | `census_store::install_epoch` | "THE STAGING RULE ADDS A CONSTANT NUMBER OF CALLS TO AN INSTALL" | installs | that test [assert host-call count] | no |

## `crates/cli` — inventory of cost-bound claims (O(1) / constant / flat / p99 / time budget)

Branch `cloud/o1p99-attack-20261110`. Read-only research; no cargo run. Scope: `crates/cli/src/**`, `crates/cli/benches/ratio.rs`, `crates/cli/tests/**`.
Raw hit counts: 282 `O(1)` lines, 25 "constant(-time/cost)" lines, ~40 p50/p99 timing lines (mostly printed measurements), ~676 "budget" lines (almost all resource caps — excluded unless the text ties them to time).
One row per (function, claim); comment lines repeating a claim are collapsed onto the first line.

Legend — **bench**: the only bench is `crates/cli/benches/ratio.rs` (rows C-CLI-01…06, all on `cli::results`). "test:" names a unit/integration test that asserts a probe/step count or a timing. **suspect**: `no` / `yes: …` / `maybe: …`; "(admitted)" = the source itself already says it is not O(1).

All `HashMap`/`HashSet` in `cli` use std `RandomState` (SipHash-1-3, per-process random keys) — grep found no custom/FNV hasher used as a map hasher (FNV-1a only computes store symbol ids). So "adversarial key collisions" is a `no` everywhere unless noted; keys are mostly blake3 digests.

| # | where | what | claim (≤15 words) | flat in | bench / test | suspect |
|---|---|---|---|---|---|---|
| 1 | crates/cli/src/results.rs:23 (also :31, :1271, :1287) | `results::Results::append` (+`append_locked_with` :1395) | "append \| O(delta + 1) local work" | ledger records (runs.bin size) | none (bench only appends while building fixtures, untimed) | **yes**: every append runs `absorb_new_records` (:1401); if any other handle/process grew the file, it calls `self.prefix.require_unchanged` (:1499 → result_set.rs:663-675, `hash_range(0..covered)`), re-reading + blake3-ing the WHOLE indexed ledger: O(ledger bytes + delta), not O(delta+1). Two handles alternating appends ⇒ O(n) per append, O(n²) total. docs/06-limits.md D-1560 (l.13444) admits it; this header, §100 (l.6517) and l.8928 still say O(delta+1)/O(delta). |
| 2 | crates/cli/src/results.rs:1463 | `Results::absorb_new_records` (:1472) | "O(records appended by others) … this is a length check" | ledger records | none | **yes**: same growth branch, O(indexed bytes) (its own comment at :1494 says so; doc above contradicts). |
| 3 | crates/cli/src/results.rs:1525 | `Results::refresh` (:1530, pub) | "O(new rows), with constant metadata checks when unchanged" | ledger records | none | **yes**: any growth ⇒ full-prefix re-hash (:1499). A read-only handle refreshed after each foreign append pays O(ledger) per refresh. |
| 4 | crates/cli/src/results.rs:24 (also :1542, bench :29) | `Results::read` | "read record i \| O(1) \| seek to HEADER + i·STRIDE, one read" | record index i / ledger size | **C-CLI-01** `the_read_does_not_depend_on_which_record` (read 0 vs 1023 on one 1,024-row ledger, ≤2.5x) | no (flock_shared + fstat + seek + 261-B read + blake3 of 253 B). Bench never varies ledger size. |
| 5 | crates/cli/src/results.rs:25 (also :1161) | `Results::len` | "O(1) — a division, not a walk" | records | **C-CLI-02** `the_count_does_not_depend_on_how_many_there_are` (64 vs 1,024) | no (one fstat). |
| 6 | crates/cli/src/results.rs:26 (also :1192, :617) | `Results::holds` | "duplicate check \| expected O(1) \| a HashMap of identities" | records | **C-CLI-03** `the_duplicate_check_does_not_scan_the_ledger` (hit vs miss, 1,024 rows); **C-CLI-06** `the_duplicate_check_stays_within_its_budget` (miss ≤65 floors) | no (std HashMap, blake3 keys). Bench gap: C-CLI-03's "hit" is `record(LARGE-1)`, the LAST row appended, so a front-to-back linear scan costs n for hit and miss alike (ratio ≈1, passes); no row varies ledger size; only C-CLI-06's absolute budget would notice. Bench header still calls it a `HashSet`. |
| 7 | crates/cli/src/results.rs:1203 | `Results::index_of_identity` (pub(crate)) | "The zero-based append-only row index for one identity. O(1)." | records | C-CLI-03 (probe half, indirectly) | no |
| 8 | crates/cli/src/results.rs:1225 (also :1243, :2710) | `Results::of_identity` | "one HashMap probe … independent of how many runs the ledger holds" | records | none (C-CLI-03 times only the probe); test `a_run_is_found_by_its_identity_and_not_by_its_position` (correctness) | no |
| 9 | crates/cli/src/results.rs:459 | `Record::seal_matches` | "O(1). One hash of a FIXED 253 bytes" | records | none directly (C-CLI-05 includes a seal via `to_bytes`) | no |
| 10 | crates/cli/src/results.rs:101 (STRIDE :129; bench :45) | `Record::to_bytes` / fixed stride | "a variable record has no stride and therefore no O(1) address" | ledger it joins | **C-CLI-04** `the_encode_does_not_depend_on_the_ledger_it_joins`; **C-CLI-05** `the_encode_stays_within_its_budget` (≤2,000 floors) | no |
| 11 | crates/cli/src/results.rs:1067-1085 | `Results::open_with` (index build) | "ONE PASS AT OPEN … O(runs) … AND IT IS PRE-SIZED" (growth-free index) | records | test counters OPENS/ROW_READS (:3005) | maybe: O(runs) admitted; `with_capacity(len/stride)` is taken from file LENGTH before any record is validated and the writer `open` has no byte ceiling, so a sparse/huge runs.bin forces a len/261-entry allocation (abort risk) before refusal. Also `PrefixDigest::over` (:1118) reads+hashes the file a second time. |
| 12 | crates/cli/src/result_set.rs:419 | `result_set::Receipts::refresh` (:424, pub) | "Refresh only newly appended receipts under a shared lock." | receipts (detail-sets.bin) | none | **yes**: `absorb_new` growth branch calls `prefix.require_unchanged` (:553) ⇒ re-hash of every indexed receipt byte, O(file) per refresh after any append. |
| 13 | crates/cli/src/result_set.rs:876-879 | `CommittedParents::refresh` (:912, pub) | "Subsequent refreshes validate file generations and index appended rows only." | ledger rows + receipts | none | **yes**: calls `Results::refresh` (:915) and `Receipts::refresh` (:926); both re-hash their whole prefix on growth. |
| 14 | crates/cli/src/result_set.rs:413 | `Receipts::of_identity` | "one in-memory hash probe after the open-time scan" | receipts | none | no |
| 15 | crates/cli/src/result_set.rs:604-608 | `PrefixDigest` (extend_with / extend_from / require_unchanged) | "own appends … extend it in O(appended bytes); only the growth branch pays O(indexed bytes)" | file bytes | none | yes (admitted here) — this is the mechanism behind rows 1-3, 12, 13, 47, 49. |
| 16 | crates/cli/src/pool.rs:847 (also :84-88, :877) | `CommittedParents::committed` as used by `pool::union_of` | "each instrument's receipt is one expected-O(1) probe of that snapshot" | instruments / ledger rows | test `pool::tests::the_union_admits_the_parent_ledger_once_not_once_per_instrument` (L1A-05, open count); `lib.rs:28689` `the_pool_union_opens_the_parent_ledger_once_for_every_instrument` | no for the probe (`Results::of_identity` = probe + flock + fstat + one read); see row 13 for refresh. |
| 17 | crates/cli/src/frontier.rs:43 (also :1397) | `frontier::Frontier::read` | "Row index. O(1) — one seek to HEADER + index·STRIDE, one read." | rows | none | no |
| 18 | crates/cli/src/frontier.rs:44 (also :975) | `Frontier::len` | "How many rows the file holds. O(1) — arithmetic on the length." | rows | none | no |
| 19 | crates/cli/src/frontier.rs:45 (also :875, :2658, :2676) | `Frontier::block` | "FIND a run's rows \| O(1) \| one hash probe into the block index" | runs / rows | test `the_block_index_locates_a_run_without_reading_a_row` (:2668) | no |
| 20 | crates/cli/src/frontier.rs:886 (also :1062) | `Frontier::holds` | "Whether this run recorded a frontier at all. O(1)." | runs | none | no |
| 21 | crates/cli/src/frontier.rs:1455 (also :1468-1476, :1556, header :45/:78) | `Frontier::of_run` (pub) | "finding the block is O(1) and reading it is O(count)" | ledger rows, receipts | none | **yes (partly admitted)**: on a read-only handle EVERY call (not only a fresh handle, as the doc says) runs `result_set::committed_receipt` (:1497), which cold-opens `runs.bin` (index build + `PrefixDigest::over` second full read) and `detail-sets.bin`: O(L+R) per call, even for an ABSENT identity. |
| 22 | crates/cli/src/frontier.rs:1013 | `Frontier::append_all` (pub) | "It is still O(1) per row: one write_all … no read and no seek per row." | rows already in file / other writers' rows | none | maybe: first `absorb_new_rows` reads every row other writers appended with one seek+read_exact each (`absorb_new_rows` :1296, seek :1305), then two O(R) duplicate-probe passes; O(R + delta), fine single-writer. |
| 23 | crates/cli/src/frontier.rs:1193 (also :1206, :1294, header :73) | `Frontier::refresh` / `absorb_new_rows` / `absorb_read_only` | "every refresh after it is O(delta)" | rows | test `a_read_only_handle_over_damaged_history_refreshes_by_the_delta` (:2342) | no (no prefix re-hash here; writer path is one seek+read per new row). |
| 24 | crates/cli/src/frontier.rs:132 | derived ranking fields (`win_rate_bp`, `avg_win`…) | "derived from them in O(1)" | rows | none | no |
| 25 | crates/cli/src/trades.rs:54 (also :552) | `trades::Trades::block` | "find a run's trades \| O(1) \| one hash probe" | runs | none | no |
| 26 | crates/cli/src/trades.rs:562 | `Trades::holds` | "Whether this run's trades are already recorded. O(1)." | runs | test `a_reopened_file_still_finds_the_run_it_was_given` (:1456) | no |
| 27 | crates/cli/src/trades.rs:55 (also :100) | row addressing (`STRIDE`, reads in `read_against_receipt`) | "read one row \| O(1) \| seek to HEADER + index * STRIDE" | rows | none | no |
| 28 | crates/cli/src/trades.rs:980 (also :72-73) | `Trades::of_run` (pub) | "O(count) after an O(1) lookup" (read-only: "not O(1)") | ledger rows, receipts | none | yes (admitted): read-only handle calls `committed_receipt` (:996) on every call ⇒ O(L+R) per lookup, even for a miss. |
| 29 | crates/cli/src/trades.rs:801 | `Trades::refresh` (:815) | "One `symlink_metadata`, O(1)." (+ new rows) | rows | none | no (`refresh_locked` reads only rows past `scanned`). |
| 30 | crates/cli/src/trades.rs:754 | `Trades::absorb_new_rows` (:907) | "O(rows appended by others), which is zero on the one-writer path." | rows | none | no |
| 31 | crates/cli/src/trades.rs:2305 | `trades::Bucket::take` (private) | "Every field is a compare or an add, so this is O(1)" | trades | none | no |
| 32 | crates/cli/src/trades.rs:2450 | `trades::by_period` (pub) | "Every probe and every fold is O(1), so this is O(trades) overall" | trades | none | **yes**: each of the 8 periods ends with `out.sort_unstable_by_key(\|b\| b.key)` (:2486) — O(B log B) with B≈T for Day/Week/Month when trades land on distinct days ⇒ O(T log T); maps pre-sized only to `min(T, 512)` so >512 buckets rehash (amortised). |
| 33 | crates/cli/src/knobs.rs:55 | `knobs::var` (set path) | "one expected-O(1) hash probe, plus a copy of the value" | knobs set / value length | none (doc test `the_cost_table_does_not_call_an_environment_read_constant`) | maybe: hash + clone are O(\|name\|+\|value\|); value length unbounded (free-text HTTP field), read ~388k times per audit per the module's own count. |
| 34 | crates/cli/src/knobs.rs:56 | `knobs::var` (unset path) | "not claimed O(1) … `std::env::var_os`: a lookup over the process environment" | env vars | none | yes (admitted): environment scan. |
| 35 | crates/cli/src/knobs.rs:57 | `knobs::set` (pub) | "`set` \| O(1) \| one hash insert" | set knobs | none | maybe: `HashMap::new()` never pre-sized (rehash growth), O(\|name\|+\|value\|) copy; accepts ANY name. |
| 36 | crates/cli/src/knobs.rs:59-60 | `knobs::clear_all` / `knobs::refused` / `count` | "O(set knobs) and that count is bounded by 24" | set / refused knobs | none | **yes**: nothing in `cli` bounds the names — `set` (pub) inserts any name and `refuse` records any name in a BTreeMap; "24" is a property of `api`'s validation, not of this module. `count` also does an O(log n) tree insert. |
| 37 | crates/cli/src/stored.rs:517 | `stored::nse_session_close_minute` | "O(1): one `kind_of` call, itself a bounded walk" | days | none | no (`pull::calendar::kind_of`: bitset + ≤9 compares, const-asserted). |
| 38 | crates/cli/src/stored.rs:573 | `stored::CashCloses::session_close_minute` | "O(1): one calendar call and one hash probe" | dated days | none | no |
| 39 | crates/cli/src/stored.rs:806 | `stored::calendar_receipt_v1` | "O(B + D) time … O(1) auxiliary space" | timestamps/days (space) | none | no |
| 40 | crates/cli/src/stored.rs:960 | `stored::calendar_policy_digest_v2` | "first call O(D) … every later call … O(1)" | calendar days | test counter `CALENDAR_POLICY_DIGEST_INITIALIZATIONS_V2` | maybe: cold first call O(D) (OnceLock) — a p100, not p99, spike. |
| 41 | crates/cli/src/stored.rs:1718 (also :1750) | `stored::calendar_receipt_v2` / `calendar_receipt_v2_for_bars` | "O(B + D) time … O(1) auxiliary space" | bars/days (space) | none | no |
| 42 | crates/cli/src/stored.rs:2066 | `stored::clipped` (pub(crate)) | "O(1) in the length of `word`, not O(len)" | word length | none | no (`take(64)` + `nth(64)`). |
| 43 | crates/cli/src/stored.rs:2307 | `stored::vwap_availability` | "O(1): one `match` on a `Copy` discriminant" | — | none | no |
| 44 | crates/cli/src/stored.rs:2635 (also :2511, :2710) | `stored::refuse_quarantined` → `store::quarantine::probe` | "O(1): one probe, two positional reads" | quarantine entries | test `quarantine_tests::a_quarantine_probe_is_one_read_and_a_torn_tail_is_repaired` | maybe: `level_read` waits on a writer with bounded "patience"; otherwise bounded bucket. |
| 45 | crates/cli/src/stored.rs:3073 | `stored::load_span` | "one `open_existing` per month and one `read_record` per bar, each O(1)" | months / bars | none | no (total O(M+B) stated). |
| 46 | crates/cli/src/stored.rs:3108 (also :3632, :3832) | `load_span_bounded`, `load_daily_context_bounded`, `load_exact_minute_context_bounded` | "Header admission is O(1) per month" | months | none | no |
| 47 | crates/cli/src/lib.rs:18345 | `recorded_row` (private) | "O(rows appended since the handle's last use) plus one O(1)-expected hash probe" | ledger rows / page size | none | **yes**: `results::with_shared_writer` (results.rs:808) calls `Results::refresh` ⇒ full-prefix re-hash whenever another process appended; `recorded_identity(page)` (:18421) scans the whole rendered page twice (O(page bytes)); any refusal drops the cached handle (results.rs:830) ⇒ next call is an O(runs) open. |
| 48 | crates/cli/src/lib.rs:19490 | static `LEDGER` mutex (record path `ensure_run_record` :20572) | "contention is nil and the O(1) append is untouched" | ledger rows | none | **yes**: the append is O(delta+1) at best and O(ledger bytes) when another process wrote (rows 1, 47). |
| 49 | crates/cli/src/lib.rs:19999 | `with_cached_handle` (frontier/trades writers) | "O(delta) per run, the same trade `results::with_shared_writer` makes" | rows | none | maybe: true for frontier/trades `refresh`; the comparison is false (results' refresh re-hashes the prefix); any refused operation drops the handle ⇒ O(rows) reopen next run. |
| 50 | crates/cli/src/lib.rs:6110 (also :6114) | `window_range_percentile` (private) | "O(1) amortised per bar … the whole walk is O(bars)" | bars, hold | none | maybe: amortised only — one bar can pop up to `hold` deque entries (strictly falling highs for `hold` bars, then a spike); `VecDeque::new()` not pre-sized; `select_nth_unstable` after. |
| 51 | crates/cli/src/lib.rs:7226 | `session_returns` (private) | "O(sessions) to build, O(1) per trade, and nothing searches" | trades / sessions | none | no |
| 52 | crates/cli/src/lib.rs:7272 | `session_index` (private) | "One pass, one comparison per bar … No sort, no search" | bars | none | no |
| 53 | crates/cli/src/lib.rs:9733 | `newest_complete` (private; `/engine/top.json`) | "Still `O(runs)` … an O(1) sidecar … was refused" | runs | test `audited_stored::top_reads_the_ledger_twice_and_refuses_damage_in_a_row_it_does_not_name` | yes (admitted). |
| 54 | crates/cli/src/lib.rs:19303 | `window_withholding` (private; via `StoredReplay::column_of` :19278) | "One forward pass over `folded` with one hash probe per bar" | walk-forward windows × folded bars | none | maybe: every window rebuilds `Admission::of(days)` (O(cuts)) and scans `folded` from index 0 to the window end, so W windows cost O(W·(N+cuts)) instead of O(window). |
| 55 | crates/cli/src/lib.rs:21778 | `note_grid_progress` | "one relaxed `fetch_add` per candidate — the counter is O(1)" | candidates | none | no (event emitted once per stride). |
| 56 | crates/cli/src/lib.rs:4056 (dup :25950) | `log_dir_from` | "the only design that is `O(1)` per event with two writers" | writers | none | no (directory choice; per-event cost is `telemetry`'s). |
| 57 | crates/cli/src/lib.rs:102 | `tick_second_check` module (status_of) | "its index answers `status_of` in O(1)" | symbols | see rows 85-86 | see rows 85-86 |
| 58 | crates/cli/src/lib.rs:643 | `feed-reconcile factor` usage text | "one stock-day's factor … O(1) per lookup, timed" | symbols × days | see row 81 | see row 81 |
| 59 | crates/cli/src/lib.rs:11910 | `cap_for_budget` / `screen_budget_ms` (`BRUTEX_SCREEN_BUDGET_MS`) | "a fixed prefix is priced and timed … turns that throughput into a count" (wall-clock budget) | candidates | none | maybe: calibrates on the first 256 candidates by evidence and extrapolates linearly; later candidates (deeper masks, more trades) can cost more, so the ms budget is not a bound. Only unrecorded generated audits reach it. |
| 60 | crates/cli/src/pool.rs:104 (also :1131) | `pool::fold` (private) | "One pass over `instruments × candidates` with O(1) work per cell" | instruments × union U | none ("UNVERIFIED performance") | maybe: ends with `retain` + `pooled.sort_by_key(..)` (:1190) O(U log U) not in the claim; per cell clones a symbol `String` (≤5 per row). |
| 61 | crates/cli/src/pool.rs:84 (also :847) | `pool::union_of` (pub(crate)) | "one `HashSet` insert per frontier row -- O(1) expected" | rows / instruments | tests in row 16 | no |
| 62 | crates/cli/src/minute_gaps.rs:274 | `minute_gaps::Admission::admits` (pub) | "one O(1) probe per bar: a map from each cut day" | cut days | test `causal_tests::the_admission_probe_and_the_fold_pairs_agree`; `measure_tests::admission_cost_per_bar_and_month` (#[ignore], real store via `AH01_STORE`) | no |
| 63 | crates/cli/src/minute_gaps.rs:504 | `first_interior_holes` (private) | "One `kind_of` per day (O(1)…) and O(1) per bar" | bars | none | no |
| 64 | crates/cli/src/minute_gaps.rs:569 | `first_expected_after` (private) | "At most MAX_WINDOWS windows, so O(1)" | windows | none | no |
| 65 | crates/cli/src/minute_gaps.rs:745 (also causal_tests :496) | `minute_gaps::withhold` (pub) | "O(1) however many days are cut, so the loop is O(bars)" | cut days | same as row 62 | no (builds `Admission` per call, O(cuts)). |
| 66 | crates/cli/src/minute_gaps.rs:799 | `minute_gaps::withhold_halted_minutes` (pub) | "two compares per minute: O(1) per minute" | minutes | none | no |
| 67 | crates/cli/src/minute_gaps.rs:836 | `minute_gaps::counted` (pub) | "One pass with one hash probe per bar." | bars | none | no |
| 68 | crates/cli/src/minute_gaps.rs:718 | `minute_gaps::tighten` (pub) | "O(d) for d cut days" | cut days | none | yes (admitted): `position` scan + `Vec::insert`; retry loops calling it per failing day make O(d²). |
| 69 | crates/cli/src/calendar_days.rs:299 | `calendar_days::DayMasks` mark/get/contains (pub(crate), private mod) | "one indexed access each, O(1) at any store size" | span days | test `a_day_costs_the_same_at_any_span_measured` (31 vs 36,525 days; p99 < 100 µs/day) | no for mark/get; `DayMasks::over` allocates O(span) chosen from census months (far-apart months ⇒ big alloc; overflow ⇒ width 0 and every mark refused). |
| 70 | crates/cli/src/calendar_days.rs:28 (also :1053) | `calendar_days::month_days` (private) | "O(1) per day … through the month's `.tix`, the bisection otherwise" | bars per month | test `the_check_costs_constant_time_per_day_measured` (p99 < 2 ms/day on a 20-bar daily month) | **yes**: without a ready `.tix` (every month under `store::time_index::INDEX_MIN_ROWS` = 1,024 rows gets none; or a deleted/stale/failing one) `BarFile::first_at_or_after` bisects (store/src/file.rs:3579-3596): O(log bars) record reads per lookup; each month also pays a `BarFile::open_existing`. |
| 71 | crates/cli/src/calendar_days.rs:475 | `calendar_days::findings` | "one `tradable_day` per held day and one per weekday … each O(1)" | held days | same test (per held day) | no; note the missing-day loop walks every day first..last (O(span), not O(held)). |
| 72 | crates/cli/src/basis_runs.rs:238 | `basis_runs::ordinal` | "O(1): two multiplications and an addition" | — | test `a_window_wider_than_twelve_hundred_months_is_refused…` | no |
| 73 | crates/cli/src/basis_runs.rs:246 | `basis_runs::window_months` | "O(1): one subtraction of two ordinals" | months | same | no |
| 74 | crates/cli/src/basis_runs.rs:315 | `Gaps::count_before` | "O(1): one clamp and one read" | window months | test `the_months_missing_inside_a_steps_window_are_two_reads_never_a_search` | no |
| 75 | crates/cli/src/basis_runs.rs:328 | `Gaps::between` (pub(crate)) | "Two reads of the counts, then the slice. O(1)" | window months (≤1,200) | test `a_steps_missing_months_cost_the_same_at_12_120_and_1200_months_measured` (p50 ≤4x) | no |
| 76 | crates/cli/src/basis_runs.rs:954 | `window_note` | "O(1) to find the months, then one word per month named" | months | same | no |
| 77 | crates/cli/src/feed_reconcile.rs:96 (also :1234) | `Seconds::put` (one GDFL 1s bar) | "One constant-time update per bar into a slot addressed by the bar's own offset" | bars/seconds | none (timings printed, "no test or bench gates") | no |
| 78 | crates/cli/src/feed_reconcile.rs:1407 | `Scratch::add_minute` (one Zerodha minute) | "One Zerodha minute. Constant time." | minutes | none | no |
| 79 | crates/cli/src/feed_reconcile.rs:1102 | `Running::admit` | "O(1): three fraction comparisons by i128 cross-multiplication and no loop" | matched minutes | none | no |
| 80 | crates/cli/src/feed_reconcile.rs:1281 | `Seconds::last_in` | "O(1) after `fill`: one slot read and no loop" | seconds | none | no |
| 81 | crates/cli/src/feed_reconcile.rs:3932 (also :3869) | `feed_reconcile::FactorIndex::factor_of` (pub) / `load` | "O(1): one hash and one fixed-stride index" | symbols × days | `factor --repeat N` prints p50/p99/max (not gated) | no (std `HashMap<String,usize>`, clones one cell). `load` is an extra full pass (counts `# carry,` lines) — not claimed O(1). |
| 82 | crates/cli/src/feed_reconcile.rs:753 | `feed_reconcile::chosen` | "Membership is one probe of a hash set sized from the listing" | listed shares | none | no |
| 83 | crates/cli/src/feed_reconcile.rs:349 | `MAX_REPEAT` timing buffer | "the bound is that buffer, 8 MB" | — | — | n/a (input cap; listed for completeness of the "p99" grep). |
| 84 | crates/cli/src/tick_second_check.rs:104 | per-row check loop | "one decoder step, one census step and a constant number of state updates" | rows | timings files only | no |
| 85 | crates/cli/src/tick_second_index.rs:3 (also :43, :617) | `tick_second_check::index::YearIndex::status_of` (pub) | "one hash probe for the symbol, one subtraction … one 20-byte read: O(1)" | symbols (1 vs 4,096) | test `tick_second_check::tests::a_status_lookup_costs_the_same_on_one_symbol_and_on_four_thousand` (p50 ≤16x) | no |
| 86 | crates/cli/src/tick_second_index.rs:808-819 | `SecondChecks::status_of` (pub) | "the year's index is found in a map of at most one entry per calendar year" | years held | none (test drives `YearIndex` only) | maybe: `BTreeMap<u16, YearIndex>` (:649) ⇒ O(log years) per lookup; years = files on disk. |
| 87 | crates/cli/src/watchdog_command.rs:51 | one `cli watchdog` run | "O(1) in the length of the log and the number of pages" | log length, notice pages | test `a_run_never_lists_the_notices_or_reads_the_log` | no (≤16 create_new attempts; drive check is one stat per missing path component). |
| 88 | crates/cli/src/watchdog_command.rs:426 | state read (`MAX_STATE_BYTES`) | "the read is O(1) whatever was left at the name" | state file size | test `a_state_that_is_binary_huge_or_a_directory_is_kept_aside…` | no |
| 89 | crates/cli/src/watchdog_command.rs:887 | `stamp_attempt` | "O(1): one 41-byte write at offset 0 and one sync" | — | none | no |
| 90 | crates/cli/src/watchdog_command.rs:909 | `read_attempt` | "0, whatever else the file holds. O(1)." | lock file size | none | no |
| 91 | crates/cli/src/watchdog_command.rs:948 | `attempt_of` | "O(1): one bounded read" | — | none | no |
| 92 | crates/cli/src/vix_reference.rs:21 (also :221) | `vix_reference::VixReferenceMonth::stamp` (:230, pub) | "Constant arithmetic and one fixed-slot read, independent of how many … records" | VIX records per month | none | no |
| 93 | crates/cli/src/operation_audit.rs:15 | `operation_audit::begin` / `read` / `completed_boundary` (pub) | "Appends and exact reads have fixed work" | invocations | none | maybe: `begin` creates one journal file per invocation in ONE directory (`own(&base,id)`) + directory fsync; create/lookup cost is filesystem- and directory-size-dependent; `read` calls `sync_all` on both files on a READ path. |
| 94 | crates/cli/src/operation_audit.rs:539 | `RunClaim::take` / `Drop` | "O(1): one `compare_exchange` to take, one to release." | — | none | no |
| 95 | crates/cli/src/operation_audit.rs:748 | `operation_audit::page` | "newest-first page of invocation IDs without scanning directories" | invocations | none | no |
| 96 | crates/cli/src/sweep_evidence.rs:952 | `sweep_evidence::read` (pub) | "Read the latest attempt for one identity without scanning its history." | attempts | tests in crates/cli/tests/sweep_evidence.rs (correctness) | no (last fixed-stride record by length). |
| 97 | crates/cli/src/selection_v6_read.rs:201 | `read_stored_selection_v6` (pub) | "A page costs O(limit), whatever the file holds." | blocks in file | test `api::selectionv6json::tests::the_page_selectors_are_bounded_and_strict`, `cli::selection_v6::tests` | no |
| 98 | crates/cli/src/candidate_universe.rs:13 (also :19) | Candidate ledger audit lookup / row seek | "a structural audit lookup and one row seek are O(1) in record count" | records | none | no |
| 99 | crates/cli/src/candidate_universe.rs:2757 (also :2764, :2769, :9555) | `produce_candidate_universe_v1` | "the O(1) run seal … retained rows grow geometrically (amortised O(1) per row)" | candidates × grid cells | test `candidate_production_seals_the_execution_series_once_for_every_closed_mask` | yes (admitted for whole op); amortised append only. |
| 100 | crates/cli/src/candidate_universe.rs:4156 | `session_close_minute_v1` | "O(1): one bounded calendar lookup plus a walk over at most MAX_WINDOWS" | — | test `session_close_minute_clamps_to_every_measured_window_end` | no |
| 101 | crates/cli/src/candidate_universe.rs:5004 (also :9521) | `reserve_directional_grid_rows` | "`try_reserve` keeps `Vec`'s amortised-O(1) append" | retained rows | test `retained_candidate_rows_grow_geometrically_per_directional_grid` (≤7 reallocations for 64 grids) | maybe: amortised; each doubling copies every retained row (p100 spike that grows with the population). |
| 102 | crates/cli/src/population_admission_writer.rs:343 | `derive_strategy_digest_v1` (pub) | "O(G) per call … `…from_validated_v1` … is O(1) per cell" | grid cells | none | yes (admitted): pub entry is O(G), O(G²) over a grid. |
| 103 | crates/cli/src/population_admission_writer.rs:389 | `derive_strategy_digest_from_validated_v1` (pub(crate)) | "O(1) per cell" (stated at :343) | grid cells | none | no (pointer-eq capability check, stored digests). |
| 104 | crates/cli/src/population_admission_writer.rs:867 | `evaluated_sides` push | "amortised O(1) append §3 rule 4 names" | population sides | none | no (amortised). |
| 105 | crates/cli/src/execution_capability.rs:20 (also :1593) | `ExecutionCapabilityLedger::completion` (pub) | "One average-O(1) completion lookup after constant-size generation checks." | records | none | no (4× fstat+stat, flock, std HashMap). |
| 106 | crates/cli/src/execution_capability.rs:1616 | `…::parameters` | "One average-O(1) exact side-parameter lookup." | records | none | no |
| 107 | crates/cli/src/execution_capability.rs:1653 | `…::capability` | "One average-O(1) committed row-capability lookup." | records | none | no |
| 108 | crates/cli/src/execution_capability.rs:993 (also :1481) | `from_population_v4` / `open_read` | "O(rows), not O(1); only a later indexed row probe is average O(1)" | rows | none | yes (admitted). |
| 109 | crates/cli/src/execution_disposition_v2.rs:23 (also :2213) | `ExecutionDispositionLedgerV2::completion` | "Average-O(1) complete-authority lookup after two generation checks." | records | none | no (metadata-only `require_generations_unchanged` :2517). |
| 110 | crates/cli/src/execution_disposition_v2.rs:2258 | `…::row` | "Average-O(1) lookup of one exact committed Population V4 row." | rows | none | no |
| 111 | crates/cli/src/admission_store.rs:23 | `admission_store` `completion`/`decision` (:770/:791) | "A committed lookup is one average-O(1) hash probe plus one fixed seek/read" | decisions | none | no |
| 112 | crates/cli/src/admission_join.rs:15 | admission join receipt probes | "Receipt probes are average O(1)." | receipts | none | no |
| 113 | crates/cli/src/selection.rs:1880 / :1892 / :2218 / :2228 | `SelectionLedgerV1/V2::receipt`, `latest_selection` | "one average-O(1) hash lookup" | receipts | none | no |
| 114 | crates/cli/src/selection_v3.rs:16 (also :1225) | `SelectionLedgerV3::receipt` | "exact and latest lookup after open are one average-O(1) hash probe" | receipts | none | no |
| 115 | crates/cli/src/selection_v4.rs:30 (also :1800) | `SelectionLedgerV4::receipt` | "Average-O(1) exact receipt lookup; no worst-case O(1) claim" | receipts | none | no |
| 116 | crates/cli/src/selection_v4_authority.rs:15 | `SelectionAuthorityLedgerV4` lookups | "completion lookup inside an already-open ledger is average O(1)" | records | none | no |
| 117 | crates/cli/src/selection_v5.rs:26 (also :4420) | Selection V5 lookup | "The probe is O(1); the lookup is not." | file bytes | doc-pin test (:4420-4424) | yes (admitted): lookup revalidates the bounded file first. |
| 118 | crates/cli/src/population_admission_v2.rs:16 (also :1870, :2101, :2107, :2148, :2225, :2474) | `PopulationAdmissionV2Ledger::reopen_structural_receipt`, `append`, `reuse_existing`, `authenticate_structural_receipt` | "average O(1) lookup only after O(file) generation validation" | file bytes / decisions | none | yes (admitted): public lookup is O(file). |
| 119 | crates/cli/src/population_admission_v3.rs:15 (also :3505, :3728) | `PopulationAdmissionV3Ledger::reopen_structural_receipt` | "Revalidates all bounded file bytes, then performs one average-O(1)" | file bytes | none | yes (admitted). |
| 120 | crates/cli/src/population_admission_v4.rs:19 | Admission V4 fixed-record offset | "fixed-record offset is constant in record count; filesystem latency is not" | records | none | no |
| 121 | crates/cli/src/population_finalization_v2.rs:1174 (also :1176, :1426, :1432) | `PopulationFinalizationV2Ledger::reopen_structural_receipt` (pub) | "The in-memory identity probe is average O(1) only after that scan" | file bytes | none | yes (admitted). |
| 122 | crates/cli/src/population_finalization_v3.rs:25 (also :1175, :1905, :2124) | `row_projection`, `reopen_structural_receipt` | "O(row-file bytes + Completion-file bytes) per row, not O(1)" | file bytes | crates/cli/tests/ledger_scan_costs.rs (doc pin) | yes (admitted): per-row projection re-hashes both files. |
| 123 | crates/cli/src/population_finalization_v4.rs:18 | Finalization V4 offset / index | "fixed-record offset arithmetic is worst-case O(1); … index has average O(1)" | records | crates/cli/tests/ledger_scan_costs.rs `a_finalization_v4_append_rescans_the_whole_ledger_and_its_docs_say_so` | no (append rescans, admitted). |
| 124 | crates/cli/src/population_v5.rs:22 (also :2297-2298, :3004, :3130) | `PopulationV5Ledger` lookup / `authenticated_row` / `commit_population_v5` | "expected/amortized O(1), with O(B) collision worst" | file bytes / records | ledger_scan_costs.rs `a_population_v5_commit_opens_appends_and_reopens_and_hashes_whole_files` | yes (admitted). |
| 125 | crates/cli/src/population_v6.rs:3228 (also :4074) | `CommittedStoredPopulationV6::selected_stored_oos_witnesses` | "Not O(1) per strategy … linear `find` over the C rows" | candidates C | doc-pin test (:4074) | yes (admitted): O(25·C) per call. |
| 126 | crates/cli/src/population_v6.rs:2632 | `PopulationV6CandidateAuthoritiesV1` boxing | "not a per-operation cost, so it does not touch the O(1) claim" | — | none | no |
| 127 | crates/cli/src/population_statistics_v2.rs:25 (also :1514, :2567, :4014) | statistics V2 lookups / `admission_candidates_v3` / `prepare_single_family_statistics_v3` | "fixed-record … O(1) in record count. Hash-map lookup is average O(1)" | records | none | yes (admitted for whole-family work). |
| 128 | crates/cli/src/population_statistics_v3.rs:16 | Statistics V3 offset | "Only fixed-record offset arithmetic is worst-case O(1)." | records | none | no |
| 129 | crates/cli/src/population_observations_v1.rs:26 (also :2231, :3525) | `reopen_audit` (pub) | "only the identity-map probe that follows is average O(1)" | file bytes | crates/cli/tests/ledger_append_lookup_costs.rs `observation_lookups_still_hash_the_whole_file_and_say_so` | yes (admitted): lookup re-hashes the file. |
| 130 | crates/cli/src/population_base_evidence_ledger_v2.rs:9 (also :473) | `BaseEvidenceLedgerReaderV2::audit` / `record` | "completion lookup and a fixed-offset record lookup are O(1) in record count" | records | none | no (metadata `require_generation`, std HashMap). |
| 131 | crates/cli/src/population_base_evidence_v2.rs:21 | Base Evidence lookup | "Only lookup and fixed-offset record read are O(1) after …" | records | none | no |
| 132 | crates/cli/src/anchored_search_lineage_v2.rs:20 (also :741) | `structural_receipt` | "Generation validation is O(file bytes); only the final hash lookup is average O(1)" | file bytes | none | yes (admitted). |
| 133 | crates/cli/src/anchored_search_lineage_v3.rs:20 (also :796) | `structural_receipt` | same as v2 | file bytes | none | yes (admitted). |
| 134 | crates/cli/src/anchored_search_lineage_v4.rs:18 | v4 receipt lookup | "in-memory receipt lookup is average O(1) after bounded validation" | file bytes | none | yes (admitted). |
| 135 | crates/cli/src/execution_v3.rs:28 / execution_v4.rs:31 | Execution V3/V4 lookups | "authenticated lookup deliberately rehashes all retained bounded files and is O(F)" | file bytes | none | yes (admitted). |
| 136 | crates/cli/src/global_replay.rs:23 (v2 :35, v3 :33) | global replay lookups / merge | "Only an already-indexed execution-capability lookup is average O(1)"; "at most 200 stream heads per emitted minute" | streams/minutes | none | no (fixed 200-stream surface). |
| 137 | crates/cli/src/global_replay_v4.rs:539 | `StreamQuality::absorb` | "each admitted priceable trade adds in O(1)" | trades | test `admitted_quality_is_summed_per_stream_and_refused_past_its_frozen_ceilings` | no |
| 138 | crates/cli/src/checksum_receipts.rs:4 | checksum receipts warm read | "Warm reads check one format block and one fixed receipt" | receipts | none | no (cold admission O(source), admitted). |
| 139 | crates/cli/src/institutional_evidence.rs:41 (also :42, :48) | cell ordinal check / cell projection | "each cell's ordinal check is O(1) … cell projection is not O(1)" | cells × execution bars | none | yes (admitted): projection O(E) per cell ⇒ O(C·E). |
| 140 | crates/cli/src/institutional_statistics.rs:25 | Statistics prep / record encoding | "Preparation and fixed-record encoding are O(1) over an already-computed receipt" | — | none | no |
| 141 | crates/cli/src/all_rung_population_v5.rs:545 (also :547, :724) | `commit_all_rung_stored_population_v5`, `commit_all_rung_stored_execution_v3` | "Fixed rung/family dispatch and root lookup are O(1)" | rungs | none | no (whole op admitted not O(1)). |
| 142 | crates/cli/src/all_rung_selection_v5.rs:13 | eight-call dispatch | "The fixed eight-call dispatch is O(1) in rung count." | rungs | none | no |
| 143 | crates/cli/src/step3_orchestrator.rs:953 (also :1118, :1202, :1724, :2170, :3044) | Step-3 commits (`commit_stored_*`, `population_v5_inputs`) | "only each already-authenticated fixed-record address is O(1)" | file bytes / candidates | none | yes (admitted). |
| 144 | crates/cli/src/pre_admission_data.rs:35 (also :3732) | fixed-record offset; `require_metadata_generation` | "It reads no content, so it is O(1) in file bytes" | file bytes | test `pages_hash_no_file_and_reseal_only_the_records_they_return` (:5119) | no (cached audit lookup hashes the file — admitted at :30). |
| 145 | crates/cli/src/candidate_trades.rs:841 (also :16-24) | `candidate_trades::pinned` (warm page path) | "Warm: one shared-lock open plus an fstat/stat … O(1) in C" | captured candidate sides C | test `a_summary_hashes_its_catalog_once_across_pages_and_still_refuses_a_change` (CUH-06, counts verifications) | maybe: cold first page per `Summary` is O(C) (whole catalog BLAKE3); warm path O(1). |
| 146 | crates/cli/src/index_stop_store.rs:170 | `index_stop_store::Reader::record` (pub) | "Exact pinned O(1) record lookup after cold admission." | records | none ("UNVERIFIED performance") | no (lease + metadata checks + Vec index). |
| 147 | crates/cli/src/index_consistency_store.rs:169 | `index_consistency_store::Reader::record` (pub) | "Look up the exact saved setting in constant indexing work." | records | none | no |
| 148 | crates/cli/src/index_stop_search_reader.rs:360 (also :695) | `Reader::latest_checkpoint` / `require_current` | "bounded cold work, not a constant-time index"; "O(checkpoints + families)" | checkpoints | none | yes (admitted). |
| 149 | crates/cli/src/index_stop_qualification.rs:9 | warm row lookup | "A warm row lookup checks fixed ancestry generations and indexes the retained rows" | rows | none | no |
| 150 | crates/cli/src/search_checkpoint.rs:4 | `search_checkpoint::Journal::publish` (pub(crate)) | "Cold discovery scans bounded directory entries; publishing does not rescan history." | checkpoints (≤1,000,000 dir entries) | none | maybe: each publish is `create_dir` + directory `fsync` inside one namespace directory of up to `DIRECTORY_LIMIT` = 1,000,000 entries (FS-dependent); resume is O(entries) (admitted). |
| 151 | crates/cli/src/boolean_grammar_campaign.rs:350 | `restore` (resume) | "Cold recovery is explicitly O(checkpoints + replayed grammar work + saved campaign evidence)" | completed batches | crates/cli/tests/c4_cli_02_limits.rs (doc pin) | yes (admitted): invocation n re-verifies n−1 batches ⇒ quadratic over a campaign. |
| 152 | crates/cli/src/live.rs:15-45 | `live::Live::publish` rewrite count (`expected_rewrites` :1078) | "The cost is logarithmic in the search and the heap settles early" | candidates N | test of `expected_rewrites` arithmetic | maybe: keep·ln(N/keep) is an expectation under random arrival order (ascending-|t| order ⇒ N rewrites); moot today — the only call site publishes once (lib.rs:21234). |
| 153 | crates/cli/src/live.rs:888 | `live::CensusCache::refresh` | "Cost includes directory entries and returned rows" | live files | none | yes (admitted): BTreeMap + directory walk. |
| 154 | crates/cli/src/research.rs:99 | `research::Window::retain` | "O(bars) setup work, NOT an O(1) whole-slice claim" | bars | none | n/a (anti-claim). |
| 155 | crates/cli/src/research_policy.rs:562 | `percentage_statistic` | "O(1) UNVERIFIED: one `match` on two small enums" | — | none | no |
| 156 | crates/cli/src/msum_builder.rs:56 | `cli msum-build` fold | "reads every committed bar once … and folds each in O(1)" | bars | none | no |
| 157 | crates/cli/src/gdfl_import_guard.rs:26 (also :780) | `gdfl_import_guard::tail_for` | "at most TAIL_WINDOW bytes, never the whole file" | sources-ledger length | test `door_costs_on_a_real_layout` (#[ignore], asserts p99 < 1,000,000 µs) | no (bounded 1 MiB pread + scan). |
| 158 | crates/cli/src/gdfl_rekey.rs:350 | `gdfl_rekey::unwalked` | "O(keys) once: one set of the walked underlyings, one probe per name" | keys | none | yes (minor): `BTreeSet` ⇒ O(K log K) build, O(log K) per probe. |
| 159 | crates/cli/src/ledger_all.rs:649 | `ledger-all` / `ledger-v6` event emission | "bounded by constants in this file rather than by the operator's data" (17 / 43 events) | months / candidates | none | no |
| 160 | crates/cli/src/zerodha_quarantine.rs:23 | `check_listing` → `pull::listing::for_store(..).span(..)` | "`pull::listing`, O(1) per symbol, UNVERIFIED" | symbols | none | maybe: depends on `pull` (`Listing::span` → master `inspect` → lifecycle lookup); not verified here. |
| 161 | crates/cli/src/population.rs:44 (also :3883, :2587) | `PopulationLedger::page` / `open_read` | "Finding the block is one hash probe. Reading … O(returned rows)" | populations / rows | none | no (one unbuffered `read_exact` per row; open O(rows+receipts) admitted). |
| 162 | crates/cli/src/pool_oos_tests.rs:227 | `pool_oos` streamed judge (space) | "holds no span and no training series" (Rust and O(1) sweep OS-1/OS-2) | span bars (space) | test `the_streamed_judge_books_every_trade_and_keeps_no_training_series` | no (space claim, test-pinned). |
| 163 | crates/cli/tests/limits_doc_drift.rs:7 (also :101-106) | doc pin of runner execution walk | "amortised O(1) per execution bar … not worst-case O(1) per cut" | execution bars | the test itself (doc text only) | n/a (pins `docs/06-limits.md`, not code). |
| 164 | crates/cli/tests/ledger_scan_costs.rs:187 / :380; tests/ledger_append_lookup_costs.rs:10/:148/:151 | doc pins | "O(row-file bytes + Completion-file bytes) per row, not O(1)" etc. | file bytes | the tests (doc text) | n/a (anti-claims pinned). |
| 165 | crates/cli/benches/ratio.rs:41 | bench header | "O(1) amortised — a `HashSet` of identities, built once at open" | records | C-CLI-03 | stale wording (now a `HashMap` with offsets); see row 6 for the bench blind spot. |

### Notes

* Bench fixture (`crates/cli/benches/ratio.rs`, `harness = false`, imports only `cli::results::{Record, Results, field}`): `fn record(n: u64) -> Record` (identity = n LE in the first 8 bytes, everything else fixed), `fn ledger(tag: &str, n: u64) -> (Results, PathBuf)` (fresh `temp_dir()/brutex-cli-bench-{tag}-{pid}`, `Results::open`, n appends), `fn cost_ps<T>(reps: u32, op: impl FnMut() -> T) -> u128` (min over `TRIALS`=40), `fn floor_ps() -> u128`, `fn ratio(label: &str, base_ps: u128, at_ps: u128) -> bool` (ceiling `CEILING_PERMILLE` = 2,500), `fn budget(label: &str, floor: u128, at_ps: u128, allowed: u128) -> bool`, `fn per(reps: u32, elapsed_ns: u128) -> u128`; sizes `SMALL`=64, `LARGE`=1,024.
* `cli` has a lib target (`crates/cli/src/lib.rs`, crate name `cli`) plus `main.rs`. Benches/tests may also use the package's normal deps (`store`, `pull`, `brutex_core`, `runner`, …).
* No bench row covers frontier, trades, result_set, knobs, stored, minute_gaps, calendar_days, basis_runs, feed_reconcile, tick_second_index, pool or any of the versioned ledgers. Timing tests that exist assert p50/p99 against generous absolutes (2 ms/day, 100 µs/day, 1 s) or p50 ratios (4x, 16x), and several are `#[ignore]`.

## O(1)/flat/p99 claim inventory — ten smaller crates

Branch `cloud/o1p99-attack-20261110`. Read-only audit; no cargo run. One row per (function, claim); repeated comment lines about one function are collapsed onto the first line. "ack" = the text itself admits the cost is not O(1) (listed so nothing is missed). Paths are repo-relative.

**Cross-cutting method note.** Almost every gated ratio row times `min over TRIALS of a per-call MEAN` (store TRIALS=60, telemetry 20, lake 40, engine `once_ps`), so a cost paid once in N calls (rehash, compaction, cold block, fsync, roll) is discarded by construction. Only C-T-01b gates a p99; C-MO-\*, C-VD-\*, C-QU-\*, C-SF-\*, C-AU-\*, C-CHG-\*, C-HA-01, C-RP-01 print p50/p99/max and (mostly) do not gate them. Most fixtures re-read one hot index (page/cache-warm) and use ≤100k–250k elements.

### store (63 rows)

| # | where | what | claim (≤15 words) | flat in | bench | suspect |
|---|---|---|---|---|---|---|
| S1 | crates/store/src/file.rs:3477 | store::file::BarFile::read_record | "Reads one record by index, in O(1)." | records in file | C-28/C-29 (warm, one fixed index), C-BC-01/02 (cold, 14 blocks cycled), C-MO-02 p99 (reader handle) | maybe: warm = one Mutex + memcpy; cold = pread ≤4,088 B + pread 4 B sidecar + CRC-32C, + fstat on the tail block (file.rs:4391). One-block cache per handle (file.rs:4172-4182): two threads/readers sharing one handle on different blocks make every read cold and contend on the Mutex. Fixtures ≤100k records (5.6 MB, page-cache warm); a cold device is UNVERIFIED (file.rs:3496). |
| S2 | crates/store/src/file.rs:4234 | BarFile::verify_block_of | "bounded by the block ... not a function of the file" | file size | C-BC-01/02 | no (≤ Layout::block_len; tail adds fstat + ≤1 block read). |
| S3 | crates/store/src/file.rs:1514 | BarFile `verified` cache / validated open | "no allocation at open and none ever ... keeps the O(1) open" | blocks in file | C-MO-01 (open+read+tix vs 1/10/100 months); test file.rs:9094 | no for open; cache thrash see S1. |
| S4 | crates/store/src/file.rs:4088 | BarFile::read_row | stack buffer, "constant per operation" | records | C-28 | no. |
| S5 | crates/store/src/file.rs:3519 | BarFile::first_at_or_after (+ locate, file.rs:3579) | "A constant number of reads, through the time index" | bars in month | C-TIX-01/02 (1,024/10,240/102,400 bars), C-MO-01/02; test time_index every_lookup_reads_at_most_one_entry_and_one_bar | yes: silent per-lookup fallback to the D-1434 bisection (⌈log2 n⌉ cold block verifies) when ONE `.tix` entry fails its checksum (`Fault::Index` → `bisect()`, file.rs:3612-3615); `time_index::confirm` checks only the first and last entries (time_index.rs:566), so a damaged middle entry survives open and every lookup in that bucket bisects; each such lookup also takes the global WARNED mutex (S13). Also bisects for legacy/absent/stale `.tix`. |
| S6 | crates/store/src/file.rs:1652 | TimeLookup::Indexed / time_index::locate (time_index.rs:529) | "at most one entry read and one bar read per lookup, whatever the month holds" | bars | C-TIX-01/02 | no (when Ready). |
| S7 | crates/store/src/time_index.rs:86 | INDEX_MIN_ROWS small-month bisection | "ceil(log2(1_024)) = 10 record reads, measured 6.3 µs p50, 10.7 µs p99" | bars <1,024 | test; D-4933 | no (≤10 reads). |
| S8 | crates/store/src/time_index.rs:566 | time_index::confirm | "Two entry reads, whatever the month holds." | bars | none | no for cost; weak validation enables S5. |
| S9 | crates/store/src/time_index.rs:~620 (extend, fn at 644) | time_index::extend (append's index step) | "a gap of g slots costs g / 64 entries" | gap since last bar | none (C-MO-03 uses gap 1) | yes: a one-bar commit after a long gap writes and syncs ⌈g/64⌉ 16-B entries — ~41,850 entries (≈670 KB) for one bar at the end of an otherwise early 1-s month (put_entries + sync_index, file.rs:4063-4071). |
| S10 | crates/store/src/file.rs:4613 | store::file::Admission::admit | "O(1) per bar — two comparisons and one remainder — so O(batch) per append" | month | none (UNVERIFIED) | no. |
| S11 | crates/store/src/file.rs:3234 | BarFile::append (one-bar commit) | O(batch) per append (S10); C-MO-03 prints p50/p99/max | bars in month | C-MO-03 (printed, never gated; dense gap-1 bars) | yes: (a) `.srt` rewritten whole every COMPACT_AT=32 runs, O(n log n) (sort_index_writer.rs:36-41, plan 215-224); (b) `.sum` rebuilt whole, O(n_valid) verified reads, on EVERY commit that takes a <60 s month back over the ½-density line (month_summary_writer.rs:202-210) — alternating gap-3 / gap-1 one-bar commits trigger it every other commit; (c) S9; (d) per-commit syncs. |
| S12 | crates/store/src/month_summary.rs:62 | month summary writer (extend_summary / whole_summary) | "pays that on every commit ... It is not amortised away." | bars | none | ack — and it is the S11(b) attack. |
| S13 | crates/store/src/file.rs:4946 | WARNED / first_warning / note_lookup_bisects | "One lock and one hash insert per bisecting DECISION ... O(1) expected" | months opened | none (UNVERIFIED) | maybe: also called per lookup on `Fault::Index` (file.rs:3613); each call allocates a PathBuf, locks a process-global Mutex, SipHashes the whole path; set never shrinks (months × Why variants, process lifetime), not pre-sized → rehash spikes. |
| S14 | crates/store/src/file.rs:4731 | already_stored (re-offer overlap) | "This is NOT an O(1) path and does not claim to be." | batch | test locating_a_batch_costs_a_bisection | ack (locate + O(batch)). |
| S15 | crates/store/src/file.rs:5700 | DurableBatch::confirm_month | "O(1): one lstat per sibling kind ... one set insert per directory level" | months in batch | none (UNVERIFIED) | maybe: `confirmed`/`directories` HashSet<PathBuf> (file.rs:5504-5513) grow with the batch, unsized → O(paths × path-len) rehash at 7·2^k; ancestor walk hashes each ancestor path. |
| S16 | crates/store/src/file_sealed.rs:296 | BarFile::read_rows_at | "Every step is O(1) a row, expected" | rows asked | test sealed_door_tests::read_rows_at_timing | no (HashMap<u64,usize> SipHash, with_capacity(rows), file_sealed.rs:322). |
| S17 | crates/store/src/sealed.rs:2255 | SealedMonth::first_at_or_after (lookup) | "one read of two table entries names the at most 17 blocks" | bars | none | no (bisects ≤17/≤64 dir entries + partition_point over ≤256 stamps; allocates 2 Vecs per lookup). |
| S18 | crates/store/src/sealed_codec.rs:621 | sealed_codec::decode_frame / decode_stamps | "amortised O(1) a row" | rows | tests sealed_codec_alloc | no (≤MAX_FRAME_LEN 14,508 B). |
| S19 | crates/store/src/layout.rs:440 | Layout::block_of | "makes ... 'verification is O(1) per read' true" | records | test geometry | no. |
| S20 | crates/store/src/format.rs:11 | format module | "the only thing that makes bar lookup O(1)" | records | C-28 | no. |
| S21 | crates/store/src/path.rs:617 | YearMonth::ist_bounds_micros | "# O(1) ... no table, no loop" | — | none (UNVERIFIED) | no. |
| S22 | crates/store/src/path.rs:68 | StorePath::new / Display | "bounded scan ... constant, independent of how many instruments" | instruments, months | none | no. |
| S23 | crates/store/src/header.rs:556 | Header::read_region (fn 599) | "does not scan past MAX_SLOTS positions" | region (mapping) size | C-01 | no. |
| S24 | crates/store/src/header.rs:1062 | header::slot_audit (fn 1102) | "a whole mapping costs what the header region costs" | region | C-HA-01 (ratio + p50/p99/max printed) | no. |
| S25 | crates/store/benches/ratio.rs:645 | store::block::seal (block.rs:75) | "sealing one block costs the same whatever the file holds" | file | C-07 | no. |
| S26 | crates/store/src/block.rs:256 | block::verify_through | "Bounded by Layout::block_len, not by the file." | file | C-BC-01 (indirect) | no (≤73 CRC extensions). |
| S27 | crates/store/src/crc.rs:34 | crc::crc32c | "O(n) in the bytes it covers and always will be" | — | C-08 | ack. |
| S28 | crates/store/src/verdict.rs:54 | verdict::Reader::lookup (fn 3911) | "at most 5 times and opens at most 2 files, whatever the day size" | records/day, days, history | C-VD-01 (cold/warm 1x/10x/50x, colliding keys, 248-day sweep p50/p99/max); test IA-VD-01 | no for the probe (BLAKE3 key, fixed 8-slot window); minor: when the 64-handle cache is full a miss does a `min_by_key` scan of 64 + PathBuf clone + remove/insert (verdict.rs:3852-3866). |
| S29 | crates/store/src/verdict.rs:3877 | verdict::Reader::year | "O(1): 2 reads of 8,812 bytes" | days in year | C-VD-02 | no. |
| S30 | crates/store/src/verdict.rs:62 | verdict::commit / table (fn 2075) / place | "is not constant: it sorts ... O(n log n)" | records/day | C-VD-04 (printed) | ack; maybe: `table()` re-runs `place` at every doubling up to MAX_CAPACITY=2^20 (verdict.rs:2075-2088); key sets whose BLAKE3 low bits cluster (≥9 in an 8-slot window, or a repeated 32-bit tag) force ~2^21 slot work then `table_too_large`. |
| S31 | crates/store/src/verdict.rs:1234 | DayHeader::mismatch_slices | "O(1) from the header's counts" | records | test IA-VD-06 | no. |
| S32 | crates/store/src/quarantine.rs:145 | quarantine::probe (fn ~786) | "two positional reads ... — O(1)" | months quarantined | C-QU-01 (p50/p99/max, hit+miss, 1/10/100x) | no (BLAKE3 bucket of 8×64 B; open+fstat+lstat+2 preads per call, uncached). |
| S33 | crates/store/src/quarantine.rs:152 | quarantine::Snapshot::load + lookup | "one read of the whole index ... each lookup after it O(1)" | — | C-QU-02 | no (HashMap with_capacity, SipHash, BLAKE3 key). |
| S34 | crates/store/src/quarantine.rs:1937 | quarantine::set_aside | "O(1) for a month no quarantine names ... O(ledger) for one a drop names" | ledger lines | none | ack: ledger is append-only and never compacted, so a named month's repair door re-reads an ever-growing ledger. |
| S35 | crates/store/src/quarantine.rs:152 | quarantine::record | "O(index) on the write path, never on a read" | index | none | ack. |
| S36 | crates/store/src/settings.rs:61 | settings::Reader::refresh | "ONE stat ... O(1), independent of how many generations exist" | generations | api `/settings.json` rows (GF-1) | no (first refresh lists the dir, ack). |
| S37 | crates/store/src/settings.rs:599 | settings::Floor::bytes_of | "O(1) (not timed: UNVERIFIED" | — | none | no. |
| S38 | crates/store/src/settings.rs:1442 | settings save read-back (landed) | "one read, O(1)" | faults | none | no. |
| S39 | crates/store/src/sort_index.rs:1050 | sort_index::walk | "MAX_RUNS + 1 header reads at most, whatever the month holds" | month | none | no (bounded; but see S11a). |
| S40 | crates/store/src/sort_index_writer.rs:36 | BarFile::keep_sort_index / plan_sort_index (215) / write_whole | "whole month O(n log n) ... every COMPACT_AT appends" | — | C-MO-03 (printed) | ack — the C-MO-03 max. |
| S41 | crates/store/src/repair.rs:62 | repair::row_ceiling | "O(1): two month bounds and one division, no I/O" | — | test | no. |
| S42 | crates/store/src/repair.rs:370 | repair::RevisionReader::open/read_record/first_at_or_after | "Block CRC validation remains lazy and constant per record read." | — | none | no (delegates to BarFile; inherits S1/S5). |
| S43 | crates/store/src/repair.rs:451 | repair::publish | "reads retain their existing O(1) per-record cost" | — | C-RP-01 | ack O(month) work. |
| S44 | crates/store/src/repair.rs:766 | repair::repair_month | "Not O(1), and not on a read path." | — | C-RP-01 (p50/p99/max, peak RSS) | ack. |
| S45 | crates/store/src/repair.rs:1261 | repair::same_bar | "O(1) UNVERIFIED: a fixed number of field compares" | — | none | no. |
| S46 | crates/store/src/flock.rs:198 | flock HOLDERS registry (take_held/let_go/forget_in) | "one hash probe when a guard starts and one when it ends: O(1) expected" | locks held | none (UNVERIFIED, L-R308-02) | maybe: process-global Mutex held across a flock(2) syscall; writer side formats a thread-name String per acquisition; key is fixed-zero-key SipHash of the path (flock.rs:227); patient queue `Turn::drop` (flock.rs:1076-1090) loops over consecutive given-up tickets in a BTreeSet under the global lock (O(g log g) in one hand-off). |
| S47 | crates/store/src/flock.rs:542 | Flock::try_lock_noted | "One fstat and two pwrite-class calls ... O(1) per acquisition" | — | none (UNVERIFIED) | no. |
| S48 | crates/store/src/staging.rs:58 | staging::create | "four at most on a name that exists ... whatever the leftover holds" | leftover size, dir | C-SF-03 (flat in dir held; 0 B vs 4 MiB printed only) | ack: unlink of a big leftover is 22–30x (L-R316-06). |
| S49 | crates/store/src/safe_open.rs:52 | safe_open::open_read / open_read_len / refuse_linked_store_root | "two host calls ... whatever the file holds" | file size, dir | C-SF-01/02/04 | maybe: "flat in its directory" only tested at 2,000 names; FS lookup at 10^5–10^6 entries untested. |
| S50 | crates/store/src/provenance.rs:97 | provenance MAX_RECORDS / read | "bounds a read: MAX_FILE_LEN is 262,208 bytes, whatever the store holds" | store | none | no (bounded, up to 256 KB). |
| S51 | crates/store/src/generation.rs:62 | generation pointer read / promote | "one fstat and one 64-byte positional read; a torn tail adds one read per skipped record" | repairs of a month | none (UNVERIFIED) | maybe: torn tail → up to MAX_RECORDS reads. |
| S52 | crates/store/src/catalog.rs:23 | catalog::walk | "This is NOT constant time, and says so" | — | — | ack. |
| S53 | crates/store/src/month_summary.rs:577 | MonthSummary::fold | "Folds the next row ... O(1). UNVERIFIED" | bars folded | none | no (fixed ops + bitmap set). |
| S54 | crates/store/src/month_summary.rs:841 | MonthSummary::padded_on | "O(1) for a day, whatever the month holds" | month | none (UNVERIFIED) | maybe: linear in caller's unbounded `windows: &[(u16,u16)]`. |
| S55 | crates/store/src/month_summary.rs:100 | month_summary::read_kept | "one read of the whole file ... decode linear in those bytes" | bars | api `/gaps.json` (MS-10) | no (≤MAX_PAYLOAD_LEN). |
| S56 | crates/store/src/time_index.rs:11 | time_index module | "a FIXED number of reads ... whatever the month holds" | bars | C-TIX | see S5/S9. |
| S57 | crates/store/src/path.rs:707 | FileKind::TimeIndex | "a timestamp into a row in a constant number of reads" | bars | C-TIX | see S5. |
| S58 | crates/store/benches/ratio/months.rs:7 | C-MO-01 open_existing+read+tix | "a reader opens its siblings by exact name and lists no directory" | months in dir | C-MO-01 (min ratio gated + spread) | no. |
| S59 | crates/store/benches/ratio/months.rs:19 | C-MO-03 one-bar commit | "one commit in about thirty pays O(n log n)" | bars | C-MO-03 (printed) | ack (S11). |
| S60 | crates/store/benches/ratio.rs:51 | C-RP-01 repair_month | "NOT O(1) and says so" | — | C-RP-01 | ack. |
| S61 | crates/store/benches/ratio/quarantine.rs:9 | C-QU-03 drop_month | "O(month), not ratioed" | bars | C-QU-03 | ack. |
| S62 | crates/store/benches/ratio.rs:1117 | verdict commit C-VD-04 | "generation_write_stays_n_log_n" | records | C-VD-04 | ack. |
| S63 | crates/store/src/file.rs:9094 | test opening_a_month_verifies_nothing... | "This is Self::validated's O(1) open, asserted" | blocks | test | no. |

### core (14 rows)

| # | where | what | claim | flat in | bench | suspect |
|---|---|---|---|---|---|---|
| C1 | crates/core/src/universe.rs:4224 | core::universe::MemberIndex::contains / position (fn 4433) | "the steps are measured ... hit <= 8 ... miss <= 12" | table/list size | tests the_probe_length_is_bounded_which_is_what_makes_it_o1, a_miss_probes_further_than_a_hit...; NO timing bench | maybe: bounded, but FNV-1a is unkeyed/public, so a ≤24-byte key can be chosen to land on the start of a table's longest cluster (11 compares on NTM_INDEX) vs an empty slot (0); never timed. |
| C2 | crates/core/src/universe.rs:4101 | core::universe::of_equity | "Six hashed probes now ... none of them growing with the list" | lists | none | maybe: 6 FNV hashes + up to 54 str compares (universe.rs:47-52) vs ~0 for a lucky miss; bounded. |
| C3 | crates/core/src/universe.rs:4437 | MemberIndex::position length guard | "A STRING TOO LONG TO BE A SYMBOL ... IS NEVER HASHED" | key length | measured pre-fix in comment (928,287x) | no (guard at SYMBOL_CAPACITY=24). |
| C4 | crates/core/src/universe.rs:4477 | MemberIndex::len | "A field read, not a walk of the slots." | table | none (UNVERIFIED) | no. |
| C5 | crates/core/src/universe.rs:100 | core::universe::nse_isin | "reads them in constant time" | — | none | no (= C1). |
| C6 | crates/core/src/instrument.rs:340 | InstrumentKey::is_sweepable | "the only true worst-case O(1) membership structure in the workspace" | — | tests | no (= C1 on FNO_INDEX, ≤10 miss steps). |
| C7 | crates/core/src/instrument.rs:259 | InstrumentKey::SWEPT / FNO membership | "probed in O(1) through FNO_INDEX" | — | tests | no. |
| C8 | crates/core/src/symbol.rs:1 | core::symbol::Symbol (Hash) | "hashing one is genuinely constant time" | input length | test hashing_feeds_the_same_number_of_bytes... | no (25-byte struct; every std map uses keyed SipHash). |
| C9 | crates/core/src/instrument.rs:18 | InstrumentKey (Hash) | "a key hashes in a constant number of machine words" | vendor | same test | no. |
| C10 | crates/core/src/isin.rs:49 | core::isin::Isin (Hash) | "Fixed 12 ASCII bytes ... hashing one is a constant number" | — | test | no. |
| C11 | crates/core/src/vendor.rs:22 | core::vendor::decode_master_row (fn 1565) | "it is O(1) per row" | field width | C-09 (28 B / bound / 100x / 4 MiB), C-09b budget, C-10 refusal | no (over_wide gate first). |
| C12 | crates/core/src/vendor.rs:1349 | Vendor::{index_segment_word, is_known_foreign_exchange (1410), index_alias (1480), index_alias_source (1506)}, unread_master_code (1537) | "Constant time: a match over string literals, not a scan." | — | none | no. |
| C13 | crates/core/src/vendor.rs:1127 | series tables EQUITY_BOARD_INDEX / SME_BOARD_INDEX / NON_EQUITY_INDEX | "Constant, and it does not grow with the lists." | — | test the_series_tables_probe_in_bounded_time (≤8) | no. |
| C14 | crates/core/src/alias.rs:56 | core::alias lineage lookups (MemberIndex<128>) | "hops are bounded by the table's length"; p50/p99/max over n=200 (1805) | lineage rows | test (printed) | no (compile-time tables; collapse_into fixed buffer). |

### lake (4 rows)

| # | where | what | claim | flat in | bench | suspect |
|---|---|---|---|---|---|---|
| L1 | crates/lake/src/batch.rs:70 | lake::batch::Batch::row | "O(1): one bounds-checked index per column ... on a group of any size" | rows in group | C-L-01 (row 0/last at 2,480/24,800/248,000), C-L-02 (sequential walk per row), C-L-04 budget; test row_lookup_does_not_scan | maybe: ~10 parallel column Vecs; benches re-read the SAME index (hot) or walk sequentially (prefetched). A random-index walk over a 10^7-row batch pays ~10 cache/TLB misses per row → RAM-model O(1) but measured ratio likely ≫3x. |
| L2 | crates/lake/src/page.rs:25 | lake::page decode | "there is no constant-time claim on this path" | page | — | ack. |
| L3 | crates/lake/src/reader.rs:93 | lake::reader::read_bounded / LakeFile::open | "refused past cap before they are read" (O(file)) | file | none | ack. |
| L4 | crates/lake/benches/ratio.rs:240 | lake::contract::ContractName parse | 26-byte vs 4 KiB name | name length | C-L-03 | no. |

### telemetry (19 rows)

| # | where | what | claim | flat in | bench | suspect |
|---|---|---|---|---|---|---|
| T1 | crates/telemetry/src/sink.rs:4 | telemetry::Sink::emit (fn 1662), written info event | "Nothing in that list is a function of how many events came before" | events in file | C-T-01 (min), C-T-01b (p99 gated), C-T-04 (floor budget); worst-case report (rolling / 1-4-8 threads) | maybe: benches emit only a tiny `Event::info`. Per emit: render linear in event width (≤12×512 B + 256 B, JSON-escaped up to ~41 KB), memcpy of the whole line into the 512-line `recent` ring (sink.rs:906-911), a field scan for `code`/`reason`, and once 16 codes are held every NEW code allocates a String + two 16-entry scans (sink.rs:917-930) — contradicts "Zero allocations in steady state" (sink.rs:14). |
| T2 | crates/telemetry/src/sink.rs:46 | Sink::roll (fn 2013), inside emit | "at most 2*keep_files renames and one open ... amortised" | — | worst-case report only (not gated) | yes for max: unlink of the 8 MiB oldest file + renames + open under the sink lock in one emit; keep_files up to 255 (u8). |
| T3 | crates/telemetry/src/sink.rs:720 | telemetry::sink::Flood::admit (fn 726) — warn ration | "O(1): one hash of the pair and one probe; at most once per FLOOD_PAIRS new pairs, one pass" | distinct pairs | none (UNVERIFIED) | maybe: once 128 live pairs (held>0 or inside their 1 s window) fill the table, EVERY warn with a new (target, message) runs `retain` over 128 and is written unfollowed (sink.rs:732-739) — the "once per FLOOD_PAIRS" amortisation fails; message is any caller `&str`. Bounded by 128; never benched (benches emit info only). Key = fixed-zero-key SipHash (DefaultHasher::new) of target+message. |
| T4 | crates/telemetry/src/sink.rs:805 | Counters (count, fn ~871; count_code, 917) | "every update is O(1) or bounded by a constant of this module" | codes, lines | none (UNVERIFIED) | maybe: see T1 (alloc per new code once full; bounded 16). |
| T5 | crates/telemetry/src/sink.rs:1070 | Inner::stamp | "One comparison and one store. O(1)" | — | C-T-01b | no. |
| T6 | crates/telemetry/src/sink.rs:1111 | Sink.run (AtomicU64) | "One relaxed atomic load per event ... O(1), and it cannot grow" | — | C-T-01b | no. |
| T7 | crates/telemetry/src/sink.rs:1432 | Sink::level_for (fn 1435) | "Bounded by MAX_TARGET_LEVELS ... a constant number of prefix comparisons" | subsystems | C-T-02 | no (≤8 × ≤48 B). |
| T8 | crates/telemetry/src/lib.rs:242 | emit_if! (269) / Sink::admits (sink.rs:1640) | "O(call-site fields) ... the only measured O(1) violation"; admits "O(1) in everything" | call-site fields | C-T-02 | no (emit_if closes it). |
| T9 | crates/telemetry/src/sink.rs:1564 | Sink::claim_run / release_run (1591) | "One compare_exchange. O(1), lock-free" | — | none | no. |
| T10 | crates/telemetry/src/sink.rs:1952 | Sink::picture (1954) / recent (1965) | "copy of a fixed-size table: no file is opened, whatever the ring holds" | ring size | test | no (recent() clones ≤512 lines, ≤~21 MB — bounded but large). |
| T11 | crates/telemetry/src/sink.rs:2380 | Audit::append (4380) / Audit::post (4459) — ordinary audited emit | "None of that grows with the stream ... UNVERIFIED as a measurement" | distinct posting values | none (C-AU-01 times only the rolling emit) | yes/maybe: the 64-handle posting cache is CLEARED wholesale on the 65th distinct path (64 `close`s in one emit, sink.rs:4461-4463); every miss = sink_dir_there + create_dir_all + open(O_CREAT) in a directory holding one file per distinct value (unbounded within a 64 MiB generation); up to 8 dimensions per error event. |
| T12 | crates/telemetry/src/sink.rs:2386 | Audit::roll (4495) / retire | "one rename inside the sink's own directory whatever the tree holds" | posting files | C-AU-01 (p50 ratio gated, 10/1k/100k files), C-AU-02 (reaper, printed) | no for the emit; reaper O(files) off-path (ack). |
| T13 | crates/telemetry/src/sink.rs:3214 | telemetry::audit_read (3219) | "O(1): at most two reads of one line." | file size | none (UNVERIFIED) | no. |
| T14 | crates/telemetry/src/tail.rs:15 | telemetry::tail::tail (fn 335) | "O(N × line width), and O(1) in the size of the file" | file size | C-T-03 (UNFILTERED `Query::last(20)` + bytes-read check) | yes: with any filter that rarely matches (`at_least(Level::Error)`, `from_target`, `from_run`) the backward walk reads until N matches or `max_scan_bytes` (DEFAULT 8 MiB, tail.rs:83), across all keep_files — cost linear in file size up to 8 MiB; only `since` exits early. |
| T15 | crates/telemetry/src/tail.rs:295 | Tail::missing | "O(1). Two seq reads off records already in hand" | — | C-T-03 | no. |
| T16 | crates/telemetry/src/record.rs:210 | record decoder / Record::field | "the documented O(1) lookup" (≤MAX_FIELDS members) | members per line | test | no (13th member refused). |
| T17 | crates/telemetry/src/sink.rs:157 | MAX_TARGET_LEVELS | "the cost is a constant and not a function of how many subsystems exist" | subsystems | C-T-02 | no. |
| T18 | crates/telemetry/src/sink.rs:167 | MIN_FILE_BYTES | "rotation — a constant amortised over thousands of writes" | — | worst-case report | see T2. |
| T19 | crates/telemetry/src/sink.rs:2807 | sink::is_audited | "one level comparison and at most one pass over its message" | message | none | no (≤256 B). |

### vocab (7 rows)

| # | where | what | claim | flat in | bench | suspect |
|---|---|---|---|---|---|---|
| V1 | crates/vocab/src/mask.rs:122 | vocab::ConditionMask::hits | "always WORDS ANDs, WORDS XORs ... with no loop and no early return" | candidate width, data | C-V-01/02/03/05 | no. |
| V2 | crates/vocab/src/mask.rs:213 | ConditionMask popcount/union/intersect/with_bit/get | "run over all six words whatever the vocabulary holds" | width | C-V-04/05 | no. |
| V3 | crates/vocab/benches/ratio.rs:188 | vocab::table::is_live / definition | direct index | position | C-V-06 | no. |
| V4 | crates/vocab/src/table.rs:1678 | vocab::table::index_of (NAME_INDEX, private fnv1a at 1624) | "measured at no more than 4 slots for a name and 7 for any other token" | names | test every_name_is_found... (no timing) | no (static 2048-slot table, length guard MAX_NAME_BYTES; steerable but ≤7). |
| V5 | crates/vocab/src/expression.rs:267 | vocab::expression::Expression::evaluate | "Θ(len) ... not O(1) in the program" | program length | test | ack (≤MAX_INSTRUCTIONS). |
| V6 | crates/vocab/src/expression_search.rs:234 | Cursor::place | "Every check is O(1) except the sibling comparison" | length | test | ack. |
| V7 | crates/vocab/src/expression_search.rs:72 | Cursor starts/depths | "checked in O(1) plus one sibling comparison" | prefix | test | ack. |

### engine (17 rows)

| # | where | what | claim | flat in | bench | suspect |
|---|---|---|---|---|---|---|
| E1 | crates/engine/src/lib.rs:43 | engine::column::Column::support (column.rs:105) | "O(1) per bar ... one fixed-six-word hits call per bar" | candidate width, bars | C-E-01/02/03/05/06/07/09 | no. |
| E2 | crates/engine/src/lib.rs:44 | condition lookup | "direct index into vocab's fixed table" | — | C-V-06 | no. |
| E3 | crates/engine/src/lib.rs:137 | engine::primitives::offer (141) | "Expected O(1) on a set pre-sized to the offered width" | offered | C-E-10 (re-inserts position 0 only: always a hit on one hot bucket) | no in production (k=1, ≤370, SipHash); the bench never times a miss or growth. |
| E4 | crates/engine/src/lib.rs:46 | engine::primitives::append (~146) / drain (2376) | "O(1) amortised ... Vec::push" | held results | C-E-11 (pre-allocated Vec, cleared between trials) | yes for max: `drain` does `out.try_reserve(batch.len())` per batch (lib.rs:2388); once a level outgrows the k≥2 heuristic, growth memcpys every held 56-B Itemset inside one drain. CLAUDE.md §3's "lane-local kept + extend per chunk" no longer matches the code (lib.rs:2362-2366). |
| E5 | crates/engine/src/lib.rs:706 | engine::Sweep::depth (712) | "O(1), not a count over the levels" | levels | test | no. |
| E6 | crates/engine/src/keep.rs:171 | engine::keep::Streamed::depth (174) | "O(1) for the reason Sweep::depth gives" | levels | test | no. |
| E7 | crates/engine/src/keep.rs:360 | engine::keep::Best::offer (375) | "O(1) to refuse, O(log cap) to admit ... never grows past its cap + 1 reservation" | offers, cap | none (UNVERIFIED) | yes/maybe: every refused/evicting offer is insert+remove on a hashbrown MaskSet held at ~76% load; a remove inside a ≥Group::WIDTH full run leaves a DELETED tombstone with no growth_left refund; when growth_left hits 0 with items > capacity/2 hashbrown RESIZES to 2× (infallible alloc + O(cap) rehash inside `offer`), falsifying the reservation claim and try_with_capacity's fallibility; later O(buckets) in-place rehashes. |
| E8 | crates/engine/src/lib.rs:2133 | engine::join_screen (2150) | "ONE PAIR IS ENOUGH, WHICH IS WHY THIS PART STAYS O(1)" | frontier width | C-E-08 (105 vs 990 wide) | no (per pair; subset probes E9). |
| E9 | crates/engine/src/lib.rs:2419 | every_non_parent_subset_is_frequent (2443) | "Theta(k), not O(1)"; per-probe flat | k | C-E-12 (k=4→320 per probe) | ack Θ(k); maybe: MaskSet hasher is unkeyed with a fixed seed (lib.rs:229-245, 375-379) — keys are data-derived frequent masks, hard to steer, but precomputable. |
| E10 | crates/engine/src/lib.rs:1193 | Ladder::walk (1208) / try_next_level_observing (1801) | "one expected-O(1) HashSet<u32> insert ... k − 2 expected-O(1) subset probes" | — | C-E-04 (walk per bar) | ack; JoinIndex::try_new sorts each previous frontier (lib.rs:2087-2099) — O(n log n) per level, not per op. |
| E11 | crates/engine/src/lib.rs:2238 | highest_position / without_highest | "O(1): the scan is over ConditionMask's six words ... UNVERIFIED" | — | none | no. |
| E12 | crates/engine/src/lib.rs:2475 | len_u64 (2484) | "O(1)" (vs iterator count) | — | — | no. |
| E13 | crates/engine/src/lib.rs:2296 | BATCH_PER_LANE (2315) / drain thread spawn | "That amortisation holds for a FULL batch, and most levels do not fill one" | — | — | ack: one OS-thread spawn per lane per drain; tail drains pay ~50 µs/lane for few candidates. |
| E14 | crates/engine/src/lib.rs:859 | DEFAULT_PAIR_BUDGET (909) | per-pair cost "flat across frontier width" | width | C-E-08/C-E-12 | no. |
| E15 | crates/engine/src/resume.rs:8 | engine::resume checkpointing | "O(total survivors), not O(1)." | — | — | ack. |
| E16 | crates/engine/src/lib.rs:250 | MaskHash / MaskHasher (238-389) | "can only make lookups slower if the distribution were poor" | — | test the_mask_hasher_separates_orderings | maybe (fixed-seed, see part c). |
| E17 | crates/engine/benches/ratio.rs:8 | support counting | "Support counting is O(bars) and that is not a violation" | — | — | ack. |

### indicators (20 rows)

| # | where | what | claim | flat in | bench | suspect |
|---|---|---|---|---|---|---|
| I1 | crates/indicators/src/evaluator.rs:312 | Calendar irregular-day set | "bounded by a compile-time length, so it is O(1)" | — | C-I-01 | no. |
| I2 | crates/indicators/src/evaluator.rs:1041 | Evaluator::stepped — crossings | "CROSSINGS.len() iterations ... O(1) per bar" | bars, candidates | C-I-01/05 | no. |
| I3 | crates/indicators/src/evaluator.rs:1066 | Evaluator::stepped — weekday bit | "one integer division and a five-arm match ... O(1)" | — | C-I-01 | no. |
| I4 | crates/indicators/benches/ratio.rs:21 | Evaluator::step / step_known | constant per bar "not constant on average" | candles fed, shape, rollover | C-I-01/02/04/05/07 | no. |
| I5 | crates/indicators/src/trend.rs:148 | trend RING (fold, 547) | "what keeps the state O(1) in candles fed" | candles | C-I-01 | no. |
| I6 | crates/indicators/src/gap.rs:80 | GapFib::bits (450) | "O(1) rather than O(rungs)" | rungs | C-I-01 | no. |
| I7 | crates/indicators/src/daily.rs:530 | DailyLevels::bits | "O(1) rather than O(levels)" | levels | C-I-01 | no. |
| I8 | crates/indicators/src/orb.rs:273 | Orb::bits / Orb (115) | "O(1) rather than O(windows). No allocation, no division." | windows | C-I-01 | no. |
| I9 | crates/indicators/src/fib.rs:320 | Prev5 | "worst-case O(1) — five comparisons, no deque, no amortisation" | sessions | C-I-01 | no. |
| I10 | crates/indicators/src/pattern.rs:43 | Patterns::step (229, 401) | "state is 5 × 56 bytes and never grows" | bars | C-I-01 | no. |
| I11 | crates/indicators/src/lib.rs:397 | CurDayFib | "Fixed size ... does not grow with the number of bars" | bars | C-I-01 | no. |
| I12 | crates/indicators/src/vwap.rs:319 | Vwap::sigma (Vwap 229) | "The exact form, in O(1)." | bars | C-I-03 context | no. |
| I13 | crates/indicators/src/vwap.rs:77 | vwap::isqrt_i128_counted (180) / bits (490) | "Bounded, not flat." (≤ITERATION_CEILING=16) | operand | C-I-03 (counts + per-iteration cost, context) | ack bounded. |
| I14 | crates/indicators/src/column.rs:605 | indicators::column::Column::build | "one amortised push per bar, into a vector reserved once" | bars | C-I-06 (20k→200k) | no (all Vecs pre-sized to bars.len()). |
| I15 | crates/indicators/src/column.rs:645 | Column::build_withholding | "One set probe more per bar ... UNVERIFIED" | withheld days | none | no (HashMap<i64,i64> SipHash, pre-sized). |
| I16 | crates/indicators/src/column.rs:363 | Column::accepted / accepts (1124) | "the evaluator's exact verdict in O(1)" | — | none | no. |
| I17 | crates/indicators/src/column.rs:884 | Column::replace_exact_positions / overlay_exact | "O(1) per row, independent of how many positions the family names" | positions | none | no. |
| I18 | crates/indicators/src/anchored.rs:25 | indicators::anchored::AnchoredEvaluator::step (382) | "every bar after the cursor is positioned reads the current reference in O(1)" | daily refs | none (UNVERIFIED) | maybe: one signal bar after a long signal gap walks every skipped daily reference in `advance_before` (anchored.rs:484-490) — O(gap days) in one step (amortised O(signal+daily) only); each step also copies the whole Evaluator twice (anchored.rs:418-424). |
| I19 | crates/indicators/src/anchored.rs:673 | overlay_exact_minute_gapfib | "Both are O(1) per signal bar ... UNVERIFIED" | — | none | no. |
| I20 | crates/indicators/src/anchored.rs:760 | overlay_exact_minute_orb_and_gapfib | "this is not an O(1) whole fold" | — | — | ack. |

### runner (30 rows)

| # | where | what | claim | flat in | bench | suspect |
|---|---|---|---|---|---|---|
| R1 | crates/runner/src/outcome.rs:617 | WindowExtremes::over (inside runner::outcome::forward, 916) | "a whole forward is O(bars) and one query is amortised O(1)" | horizon, bars | none (UNVERIFIED); test a_backward_right_end_is_answered_in_constant_reads counts reads | maybe: amortised only — one query can push Θ(H) bars; the first backward query builds BlockExtremes over the whole slice, O(n), inside one query (outcome.rs:632, 695); deques not pre-sized. |
| R2 | crates/runner/src/outcome.rs:778 | BlockExtremes::over (828) | "every query after it is O(1) ... two overlapping runs, O(1)" | — | test | no (≤2·EXTREME_BLOCK=128 reads). |
| R3 | crates/runner/src/outcome.rs:1902 | OverlapWindow::observe / retire_through (in runner::outcome::edge, 2168) | "O(1) amortised per hit ... UNVERIFIED ... No bench row covers edge" | hits, horizon | none | maybe: retire_through walks the timing wheel slot by slot since the previous hit (up to one full turn), so per-HIT cost is O(bars since the last hit); sparse hits (the rare-winner case) pay O(gap) each — amortised per bar only. |
| R4 | crates/runner/src/outcome.rs:2161 | runner::outcome::edge | "hit is folded in O(1) amortised through OverlapWindow" | hits | none; test edge_visits_no_queued_hit (5347) | see R3. |
| R5 | crates/runner/src/outcome.rs:1136 | prefix_median_steps_over | "two-heap running median: O(log g) per bar" | — | — | ack O(log g). |
| R6 | crates/runner/src/outcome.rs:1467 | Edge::payoff_bp accumulators | "O(1) per observation, no" second pass | — | none | no. |
| R7 | crates/runner/src/outcome.rs:321 | SessionBounds::last_fill_bar | "the same O(1) lookup" | — | none | no. |
| R8 | crates/runner/src/trade.rs:363 | runner::trade::walk / SliceFacts::at_timestamp | "a HashMap lookup, so EXPECTED O(1), not worst-case" | bars | test the_walk_cost_paragraph... (text only) | no: std RandomState SipHash, with_capacity(bars.len()) (trade.rs:539); keys are bar stamps. |
| R9 | crates/runner/src/trade.rs:252 | hole_offset / priceable_before_hole | "O(1), UNVERIFIED as a measured bound" | — | none | no. |
| R10 | crates/runner/src/trade.rs:451 | SliceFacts::broken_prefix | "the same two-read O(1) check" | — | none | no. |
| R11 | crates/runner/src/trade.rs:905 | held | "Two O(1) reads of facts" | — | none | no. |
| R12 | crates/runner/src/trade.rs:1603 | forced_exits | "projects its O(1) answers into the table" | — | none | no. |
| R13 | crates/runner/src/rank.rs:344 | runner::rank::rank | "O(log keep) to admit or O(1) to reject. Memory is O(keep)" | combinations | C-R-02 (report, partial) | no (per-combination edge pass O(bars), ack). |
| R14 | crates/runner/src/lib.rs:326 | run_ranked | "O(log keep) to admit" | — | — | no. |
| R15 | crates/runner/src/align.rs:46 | align (reproject merge) | "O(1) amortised per bar ... UNVERIFIED" | — | none | no. |
| R16 | crates/runner/src/identity.rs:71 | runner::identity::identity / data_digest_with_execution (390) | "Both are O(1) per bar"; identity O(1) in bar count | bars | C-R-03 | no. |
| R17 | crates/runner/src/resample.rs:273 | runner::resample::resample | "each bar costs a fixed number of checked operations, O(1)" | — | none | no. |
| R18 | crates/runner/src/replay_mask.rs:13 | replay_mask | "time and space are O(1) at the fixed V1 mask width" | — | none | no. |
| R19 | crates/runner/src/admission.rs:27 | admission policy V1 | "time and space are O(1) because policy V1 has a fixed set" | — | none | no. |
| R20 | crates/runner/src/bootstrap.rs:1109 | RomanoWolfAdjustedReceiptV1::candidate (1129); bootstrap_zero_v2.rs:223 Receipt::candidate | "Candidate result by caller position, O(1)." | strategies | none (UNVERIFIED) | no (Vec::get). |
| R21 | crates/runner/src/bootstrap.rs:709 | white_reality_check_receipt_v1 / spa_receipt_v1 (785) / romano_wolf (1310) | "not O(1)" | — | — | ack. |
| R22 | crates/runner/src/bootstrap.rs:1767 | stationary_indices_into | reused buffer (OE-2) | — | test | no. |
| R23 | crates/runner/src/grid.rs:438 | grid sqrt helper | "One sqrt and a fixed number of arithmetic operations -- O(1)" | — | none | no. |
| R24 | crates/runner/src/grid.rs:1606 | refused_at / pessimistic_offset (1631) / one_variant (4437) | "O(1), UNVERIFIED"; "every read above is O(1)" | — | none | no. |
| R25 | crates/runner/src/exit_grid_policy.rs:584 | require_exact_execution_subslice | "O(1), no bar read. Only an equal slice held in separate memory takes the linear search" | context length | test a_view_into_the_context_is_located_without_a_search (scan counter) | yes (ack fallback): a copied (non-view) execution slice → `position` scan + O(len) compare per call (exit_grid_policy.rs:611-617). |
| R26 | crates/runner/src/exit_grid_policy.rs:286 | ExecutionDigestsV1 / with_digests (403) | "at O(1) per run" | — | test | no. |
| R27 | crates/runner/src/exit_grid_policy.rs:2291 | ratio_bitmap / authorize_coordinate (2564) / classify_coordinate (2607) | "O(1): one row-offset lookup, one checked cell lookup ... UNVERIFIED" | grid size | none | no (+ one BLAKE3 over fixed-size context). |
| R28 | crates/runner/src/exit_grid_policy.rs:1555 | digest accessors (373, 1555, 1569) | "an O(1) copy" / "an O(1) identity check" | — | — | no. |
| R29 | crates/runner/src/excursion.rs:202 | ExcursionLadder::from_excursions | "expected O(n) per ladder and O(1) per trade" | trades | test | no (std select_nth_unstable has a linear worst-case fallback). |
| R30 | crates/runner/src/validate.rs:568 | issued_authority_projection (849, 1125, 2679) | "read in O(1) per Admission" | — | none | no. |

Also ack-only (no row needed beyond this note): closed.rs:122 redundant_between "Not O(1) per itemset ... O(k)"; topn.rs:21 "A complete brute-force search is not O(1)"; report.rs:7 / indicators column.rs:82 "a billion O(1) calls is still a billion calls"; expression.rs:102 evaluate_rows (Θ(len) per row, V5); lib.rs:951 auto_prepared_reporting.

### costs (12 rows)

| # | where | what | claim | flat in | bench | suspect |
|---|---|---|---|---|---|---|
| K1 | crates/costs/src/lib.rs:29 | costs::regime lookups (stt_options_rate, exchange_charge_rate, stt_exercise_rate, …) | "Every lookup here is O(1) by construction" | regime rows | C-K-01/02/03/04/09 | no (bounded scan of a compile-time array ≤MAX_LATER_ROWS, regime.rs:83). |
| K2 | crates/costs/benches/ratio.rs:26 | costs::strike::{strike_at, at_the_money}, moneyness_steps | "claimed O(1) for" strike/moneyness/quantity/expiry | distance, lots | C-K-05/06/07/08 | no. |
| K3 | crates/costs/benches/ratio.rs:26 | costs::lot::{contract_quantity, notional}, expiry::next_*_expiry | O(1) | lots, date | C-K-07/08 | no. |
| K4 | crates/costs/src/trip.rs (charge_stack / price) | costs::trip::charge_stack / price | flat stack per trip | quantity, premium | C-K-10/11/12/13 | no. |
| K5 | crates/costs/src/lot.rs:455 | costs::lot::lot_size_of | "O(1): one compare, at most RULES_PER_SLOT window tests, a bounded walk" | — | none | no (compile-time tables). |
| K6 | crates/costs/src/lot.rs:636 | costs::lot::LotTable::lot_on (1003) / rows_of (983) | "one hash probe and a binary search of at most this many rows" | table size (key length NOT covered) | test lot::table_tests::a_lookup_is_flat_in_the_table (table size only); no bench | yes: `underlying: &str` is unbounded — SipHash over the whole key (lot.rs:985) and on a miss `underlying.to_owned()` copies it into LotAbsence::NoRow (lot.rs:1013): a 4 MiB key is ~10^5× a "NIFTY" probe — the defect class core fixed in MemberIndex::position with a SYMBOL_CAPACITY guard. (partition_point ≤ log2 64.) |
| K7 | crates/costs/src/stack.rs:400 | costs::stack::ClientSecurityDay::add (474) / settle (541) | "updates the totals in constant time" | orders, quantity, premium | C-CHG-01/02/03/04 | no (fixed i128 totals). |
| K8 | crates/costs/src/stack.rs:1265 | ClientDay::add (1336) / settle | "six checked additions ... whatever the number of securities" | securities | C-CHG-06 | no. |
| K9 | crates/costs/src/ledger.rs:2 | costs::ledger::DayLedger::add (260) / exercise (285) / close (307) / order_lines_unrounded (405) | "O(1): C-CHG-07 / C-CHG-10 / C-CHG-08" | orders | C-CHG-07/08/09/10 (p50/p99/max) | no. |
| K10 | crates/costs/src/exact.rs:22 | Ratio::apply / round_half_up / Exact::sum | "No loop, no allocation, nothing that depends on a magnitude." | magnitude | C-K-13 floor | no (i128). |
| K11 | crates/costs/benches/stack.rs:1 | costs::dhan_calc::quote | p50/p99/max per order | size | C-CHG-05 | no. |
| K12 | crates/costs/src/regime.rs:68 | regime (historic binary search) | "'effectively O(1)' because N <= 3, explicitly declining" | — | C-K-02 | no (now array index). |

### greeks (3 rows)

| # | where | what | claim | flat in | bench | suspect |
|---|---|---|---|---|---|---|
| G1 | crates/greeks/benches/ratio.rs (C-G-01/02/05) | greeks::bsm price / greeks | per-price cost flat in moneyness and expiry | moneyness, expiry | C-G-01/02/05 | no (normal.rs fixed-depth loops are test references only). |
| G2 | crates/greeks/src/solver.rs:3 | greeks::solver | "This is not O(1) and it is not claimed to be" (p99 iterations quoted, :69) | quote difficulty | C-G-03 (per model evaluation) | ack. |
| G3 | crates/greeks/src/lib.rs:59 | greeks crate doc | "The solver is not O(1), and does not claim to be" | — | — | ack. |

### Hash functions and BuildHashers (for part c)

* FNV-1a, the shared one: `crates/core/src/universe.rs:4541` `pub const fn fnv1a(s: &str) -> u64` (offset basis 0xcbf2_9ce4_8422_2325, prime 0x0000_0100_0000_01b3, xor-then-multiply per byte, no length bound of its own); slot fold `pub(crate) const fn mask(hash, n) = hash & (n-1)` at universe.rs:4513. A private copy: `crates/vocab/src/table.rs:1624` `const fn fnv1a(bytes: &[u8])` for NAME_INDEX[2048].
* No HashMap/HashSet anywhere in the workspace uses FNV as its BuildHasher. FNV feeds only: compile-time open-addressed `MemberIndex<N>` tables (core::universe FNO_INDEX<1024>, NTM_INDEX<2048>, NIFTY 500/200/100/50 indexes; core::vendor EQUITY_BOARD_INDEX, SME_BOARD_INDEX<8>, NON_EQUITY_INDEX<512>; core::alias lineage MemberIndex<S> tables, NOT_ALIASED_INDEX<8>); vocab NAME_INDEX; the 32-bit store header `symbol_id = fnv1a(symbol) as u32` (pull/src/ingest.rs:3838, 5366; pull/src/gdfl_rekey.rs:145; api/src/bars.rs:493; api/src/provenance.rs:758); and fingerprints (costs/src/schedule.rs:1723 tests, runner/src/grid.rs:7934, indicators/src/gap.rs:1461, pull/src/upkeep.rs:581-589).
* The ONLY custom BuildHasher: `engine::MaskHash` (crates/engine/src/lib.rs:238-245) / `MaskHasher` (279-389): seed fixed 0x9e37_79b9_7f4a_7c15, per 8-byte word `h = fold_halves(u128(h ^ w) * 0x517c_c1b7_2722_0a95)` (wyhash-style mum), no finalizer, unkeyed. `type MaskSet = HashSet<ConditionMask, MaskHash>` (lib.rs:407), key `vocab::ConditionMask` ([u64; 6], hashed as length prefix 6 + six words). Used by `JoinIndex.frequent` (lib.rs:2082, rebuilt per level from the previous frontier), `keep::Best.masks` (keep.rs:269, insert/remove churn), and tests (lib.rs:4064, 5104).
* Every other map in the ten crates is std `RandomState` (SipHash-1-3, random per-process keys): store verdict.rs:2589 HashMap<u32,usize>, 3180 HashSet<(u64,String)>, 3808 HashMap<PathBuf,Handle>; quarantine.rs:1078 HashMap<([u8;16],u32),Entry>, 2345-2400 HashMap<String,Instrument>/HashMap<(String,String),Entry>/HashMap<String,Vec<Cause>>, 2567-2596 HashSet<i64>; file.rs:4952 HashSet<(PathBuf,Discriminant<Why>)>, 5504-5513 HashSet<PathBuf>×3; file_sealed.rs:322 HashMap<u64,usize>; repair.rs:630 HashSet<i64>; settings.rs:565 HashSet<&str>; flock.rs:219 HashMap<u64,Held> (the u64 key itself is fixed-zero-key `DefaultHasher::new()` of the lock path, flock.rs:227); telemetry sink.rs:715 HashMap<u64,Pair> (u64 key = fixed-zero-key DefaultHasher of target+message, sink.rs:727-731), sink.rs:3430 HashMap<PathBuf,File>; engine lib.rs:1524 HashSet<u32>, resume.rs:207/282 HashSet<u32>; indicators column.rs:672 HashMap<i64,i64>; runner trade.rs:469 HashMap<i64,usize>, closed.rs:138-188 HashMap<ConditionMask,u64>/HashSet<ConditionMask>, rank.rs:268/636/700 HashSet<ConditionMask>, outcome.rs:192/211 HashMap<i64,(u64,Option<usize>)>/HashSet<i64>, lib.rs:474 HashSet<ConditionMask>; costs lot.rs:884 HashMap<String,Range<usize>>. core has none outside tests.

## Cost claims made in the documents — brutex `cloud/o1p99-attack-20261110`

Read-only extraction, 2026-11-10. Every row cites `file:line` in the tree as
checked out. "Stat" says what the named proof actually gates on, which decides
whether a p99 tail could hide behind it:

* **min-N ratio** — gate 8 harness `cost_ps`: each trial times a loop of R calls
  and divides by R (a MEAN per call); the MINIMUM over N trials is kept; the 1×
  figure is divided into the larger-size figure and gated at **3.0×** (2.5× in
  `cli`, 4.0× in a few `api` unit tests). N per crate (bench `TRIALS` const):
  store 60, pull 60 (append rows 12), core 60, costs 60, cli 40, greeks 40,
  lake 40, vocab 40, indicators 24, telemetry 20, runner 15, api 8, engine 8.
  A min-of-means cannot see a tail at all (the store bench's own comment:
  "a shared runner's scheduler can only ever make a sample slower").
* **min-N budget** — same min-of-means, divided by a "floor" op timed in the
  same run, gated at a fixed number of floors.
* **p99 printed** — p50/p99/max (nearest rank over n single calls) printed
  beside the ratio, *never gated*.
* **p99 gated / p50 gated** — the stated percentile is the asserted quantity.
* **count** — an operation/probe/read count asserted; machine-independent.
* **none** — the named proof has no timing and no count (verified by reading the
  test body for `Instant`/`elapsed`: zero).

Most ratio benches re-probe ONE fixed key/index (or cycle ≤14), so they time a
warm, cache-resident path; noted per row where verified in the bench source.

### A. `docs/04-invariants.md` — gate 8 cost rows (`C-*`, `BBENCH-*`, `MS-10`)

| Id | Where | Claim (one line) | Dimension · sizes measured | Proof (bench fn) | Stat |
|---|---|---|---|---|---|
| C-01 | docs/04-invariants.md:203 | Reading the header costs the same at 1×/10×/100× the region offered | header region size · 1×/10×/100× | `store::bench::header_read_is_flat` | min-60 ratio |
| C-02 | :204 | Per-candidate eval on live `Column::support` does not grow with k | candidate k · 1→4→8 (0.971×/0.996×); k=384 via C-E-09 | `engine::bench::support_costs_the_same_per_bar_at_every_depth`; `…live_support_is_flat_across_the_entire_mask_width` | min-8 ratio |
| C-03 | :205 | k=1 duplicate rejection flat in already-offered positions | accepted positions · 1,000/10,000/100,000 (prod ≤384) | `engine::bench::duplicate_rejection_costs_the_same_however_much_is_seen` | min-8 ratio; row says "expected-cost, not worst-case" |
| C-04 | :206 | Result append within the level's reservation is flat 1×→100× held | results held · 1,000/10,000/100,000 (1.015×/1.204×) | `engine::bench::result_append_costs_the_same_however_many_are_held` | min-8 ratio; amortised outside reservation |
| C-07 | :207 | Sealing one block costs the same at 1×/10×/100× file record count | records in file · 1×/10×/100× (block in memory, no file read) | `store::bench::block_seal_is_flat` | min-60 ratio |
| C-08 | :208 | Table CRC beats bit-by-bit kernel ≥3× | speedup, same process | `store::bench::checksum_beats_the_bit_loop` | min-60 speedup |
| C-28 | :209 | Bar lookup (`BarFile::read_record`) costs the same whatever the file holds (index 0 and LAST) | records · 1,000/10,000/100,000 (≤5.6 MB temp file); same index ×200 per trial (warm) | `store::bench::record_read_is_flat_in_the_file` | min-60 ratio |
| C-29 | :210 | `read_record` ≤ 800 address-arithmetic floors | one 10,000-record file, index 9,999 repeated | `store::bench::record_read_stays_within_its_budget` | min-60 budget (800) |
| C-BC-01 | :211 | COLD bar lookup (block pread + sidecar pread + CRC + fstat) flat in file size | records · 1,000/10,000/100,000; 14 indices in 14 blocks cycled | `store::bench::cold_record_read_is_flat_in_the_file` | min-60 ratio |
| C-BC-02 | :212 | Cold read ≤ 10,000 floors | as C-BC-01 | `store::bench::cold_record_read_stays_within_its_budget` | min-60 budget (10,000) |
| C-BC-03 | :213 | Cold verify path allocates nothing | source shape + edge behaviour | `store::cold_read::the_cold_verify_path_reads_into_the_stack_not_the_heap` (+5 behaviour tests) | none (shape) |
| C-TIX-01 | :214 | Time→row (`BarFile::first_at_or_after` via `.tix`) flat in file size | 1-s bars · 1,024/10,240/102,400 (prod month ≤2,678,400); 14 stamps cycled; warm handle | `store::bench::time_lookup_is_flat_in_the_file` | min-60 ratio |
| C-TIX-02 | :215 | Time lookup ≤ 800 floors | 10,000-bar month | `store::bench::time_lookup_stays_within_its_budget` | min-60 budget |
| C-27 | :216 | One dashboard render ≤ 2,100 floors | — | `api::bench::the_dashboard_stays_within_its_budget` | min-8 budget |
| C-09 | :217 | Vendor row decode flat in field width | field · 28 B vs 4 MiB | `core::bench::decode_is_flat_in_field_width` | min-60 ratio |
| C-10 | :218 | Over-wide field refused | behaviour | `core::bench::an_over_wide_row_is_refused` | refusal check |
| C-09b | :219 | Decode ≤ 110 floors | — | `core::bench::decode_stays_within_its_budget` | min-60 budget |
| C-11 | :220 | Census read beats re-deriving by ≥100× | 10,000 entries, one process | `pull::bench::census_beats_the_scan_it_replaces` | min-60 speedup |
| C-12 | :221 | Manifest entry lookup (hit and miss) flat at 1×/10×/100× census | entries · 1,000/10,000/100,000 (prod census ~385,442, L-WP81-6); ONE fixed key ×20,000 | `pull::bench::entry_lookup_is_flat` | min-60 ratio; hash probe = expected O(1) (M-36) |
| C-13 | :222 | Append after load flat at 1×/8×/32× census | 1,792/14,336/57,344 (=7·2^k) and 1,000/10,000/50,000; batch 256 | `pull::bench::append_after_load_is_flat` | min-12 ratio; amortised past n_valid appends |
| C-26 | :223 | Entry lookup ≤ 240 floors | — | `pull::bench::entry_lookup_stays_within_its_budget` | min-60 budget |
| C-RF-01 | :224 | Dated risk-free lookup flat in table | auctions · 52/520/5,200, 1,024 days; measured 1.000× / **1.700×** (cache), p99 2,238→4,191 ps | `pull::bench::rate_lookup_is_flat` | min-60 ratio; p99 printed |
| C-14 | :225 | Instruments page render flat 2,787→50,000 instruments, every sort/pill/hatch/deep page | instruments · 2,787 vs 50,000 | `api::bench::every_order_and_pill_is_flat`, `api::bench::the_hatch_and_the_last_page_are_flat` | min-8 ratio |
| C-15 | :226 | One more instrument costs ≤1 ns per request (slope) | slope between 2 sizes drawing same rows | `api::bench::gate` (C-15 lines) | min-8 slope |
| C-16 | :227 | Dashboard flat 2→50,000 instruments | instruments | `api::bench::the_dashboard_is_flat` | min-8 ratio; doc admits it stayed green at 1.954× through a real doubling (:262-267) |
| C-17 | :228 | Per-rendered-row cost does not grow | rows drawn | `api::bench::gate` (C-17 lines) | min-8 ratio |
| C-G-01 | :1247 | Closed-form price flat across contracts | moneyness 0.1×/10× spot, 1 day–5 years | `greeks::bench::a_price_costs_the_same_whatever_the_contract_is` | min-40 ratio |
| C-G-05 | :1248 | One BS price ≤ 100 floors | — | `greeks::bench::a_price_stays_within_its_budget` | min-40 budget |
| C-G-02 | :1249 | Full greek set flat across contracts | as C-G-01 | `greeks::bench::the_full_greek_set_costs_the_same_whatever_the_contract_is` | min-40 ratio |
| C-G-03 | :1250 | One model evaluation flat however hard the quote (solve is NOT O(1); ≤86 evals) | per-evaluation, 7 vs 79 evals | `greeks::bench::one_model_evaluation_costs_the_same_however_hard_the_quote_is` | min-40 ratio |
| C-K-01..04 | :1337-1340 | Regime lookup flat by row selected / row count; refusal costs what a rate costs; pre-boundary still refuses | 2- vs 3-row tables | `costs::bench::the_selected_row_does_not_change_the_cost` … `the_pre_boundary_window_still_refuses` | min-60 ratio |
| C-K-05..09 | :1387-1391 | ATM rung, strike, quantity, expiry flat in magnitude/depth/lots/calendar | spot ₹50 vs ~i64::MAX; 1 vs 10^6 lots | `costs::bench::the_spot_magnitude_does_not_change_the_rung_cost` … | min-60 ratio |
| C-K-10..12 | :1436-1438 | Charge stack / entry point flat in trade size, date, underlying | 1 lot vs 10^6 | `costs::bench::the_trade_size_does_not_change_the_charge_stack_cost` … | min-60 ratio |
| C-K-13 | :1439 | Charge stack ≤ 120 floors | — | `costs::bench::the_charge_stack_stays_within_its_budget` | min-60 budget |
| C-L-01 | :1549 | `Batch::row` flat in batch size; last row = first row | rows · 2,480/24,800/248,000 | `lake::bench::row_lookup_is_constant_in_batch_size` | min-40 ratio |
| C-L-02 | :1550 | Full walk is linear (per-row flat) | 2,480→248,000 | `lake::bench::iteration_is_linear_per_row` | min-40 ratio |
| C-L-03 | :1551 | Contract-name parse does not scan the name | 26 B vs 4 KiB | `lake::bench::contract_parse_does_not_scan_the_name` | min-40 ratio |
| C-L-04 | :1552 | Row lookup ≤ 40 floors | — | `lake::bench::row_lookup_stays_within_its_budget` | min-40 budget |
| C-T-01 | :1786 | One `emit` flat with 1,000/10,000/100,000 events in file | events in file; **rotation disabled** (`NO_ROTATION`), single thread | `telemetry::bench::emit_cost_does_not_grow_with_the_file` | min-20 ratio of means |
| C-T-01b | :1787 | Same, at p99 | as C-T-01 | `telemetry::bench::the_tail_is_flat_in_the_size_of_the_file_too` | **p99 gated** (3.0×), rotation disabled |
| C-T-02 | :1788 | Filtered event touches nothing, flat | as C-T-01 | `telemetry::bench::a_filtered_event_touches_nothing_and_stays_flat` | min-20 ratio |
| C-T-03 | :1789 | Tail of last 20 events O(1) in file size, same bytes read | 1,000/10,000/100,000 events; 8,192 B read each | `telemetry::bench::the_tail_is_flat_in_the_size_of_the_file` | min-20 ratio + byte count |
| C-T-04 | :1790 | Written emit ≤ 1,000 floors | — | `telemetry::bench::emit_stays_within_its_budget` | min-20 budget |
| C-V-01..04 | :1920-1923 | Mask hit=miss; miss same in every word; flat in candidate width (1→328 bits); popcount/union/intersect flat in bits set | 6 words; 1 vs 328 bits | `vocab::bench::a_hit_and_a_miss_cost_the_same` … `the_set_operations_do_not_grow_with_the_bits_set` | min-40 ratio (+ source-shape guard `vocab::mask::hits_does_the_same_work_for_every_input`) |
| C-V-05 | :1924 | Every mask op ≤ 12 floors | — | `vocab::bench::no_operation_costs_more_than_its_budget` | min-40 budget |
| C-V-06 | :1925 | Condition lookup (`table::definition`/`is_live`) flat wherever it lands | positions 0, 369, mid, 434 | `vocab::bench::a_condition_lookup_costs_the_same_wherever_it_lands` | min-40 ratio |
| C-I-01 | :1972 | One candle flat however many came before | 1,000/50,000/200,000 candles | `indicators::bench::a_candle_costs_the_same_however_many_came_before` | min-24 ratio |
| C-I-02 | :1973 | One candle flat whatever it contains (two-sided) | motionless/wandering/trend; measured **0.535× (1.87× inverted)** | `indicators::bench::a_candle_costs_the_same_whatever_it_contains` | min-24 ratio (admitted §52) |
| C-I-03 | :1974 | `isqrt_i128` iterations ≤16 at 1, i64::MAX, 1e30, i128::MAX | operand | `indicators::bench::the_integer_square_root_is_bounded_and_flat_per_iteration` | count (bounded, not flat: §51) |
| C-I-04 | :1975 | Session rollover costs what an ordinary candle costs | mid-session vs rollover | `indicators::bench::a_session_rollover_costs_what_an_ordinary_candle_costs` | min-24 ratio |
| C-I-05 | :1976 | One bar's evaluation ≤ 2,000 floors | — | `indicators::bench::a_candle_stays_within_its_budget` | min-24 budget |
| C-I-06 | :1977 | Column build flat per bar | 20,000 vs 200,000 bars | `indicators::bench::the_column_costs_the_same_per_bar_however_long_it_is` | min-24 ratio |
| C-I-07 | :1978 | `step_known` ≤ 5,200 floors (not measured on CI) | — | `indicators::bench::a_known_step_stays_within_its_budget` | min-24 budget |
| C-E-01 | :2033 | Support counting per bar flat in column length | 10,000/100,000/1,000,000 bars (1.271× / 0.663×) | `engine::bench::support_costs_the_same_per_bar_at_every_column_length` | min-8 ratio |
| C-E-02 | :2034 | Live support per bar flat in k | k 1/4/8 | `engine::bench::support_costs_the_same_per_bar_at_every_depth` | min-8 ratio |
| C-E-02b | :2035 | Historical Θ(k) vertical layout retained; current body one fixed-width hit | shape + differential test | `engine::column::tests::the_live_support_body_is_one_fixed_width_hit_test` | shape |
| C-E-03 | :2036 | Per-bar cost independent of answer (all vs none match) | 100,000 bars; 0.710× | `engine::bench::support_costs_the_same_whether_bars_match_or_not` | min-8 ratio |
| C-E-04 | :2037 | Whole ladder walk not dearer per bar (one-sided) | 10,000 vs 100,000 bars, fixed live set (1M absent) | `engine::bench::a_ladder_walk_does_not_get_dearer_per_bar` | min-8 one-sided ratio |
| C-E-05 | :2038 | Supported bar ≤ budget floors | k=1/k=8 | `engine::bench::support_stays_within_its_budget` | min-8 budget |
| C-E-06 | :2039 | Live column in same class as fixed-width reference | k=1/k=8 | `engine::bench::live_column_costs_like_the_fixed_width_reference` | min-8 ratio |
| C-E-07 | :2040 | Fingerprinted support independent of answer | all vs none | `engine::bench::fingerprinted_support_costs_the_same_whether_bars_match_or_not` | min-8 ratio |
| C-E-08 | :2041 | One join pair (production `join_screen` via `JoinProbe`/`JoinIndex`) flat whatever the frontier holds | frontier · 105 vs 990 masks (455 vs 14,190 pairs) | `engine::bench::one_join_pair_costs_the_same_at_every_frontier_width` | min-8 ratio |
| C-E-09 | :2042 | Live support flat k=1 vs k=384 | k | `engine::bench::live_support_is_flat_across_the_entire_mask_width` | min-8 ratio |
| C-E-10 | :2043 | Duplicate rejection via `engine::primitives::offer` | 1,000/10,000/100,000 accepted | `engine::bench::duplicate_rejection_costs_the_same_however_much_is_seen` | min-8 ratio; expected/amortised only |
| C-E-11 | :2044 | Result append via `engine::primitives::append` (reserved) | 1,000/10,000/100,000 held (1.015×/1.204×; replica 1.560×) | `engine::bench::result_append_costs_the_same_however_many_are_held` | min-8 ratio; amortised |
| C-E-12 | :2045 | One subset-prune probe flat at every depth (prune is Θ(k)) | k=4 → k=320, single k-set frontier | `engine::bench::one_subset_probe_costs_the_same_at_every_depth` | min-8 one-sided ratio |
| C-R-01 | :2063 | Column build per offered bar flat | 3,000 vs 12,000 bars | `runner::bench::the_column_build_costs_the_same_per_bar_at_every_column_length` | min-15 ratio |
| C-R-02 | :2064 | Report costs nothing per bar | 1,124 vs 10,124 swept bars | `runner::bench::the_report_costs_nothing_per_bar` | min-15 ratio |
| C-R-03 | :2065 | Run identity independent of bars digested (`data_digest` itself O(bars)) | digest inputs | `runner::bench::the_identity_does_not_depend_on_how_many_bars_the_digest_covered` | min-15 ratio |
| C-R-04/05 | :2066-2067 | Column build ≤ 5,000 / equity (VWAP) ≤ 14,500 floors (not on CI) | — | `runner::bench::the_column_build_stays_within_its_budget`, `…the_equity_column_build_stays_within_its_budget` | min-15 budget |
| C-CLI-01 | :2834 | Ledger record *i* read independent of *i* | record 0 vs 1,023 of a 1,024-record file | `cli::bench::the_read_does_not_depend_on_which_record` | min-40 ratio (2.5×) |
| C-CLI-02 | :2835 | Count independent of how many | 64 vs 1,024 records | `cli::bench::the_count_does_not_depend_on_how_many_there_are` | min-40 ratio |
| C-CLI-03 | :2836 | Duplicate check is a hash probe (set built at open, O(runs), excluded) | hit vs miss on an open ledger | `cli::bench::the_duplicate_check_does_not_scan_the_ledger` | min-40 ratio |
| C-CLI-04 | :2837 | Encode independent of ledger | — | `cli::bench::the_encode_does_not_depend_on_the_ledger_it_joins` | min-40 ratio |
| C-CLI-05/06 | :2838-2839 | Encode ≤2,000 floors; `Results::holds` (SipHash 32 B + probe) ≤65 floors | — | `cli::bench::the_encode_stays_within_its_budget`, `cli::bench::the_duplicate_check_stays_within_its_budget` | min-40 budget |
| C-GI-01 | :6927 | Option file of a day found by one hash probe, flat 1×/10×/100× listing | listing · 1,000/10,000/100,000 (real day 11,524); ONE key ×20,000 | `pull::bench::gdfl_option_lookup_is_flat` | min-60 ratio; p99 printed (miss key) |
| C-GI-02 | :6928 | Stock-day share file by slot index, flat | 1×/10×/100× listing | `pull::bench::gdfl_stock_lookup_is_flat` | min-60 ratio |
| C-GI-03 | :6929 | One stored second by row index flat; read by TIME is the bisection (printed, not gated) | 1×/10×/100× one-second month | `pull::bench::gdfl_second_by_index_is_flat` | min-60 ratio |
| C-AU-01 | :7013 | Audited emit that triggers a roll flat in posting files retired | 10/1,000/100,000 posting files | `telemetry::bench::an_audit_roll_never_waits_for_a_delete` | **median** ratio |
| C-AU-02 | :7014 | Reaper delete of a generation timed (tracks files; never gated) | 10/1,000/100,000 | same | p99 printed |
| C-LG-01 | :7015 | Keyed `/logs.json` page and importer-evidence reads flat | audited lines/dir · **200 vs 2,000** (8 dirs) | `api::bench::reads_are_flat` | p50 gated |
| C-LG-02 | :7016 | `/imports.json` runs page and `run=<id>` flat; `/dayflags.json` day band O(runs) printed | `run` lines · 1,000/10,000/100,000 | `api::bench::reads_are_flat` | p50 gated; day band p99 printed only |
| C-AL-01 | :7044 | Alert put (read newest 1,024, 128 B append, `sync_data`) flat in queue | records · 1,024/10,240/102,400 | `pull::bench::alert_put_is_flat` | min-60 ratio; p99 printed |
| C-AL-02 | :7045 | `alerts::window` read flat in queue | as above | `pull::bench::alert_window_is_flat` | min-60 ratio; p99 printed |
| C-VD-01 | :7091 | Verdict lookup flat in the day (cold+warm, colliding keys) | records · 2,758 at 1×, 10×, 50× (100× refused); cold p99 0.58–0.79 ms vs p50 70–111 µs | `store::bench::verdict_lookup_is_flat_in_the_day`, `…verdict_ceiling_is_refused_whole`, `…verdict_lookup_over_a_year` | min ratio; p99 printed, never gated |
| C-VD-02 | :7092 | `.vdi` year index read flat (fixed 2×8,812 B) | 1 vs 248 days | `store::bench::year_index_read_is_flat` | min ratio |
| C-VD-04 | :7093 | Generation commit is O(n log n) + 3 syncs (per-record flat) | 2,758 vs 27,580 records | `store::bench::generation_write_stays_n_log_n` | min ratio; p50/p99 printed |
| C-RP-01 | :7103 | Month repair per merged row flat (linear op) | 448,317 vs 2,678,400 rows; peak RSS ≤640 MiB | `store::bench::month_repair_is_linear_and_leaves_the_month_indexed` | ratio; p99 printed |
| C-RK-01 | :7151 | One pack's receipts read flat in packs held | 1×/10×/100× packs; p99 215 µs–1.11 ms, max 8.68 ms | `pull::bench::keep_lookup_is_flat` | min-60 ratio; p99 printed never gated |
| C-RK-02 | :7152 | One kept-answer append flat | 1×/10×/100× packs; p99 16–51 ms, max 152.8 ms (`F_FULLFSYNC`) | `pull::bench::keep_append_is_flat` | min-60 ratio; p99 printed never gated |
| C-NFO-01 | :7788 | `pull::gdfl_nfo::read_file` per row flat (linear op) | rows · 1,000/10,000/100,000 | `pull::bench::nfo_read_is_flat_per_row` | min ratio; p99 printed |
| C-FD-01 | :7789 | `pull::fold::fold_from_snapshots` per bar flat (linear op) | snapshots · 1,000/10,000/100,000 | `pull::bench::fold_is_flat_per_bar` | min ratio; p99 printed |
| C-CHG-01..10 | :7966-7975 | Order/day/settle/quote/ledger add/close/lines/exercise costs do not track size or count | 1 vs 10^7 shares; 2 vs 2,000,000 orders | `costs::bench::an_orders_cost_does_not_track_its_size` … `a_ledger_exercise_does_not_track_the_exercises_before_it_or_its_size` (crates/costs/benches/stack.rs) | **ratio of medians** (3.0×); p99 printed; limits note a 16-call batch mean hides per-call cost (D-5131) |
| C-PIT-01 | :8148 | F&O membership `members`/`is_member` flat in held days | 1/10/100 months (prod ~240) | `pull::bench::fno_membership_lookup_is_flat` | ratio; p99 printed |
| C-ZV-01 | :8211 | One landed window's verification flat in instruments; spool append flat | 1×/10×/100× instruments / records | `pull::bench::zerodha_window_is_flat` | min ratio; p99 printed |
| C-ZV-02 | :8212 | Spool merge per record flat (one O(n log n) sort) | 343/3,430/27,440 records | `pull::bench::zerodha_merge_per_record_is_flat` | min ratio |
| C-ZV-03 | :8213 | Merge-log step (index probe + 16 B append + sync) flat | 1×/10×/100× records logged | `pull::bench::zerodha_merge_log_step_is_flat` | min ratio |
| C-HA-01 | :8225 | `store::header::slot_audit` flat in region handed (reads exactly 2 slots) | 32,768 B ×1/10/100 | `slot_audit_is_flat` (store ratio bench) | min-60 ratio; p99 printed |
| C-ZI-01/02 (ZI-09) | :8484 | Listing probe and identity judge per minute flat | 1×/10× master rows; 1×/10×/100× answer | `crates/pull/benches/ratio/identity.rs` | min ratio; p99 printed |
| C-MS-01 (R428-06) | :8642 | `token_on` O(1) in snapshots (one day-table index + ≤2 hash probes) | 1/10/100 snapshots | `crates/pull/benches/ratio/master_snapshots.rs` | min ratio; p99 printed |
| C-MS-02 (R428-09) | :8645 | Futures plan opens only newest snapshot | 1/10/100 snapshots on disk | same | min ratio |
| C-PD-01/02 (R427-11) | :8956 | Padded judgement and cash derive fold flat per minute | 1×/10×/100× a month's minutes | `crates/pull/benches/ratio/padded.rs` | min ratio |
| MS-10 | :7660 | `/gaps.json` kept answer flat in bars, linear in months | 1-s months 1,024/10,240/102,400; 240-month span vs ten 24-month tiles (p50 ≤1.5×, p99 ≤2×) | `crates/api/benches/ratio/gaps.rs` `months_are_flat`, `spans_are_bounded` | rotated rounds; p99 gated on span |
| BBENCH-01 | :9189 | Whole month read paged via `/bars/window.json` flat per row | 1,024/10,240/102,400 bars (prod ≤2,678,400) | `bench_paged_month::…` + bench "paged p50 per row" | **p50** gated |
| BBENCH-03 | :9191 | Whole-month `/bars.json` refusal O(1) in month | 60,001/102,400/2,592,000 rows | `reads::refusals_are_flat` | **p50** gated |

### B. `docs/04-invariants.md` — other rows that state O(1)/flat/bounded cost

| Id | Where | Claim (one line) | Dimension · sizes | Proof named | Stat |
|---|---|---|---|---|---|
| S-BISECT-01 | :117 | `first_at_or_after` (no `.tix`) ≤ ceil(log2(n_valid+1)) record reads + ≤1 cold verify each; "is not O(1)" | 0–150 records exhaustive; 2^62 arithmetic | `store::file::first_at_or_after_never_probes_more_than_the_bisection_height` | count (admits O(log n); wall-clock UNVERIFIED) |
| S-PROBESTORE3-01 | :120 | Catalog walk stays O(entries), symlink loops finish | — | `store::catalog::a_symlink_loop_finishes_promptly_and_is_counted` | count / 10 s bound |
| P-36 | :418 | TOTP loop bounded by `MAX_SECRET_LEN` | — | `pull::totp::the_length_bound_is_checked_before_a_single_character_is_decoded` | shape |
| I-39 | :603 | Hashing `Symbol`/`InstrumentKey` feeds same bytes at 1 char and capacity | bytes fed | counting hasher test | count |
| A-28 | :753 | `StoreFilter::keeps` haystack bounded by the type | — | `api::census::the_symbol_arm_searches_a_haystack_the_symbol_type_bounds` | shape |
| AJ-01 | :1572 | `/audit.json` read bounded by page (`MAX_PAGE_RECORDS`) | — | `api::audit_json::a_page_reads_at_most_the_records_it_shows` | count |
| GF-03 | :2356 | Every feed answers every rung "in O(1)" (5×11 matrix) | — | `pull::vendor::every_feed_answers_every_rung_with_one_of_two_verdicts` | **none** (no timing, verified) |
| CJ-08 | :2399 | `(vendor,tier)→ids` array index and `(vendor,exchange,ISIN)→id` hash probe O(1) | 100 vs 4,000 ISINs; best of 9×200,000 | unit test in `api` | min-9, 4.0× |
| CV-11 / BRED-07 | :2427 / :8790 | `Coverage::of` one array index, flat in universe | universes 40× apart; fastest of ≥7 and **up to 31** alternating pairs | `api::coverage::tests::the_lookup_does_not_grow_with_the_universe` | best-of-N, 4.0× |
| SS-06 | :2587 | `/instruments.json` bar count is one pass / facet index | — | `api::server::cost_limits_test…` | count |
| BT-04 | :2800 | Backtest runs newest-first by seeking, O(limit) | — | `backtest::runs_arrive_newest_first` | shape |
| LG-06 | :2981 | Ledger run's event count bounded by constants | — | (row) | count |
| EB-01 | :3436 | Runner reads the shared acceptance bitmap in O(1) | — | `indicators::column::tests::acceptance_bitmap_records_a_duplicate_timestamp_in_constant_time_shape` | **none** (verified) |
| AF-40 | :5356 | Past-the-commit read bounded by tail block's room | 68/66 records | (row) | count |
| (row) | :6020 | `Cursor::decode` bounded by `CURSOR_BYTES`, not at² | — | `expression_search::invariant_tests::one_check_of_the_whole_prefix_equals_a_check_at_every_index` | equivalence |
| (row) | :6211 | `edge` folds overlap pairs in O(1) amortised from running sums | — | `runner::outcome::overlap_window_tests::…` | correctness/visit count |
| (row) | :6217 | `walk` names one expected-O(1) `SliceFacts::at_timestamp` hash probe per signal | — | `runner::trade::tests::the_walk_cost_paragraph_names_the_calls_the_walk_makes` | prose pin |
| AFF-43 | :6654 | `WindowExtremes::over` reads ≤ 3n + 130·q bars total | 2,000-bar window | `runner::outcome::…` | count |
| AFG-04 | :6745 | `Sweep::depth` O(1) | — | `engine::tests::depth_is_the_level_count_less_the_empty_last_level` | shape |
| AFG-09 | :6748 | `api::detail::Cached` holds slot lock only for an O(1) look/install/clear | — | `api::detail::tests::the_slot_lock_is_free_while_a_handle_opens_or_refreshes` | concurrency |
| AFG-28 | :6760 | Indexed month: `first_at_or_after` ≤1 index-entry read (+1 bar read daily) up to 2,678,400 bars | every size to 1-s ceiling | (row tests) | count |
| CM-13 / CM-16 | :6905 / :6908 | Ticker map one index into ticker-slot array; zip day one index + one central-directory read | — | (row tests) | count/shape |
| IA-AL-14 | :7035 | Older alert page is one read at any depth | 2,500-record queue | `pull::alerts::tests::an_older_page_is_one_read_at_any_depth` | count |
| SC1-08 / SC1-09 | :7204 / :7205 | Codec loops bounded (varint ≤10 B, gcd ≤128 steps); one `Vec::reserve` (amortised) per accepted frame | — | (row tests) | count/alloc count |
| SC2-03 | :7318 | Sealed month: row/page/lookup cost same at 554 and 2,678,400 bars (counted reads) | 554 vs 2,678,400 | `the_largest_month_reads_at_the_cost_of_the_smallest` | count (reads/bytes) |
| SC3-08 | :7345 | Raw and sealed 1-s forms answer at p99, flat with the month | 2,250/22,500/225,000 bars; 2,000 pages | `both_second_formats_answer_the_same_bars_at_p99` (crates/store/tests/sealed_door.rs) | p99 gated: ≤3×p99(1×) **+200 µs**; BRED-55: first of ≤5 runs that holds |
| SC4-11 / BRED-09 | :7373 / :8792 | Write door (open, re-offer, refused batch) flat 1×/10×/100× bars | 2,250 vs 225,000 bars | `the_write_door_costs_the_same_at_one_ten_and_a_hundred_times_the_bars` | p50/p99 ≤3× **+1 ms**; one of up to 3 runs |
| AH02-08 | :7522 | O(1) per write in the head-deadline timer, counted | 400 writes | `api::server::head_deadline_body_tests::…` | count |
| R332-08 | :7541 | A run never reads its log nor lists notice pages | 10,000 lines / 500 pages | `a_run_never_lists_the_notices_or_reads_the_log` | behaviour |
| R301-05 | :7583 | /db address encode+decode under 1 ms at p99; held name is one `Set` probe | n = 1,000 | `web/tests/db-address.test.js` | p99 absolute (1 ms) |
| FR-16 | :7689 | One factor authority, O(1) per lookup | — | `the_factor_reader_answers_every_day_from_the_newest_file` | **none** (verified) |
| R406-07 | :7771 | Rehearsal gate one lookup per start, flat 1→10,001 kinds | 1 vs 10,001 kinds, n=1,000 | `pull::rehearsal::tests::the_gate_is_one_lookup_per_start` | p50 ≤20× |
| CAL-12 | :7849 | `tradable_day`, `excluded_between`, `year_exclusions`, `year_record` "constant-cost, measured" | ONE calendar size, n ≥200/270 | `pull::calendar::tradable_tests::tradable_day_and_every_bucket_are_bounded` | p99 absolute <50/100 µs (no cross-size ratio) |
| CAL-20 | :7857 | Calendar check O(1) per day per feed; same at 31- and 36,525-day span | 31 vs 36,525 days | `cli::calendar_days::tests::the_check_costs_constant_time_per_day_measured`, `…a_day_costs_the_same_at_any_span_measured` | p99 absolute (<2 ms/day; <100 µs/day each span), no ratio |
| IA2-27 | :7903 | Journal keeps per-key state in O(1) a line; history in three hash probes | — | `the_journal_keeps_each_keys_hole_reason_first_landing_and_last_done` | **none** (verified) |
| IA-ZV-X8 / IA-ZV-X25 | :8183 / :8201 | Gap ledger reads only appended bytes; merge log answers a day by one probe, capped 64 MiB | — | (row tests) | behaviour |
| R315-20 | :8382 | Web report cost bounded by `MAX_REPORT_BYTES` cut | 4,000,000 chars | `web/tests/speed.test.js` | behaviour |
| RUST1-10 | :8545 | `/disk.json` call p99 < `/bin/df` child p50 | n = 1,000, run by hand once | `api::server::disk_route::tests::the_call_at_its_p99_costs_less_than_the_df_child_at_its_p50` (`#[ignore]`) | p99, ignored test |
| R330-16 | :8632 | Carried risk-free lookup "stays O(1) at p99", flat 52→5,200 rows | 52 vs 5,200 rows | `pull::riskfree::tests::a_carried_lookup_is_flat_in_the_table` | p99 ≤ max(10×p99(52), **2,000 ns**) |
| R426-11 / R426-14 | :8685 / :8688 | Corporate-action run/action lookup is one index whatever the share holds | 1/64/4,096 rows | `pull::corporate_actions::tests::an_action_lookup_is_one_index_at_1_64_and_4096_rows_measured` | **median** ≤ 4×max(median(1), 200 ns) |
| R426-27 | :8701 | Missing-month lookup same at 12/120/1,200 months | n = 2,000 samples of 64 | `cli::basis_runs::tests::a_steps_missing_months_cost_the_same_at_12_120_and_1200_months_measured` | **median** ≤ 4×max(·, 200 ns) |
| BRED-04 / BRED-20 / BRED-22 | :8787 / :8803 / :8805 | Rescan/quarantine/not-owed answers are one hash probe of a pre-sized set (was binary search / 580× search) | MAX_GAPS answers | `pull::gdfl_import::tests::…`, `pull::gaps::not_owed_test…` | behaviour; BRED-22 "within a small factor" |
| BRED-25 | :8808 | /db keystroke p99 inside one 16 ms frame, no census row read | GDFL-sized census | `web/tests/db-search.test.js` | p99 of per-keystroke **fastest-of-10** |
| BRED-29 | :8812 | A census change never makes a request wait (<50 ms) | 1,000 requests under 5 s pause | `api::server::census_cache::warm_tests::a_census_change_never_blocks_a_request` | max <50 ms in **any one of up to 7 runs** |
| BRED-30 | :8813 | Feed list p99 <10 ms beside 32 looping clients | control vs loaded | `the_feed_list_stays_fast_while_32_clients_loop_each_route` (crates/api/tests/pool_load.rs) | p99 in **first of up to 7 runs that holds** |
| T2S-12 | :8887 | Tick-to-second index answers in O(1) | — | `the_index_round_trips_and_refuses_what_is_not_its_format` | **none** (verified) |
| D1W-17 | :8932 | Held month answers a page in time not growing with shares | 10/100/1,000 shares, n=200 | `a_held_month_answers_in_time_that_does_not_grow_with_its_shares` (crates/api/src/feed_checks_tests.rs) | **p50** < 10×p50(10) **+200 µs** |
| R313-22 / -23 / -35 | :9013 / :9014 / :9026 | Day band warm answer flat 100× within 3×; `/dayflags.json` and cold share page measured on real store | p50/p99/max n=200 | ignored tests (`--ignored`) | hand-run only |
| R418-06 | :9043 | Quarantine probe is one read | — | `store::quarantine::tests::a_quarantine_probe_is_one_read_and_a_torn_tail_is_repaired` | count |
| R422-06 | :9116 | Header-shape memory capped at 1,024 shapes | — | (row tests) | bound |
| C-O1CLI1-01 | :2841 | A support descent loads stored inputs once (cache) | 3 supports | `cli::audited_stored::tests::a_descent_loads_its_stored_inputs_once` | load count |
| (rows) | :6033-6034 | Recovery plan seeding / successor costs same record syncs for 2 vs 40 windows | 2 vs 40 | `api::recovery::tests::…` | sync count |
| AG05-05 / RA-287 | :8338 / :7084 | Staging door adds constant host calls (1 free, 4 over leftover) | 0 B vs 4 MiB leftover, 2,000 names | `a_create_makes_one_host_call_on_a_free_name_and_four_over_a_leftover` | call count |
| R407-19 | :7730 | A press over N held days adds a constant number of journal lines | — | `…::repair_r1::held_days_are_journalled_once_not_once_a_press` | count |
| C4-RUNNER-02 | :5906 | Unissued opaque validation refused by "the O(1) door" | — | `validate::tests::an_unissued_opaque_validation_refuses_the_sealed_projection` | none (correctness) |

### C. `docs/07-o1-architecture.md`

| Id | Where | Claim | Proof / measurement | Stat |
|---|---|---|---|---|
| Law 1 | docs/07-o1-architecture.md:15 | Fixed width everywhere; 2- and 24-letter ticker cost the same | I-39 (bytes fed) | count |
| Law 2 | :18-21 | Pre-size every map; reservation does NOT give worst-case O(1); "expected O(1)" | M-17/M-19 | — |
| Law 3 | :23 | Never scan to answer "how many" — counters | C-11 | min ratio |
| Layer 1 | :39 | `Symbol` 24 B, `Isin` 12 B, `InstrumentKey` Copy, structural hash ✓ | — | — |
| Layer 2 | :40 | No cryptographic hash on trusted path: FNV-1a ◐ (note C-CLI-06 uses SipHash) | — | — |
| Layer 3 | :41 | Maps `with_capacity(reservation_for(n))` ×2; lookup expected O(1); append amortised ◐ | C-12, C-13 | min ratio |
| Layer 4 | :42, :157-167 | Membership: compile-time open-addressed, never `binary_search`; hit ≤8, miss ≤12 probes ✓; `of_equity` ≤6 misses | `core::universe::the_probe_length_is_bounded_which_is_what_makes_it_o1`, `core::universe::a_miss_probes_further_than_a_hit_and_its_bound_is_measured_too` | count |
| Layer 5 | :43 | Address `base + header + i·stride` ✓ | C-28/C-29 | min ratio |
| Layer 6/8 | :44, :46 | One 48-byte mask per bar; one fixed-width hit per candidate/bar ✓ | C-V-*, C-E-02/09 | min ratio + shape |
| Layer 7 | :45 | Residency: build once, reuse ◐ (logical, not physical pinning) | — | — |
| Layer 9 | :47 | Zero allocation in hot loop — **○ NOT BUILT** | — | — |
| Layer 10 | :48 | Work-stealing — **○ NOT BUILT** | — | — |
| Layer 11 | :49 | Whole records per block ✓ | S-20 | compile-time |
| Layer 12 | :50, :101-122 | Bounded page from precomputed order ◐; substring search O(universe) exception; catalog build has no ceiling | C-14..C-17 | min-8 ratio |
| Layer 13 | :51, :139-148 | Counters, never scans ◐; filtered census walks; saving vs directory walk is EXTRAPOLATION | C-11 | — |
| table | :74-85 | Mask eval fixed 384-bit; Column::support 0.971×/0.996×/0.942×; membership probe counts; page render 0.974–1.084×; dashboard 0.974×; search 6.53 ms NOT flat; checksum ~7,050 ns; bar read past RAM ~100 ns → 61,566 ns (616×); census 193,449×/402,568×; manifest lookup 0.994–1.049× | benches above | single-run figures |
| browser | :211-215 | /db sequential page O(1) (17 ms); jump to page O(1) in distance (67–90 ms, one request); jump to read month O(1) (105 ms) | hand measurement; **no browser bench** (:292-296) | none |

### D. `docs/01-architecture.md` §6 "Where the constant-time claims live"

| Operation | Where | Claim | Measured by (as cited) | Note |
|---|---|---|---|---|
| Locate a slice | docs/01-architecture.md:449 | O(1), path join | **—** (nothing) | kernel path resolution is per-component and filesystem-dependent; unmeasured |
| Read bar *i* | :450 | O(1), `base + 32768 + i·56` | **`C-01`** | C-01 times the HEADER read, not the bar read (04-invariants :296-312 says so); bar read is C-28/C-29/C-BC-01 — stale citation |
| Read condition bits for bar *i* | :451 | O(1), slice index of `[u64;6]` | `C-E-01` | C-E-01 times support per bar |
| Test one candidate against one bar | :452 | O(1), branchless | `C-V-01..03` + source guard | |
| Reject a duplicate | :453 | expected O(1) at k=1; none at k≥2 | `C-E-10` | admitted expected-only |
| Append one result | :454 | O(1) amortised | `C-E-11`/`C-04` | admitted amortised |
| Fold one candle | :455 | O(1), fixed state, no allocation | `C-I-01/02/04` | C-I-02 shows 1.87× content dependence (§52) |

### E. `docs/02-store-format.md`

| Where | Claim (function/route) | Bound stated |
|---|---|---|
| docs/02-store-format.md:28-31 | Address of record *i* `base + 32768 + i*56`: "an add, a multiply, a load. No search" | O(1) arithmetic |
| :427-438 | Read path verifies one block CRC per read (`read_row` → `verify_block_of`); "Read O(1) precisely": fixed 4,088-byte block CRC | O(1) per read, 4,088 B CRC (C-07, C-28/29) |
| :636-645 | `.tix`: at most one entry read (+1 bar read on daily rung) whatever `n_valid`; first lookup validates header + 2 entries | O(1) (C-TIX-01) |
| :655-663 | Month without usable `.tix` → D-1434 bisection, `ceil(log2(n_valid+1))` reads; writer rebuild O(n_valid) once | admitted O(log n) |
| :556-562 | Small month (<1,024 bars) has no `.tix`: bisection ≤10 bar reads | admitted O(log n) |
| :801-807 | `store::provenance::read` one open + fstat + pread ≤262,208 B; `Sidecar::covering` binary search over ≤1,024 records — "bounded by constants" | bounded O(log 1024) |
| :1061-1080 | Sealed month: row→block division; `first_at_or_after` = 2 table entries + ≤64 directory entries in one read + one frame; wider range bisected | O(1) reads; bisect edge case |
| :1208 | `store::month_summary::read_beside` one open + one read | O(1) |
| :1546-1549 | Census compaction one pass + one sort O(keys·log keys) once per fill | admitted, amortised |
| :1956-1959 | Completion probe average O(1); open/reconciliation linear | expected only |
| :2322-2324 | Exact lookup on a validated unchanged handle average O(1); preparation/reopen O(rows) | expected only |
| :2478-2482 | Validated sequence seek worst-case O(1) in records; indexed lookup average O(1); open/hash/durability not O(1) | mixed |
| :2528-2532 | One fixed-record seek worst-case O(1); open O(file bytes) | mixed |
| :2564-2567 | One candidate seek worst-case O(1) | O(1) |
| :3470-3474 | Day image build: one sort O(n log n) | admitted |
| :3681-3684 | Merge log: a day's step is one probe of the index (C-ZV-03); log >64 MiB refused | O(1) + cap |
| :4473-4478 | Alert readers: one read of ≤1,024 records | bounded |
| :4643-4644 | Zip index: names unique; "a lookup hashes the index by name once and is O(1) a probe" | expected O(1) |
| :4897-4901 | Settings refresh: one `lstat` O(1); first refresh/save lists directory O(generations) | mixed |

### F. Other documents

| Where | Claim |
|---|---|
| CLAUDE.md:138-175 (§3 rule 4) | Bar lookup, condition lookup, mask evaluation, duplicate rejection, result append each O(1); qualified: mask eval O(1) per bar via `Column::support`; k=1 duplicate rejection **expected/amortised O(1)**, none at k≥2; result append `Vec::push` at k=1 / `Vec::extend` at k≥2, **amortised O(1)** (an expanding level can outgrow the reservation) |
| AGENTS.md:114-116 | Same five operations each O(1); "a change that makes one of them scan fails the bench gate" |
| AGENTS.md:120-124 | Installed capacity is not "total O(1) assurance" |
| README.md:44-47 | Same five O(1); "the sweep itself is combinatorial — each step is constant, the number of steps is not" |
| docs/25-macbook-runtime-budget.md:41-44 | Finite batches/fixed masks/indexes "do not make an exhaustive search, all persisted history, a cold integrity scan or filesystem latency O(1)" |
| docs/32-web-performance-20260908.md:105-110, :118-120 | Server timings (0.676/1.125 ms; 6.33–8.48 ms) are observations, "not percentile or worst-case latency guarantees" |
| docs/32-web-performance-20260908.md:147 | DB page: paging verified; does NOT establish "every sort or full-inventory operation is O(1)" |
| docs/32-web-performance-20260908.md:187-190 | "total O(1) time or space have not been established. O(1) indexed operations and bounded page reads do not make a growing inventory or exhaustive search constant-size work" |

## The admitted limits the attack excluded (curated list)


Format: operation (function / route) — what is admitted — where.

#### Whole-system statements
- Total sweep / whole-run work is not O(1); only per-operation cost is — §1 docs/06-limits.md:9; §5 :89 (C(328,k): k=3 5.8 M itemsets/311 MiB, k=4 473 M/24.7 GiB); "Ranked streaming removes retention, not the frontier wall" :162; §119 :7025; §129 :7387; §131 :7432; docs/07-o1-architecture.md:87-89; docs/01-architecture.md:481-486; README.md:46-47.
- Large-scale (billions) figures are extrapolations — §4 :58.
- Vendor pull can never be O(1) — §2 :23.
- Peak memory unmeasured (E-08) — §53 :3798; docs/04-invariants.md:193.
- Browser half of layer 12: no browser bench, substring filtering O(universe), `barWindowed`/`timeOptions`/CSV export O(loaded) — docs/07-o1-architecture.md:236-298; §30 :2653.
- Exhaustive search / persisted history / cold scans / filesystem latency not O(1) — docs/25-macbook-runtime-budget.md:41-44; docs/32-web-performance-20260908.md:147, :187-190.

#### Store (`crates/store`)
- `BarFile::first_at_or_after` without a usable `.tix` (absent, stale, refused, written outside `append`): bisection, `ceil(log2(n_valid+1))` reads (≤22 at 2,678,400), wall clock UNVERIFIED — "Timestamp lookup is a bisection, not O(1)" D-1434 :12890; :12973; S-BISECT-01 (04-invariants :117); docs/02-store-format.md:655-663.
- Small months (<1,024 bars) have no `.tix`: bisection ≤10 reads; sorted page orders ≤1,023 rows itself — L-SC-13 :18231, L-SC-3 :18241, D-4933 :18229.
- `.tix` rebuild on writer open: O(n_valid) verified reads once per month — docs/02-store-format.md:663-667; :12973.
- Sealed `.sbar` month: one row decodes its whole frame (≤256 rows, ≤14,508 B CRC), no block cache — L-SC-1 :18395, L-SC-17 :18728; widened superblock bisects — L-SC-18 :18729; encoder O(rows) per column — L-SC-15 :18397; `Vec` amortised growth on accepted frame — SC1-09.
- Block CRC is O(bytes covered) (4,088 B per cold read) — §14 :649.
- Cold bar lookup: own 10,000-floor budget; device latency share UNVERIFIED — :12741 (D-0914); docs/04-invariants.md:301-312.
- `BarFile::append` / write door: O(batch) + `already_stored` bisection + old-tail verify (D-0910) — "Rust and O(1) sweep, data side" :16137; :11738; L-SC4-02 :18738; tail block sealed past commit :9453 (D-0688).
- Month repair `repair::repair_month(_with)`: O(held + offered), 537 MB peak RSS at a full 1-s month — L-IA-26 :17402/:17415/:17434; L-SC4-01 :18737.
- `store::catalog::walk`: O(entries log entries) — §86 :5745-5827.
- Verdict store: commit O(n log n) + 3 syncs (L-IA-4 :17271); lost year index needs O(names) listing (L-IA-43 :17277); `staging::create` 4 host calls, unlink cost grows with leftover (L-IA-44 :17278, L-R316-06 :18409); day ceiling a band (L-IA-33, RA-261).
- Settings: first refresh and every save list the directory, O(generations) — L-R305-1 :18413, L-R305-6 :18512.
- Calendar derivation: one positional read per record, O(records) — :12621 (D-1443).
- `Sidecar::covering` binary search over ≤1,024 records (allowlisted `store/src/provenance.rs`) — docs/02-store-format.md:805.

#### Engine / vocab / indicators / greeks
- Apriori level join O(|frontier|²); subset prune Θ(k) per candidate; frontier sort O(|F| log |F|); pair budget — "Engine join costs that are not O(1)" :13055 (D-1438..D-1440, D-0924); C-E-12.
- k=1 duplicate rejection expected/amortised O(1) (`HashSet<u32>`), result append amortised (`Vec`) — §53 :3798; CLAUDE.md:152-175; docs/01-architecture.md:453-454.
- Support counting O(bars) per candidate — §1; docs/04-invariants.md:2015-2019.
- Boolean expression evaluation Θ(program length) per bar — :11903; expression search cursor O(at) — :11887, :11504; grammar search node budget — :11335.
- `isqrt_i128` bounded (≤16 iterations), not flat — §51 :3701.
- One candle's cost varies 1.87× with content — §52 :3770.
- Ladder ceiling derived from cores (proxy for memory) — §93 :6176; `REFERENCE_CORES` §94 :6227.
- `greeks::solver` not O(1) (≤86 model evaluations); `exp`/`ln` not measured for constant time — §29 :2082, :2150.

#### Pull / ingest (`crates/pull`, `cli` import verbs)
- Manifest: lookups expected O(1); append rehash-free only for first `n_valid` appends then amortised; filtered census walks entries; saving vs directory walk is EXTRAPOLATION — §17 :747; §23 :1067-1159; M-36.
- Census rewritten whole per broker window, O(entries) — §34 :2828; `census::held_series` O(keys log keys) — §32 :2709.
- Folder-feed reach O(members) — §65 :4467; `named_error_of` O(body) — §83 :5608-5639; CSV row linear in its line — :11185; 2xx refusal check O(body) — :11254; census walk bounded by members — :11464.
- Groww chain pricing re-reads O(bars) — §88 :5828; pull-ladder probe cost unmeasured — §69 :4618.
- Pull ingest/fold/cache costs that grow with month or census (`derive_all`, `complete_minutes_with_calendar`, …) — D-0955 :12097-12189; API/pull costs that grow with the store — D-1446 :12190-12450.
- GDFL import at one second: `NfoDay::locate` hashes the ticker (O(ticker length)), O(entries) listings — :16621; capital-market reader O(file bytes)/O(entries) — :11594.
- Before-first-bar check O(months) — :16715; `zerodha_nse_checked` per-parse read — :16746; minute-pass refusal classes — :16783.
- Ingest automation rows: L-IA-1 import O(entries+bytes) :17346; L-IA-2 journal load O(lines) :17347; L-IA-17 spool merge O(n log n) :17386; L-IA-18/20/62/63/64/65 :17387-17392; L-IA-48 kept-answer append O(body) :17292; L-IA-49 lock wait ≤30 s :17293; L-IA-50 receipts ≤1 MiB :17294; L-IA-52 O(months in window) :17296; L-IA-8/59/60/61 O(entries) :20726-20729; L-IA-23/53 :20010-20012; L-IA-28 :21980.
- Alerts: put/window one read of ≤1,024 records + fsync (L-IA-30 :17055/:17127); older alerts need O(records after) (L-IA-34 :17144/:17172/:17194).
- Scrub pass O(store), census re-read+sorted per run (385,442 entries) — L-WP81-6 :17970; start reads recovery history — L-WP81-7 :17976; admission queue waits ≤250 ms then 429 — L-WP81-8 :17981.
- Upkeep runner O(entries)/O(files × months) — L-R317-02/03 :21489-21490.
- Kite master snapshots: `token_on` first use O(rows of snapshot), futures plan O(snapshots × rows) before the first-listed index, catalog listing O(entries) — L-R428-01..13 :21611-21623.
- Corporate actions / basis runs O(shares × days), O(steps) — L-R426-02..08 :21630-21636.
- GDFL re-key O(lines)/O(month files)/O(entries) — L-AG02-1..12 :19000-19011.
- Point-in-time F&O membership build O(months + records + member entries) — L-PIT-1/2 :20677-20678.
- Reference data load O(generations + rows) — L-R330-01/03 :21569-21571.
- Pull audit journal append waits on `flock` up to `APPEND_LOCK_WAIT`; holder registry unmeasured — L-AG06-01..11 :20858-20869; L-BRED23-16 :20859.
- Tick store: files larger than the tool's; rebuild O(file bytes) — L-TSW-4/5 :18977-18978.
- Options lineage O(contracts log contracts) per underlying-day — :17213.

#### API routes (`crates/api`)
- Instruments page substring search O(universe) (6.53 ms at 50,000), catalog build unbounded — §24 :1331-1442; docs/07-o1-architecture.md:80, :114-122.
- `/store`: percentage columns are a whole-census walk per request — §41.1 :3131; `held_series` §32 :2709; D-0171 violation :5147.
- `/ingest/status.json` O(entries) — §48 :3606; `/health` reads all note bytes — §67 :4533.
- `/audit.json` one count per census snapshot — :10418; sorted/extremes window reads every bar in range — D-0733 :10446.
- Unpinned `/index-stop-ranking.json` walks checkpoint directory O(reservations) — :10514 (D-0904); C4-API-04-01.
- `/frontier.json` refresh O(delta) — :10781; `/trades.json` refresh O(new rows) — :10935.
- JSON renderers: cold admissions and per-request walks (`/boolean-candidates.json`, evidence pages O(C), campaign history O(H), trade page O(trades)) — :12677 (D-1444); JR-03/04/06.
- Census-backed GET routes: one `stat` per vendor when unchanged — :9333 (D-0686); screen-budget refusals and census faults, O(H log H) — :9899 (D-0695).
- `/logs.json` and `/logs` read up to 8 MiB per admitted request — :16563 (D-2327).
- `/dayflags.json` day band decodes every `run` line: O(runs) — L-IA-39 :16987/:17003; filtered-day records — L-AF2-2..6 :19379-19383; R3-13 day band rules — :21985.
- `/feeds.json` held spans O(keys) per census generation; journal reach; archive O(members) — L-R305-2/3/5 :18450/:18484/:18527; `/calendar.json` — L-R305-4 :18540.
- `/gaps.json` O(months asked); CASH month decodes masters per request O(D) — :19204; L-CSL-3 :18703; MS-10.
- `/pull/repair.json` O(days asked) / O(months held) — L-R401-03 :18575, L-R318-01 :18931.
- `/bars/window.json` paged whole month is O(rows of the month); `/bars.json` capped at 60,000 rows — L-BBENCH-1 :22263; "Printed, never asserted: O(months) and O(entries)" :19849.
- `/disk.json` measured at most once per 30 s; seal check per vendor — L-WP81-4 :17914; L-AG05-03/06 :20875-20878.
- `/sources/day.json` reads month `.auc` whole on a miss — L-AG04-01 :19928; `/pull/quarantine.json` — L-R418-15 :22103.
- Detail/drill-down/admission routes are output-sensitive or linear: §100 :6440, §111 :6740, §112 :6798, §115 :6900, §116 :6924, §118 :6985, §125 :7226.
- **Known p99 outliers / targets NOT MET**: keyed log searches do not hold 5 ms at p99 with 60 clients or beside a writing import — L-R324-07 :20611; first-content p99 <300 ms NOT MET — L-R303-01 :20529; four page targets NOT MET on all seven pages — L-R315-01 :20899; page-side targets — L-R304-03 :20746; `/ingest` first content p99 target measured only loaded — L-R401-01 :18573; cold maxima carried browser start — L-R315-15/16 :20913-20914.

#### `cli`
- `screen` sorts every priced row twice, O(n log n) up to 10,000,000 rows — :11776 (AU-O1CLI-6).
- Four commands load a span for one number then again; audit kernel reads contexts twice; parallel rungs re-read the minute span; a rung loads its span twice — :11794, :11814, :11834, :11856 (o1cli-5/4/3/2).
- Descent 175× cheaper, still not constant-time — §91 :6047; knob read of an unset knob not O(1) — :10999.
- Whole-file rehash/rescan per ledger call (D-0934 :11009); Global Replay V1/V3 re-read whole ledger, O(P²) — :11372; Selection growth rechecks, two lookups not O(1) — :11440; Search Lineage V2/V3 Θ(N²) — :15790; single-stop search re-verifies history — :15810; recording a run reopens chosen-trades — :15828; stored completeness re-walks per cell — :15854; Boolean integrity re-reads — :10792; three resumable Boolean paths re-verify — :10675; audited span joins Θ(N²) — :10967.
- Ledger pages/lookups hash whole ledger files (O(file bytes)) — D-1681 :16282; Ledger V6 route — :16311; sweep-evidence ranking buffered whole — :16334; range rung row read-back O(runs) — :16346; screening/tier-ladder/report costs — :16415; `cli top` reads the ledger twice (AFG-19); Selection V5 lookup O(F) (C4-CLI-15-05).
- Population/Admission/Finalization/Replay/Selection V2–V6 authorities: fixed records, linear or worse total work — §§137-170 :7591-8792; `cli pool` §171 :8793.

#### Telemetry / logs
- Rolling `emit` up to 4,859× the median; p99 rises 161× from 1 to 8 threads; benches use `NO_ROTATION` and min — §46 :3449-3565.
- Audit-index generation delete O(posting files), reaper thread — L-IA-36 :16917/:16946; importer ring tails O(new bytes) — L-IA-37 :16963; keyed log search over 8 directories — L-IA-13 :16877/:16897.
- Invocation journal grows one file per audited request — :12451; state stream never rolls — L-R324-01 :20605.

---


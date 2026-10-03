# o1store2: O(1) audit of core, store, lake, pull, telemetry at HEAD 1087e54

## Verdict

CLAUDE.md §3 rule 4's "bar lookup" holds at HEAD, but only for lookup by index. Within one instrument-month the index is a computed offset: `Layout::offset_of`, then a `pread` of the verified block, served from a per-handle `Mutex<VerifiedBlock>` with a fixed `[u8; 4_088]` buffer and no heap allocation. Measured over a 37x larger file: warm ratio 0.974, cold-block ratio 1.100.

Lookup by (instrument, timestamp) is not O(1):
- (instrument, month) resolves to a path in O(1).
- Timestamp to index is a bisection (`first_at_or_after`). Measured ratio is 2.268 for 37x more bars, which is log n plus extra cold block verifies. This is DOCUMENTED in docs/06-limits.md "Timestamp lookup is a bisection, not O(1) — D-1434".
- That section's ceiling ("31 × 375 = 11,625 records, so the bound is fourteen reads") is wrong at the store layer. The store admits off-session minutes by design (D-0915), and my probe committed 37,960 one-minute bars into one month (height 16). A one-second month allows up to 2,678,400 bars (height 22).

Duplicate detection on ingest is still O(log n + batch) per re-offered batch (DOCUMENTED, same section). On the pull side, deduplication is a pre-sized `HashMap` insert per row, which is expected O(1).

Calendar, session, universe, lock and telemetry operations are O(1) or bounded by a compile-time constant. The rate governor's `reserve` is O(WINDOW_COUNT=3), with no retry loop.

Status of prior findings:
- FIXED: o1store-1 (`emit_if!`), o1store-2 (`MAX_PRICE_TEXT`), probestore-3 (the catalog no longer follows symlinks), and the cold-verify heap allocation (D-1433).
- Not re-checked here: the prior pass's probestore-1, -2, -4 to -7. They are behaviour bugs, not O(1) findings, so they are outside this task. Do not read them as fixed.

New:
- Two docs/06-limits.md passages still describe heap buffers on the cold verify path that no longer exist.
- The "fourteen reads" ceiling is understated.
- Two info items.

## Findings

| id | sev | crate | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| o1store2-1 | low | store (docs) | docs/06-limits.md:12037-12038, 12057; crates/store/src/file.rs:2257-2260 | The D-1434 register gives the bisection ceiling as "A one-minute month holds at most 31 × 375 = 11,625 records, so the bound is fourteen reads". The store does not enforce session hours (`Admission` checks month and grid only, file.rs:2737-2741 "Session membership. 03:00 IST is inside the month and on the minute grid, and it is admitted"). A 1-min month can therefore hold 31×1440 = 44,640 records (height 16), and a 1-s month (`Timeframe::SECOND_1`, path.rs:398, in `KNOWN` at 497) up to 2,678,400 (height 22). The probe-count test pins 11,625 as the "ceiling month", so it does not cover this. The `/bars.json` "up to 28 reads" figure is understated in the same way. | Probe: `BarFile::open_or_create` + `append` of 37,960 consecutive one-minute bars into 2024-06 gave `Ok(Appended::Committed{first_index:0,n_valid:37960})` (assert passed). Output: `log2 heights: small=10 large=16`. | NEW (each half is documented on its own: D-0915 / S-30-session, and the D-1434 bound; the contradiction between them is not) |
| o1store2-2 | low | store (docs) | docs/06-limits.md:12051-12053 and 9118-9119 | Both passages still list heap allocations on the cold verify path: "a heap buffer of the same size" (D-1434 section) and "one `fstat`, two heap buffers, a `pread`..." (D-0688 tail section). Since D-1433/D-0914 the covered bytes go into the handle's fixed `cache.bytes: [u8; MAX_BLOCK_LEN]` (file.rs:279) and past-commit records into a stack array (file.rs:2533 `let mut beyond = [0u8; MAX_BLOCK_LEN];`). `crates/store/tests/cold_read.rs:167` fails the build if either function body contains `vec!`. The register overstates the cost and contradicts a gate. | file.rs:2499-2505 `.and_then(\|span\| cache.bytes.get_mut(..span))`; 06-limits:12052 "a heap buffer of the same size, and a CRC-32C over it". | NEW (the code change is KNOWN:fix-queue_lane3-b.md:905 / ET-o1-proof-coverage-4, fixed by D-1433; the stale register text is not listed) |
| o1store2-3 | info | store (docs) | docs/05-decisions.md D-1433 (line ~51215) | The decision says "a fixed `[u8; MAX_BLOCK_LEN]` (4,096;". The code is `const MAX_BLOCK_LEN: usize = 4_088;` with `const _: () = assert!(MAX_BLOCK_LEN == 4_088);` (file.rs:234, 238). This is a wrong number in an append-only ledger and has no cost impact. | quoted | NEW |
| o1store2-4 | info | lake | crates/lake/src/reader.rs:87-112 | `LakeFile::open` still calls `fs::read(path)` with no byte cap, which is O(file bytes) and unbounded (docs/07-o1 law 5 says "Bound every input at the boundary"). Its open event also builds `path.display().to_string()` before `telemetry::emit` filters, once per file. The module doc (reader.rs:3-8) states the O(file bytes) cost. The cap is not stated, and lake still has no workspace consumer. Fix: a `MAX_LAKE_BYTES` metadata check before the read (the pattern `pull::archive::read_bounded`, archive.rs:834, now uses), and `emit_if!`. | `let raw = fs::read(path).map_err(...)` | DOCUMENTED (cost):reader.rs module doc. The missing cap was noted but not filed by the prior o1store row 23. |

Nothing else new. Per-price `number.to_string()` allocations (pull http.rs:1414, rolling.rs:733) are now capped by `MAX_PRICE_TEXT = 64` (core price.rs:30, 223). They are O(1) and allocating, and are not filed.

## Prior rows re-verified at HEAD

| prior # | HEAD location | status |
|---|---|---|
| 1 read_record/read_row | file.rs:2225, 2318 (stack `[0u8; MAX_ROW_LEN]` 2330) | O(1), still holds. The warm path now copies out of verified bytes under a `Mutex` (file.rs:2355-2380): **FIXED-since-prior** for ET-bars-candles-store-0 (D-1433). |
| 2 warm hit | file.rs:2364-2378 `if cache.block != block \|\| cache.n_valid != n_valid` | O(1): one uncontended lock plus a copy. One handle shared across threads serialises on it (stated in D-1433). |
| 3 cold miss | file.rs:2449-2560 | O(block). The `vec!` is gone, so this is **FIXED-since-prior** (ET-o1-proof-coverage-4). Cold read is now benched: crates/store/benches/ratio.rs `C-BC-01 cold read_record, 10x/100x file`, so ET-bars-candles-store-9 is partly closed. |
| 4 past_the_commit | file.rs:2573-2598 | O(1). It takes a stack `room` (D-0914), with no allocation. |
| 5 first_at_or_after | file.rs:2266, 2903-2925 | NOT O(1). DOCUMENTED: 06-limits:12027 (D-1434), with the understated ceiling described in o1store2-1. |
| 6 already_stored | file.rs:2850-2885 | NOT O(1) per batch. DOCUMENTED, same section. |
| 7 suffix_that_follows | file.rs:2615-2659 | O(1) per overlapping bar. Unchanged. |
| 8 survey + new `Admission::admit` | file.rs:2669, 2785-2808 | O(1) per bar (two compares plus one `rem_euclid`). |
| 9 append forward | file.rs:2021-2197 | O(batch), with 3 `sync_all` per batch. Unchanged. |
| 10 seal_committed | file.rs:1808-1866 | O(blocks touched). It still does `let mut bytes = vec![0u8; span];` (1859) per touched block per append. That is a heap allocation, O(1) per bar and off the read path, and is not filed. A `[u8; MAX_BLOCK_LEN]` stack buffer would remove it. New `verify_old_tail_before_reseal` (1899) adds one block verify per append. |
| 11 append telemetry | file.rs:2185-2194 `telemetry::emit_if!(` | **FIXED-since-prior** (o1store-1). |
| 12 open | file.rs:1215, 1368, 1482, 3002-3040 | O(1): a <= 32 KiB header region and `MAX_SLOT_COUNT` decodes. New `open_read` (6005) is `O_NONBLOCK` plus one `fstat`. Locks are `Flock::try_lock`/`try_lock_shared` (flock.rs:109, 121), non-blocking with no retry loop. |
| 13-14 Layout resolve/offset | layout.rs:294 and arithmetic | O(1), unchanged. |
| 15 crc::update | crc.rs:148 | O(bytes), at most one block. DOCUMENTED §14. |
| 17 AuditedBarFile::read_record | checksum_audit.rs:243-268 | O(block). Unchanged apart from the `open_flags` constants. DOCUMENTED docs/24. |
| 18 audit | checksum_audit.rs:274 | O(source bytes). DOCUMENTED. |
| 19 catalog::walk | catalog.rs:279-319 (sort 317, dedup 318) | O(entries + held log held). DOCUMENTED §86. probestore-3 is **FIXED-since-prior**: `admit` uses `file_type()`, and a symlink is counted, not followed (catalog.rs:338-351). |
| 20 repair::publish | repair.rs:238, 299 | O(rows), capped by `MAX_ROWS`. It now also calls `Admission::admit` (O(1)/row). `sync_ancestors` (487) loops once per directory level to the root, which the path depth bounds. |
| 21-22 StorePath, Timeframe::from_secs | path.rs:837, 536 | O(1). New `ist_bounds_micros` is closed-form const arithmetic. |
| 23 LakeFile::open | reader.rs:87 | See o1store2-4. |
| 25 Columns::index_of | reader.rs:406 | O(columns), fixed schema. Unchanged. |
| 27 Batch::row | batch.rs:73 | O(1). |
| 29-31 MemberIndex contains/position | universe.rs:4385, 4424 | O(1), compile-time table. universe.rs is unchanged since bc53131. |
| 32 is_sweepable | instrument.rs:354-372 | `FNO_INDEX.contains` (hash) plus `.all(!=)` over the 5-entry `FNO_INDEX_UNDERLYINGS`: bounded by a compile-time constant. No production `FNO_UNDERLYINGS.contains` exists in these 5 crates; every slice scan of it is under `#[cfg(test)]`. |
| 33-34 decode_master_row / board_of | vendor.rs:1429, 1149, over_wide 407 | O(1), bounded by `MAX_FIELD_BYTES`. The diff adds only `trim_ascii` and refusals. |
| 35 from_rupee_text_half_up | price.rs:217-224 `if text.len() > MAX_PRICE_TEXT { return Err(PriceError::TooLong); }` | **FIXED-since-prior** (o1store-2). |
| 39-43 telemetry emit | sink.rs:1089-1116 (fast floor 1098, overrides 1105, `Mutex` 1115); lib.rs:262 `emit_if!` | O(1). Measured (below). |
| 44 roll | sink.rs:1267 | Amortised O(1) per event. |
| 45 tail | tail.rs:366, 480, 586, 595 | O(N × line width), with `max_scan_bytes` now enforced per read (`take = READ_BLOCK.min(pos).min(budget)`). DOCUMENTED (module doc). |
| 47 resume_seq becomes resume_point | sink.rs:1381 | Per open: at most `keep_files` (u8) `metadata` calls plus one 64 KiB tail read. Bounded. |

## New or changed operations since bc53131 (pull and other crates)

| path | file:line | unit | cost | verdict | notes / documented |
|---|---|---|---|---|---|
| `calendar::kind_of` | pull/src/calendar.rs:392-425 | per day | range check, bitmap byte, then 5 + 4 fixed compares (`const _: () = assert!(LENGTH_UNMEASURED.len() == 5 && IRREGULAR.len() == 4)`) | bounded (compile-time) | Measured ratio 0.976 (below). |
| `calendar::sessions_between` | calendar.rs:451-465 | per call | loop over `first..=last`, capped at `LAST_DAY-FIRST_DAY` (~2,468) | bounded (compile-time) | Test-only callers. |
| `Calendar::from_observed` | calendar.rs:1285-1340 | per derived calendar | `vec![DayKind::Closed; span]`, O(span + observed). The span is bounded only by the observed days (api caller: YearMonth 1970..9999) | NOT O(1) | DOCUMENTED: 06-limits:11077 ("The derived-calendar render ... is still O(span)"). Measured ratio 3735.2 for 1000x span. `Runtime::kind_of`/`Calendar::kind_of` (1069, 1397) are O(1) indexed gets. |
| `session::Day::{new, succ, months_before, days_from_epoch, from_days}`, `IstMoment::from_epoch_secs` | pull/src/session.rs:358-620, 707 | per call | closed-form const arithmetic | O(1) | `months_before` doc: "There is no loop here". |
| ingest dedup (pull side) | pull/src/ingest.rs:1058, 1088 `seen.insert(row.timestamp, *row)` | per row | `HashMap::with_capacity(raw.rows.len())` insert | expected-amortised O(1) | The store side (`already_stored`) is DOCUMENTED D-1434. |
| rate `reserve` | pull/src/rate.rs:940-961 | per request | 3 windows (`WINDOW_COUNT = 3`, rate.rs:199), one sleep | O(1) | It replaced the unbounded admit/sleep loop (o1api-54, D-1203). Rustdoc says "UNVERIFIED as a measurement". |
| `wait_for_permit` | pull/src/http.rs:661-705 | per request | one `Mutex` + `reserve` + one `tokio::sleep` | O(1) | No retry loop. |
| body read | http.rs:2278-2309, 2635-2662 | per response | O(min(body, cap)) | bounded by cap | Breaks or refuses at `cap`. |
| `archive::read_bounded` | pull/src/archive.rs:834-858 | per member | O(member bytes) <= `MAX_MEMBER_BYTES` (256 MiB), checked before the read | bounded | DOCUMENTED 06-limits:11071-11073. |
| `Admission::admit` | store/src/file.rs:2785 | per bar | 2 compares + `rem_euclid` | O(1) | Rustdoc says "UNVERIFIED as a measurement". |
| `Header::decode_parts` reserved/flags checks | store/src/header.rs (+D-1353/1354) | per slot | 2 u32 reads | O(1) | |
| EINTR loops `read_fully`/`write_fully` | store/src/file.rs:3148-3240 | per I/O | retries on `Interrupted` only, refuses `Ok(0)` | O(1) | Standard. Not bounded against a signal storm, which is a host property. |
| telemetry `last_record`/`resume_point` | telemetry/src/sink.rs:1381+ | per open | <= keep_files stats + <= 64 KiB read | bounded | |

## Measurements (Rust probes, run then deleted)

The test profile is `[optimized + debuginfo]` per cargo's banner. The 4-core box was shared with other workers' cargo jobs, so absolute ns values are noisy. The ratios are as printed.

**Probe 1**, `cargo test -p store --test zz_audit_o1store2_1 -- --nocapture`. Small file 1,022 bars (14 blocks), large file 37,960 bars (520 blocks), one-minute, 2024-06. Cold reads pick a random block different from the previous one, so every read is a cache miss.
```
n_small=1022 n_large=37960 (x37.1)
read_record warm  ns/op: small=25.5 large=24.8 ratio=0.974
read_record cold  ns/op: small=3129.1 large=3442.6 ratio=1.100
first_at_or_after ns/op: small=15372.8 large=34862.1 ratio=2.268
dup append (already_stored) ns/op: small=16634.3 large=36519.9 ratio=2.195
log2 heights: small=10 large=16
test probe_store_lookup_costs_at_two_sizes ... ok
```
Reading: index lookup is flat (O(1) holds, warm and cold). Timestamp lookup and duplicate re-offer grow with the file. They grow faster than height alone (10 to 16, x1.6) because more probes land in distinct blocks and each pays a cold verify of about 3 µs. The assert `Appended::Committed{n_valid:37960}` passing is the evidence for o1store2-1.

**Probe 2**, `cargo test -p pull --test zz_audit_o1store2_2 -- --nocapture`:
```
from_observed with 2 observations: span 1,000 -> 722 ns, span 1,000,000 -> 2698094 ns, ratio=3735.2; span()=1000000
kind_of ns/op: day 18300=7.78 day 20700=7.59 ratio=0.976
test probe_calendar_costs ... ok
```

**Probe 3**, `cargo test -p telemetry --test zz_audit_o1store2_3 -- --nocapture`. The first run gave ratios of 1.571 (written) and 1.520 (filtered). The filtered path is one relaxed load and cannot depend on file size, so that 1.5x was box noise. The second run used reversed order (large, small, large):
```
rerun large: written=823 filtered=9.24
emit written ns/op: after 1,000 lines=834 after 200,000 lines=864 ratio=1.036
emit filtered ns/op: after 1,000 lines=9.25 after 200,000 lines=9.21 ratio=0.996
```

## Housekeeping

All three probe files were deleted. `git status --porcelain | grep -c o1store2` gives `0`. The untracked files remaining (`zz_audit_attacksweep_*`, `zz_audit_o1eng2_*`, `zz_audit_attackdata_1`) belong to other workers.

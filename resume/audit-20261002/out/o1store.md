# o1store — O(1) audit of crates store, lake, core, telemetry (commit bc53131)

Method: source reading only. No cargo run; no measurement was needed for these verdicts, so none is claimed.
Docs read first: CLAUDE.md §3 rule 4, docs/07-o1-architecture.md (all of it), docs/06-limits.md §§1, 17, 23, 37, 86 and others found by grep,
docs/24-checksum-admission.md §cost. Cross-checked against queued_index.txt + fq/lane1.md, docs/11-findings.md, docs/05-decisions.md.

## A. Hot-path table

Columns: # | name | file:line | unit | real complexity | verdict | why / documented? / achievable?

### store
| # | name | file:line | per | complexity | verdict | notes |
|---|---|---|---|---|---|---|
| 1 | `BarFile::read_record` -> `read_row` | store/src/file.rs:1895, 1981 | per bar lookup | O(1): `offset_of(index)` arithmetic, one `pread` into `[0u8; MAX_ROW_LEN]` stack buffer (file.rs:2000 `let mut image = [0u8; MAX_ROW_LEN];`) | O(1) | Syscall latency is the device's (stated UNVERIFIED in rustdoc). Benches only time the warm path: QUEUED:ET-bars-candles-store-9, ET-o1-proof-coverage-4. |
| 2 | `verify_block_of`, warm hit | file.rs:2072, 2081 | per bar read | O(1): `if self.verified.load(Ordering::Relaxed) == block { return Ok(()); }` | O(1) | The served bytes are never checked on this path: QUEUED:ET-bars-candles-store-0. |
| 3 | `verify_block_of`, cold miss | file.rs:2097-2140 | first read in a block | O(block) = O(1) in the file: `let mut bytes = vec![0u8; span];` (2105), a pread of up to 4,088 B, a 4-B sidecar pread, CRC-32C | O(1) (bounded by BLOCK_LEN) | Heap allocation for each cold block. A stack `[u8; BLOCK_LEN]` would remove it (BLOCK_LEN is a const). QUEUED:ET-o1-proof-coverage-4 (names the `vec!` at 2105). |
| 4 | `past_the_commit` | file.rs:2167-2190 | each tail-block verify | O(1): returns early off the tail; on the tail, one `fstat` and `vec![0u8; span]` with span < one block | O(1) | Allocates even for span 0 (`vec![0u8; 0]` does not allocate, so only a non-empty span pays). QUEUED (same as #3). |
| 5 | `first_at_or_after` (public, also inside the re-pull path) | file.rs:1931, 2397-2418 | per timestamp->index lookup | O(log n_valid) record reads, n = records in the month (~8,250 for 1-min, so ~14 probes). Each probe in a new block pays #3 | NOT O(1) | Rustdoc says `log2(n_valid)` reads and "UNVERIFIED as a measured bound"; not in docs/06-limits. O(1) IS achievable for minute data: within one session the index is arithmetic, `(ts - session_open)/timeframe` plus a per-day start index (a fixed ~23-entry day->first-index table in the header or a sidecar), followed by one verifying read. Needs a format addition, so it is a decision. QUEUED:W3-store1-0. |
| 6 | `already_stored` (re-pull path) | file.rs:2344-2378 | per re-offered batch | O(log n_valid + batch) reads | NOT O(1) per batch (O(1) per offered bar after the locate) | Rustdoc: "This is NOT an O(1) path and does not claim to be". Not in 06-limits. The batch term is inherent (every bar must be compared). The log term goes the same way as #5. QUEUED:W3-store1-1. |
| 7 | `suffix_that_follows` | file.rs:2209-2250 | per partially overlapping batch | `batch.partition_point` O(log batch) + one read per overlapping bar | O(1) per bar | Inherent: every overlapping bar is compared. Recursion into `append` happens once. Documented in its own `# Cost`. |
| 8 | `survey` | file.rs:2263-2302 | per append, per bar | O(batch), O(1) per bar | O(1)/bar | Inherent validation. |
| 9 | `BarFile::append` forward path | file.rs:1711-1882 | per batch | O(batch) bytes (`Vec::with_capacity(batch.len()*R::LEN)`), 2 writes, 3 `sync_all` (bars x2, crc x1) | O(1) syscalls per batch, O(1) per bar | fsync is per batch, never per record. Correct by design (docs/02 §5). |
| 10 | `seal_committed` | file.rs:1550-1612 | per append | O(blocks touched) x one block re-read + CRC + 4-B write | O(1) per bar (bounded by batch) | Re-reads and re-CRCs the whole old tail block instead of extending, which is still bounded. Correctness issue QUEUED:W3-store1-3. |
| 11 | append's committed telemetry event | file.rs:1857-1861 | per append | `telemetry::Value::Str(&self.bars_path.display().to_string())` is evaluated BEFORE `emit` filters | O(1) but FALSE CLAIM | Comment at 1854-1856 says "a normal run at the `Info` floor pays one relaxed atomic load and writes nothing". It also pays a path format plus a heap `String` on every append. Fix: `telemetry::emit_if!`. Same class as D-0087 (15 allocating sites in api/pull), but this store site and its false sentence are not listed. **NEW (o1store-1).** |
| 12 | `open_or_create` / `open_existing` -> `validated` -> `read_header` | file.rs:966, 1116, 1223, 2496 | per open | O(1): reads <= 32,768-B header region (`vec![0u8; ...REGION_LEN]`), at most MAX_SLOT_COUNT slot decodes (`highest_claim` 2528, `header::committed` loop `for _ in 0..MAX_SLOTS` header.rs:620) | O(1) | Constant 32 KiB per open. Blocking opens: QUEUED:AC-whp-cx-0. |
| 13 | `Layout::resolve` | store/src/layout.rs:294-301 | per open | `.iter().find(\|layout\| layout.version == version)` over a one- or few-row const table | O(1) (fixed table) | — |
| 14 | `Layout::offset_of` / `block_of` / `covered_byte_range` | layout.rs:384, 422, 501 | per read | const arithmetic | O(1) | Layer 5. |
| 15 | `crc::update` | store/src/crc.rs:148-170 | per block | O(bytes), slice-by-8 | O(1) per block (bytes <= 4,088) | Documented in 06-limits §14 (O(n) in bytes covered, always). |
| 16 | `block::sealed_past_the_commit` | store/src/block.rs:303-330 | tail verify after a crash | `for ... in within.chunks_exact(stride)`, within <= one block | O(1) | Bounded by block_len. |
| 17 | `AuditedBarFile::read_record` (warm audited row) | store/src/checksum_audit.rs:245-273 | per bar | O(block). Each row pays `require_current` twice (each = `checksum_inputs` with a `with_extension` PathBuf alloc + 3 `fstat` + 3 `lstat`), a third `checksum_inputs`, one block pread (<= 4,088 B) + 4-B pread, a full CRC of the block. No block cache | O(1) (fixed-block), heavy constant | A forward walk pays 73x the CRC work of `BarFile`'s cached path, plus 12 metadata syscalls and 3 allocations per row. The cli wrapper adds 3 more `require_current` per row (cli/src/checksum_receipts.rs:86-90). DOCUMENTED: docs/24-checksum-admission.md:308-309 ("A warm row check has fixed-block work"). The redundancy is by design (TOCTOU re-authentication), so no finding. |
| 18 | `checksum_audit::audit` | checksum_audit.rs:276-334 | per audited open | O(source bytes) (`for block in 0..blocks`) | NOT O(1), inherent | DOCUMENTED: docs/24:308 ("Cold audit ... O(source bytes/records)"). Capped by `max_bytes`. |
| 19 | `catalog::walk` + `classify` | store/src/catalog.rs:180-216, 227 | per CLI listing | O(files) + `out.held.sort_unstable()` O(h log h) | NOT O(1), inherent | DOCUMENTED: 06-limits §86. Only caller is cli/src/batch.rs:317, which matches §86. `classify` allocates a `Vec<&str>` and 3 Strings per file, so O(1) per entry. |
| 20 | `repair::publish` / `retain_timestamps` | store/src/repair.rs:238, 295-324 | per repair | O(n_valid + merged), capped by MAX_ROWS | NOT O(1), inherent | Every row must be compared. Off any per-bar path. Not in 06-limits (rustdoc only). |
| 21 | `StorePath::new` / `check_segment` / `to_path_buf` | store/src/path.rs:766, 956, 904 | per open | O(segment) with `text.len() > MAX_SEGMENT_LEN` refused first | O(1) | Law 5 respected. |
| 22 | `Timeframe::from_secs` | path.rs:523-528 | per parse | `.iter().find` over `Timeframe::KNOWN` (fixed) | O(1) | — |

### lake
| # | name | file:line | per | complexity | verdict | notes |
|---|---|---|---|---|---|---|
| 23 | `LakeFile::open` | lake/src/reader.rs:78 | per file | `fs::read(path)` O(file bytes), no size ceiling | NOT O(1), inherent | DOCUMENTED in module rustdoc reader.rs:3-8 ("O(file bytes) and it is not pretended otherwise"). Law-5 note: the read is unbounded. No byte cap exists before `fs::read`, only "largest measured is a few megabytes". The telemetry line also builds `path.display().to_string()` before `emit` (reader.rs:99). That is once per file, so it is minor and not filed. |
| 24 | `read_row_group` | reader.rs:180-298 | per row group | O(rows x columns), O(1) per row per column; `rows > self.bytes.len()` guard | O(1)/row | DOCUMENTED (rustdoc reader.rs:7). |
| 25 | `Columns::index_of` | reader.rs:341-346 | per column read (7 or 19 per group; `mismatch` calls it again) | `(0..schema.num_columns()).find(\|i\| schema.column(*i).name() == name)` gives O(columns) string compares, O(columns^2) per group | O(1) (fixed schema) | Bounded because `detect` pins the exact schema, so this is not a scan in the rule-4 sense. Arithmetic is achievable: `detect` already verifies leaf i == spec i (schema.rs:124-140), so the index is the spec position. Cosmetic. Not filed. |
| 26 | `expand` / `price` / `optional_price` / `greeks` | reader.rs:478-512, 602-684 | per row | O(1) per row (the `defs.iter().filter(..).count()` at 503 runs only on the error arm) | O(1)/row | — |
| 27 | `Batch::row` / `iter` | lake/src/batch.rs:73-93 | per row | indexed `get(i)` on each column | O(1) | Proved by `lake::batch::row_lookup_does_not_scan`. |
| 28 | `ContractName::parse` | lake/src/contract.rs:128 | per name | `name.len() > MAX_NAME_BYTES` refused first; `parse_month` walks 12 fixed months | O(1) | — |

### core
| # | name | file:line | per | complexity | verdict | notes |
|---|---|---|---|---|---|---|
| 29 | `MemberIndex::position` / `contains` | core/src/universe.rs:4424-4463, 4385 | per membership lookup | FNV-1a over <= SYMBOL_CAPACITY bytes (`symbol.len() > SYMBOL_CAPACITY` refused), open-addressed linear probe; worst probe measured 6/7 | O(1) worst-case (compile-time table) | Layer 4. Documented 07-o1 table. |
| 30 | `of_equity` | universe.rs:4114-4134 | per instrument | 6 x #29 | O(1) | — |
| 31 | `nse_isin` | universe.rs:4201-4204 | per lookup | #29 + `Isin::new` (fixed 12 B) | O(1) | — |
| 32 | `InstrumentKey::is_sweepable` | core/src/instrument.rs:354-374 | per instrument | `SWEPT` (2) `.any`, `FNO_INDEX.contains` (#29), 5 fixed compares | O(1) | — |
| 33 | `decode_master_row` | core/src/vendor.rs:1415 | per master row | `over_wide()` (vendor.rs:396, 11 field-length reads, `> MAX_FIELD_BYTES` refused) before any scan. Then the `TEST_MARKERS` substring scan, `collapse_spaces` (<= SYMBOL_CAPACITY), `segment_of` slice `.contains(&code)` on <= 4 entries (vendor.rs:1214, allowlisted by gate 11) | O(1) (bounded by MAX_FIELD_BYTES) | Law 5 enforced at the boundary (D-0033). |
| 34 | `board_of` | vendor.rs:1138-1152 | per row | 3 x #29 | O(1) | D-0065. |
| 35 | `Paisa::from_rupee_text_half_up` | core/src/price.rs:180-230 | per price cell (pull http.rs:1248 and rolling.rs:679 per bar; vendor.rs:1805 per master strike) | O(text.len()): `whole.bytes().all(..)`, `fraction.bytes().all(..)`, `fraction.bytes().skip(3).any(..)`, `whole.parse()`. No length bound inside core | O(1) only through callers | Law 5 ("bound every input at the boundary") is met by callers, not by the function. vendor.rs:1805 is gated by `over_wide`. http.rs:1248 passes a `serde_json::Number::to_string()` (bounded shortest round-trip unless `arbitrary_precision`, UNVERIFIED which features are on). rolling.rs:679 passes `cell.to_string()`, unbounded by type. Also note: those two pull callers allocate a `String` per price per bar (pull is outside this audit's scope; mentioned, not filed). Achievable: a `MAX_PRICE_TEXT` refusal at the top of the function. **NEW (o1store-2), low.** |
| 36 | `Symbol::new` / `Isin::new` / `VendorId::new` | core/src/symbol.rs:62, isin.rs:81, vendor.rs:286 | per parse | fixed-capacity copy after a length refusal | O(1) | Layer 1. |
| 37 | `Paisa::from_rupees_half_up`, checked ops | price.rs:98, 261, 273 | per price | arithmetic | O(1) | — |
| 38 | `blake3::Hasher::update` / `hash` | core/src/blake3.rs:421, 452 | per digest | O(input bytes) | NOT O(1), inherent | A hash over n bytes is n work. Run-identity input is fixed-size. Data digests are per file, off per-bar paths. |

### telemetry
| # | name | file:line | per | complexity | verdict | notes |
|---|---|---|---|---|---|---|
| 39 | `Sink::emit_for_run` fast reject | telemetry/src/sink.rs:1085-1087 | per event | one relaxed load + compare | O(1) | — |
| 40 | `Sink::level_for` / `admits` | sink.rs:839-866, 1055-1060 | per admitted event when overrides exist | <= MAX_TARGET_LEVELS (8) prefix compares (sink.rs:156, 342) | O(1) | Rustdoc says "UNVERIFIED as a measurement". |
| 41 | `Event::with` | telemetry/src/event.rs:154-164 | per field | fixed `[_; MAX_FIELDS]` slot, overflow counted | O(1), no alloc | — |
| 42 | `encode::line` / `push_capped` / `push_value` | telemetry/src/encode.rs:43-91, 93-137, 208-222 | per written event | O(sum of capped fields): target/msg/key/str values truncated to MAX_*_BYTES, <= MAX_FIELDS fields. Writes into the reused `inner.buf` (`inner.buf.clear()` sink.rs:1106) | O(1) | — |
| 43 | `Sink::emit_for_run` write | sink.rs:1102-1197 | per written event | process-wide `Mutex` + one `write_all` (no fsync: `sync` is separate, sink.rs:1211) | O(1) per event | Serialises all emitting threads. That is contention, not complexity class. |
| 44 | `Sink::roll` | sink.rs:1254-1290 | once per max_file_bytes | <= 2*keep_files renames/removes + path `format!`s | amortised O(1) per event | Rustdoc states the bound. |
| 45 | `tail::walked` / `walk_back` / `take_line` | telemetry/src/tail.rs:358-418, 469-547, 558 | per /logs query | O(N x line width + scanned bytes), capped by `max_scan_bytes`, MAX_LIMIT, MAX_LINE_BYTES (carry reset at 534-536). `read_already.contains(&id)` scans <= keep_files (u8) entries. Each `read_block` allocates a fresh `vec![0u8; span]` | NOT O(1) in N (inherent), O(1) in file size | DOCUMENTED in tail.rs module doc ("O(N x line width), and O(1) in the size of the file"). The `contains` is allowlisted by gate 11. |
| 46 | `Record::decode` / `object` / `field` | telemetry/src/record.rs:110, 241-280, 74-80 | per line read | O(line), fields capped at MAX_FIELDS (`Some(b',') if out.len() < MAX_FIELDS`), `field` is `.iter().find` over <= 12 | O(1) (bounded) | Allocates a String per key, which is bounded. |
| 47 | `resume_seq` | sink.rs:1338-1355 | per sink open | reads the last <= 64 KiB and splits/decodes backwards | O(1) | Startup only. |

## B. Findings

| id | severity | crate | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| o1store-1 | low | store | crates/store/src/file.rs:1854-1861 | The append's committed-event comment claims a filtered run "pays one relaxed atomic load and writes nothing". The event's `file` field is built with `self.bars_path.display().to_string()` before `telemetry::emit` can filter it, so every append also pays a path format plus a heap `String`, at the default Info floor. The cost is constant per batch, so the rule-4 class is unaffected, but the stated cost is false. `telemetry::emit_if!` exists for exactly this. | `// \`Debug\`: this is the per-member write path, so a normal run at the // \`Info\` floor pays one relaxed atomic load and writes nothing.` followed by `telemetry::emit(&telemetry::Event::debug("store.append", "committed").with("file", telemetry::Value::Str(&self.bars_path.display().to_string()))` | NEW (same class as D-0087's "15 allocating sites", docs/05-decisions.md ~8167-8180, which names api/pull sites but not this one) |
| o1store-2 | low | core | crates/core/src/price.rs:180-221 | `Paisa::from_rupee_text_half_up` walks its whole input three times (`bytes().all`, `bytes().all`, `bytes().skip(3).any`) with no length refusal. Its O(1) per call rests entirely on callers bounding the text (law 5). One caller (vendor.rs:1805) is bounded by `over_wide`. The pull callers (http.rs:1248, rolling.rs:679) pass a freshly allocated `to_string()` of a decoded cell, and the core function would accept an arbitrarily long digit string. | `if !whole.bytes().all(\|b\| b.is_ascii_digit()) \|\| !fraction.bytes().all(\|b\| b.is_ascii_digit())` ... `let tail_nonzero = fraction.bytes().skip(3).any(\|b\| b != b'0');`. No `text.len() >` check in the function. | NEW |

## C. Documented O(1) claims re-checked

* CLAUDE.md §3 rule 4 "bar lookup O(1)": holds on the warm and cold path (#1-#4). The served-bytes-unverified defect and the warm-only bench are QUEUED (ET-bars-candles-store-0/-9, ET-o1-proof-coverage-4).
* 07-o1 layer 4 "never binary_search": `first_at_or_after` is a hand-written bisection (#5). It is not spelled `binary_search` and is off rule 4's five operations, but the operation is not O(1) and is absent from 06-limits. QUEUED:W3-store1-0 / W3-store1-1.
* 07-o1 layers 1, 4, 5, 11 for core/store: verified as stated (#14, #29-#36).
* `read_row`'s "buffer is on the STACK" claim (file.rs:1945-1963): true for the record. The cold block verify still heap-allocates (#3, QUEUED).
* `Batch::row` O(1) claim (lake): true.
* telemetry `admits`/`emit` O(1): true (bounded by MAX_TARGET_LEVELS, MAX_FIELDS and the byte caps). The rustdoc honestly labels it unmeasured.
* No HashMap built per call, no sort per call on a per-bar or per-record path, no file reopened per record (except the audited door's metadata re-reads, which are documented), no fsync per record, and no quadratic nested loop over data was found in these four crates.

git status: no repo files touched.

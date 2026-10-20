# cloud-attack-law-20261016 — lens 2 appendix: every hidden-fallback hit, classified

Companion to `reports/cloud-attack-law-20261016.md`. One row per non-test source line of `crates/**` (base `cloud/base-20261016`, c9e3472e) that holds `unwrap_or(`, `unwrap_or_default(`, `unwrap_or_else(`, `.ok()`, `let _ =`, `Err(_) =>`, `filter_map(`, `.flatten()`, `eprintln!`, `.is_ok()` or `_ =>`, each read in its enclosing function and classified. The 916 `let _ = write!/writeln!` lines are not listed: their receivers are `String`s (the io-typed writers at `crates/api/src/server.rs:99-116` return their results). Line numbers are of the BASE tree; a fix on this branch may have moved a line by a few.

Classes: **SWALLOWED** a real failure becomes a default, a skip or a log-then-continue with the output not marked degraded (a finding); **SURFACED** named and counted, refused or alerted; **BENIGN-OPTION** an `Option` default that hides no failure; **BENIGN-INFALLIBLE** the ignored result cannot fail or its failure is irrelevant by construction; **BENIGN-MATCH** a `_ =>` arm that is not a default on a failure.

## Counts per crate

| crate | SWALLOWED | SURFACED | BENIGN-OPTION | BENIGN-INFALLIBLE | BENIGN-MATCH | AMBIGUOUS | total |
|---|---|---|---|---|---|---|---|
| api | 33 | 149 | 455 | 399 | 181 | 0 | 1217 |
| cli | 7 | 140 | 148 | 323 | 198 | 0 | 816 |
| core | 0 | 1 | 6 | 14 | 15 | 0 | 36 |
| costs | 0 | 0 | 7 | 0 | 1 | 0 | 8 |
| engine | 0 | 4 | 1 | 9 | 4 | 0 | 18 |
| greeks | 0 | 0 | 0 | 0 | 1 | 0 | 1 |
| indicators | 0 | 1 | 13 | 33 | 7 | 0 | 54 |
| lake | 0 | 4 | 16 | 12 | 8 | 0 | 40 |
| pull | 16 | 46 | 219 | 236 | 116 | 0 | 633 |
| runner | 4 | 10 | 103 | 94 | 19 | 0 | 230 |
| store | 0 | 18 | 33 | 140 | 25 | 0 | 216 |
| telemetry | 2 | 4 | 10 | 29 | 18 | 0 | 63 |
| vocab | 0 | 1 | 5 | 5 | 12 | 0 | 23 |
| **all** | **62** | **378** | **1016** | **1294** | **605** | **0** | **3355** |

## Every hit

| where (base file:line) | class | reason |
|---|---|---|
| `crates/store/src/block.rs:316` | BENIGN-INFALLIBLE | usize::try_from(u64 stride) cannot fail on 64-bit; saturation yields no record -> None refusal (documented 311-315) |
| `crates/store/src/block.rs:318` | BENIGN-INFALLIBLE | usize::try_from(block_len) cannot fail on 64-bit targets; documented 311-315 |
| `crates/store/src/block.rs:320` | BENIGN-OPTION | past shorter than room: scan all of past; CRC match still required to accept a record |
| `crates/store/src/block.rs:353` | BENIGN-INFALLIBLE | usize->u64 len; u64::MAX makes seal refuse BlockLengthMismatch (documented 347-351) |
| `crates/store/src/checksum_audit.rs:56` | BENIGN-INFALLIBLE | silences unused var on unsupported OS; function returns Err(Unsupported) next line |
| `crates/store/src/crc.rs:140` | BENIGN-INFALLIBLE | u8 index into 256-entry table; None arm unreachable (documented 133-137) |
| `crates/store/src/file.rs:348` | BENIGN-OPTION | record(): None means 'not inside verified bytes'; caller re-verifies/refuses |
| `crates/store/src/file.rs:1520` | BENIGN-MATCH | exhaustive kind->geometry dispatch; non-sidecar kinds are Bars (documented 1513-1515) |
| `crates/store/src/file.rs:1533` | BENIGN-MATCH | exhaustive kind->table dispatch, same input as geometry_of |
| `crates/store/src/file.rs:1637` | BENIGN-INFALLIBLE | rendered store path always has a parent; documented unreachable (1633-1636) |
| `crates/store/src/file.rs:1746` | BENIGN-INFALLIBLE | len <= REGION_LEN_U64 checked on 1745, conversion cannot fail |
| `crates/store/src/file.rs:2431` | BENIGN-INFALLIBLE | parent() of sidecar path always exists; fsync error propagated with ? |
| `crates/store/src/file.rs:2543` | BENIGN-INFALLIBLE | poison recovery on verified-cache; the write is NO_BLOCK (forget), safe whatever a panicker left |
| `crates/store/src/file.rs:2588` | BENIGN-OPTION | path used only to name a barrier failure; falls back to bars path when no sidecar path kept |
| `crates/store/src/file.rs:2626` | SURFACED | conversion failure -> ok_or(StoreError::NotCommitted) refusal on 2628 |
| `crates/store/src/file.rs:2634` | BENIGN-INFALLIBLE | poison recovery; cache forgotten (NO_BLOCK) before verify, so a panicker leaves nothing cached |
| `crates/store/src/file.rs:2854` | SURFACED | diagnose_overlap None -> original Format refusal returned; still Err |
| `crates/store/src/file.rs:3203` | BENIGN-INFALLIBLE | parent() of .tix path always exists; fsync error propagated |
| `crates/store/src/file.rs:3271` | SURFACED | bucket conversion failure -> Why::Entry refusal (3273) |
| `crates/store/src/file.rs:3328` | SURFACED | non-Ready tix state = bisecting, already WARNed by note_lookup_bisects (file.rs:4470); index is an accelerator |
| `crates/store/src/file.rs:3470` | BENIGN-OPTION | extend always yields the ending entry (documented 3468); EMPTY only for an empty prefix |
| `crates/store/src/file.rs:3506` | BENIGN-INFALLIBLE | parent() always exists; fsync error propagated and file removed |
| `crates/store/src/file.rs:3665` | BENIGN-INFALLIBLE | poison recovery; FORGET FIRST at 3668 so panic leaves nothing cached |
| `crates/store/src/file.rs:3790` | SURFACED | conversion failure -> StoreError::Format OffsetOverflow (3792-3794) |
| `crates/store/src/file.rs:4069` | BENIGN-OPTION | batch non-empty and checked ordered; previous is Some unless single bar (then last==first) |
| `crates/store/src/file.rs:4379` | BENIGN-INFALLIBLE | const ENTRY_LEN=16 conversion cannot fail; only sizes a capacity hint |
| `crates/store/src/file.rs:4401` | BENIGN-INFALLIBLE | parent() always exists; fsync error propagated |
| `crates/store/src/file.rs:4456` | BENIGN-INFALLIBLE | poison recovery on warn-once dedup set; worst case a duplicate WARN |
| `crates/store/src/file.rs:4603` | BENIGN-INFALLIBLE | want <= REGION_LEN_U64 by min() on 4602, conversion cannot fail |
| `crates/store/src/file.rs:4634` | BENIGN-INFALLIBLE | const SLOT_STRIDE conversion cannot fail |
| `crates/store/src/file.rs:4637` | BENIGN-OPTION | torn/undecodable slots are expected; read_region already refused a region with none (documented 4625-4628) |
| `crates/store/src/file.rs:4640` | BENIGN-OPTION | 0 only when no slot decodes, which read_region refuses first (documented 4625-4628) |
| `crates/store/src/file.rs:4807` | BENIGN-INFALLIBLE | created dir always has a parent below root; fsync error propagated |
| `crates/store/src/file.rs:5103` | BENIGN-INFALLIBLE | poison recovery; every field written whole under the lock (documented 5097-5098) |
| `crates/store/src/file.rs:5281` | BENIGN-INFALLIBLE | poison recovery while recording failed barriers; Err(classify(..Sync..)) still returned 5284 |
| `crates/store/src/file.rs:5307` | BENIGN-INFALLIBLE | parent() of a store file always exists; flushes the dir in place of a removed file |
| `crates/store/src/file.rs:5396` | BENIGN-OPTION | None = no outer gathering state to restore; empty list correct |
| `crates/store/src/file.rs:5543` | BENIGN-MATCH | unsupported-target O_SYNC flag; writes_through only true where O_SYNC verified (5106-5107) |
| `crates/store/src/file.rs:5576` | BENIGN-MATCH | defence-in-depth link check; any other error falls to O_NOFOLLOW open which refuses by name |
| `crates/store/src/file.rs:5596` | BENIGN-MATCH | stops at first unexaminable component; create_below/O_NOFOLLOW open refuse after |
| `crates/store/src/file.rs:5679` | SURFACED | conversion failure -> FormatError::BlockLengthMismatch refusal (5681-5684) |
| `crates/store/src/file.rs:5869` | BENIGN-INFALLIBLE | poison recovery while recording a failed barrier; Err returned 5871 |
| `crates/store/src/file.rs:5930` | BENIGN-INFALLIBLE | usize->u64 cannot fail on 64-bit; documented 5925-5928 |
| `crates/store/src/file.rs:9764` | BENIGN-INFALLIBLE | silences unused var on unsupported OS; returns Err(Unsupported) |
| `crates/store/src/generation.rs:159` | BENIGN-INFALLIBLE | 4-byte slice -> [u8;4] cannot fail; any None makes decode return None (record refused) |
| `crates/store/src/generation.rs:165` | BENIGN-INFALLIBLE | 8-byte slice -> [u8;8] cannot fail |
| `crates/store/src/generation.rs:171` | BENIGN-INFALLIBLE | 8-byte slice -> [u8;8] cannot fail |
| `crates/store/src/generation.rs:181` | BENIGN-OPTION | invalid revision -> decode None, record treated as not valid (refused by caller), not defaulted |
| `crates/store/src/generation.rs:196` | BENIGN-INFALLIBLE | fixed [u8;RECORD] array; 0..60 always in range |
| `crates/store/src/generation.rs:424` | BENIGN-INFALLIBLE | made dir always has a parent; sync error propagated with ? |
| `crates/store/src/generation.rs:521` | BENIGN-MATCH | defence-in-depth link walk; stop at first unexaminable component, open below uses no_follow |
| `crates/store/src/generation.rs:561` | BENIGN-INFALLIBLE | u128 micros > i64 only after year 294k; promoted_at stamp only |
| `crates/store/src/header.rs:673` | SURFACED | returns Err with the newest refusal kept, NoValidHeader only if none recorded |
| `crates/store/src/header.rs:680` | BENIGN-OPTION | third tuple elem = why walked back; placeholder when newest validated (caller compares generations) |
| `crates/store/src/header.rs:690` | SURFACED | 'no older candidate' dropped on purpose; newest refusal kept and returned (comment 687-689) |
| `crates/store/src/header.rs:692` | SURFACED | returns Err with the kept refusal |
| `crates/store/src/header.rs:850` | SURFACED | undecodable slot recorded into fault (most informative) and returned at 883 |
| `crates/store/src/header.rs:883` | SURFACED | no candidate -> Err(fault or NoValidHeader) |
| `crates/store/src/header.rs:917` | BENIGN-MATCH | informativeness rank of a refusal; total ranking, not a failure default |
| `crates/store/src/provenance.rs:192` | BENIGN-MATCH | calendar month length: remaining months (1,3,5,7,8,10,12) have 31 days; month range checked 183 |
| `crates/store/src/provenance.rs:207` | BENIGN-OPTION | from_yyyymmdd returns None for an unrepresentable date; caller decides refusal |
| `crates/store/src/provenance.rs:208` | BENIGN-INFALLIBLE | value < 100 always fits u8; Option path refuses anyway |
| `crates/store/src/provenance.rs:209` | BENIGN-INFALLIBLE | value < 100 always fits u8 |
| `crates/store/src/provenance.rs:384` | BENIGN-INFALLIBLE | min(NAME_CAP=128) always fits u16 |
| `crates/store/src/provenance.rs:396` | BENIGN-OPTION | absent commit encoded as zeros with FLAG_COMMIT clear (376-378); presence recorded |
| `crates/store/src/provenance.rs:397` | BENIGN-OPTION | name shorter than cap -> whole name; truncation recorded in FLAG_TRUNCATED |
| `crates/store/src/provenance.rs:399` | BENIGN-INFALLIBLE | fixed 256-byte image, ..252 always in range |
| `crates/store/src/provenance.rs:415` | BENIGN-INFALLIBLE | &[u8;256], ..252 always in range; checksum mismatch refused 416-418 |
| `crates/store/src/provenance.rs:538` | BENIGN-INFALLIBLE | blake3 hash is 32 bytes >= PATH_DIGEST_LEN |
| `crates/store/src/provenance.rs:563` | BENIGN-INFALLIBLE | fixed [u8;64], ..60 in range |
| `crates/store/src/provenance.rs:579` | BENIGN-INFALLIBLE | &[u8;64], ..60 in range; mismatch refused HeaderFault::Checksum |
| `crates/store/src/provenance.rs:722` | BENIGN-MATCH | refusal arm is the guarded one; _ is the passing case of a validation |
| `crates/store/src/provenance.rs:1003` | BENIGN-MATCH | Error::source: only Io variant has a source |
| `crates/store/src/provenance.rs:1270` | BENIGN-INFALLIBLE | bytes has >=64 (first_chunk::<64> checked at 1253) |
| `crates/store/src/provenance.rs:1387` | BENIGN-INFALLIBLE | floor_char_boundary always a valid boundary; truncation flagged separately |
| `crates/store/src/provenance.rs:1410` | BENIGN-MATCH | non-hex digit -> None, caller refuses (commit_bytes returns LandingFault) |
| `crates/store/src/provenance.rs:1421` | BENIGN-INFALLIBLE | nibble 0..15 into 16-entry table |
| `crates/store/src/provenance.rs:1458` | BENIGN-INFALLIBLE | usize->u64 lossless on 64-bit |
| `crates/store/src/provenance.rs:1476` | BENIGN-INFALLIBLE | len <= MAX_FILE_LEN checked 1470; read_exact refuses short read |
| `crates/store/src/provenance.rs:1520` | BENIGN-INFALLIBLE | sidecar path always has a parent |
| `crates/store/src/provenance.rs:1541` | BENIGN-MATCH | defence check; absent ok, other errors met by the following open which refuses |
| `crates/store/src/repair.rs:621` | BENIGN-INFALLIBLE | capacity hint only; RowLimit refused by publish (comment 614-618) |
| `crates/store/src/repair.rs:726` | BENIGN-OPTION | inside is_some_and(previous) branch, previous is Some; value only names the refusal |
| `crates/store/src/safe_open.rs:335` | SURFACED | lstat failure keeps the host's own open refusal (ReadRefusal::Io, 344-348) |
| `crates/store/src/safe_open.rs:459` | BENIGN-OPTION | link target is optional detail on an already-returned Linked refusal |
| `crates/store/src/sealed.rs:765` | BENIGN-MATCH | Error::source dispatch; variants without a source |
| `crates/store/src/sealed.rs:817` | BENIGN-INFALLIBLE | usize->u64 lossless on 64-bit |
| `crates/store/src/sealed.rs:824` | BENIGN-INFALLIBLE | head is fixed HEAD-size image; range always present |
| `crates/store/src/sealed.rs:826` | BENIGN-INFALLIBLE | fixed image range |
| `crates/store/src/sealed.rs:831` | BENIGN-INFALLIBLE | fixed image range |
| `crates/store/src/sealed.rs:840` | BENIGN-INFALLIBLE | fixed [u8;ENTRY] image, ..20 in range |
| `crates/store/src/sealed.rs:865` | BENIGN-INFALLIBLE | entry bytes from chunks_exact(ENTRY); mismatch refused SealedError::Entry |
| `crates/store/src/sealed.rs:894` | BENIGN-OPTION | XOR-delta codec: bytes before the first 256 have predecessor 0 by definition (decoder mirrors at 945) |
| `crates/store/src/sealed.rs:913` | BENIGN-INFALLIBLE | run < 256 (flushed at 256), less <= 255 fits u8 |
| `crates/store/src/sealed.rs:945` | BENIGN-OPTION | decoder mirror of 894: predecessor 0 for the first 256 bytes |
| `crates/store/src/sealed.rs:967` | BENIGN-INFALLIBLE | callers refused zero timeframe/empty span (comment 962-963) |
| `crates/store/src/sealed.rs:968` | BENIGN-INFALLIBLE | same as 967; a mismatching table_entries is then refused by head validation |
| `crates/store/src/sealed.rs:1158` | BENIGN-INFALLIBLE | fixed 512-byte head image |
| `crates/store/src/sealed.rs:1161` | BENIGN-INFALLIBLE | fixed 512-byte head image |
| `crates/store/src/sealed.rs:1177` | BENIGN-INFALLIBLE | fixed head image; month then validated against the path |
| `crates/store/src/sealed.rs:1325` | BENIGN-INFALLIBLE | position from iter over the same 6 extremes; name lookup always hits; refusal returned |
| `crates/store/src/sealed.rs:1503` | BENIGN-INFALLIBLE | block index of a month cannot exceed u32 |
| `crates/store/src/sealed.rs:1521` | BENIGN-OPTION | empty/all-null column encodes (0,0); check_contents 1308-1318 requires exactly that |
| `crates/store/src/sealed.rs:1535` | BENIGN-OPTION | no bars -> default extremes; check_contents 1322-1327 requires default when n_valid==0 |
| `crates/store/src/sealed.rs:1654` | BENIGN-INFALLIBLE | capacity hint, min(16384) |
| `crates/store/src/sealed.rs:1694` | SURFACED | codec refusal -> SealedError::TooLarge (1695); reason detail dropped but refused |
| `crates/store/src/sealed.rs:1834` | SURFACED | undecodable provenance header -> SealedError::ProvenanceForeign refusal (1835-1836) |
| `crates/store/src/sealed.rs:1962` | SURFACED | conversion/range failure -> io UnexpectedEof error |
| `crates/store/src/sealed.rs:2093` | BENIGN-OPTION | first table entry has no predecessor; 0 is the monotone floor; order refused TableOrder |
| `crates/store/src/sealed.rs:2165` | BENIGN-INFALLIBLE | entries read with count >= lead+1, slice always present |
| `crates/store/src/sealed.rs:2178` | BENIGN-INFALLIBLE | u32 difference to usize cannot fail |
| `crates/store/src/sealed.rs:2179` | BENIGN-INFALLIBLE | u32 difference to usize cannot fail |
| `crates/store/src/sealed.rs:2181` | BENIGN-INFALLIBLE | span checked by self.span(..)? at 2167; empty slice would fail frame CRC -> refusal anyway |
| `crates/store/src/sealed.rs:2247` | BENIGN-INFALLIBLE | capacity hint only |
| `crates/store/src/sealed.rs:2249` | BENIGN-INFALLIBLE | first - k0*rows < rows, fits usize |
| `crates/store/src/sealed.rs:2250` | BENIGN-INFALLIBLE | count <= MAX_RANGE_ROWS checked 2234 |
| `crates/store/src/sealed.rs:2251` | BENIGN-INFALLIBLE | frames() checks each frame's row count (FrameRows); range always present. Defence: else Ok(wanted), 0 appended |
| `crates/store/src/sealed.rs:2285` | BENIGN-OPTION | ts before origin clamps to slot 0; lookup still searches from block 0 (correct answer) |
| `crates/store/src/sealed.rs:2343` | BENIGN-INFALLIBLE | lo - from is 0 or 1 |
| `crates/store/src/sealed.rs:2344` | BENIGN-INFALLIBLE | entries hold >= skip entries by request size |
| `crates/store/src/sealed.rs:2353` | BENIGN-MATCH | block 0 starts at byte 0 by format definition |
| `crates/store/src/sealed.rs:2403` | BENIGN-INFALLIBLE | u32->usize cannot fail; unpack_provenance refuses overrun/length mismatch |
| `crates/store/src/sealed.rs:2438` | BENIGN-INFALLIBLE | body length >= 4 implied by u16_at reads/CRC framing |
| `crates/store/src/sealed_codec.rs:526` | SURFACED | row count overflow -> EncodeError::TooManyRows (528) |
| `crates/store/src/sealed_codec.rs:884` | SURFACED | bad count -> FrameError::ExceptionCount refusal (886) |
| `crates/store/src/sealed_codec.rs:1082` | BENIGN-OPTION | empty residual set -> base 0, nothing to encode |
| `crates/store/src/sort_index.rs:280` | BENIGN-INFALLIBLE | const FENCE fits u32 |
| `crates/store/src/sort_index.rs:283` | BENIGN-INFALLIBLE | fixed [u8;64], ..60 in range |
| `crates/store/src/sort_index.rs:295` | BENIGN-INFALLIBLE | &[u8;64]; mismatch refused Why::Header |
| `crates/store/src/sort_index.rs:676` | BENIGN-INFALLIBLE | NARROW chosen only when max-base fits u32 (750-753); v>=base by sort |
| `crates/store/src/sort_index.rs:746` | BENIGN-INFALLIBLE | nulls <= entries.len() by construction of ordered() |
| `crates/store/src/sort_index.rs:752` | BENIGN-OPTION | is_ok() as a width-selection predicate, not a swallowed error |
| `crates/store/src/sort_index.rs:762` | BENIGN-INFALLIBLE | capacity hint only |
| `crates/store/src/sort_index.rs:763` | BENIGN-INFALLIBLE | capacity hint only |
| `crates/store/src/sort_index.rs:766` | BENIGN-INFALLIBLE | block*FENCE < rows fits usize |
| `crates/store/src/sort_index.rs:768` | BENIGN-INFALLIBLE | min(rows) fits usize |
| `crates/store/src/sort_index.rs:770` | BENIGN-INFALLIBLE | from..until within body by construction |
| `crates/store/src/sort_index.rs:771` | BENIGN-INFALLIBLE | every block has >= 1 entry |
| `crates/store/src/sort_index.rs:805` | BENIGN-OPTION | extremes None only for zero rows, which the run encoder refuses earlier (end<=start refused on decode) |
| `crates/store/src/sort_index.rs:809` | BENIGN-INFALLIBLE | capacity hint only |
| `crates/store/src/sort_index.rs:840` | BENIGN-INFALLIBLE | const offset fits usize |
| `crates/store/src/sort_index.rs:849` | BENIGN-INFALLIBLE | const offset fits usize |
| `crates/store/src/sort_index.rs:856` | BENIGN-INFALLIBLE | const offset fits usize |
| `crates/store/src/sort_index.rs:857` | BENIGN-INFALLIBLE | fixed [u8;512], range present |
| `crates/store/src/sort_index.rs:861` | BENIGN-INFALLIBLE | const offset fits usize |
| `crates/store/src/sort_index.rs:881` | BENIGN-INFALLIBLE | const offset fits usize |
| `crates/store/src/sort_index.rs:882` | BENIGN-INFALLIBLE | &[u8;512], range present; mismatch refused 885 |
| `crates/store/src/sort_index.rs:884` | BENIGN-INFALLIBLE | const offset fits usize |
| `crates/store/src/sort_index.rs:904` | BENIGN-INFALLIBLE | const offset fits usize |
| `crates/store/src/sort_index.rs:938` | BENIGN-MATCH | unknown kind refused by name |
| `crates/store/src/sort_index.rs:941` | BENIGN-INFALLIBLE | const offset; is_none_or refuses if absent |
| `crates/store/src/sort_index.rs:947` | BENIGN-INFALLIBLE | const offset fits usize |
| `crates/store/src/sort_index.rs:987` | BENIGN-INFALLIBLE | buffer sizing only; min(BODY_READ) |
| `crates/store/src/sort_index.rs:990` | BENIGN-INFALLIBLE | size <= buf.len() by min at 989 |
| `crates/store/src/sort_index.rs:1366` | BENIGN-INFALLIBLE | body = table-4 < bytes.len(); mismatch refused 1369-1370 |
| `crates/store/src/sort_index.rs:1376` | BENIGN-INFALLIBLE | block < info.blocks, conversion checked 1373 |
| `crates/store/src/sort_index.rs:1378` | BENIGN-INFALLIBLE | range inside CRC-checked table by size arithmetic 1359 |
| `crates/store/src/sort_index.rs:1386` | BENIGN-INFALLIBLE | fences_len <= body by 1359/1373 arithmetic |
| `crates/store/src/sort_index_writer.rs:93` | BENIGN-INFALLIBLE | store-rendered .srt path is ASCII yyyy-mm; fallback unreachable, still a sibling staging name |
| `crates/store/src/sort_index_writer.rs:194` | BENIGN-INFALLIBLE | poison recovery to snapshot cache fields; restored after work |
| `crates/store/src/sort_index_writer.rs:252` | BENIGN-INFALLIBLE | capacity hint only |
| `crates/store/src/sort_index_writer.rs:578` | BENIGN-INFALLIBLE | sidecar path always has a parent; fsync error mapped and returned |
| `crates/store/src/staging.rs:191` | BENIGN-MATCH | Error::source dispatch |
| `crates/store/src/staging.rs:236` | BENIGN-OPTION | unknown file type still refused NotRegular, named Unknown |
| `crates/store/src/staging.rs:249` | BENIGN-MATCH | Ok or NotFound (raced) both mean name free; other errors returned 248 |
| `crates/store/src/time_index.rs:210` | BENIGN-OPTION | negative slot -> None 'outside the month'; documented contract |
| `crates/store/src/time_index.rs:217` | BENIGN-INFALLIBLE | slot < slot_count (month-bounded) fits i64 |
| `crates/store/src/time_index.rs:247` | BENIGN-INFALLIBLE | fixed [u8;64] |
| `crates/store/src/time_index.rs:263` | BENIGN-INFALLIBLE | &[u8;64]; mismatch refused Why::Header |
| `crates/store/src/time_index.rs:347` | BENIGN-INFALLIBLE | fixed [u8;16] |
| `crates/store/src/time_index.rs:356` | BENIGN-INFALLIBLE | &[u8;16]; mismatch -> None, caller refuses as Why::Entry |
| `crates/store/src/time_index.rs:484` | BENIGN-INFALLIBLE | x % 64 fits u32 |
| `crates/store/src/verdict.rs:345` | BENIGN-MATCH | unknown code byte -> None; decode refuses VerdictError::Unknown (1777) |
| `crates/store/src/verdict.rs:529` | BENIGN-INFALLIBLE | count_ones <= 64 fits usize; capacity hint |
| `crates/store/src/verdict.rs:538` | BENIGN-OPTION | unnamed bit rendered flag_<n>, never omitted (documented 524-526) |
| `crates/store/src/verdict.rs:576` | BENIGN-MATCH | unknown code -> None, callers refuse |
| `crates/store/src/verdict.rs:719` | BENIGN-OPTION | unknown feed -> None -> FeedUnknown refusal (839-841) |
| `crates/store/src/verdict.rs:745` | BENIGN-MATCH | unknown exchange byte -> None, refused by decoder |
| `crates/store/src/verdict.rs:766` | BENIGN-MATCH | unknown segment byte -> None, refused by decoder |
| `crates/store/src/verdict.rs:873` | BENIGN-INFALLIBLE | day >= 1 Jan of its own year, difference >= 0 |
| `crates/store/src/verdict.rs:934` | BENIGN-INFALLIBLE | end walked to a char boundary; whole text kept in `whole` |
| `crates/store/src/verdict.rs:966` | BENIGN-OPTION | display full key when truncated, else the stored text |
| `crates/store/src/verdict.rs:979` | BENIGN-INFALLIBLE | u64>>32 always fits u32 |
| `crates/store/src/verdict.rs:984` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/store/src/verdict.rs:985` | BENIGN-INFALLIBLE | masked value < capacity fits usize |
| `crates/store/src/verdict.rs:1229` | BENIGN-INFALLIBLE | Code byte < CODE_SLOTS; counts array covers every code |
| `crates/store/src/verdict.rs:1515` | BENIGN-MATCH | Display: variants with no fields print their word |
| `crates/store/src/verdict.rs:1642` | BENIGN-MATCH | Error::source dispatch |
| `crates/store/src/verdict.rs:1702` | BENIGN-INFALLIBLE | callers pass end <= len (record stride/length checked first) |
| `crates/store/src/verdict.rs:1719` | BENIGN-INFALLIBLE | u64->usize lossless on 64-bit |
| `crates/store/src/verdict.rs:1725` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/store/src/verdict.rs:1770` | BENIGN-INFALLIBLE | bytes.len()==RECORD_STRIDE checked 1764 |
| `crates/store/src/verdict.rs:1772` | BENIGN-INFALLIBLE | len <= field.len() by take_while |
| `crates/store/src/verdict.rs:1773` | SURFACED | non-UTF-8 key -> corrupt(..'key field is not a key') refusal 1775 |
| `crates/store/src/verdict.rs:1919` | BENIGN-INFALLIBLE | feed validated by series.feed_byte()? before build (2332, 3155) |
| `crates/store/src/verdict.rs:2084` | BENIGN-OPTION | overflow -> usize::MAX > MAX_CAPACITY -> TableTooLarge refusal |
| `crates/store/src/verdict.rs:2103` | BENIGN-OPTION | place() None -> next capacity or TableTooLarge refusal |
| `crates/store/src/verdict.rs:2229` | BENIGN-INFALLIBLE | payload <= u16::MAX enforced by check_entry (2215) before build |
| `crates/store/src/verdict.rs:2281` | BENIGN-INFALLIBLE | records count already bounded to u32 (u32_of); NONE unreachable |
| `crates/store/src/verdict.rs:2400` | BENIGN-INFALLIBLE | details.first sized one per planned record |
| `crates/store/src/verdict.rs:2422` | BENIGN-INFALLIBLE | bytes always >= TRAILER_LEN by construction |
| `crates/store/src/verdict.rs:2451` | BENIGN-INFALLIBLE | length checked 2447; mismatch refused TrailerMismatch |
| `crates/store/src/verdict.rs:2458` | BENIGN-INFALLIBLE | length checked 2447 |
| `crates/store/src/verdict.rs:2472` | BENIGN-INFALLIBLE | bytes.len()==geo.len checked 2462; empty slice would be RecordCorrupt anyway |
| `crates/store/src/verdict.rs:2534` | BENIGN-MATCH | refusal arm: record not where probe finds it -> corrupt |
| `crates/store/src/verdict.rs:2584` | BENIGN-INFALLIBLE | section inside geo.len (checked 2462); entry-count mismatch refused 2622 |
| `crates/store/src/verdict.rs:2630` | BENIGN-OPTION | no detail entry -> 0, compared to record.detail_at and refused on mismatch |
| `crates/store/src/verdict.rs:2632` | BENIGN-INFALLIBLE | offset bounded by detail section size |
| `crates/store/src/verdict.rs:2754` | BENIGN-INFALLIBLE | days has YEAR_DAYS entries; day_of_year < YEAR_DAYS |
| `crates/store/src/verdict.rs:2916` | BENIGN-INFALLIBLE | steps saturation only lengthens a bounded wait |
| `crates/store/src/verdict.rs:2928` | BENIGN-INFALLIBLE | ms value in a refusal; saturates |
| `crates/store/src/verdict.rs:2957` | BENIGN-INFALLIBLE | unsupported-OS unused-var silence; flags-only helper |
| `crates/store/src/verdict.rs:3072` | BENIGN-INFALLIBLE | verdict image paths always have a file name |
| `crates/store/src/verdict.rs:3081` | BENIGN-MATCH | Ok or NotFound both fine; other errors returned 3080 |
| `crates/store/src/verdict.rs:3382` | BENIGN-OPTION | new year index (no entry) -> generation 0 = nothing committed |
| `crates/store/src/verdict.rs:4017` | BENIGN-INFALLIBLE | fixed [u8;TABLE_OFFSET] buffer |
| `crates/lake/src/batch.rs:84` | BENIGN-OPTION | Option<Option<_>> flatten: inner None is a real null; cash file has empty greeks vectors by design (82-83) |
| `crates/lake/src/batch.rs:85` | BENIGN-OPTION | same as 84; reader decodes every column to exactly `rows` (expand 615-622) |
| `crates/lake/src/batch.rs:86` | BENIGN-OPTION | same as 84 |
| `crates/lake/src/batch.rs:92` | BENIGN-INFALLIBLE | Batch::new is crate-private; reader builds equal-length columns (expand 615); from_cash_columns checks |
| `crates/lake/src/contract.rs:250` | BENIGN-INFALLIBLE | expiry month is 1..=12 (validated Day); Display only |
| `crates/lake/src/contract.rs:355` | SURFACED | non-representable strike -> ContractError::StrikeNotRepresentable (357) |
| `crates/lake/src/contract.rs:366` | BENIGN-MATCH | unknown side refused ContractError::UnknownSide |
| `crates/lake/src/error.rs:482` | BENIGN-MATCH | Error::source dispatch |
| `crates/lake/src/footer.rs:122` | BENIGN-INFALLIBLE | shift<=63 so checked_shl never None (10th-byte bits >u64 drop silently; pre-check before parquet parse) |
| `crates/lake/src/footer.rs:150` | BENIGN-INFALLIBLE | 4-byte slice -> [u8;4]; None refused 'too short' 152 |
| `crates/lake/src/footer.rs:154` | BENIGN-INFALLIBLE | u32->usize lossless; compared with room and refused 155-158 |
| `crates/lake/src/footer.rs:206` | BENIGN-INFALLIBLE | kinds is [u8;2], next toggles 0/1 (footer.rs:79, 207) |
| `crates/lake/src/footer.rs:229` | BENIGN-MATCH | refusal arm is the guard; _ is the within-bound pass |
| `crates/lake/src/footer.rs:236` | BENIGN-OPTION | too-wide field id cannot equal a schema id; stepped over as unknown (documented 234-235) |
| `crates/lake/src/footer.rs:242` | BENIGN-OPTION | same as 236 |
| `crates/lake/src/page.rs:275` | BENIGN-INFALLIBLE | usize->u64 lossless; length mismatch refused 278 |
| `crates/lake/src/page.rs:396` | SURFACED | unparseable trailing page -> counted stranded -> LakeError::UnreadChunkBytes (reader.rs:711-716) |
| `crates/lake/src/page.rs:403` | BENIGN-MATCH | page types that may hold rows are not rowless; counted stranded |
| `crates/lake/src/page.rs:405` | SURFACED | as 396: counted stranded and refused UnreadChunkBytes |
| `crates/lake/src/page.rs:500` | BENIGN-OPTION | parquet optional is_sorted; absent means false by the format spec |
| `crates/lake/src/page.rs:547` | BENIGN-INFALLIBLE | pos <= chunk.len(); remainder measured, stranded refused (reader.rs:712) |
| `crates/lake/src/reader.rs:110` | BENIGN-INFALLIBLE | capacity hint only (documented 107-109) |
| `crates/lake/src/reader.rs:166` | SURFACED | refusal logged at Error level 'lake.file refused' AND Err returned (174) |
| `crates/lake/src/reader.rs:452` | BENIGN-INFALLIBLE | 4-byte slice -> [u8;4]; None means file too short -> refused by caller |
| `crates/lake/src/reader.rs:456` | BENIGN-INFALLIBLE | same as 452 |
| `crates/lake/src/reader.rs:519` | BENIGN-OPTION | parquet: chunk starts at dictionary page if present, else first data page (spec); offsets range-checked 521 |
| `crates/lake/src/reader.rs:727` | BENIGN-MATCH | wrong physical type refused self.mismatch |
| `crates/lake/src/reader.rs:737` | BENIGN-MATCH | wrong physical type refused |
| `crates/lake/src/reader.rs:749` | BENIGN-MATCH | wrong physical type refused |
| `crates/lake/src/reader.rs:819` | BENIGN-OPTION | columns exactly `rows` long (expand); flatten = null; partial set refused PartialGreeks 845 |
| `crates/lake/src/reader.rs:820` | BENIGN-OPTION | as 819 |
| `crates/lake/src/reader.rs:821` | BENIGN-OPTION | as 819 |
| `crates/lake/src/reader.rs:822` | BENIGN-OPTION | as 819 |
| `crates/lake/src/reader.rs:823` | BENIGN-OPTION | as 819 |
| `crates/lake/src/reader.rs:824` | BENIGN-OPTION | as 819 |
| `crates/lake/src/reader.rs:825` | BENIGN-OPTION | as 819 |
| `crates/lake/src/reader.rs:826` | BENIGN-OPTION | as 819 |
| `crates/lake/src/reader.rs:847` | BENIGN-INFALLIBLE | n in 1..=7 fits u8; inside a refusal |
| `crates/lake/src/schema.rs:96` | BENIGN-MATCH | reporting enum; Other then refused as a type mismatch |
| `crates/lake/src/schema.rs:292` | BENIGN-OPTION | display text for a refused timestamp annotation; MICROS when only the zone differs |
| `crates/core/src/alias.rs:674` | BENIGN-INFALLIBLE | buffer holds only ASCII alphanumerics -> valid UTF-8 |
| `crates/core/src/blake3.rs:144` | BENIGN-INFALLIBLE | masked value fits u32 (documented 140-143) |
| `crates/core/src/blake3.rs:145` | BENIGN-INFALLIBLE | u64>>32 fits u32 |
| `crates/core/src/blake3.rs:310` | BENIGN-INFALLIBLE | const BLOCK_LEN=64 fits u32 |
| `crates/core/src/blake3.rs:340` | BENIGN-INFALLIBLE | block_len <= 64 fits u32 |
| `crates/core/src/blake3.rs:356` | BENIGN-INFALLIBLE | const BLOCK_LEN fits u32 |
| `crates/core/src/instrument.rs:67` | BENIGN-MATCH | unknown exchange refused, no default (documented 60-62) |
| `crates/core/src/instrument.rs:104` | BENIGN-MATCH | unknown segment refused |
| `crates/core/src/instrument.rs:176` | BENIGN-MATCH | month outside 1..12 already refused at 154; arm exists for exhaustiveness (test 753-754) |
| `crates/core/src/instrument.rs:370` | BENIGN-MATCH | any other segment/kind pair is not sweepable (CLAUDE.md §1 surface) |
| `crates/core/src/instrument.rs:508` | BENIGN-INFALLIBLE | length capacity checked above; None = refused construction |
| `crates/core/src/instrument.rs:539` | BENIGN-INFALLIBLE | as 508 |
| `crates/core/src/instrument.rs:569` | BENIGN-INFALLIBLE | constructor admits only ASCII within capacity (564-568) |
| `crates/core/src/isin.rs:98` | BENIGN-MATCH | character-class rule for national number; disallowed refused 100-101 |
| `crates/core/src/isin.rs:121` | BENIGN-INFALLIBLE | constructor admits only ASCII |
| `crates/core/src/knob.rs:79` | BENIGN-MATCH | unrecognised switch value refused by name (70-71) |
| `crates/core/src/price.rs:239` | BENIGN-OPTION | optional leading '+' sign; grammar, not a failure |
| `crates/core/src/price.rs:241` | BENIGN-OPTION | no fractional part; empty-both refused 242-243 |
| `crates/core/src/symbol.rs:76` | BENIGN-MATCH | disallowed byte refused Malformed |
| `crates/core/src/symbol.rs:91` | BENIGN-INFALLIBLE | constructor admits only ASCII within capacity |
| `crates/core/src/universe.rs:4217` | BENIGN-OPTION | absent ISIN -> None by design; malformed entry is a build failure via every_transcribed_isin_is_well_formed (4212-4216) |
| `crates/core/src/vendor.rs:328` | BENIGN-INFALLIBLE | bytes came from a &str, len <= capacity (322-325) |
| `crates/core/src/vendor.rs:329` | BENIGN-INFALLIBLE | as 328 |
| `crates/core/src/vendor.rs:1007` | BENIGN-OPTION | documented: non-UTF-8 routes to Symbol::new's Malformed one step later (1001-1003) |
| `crates/core/src/vendor.rs:1281` | BENIGN-INFALLIBLE | inner match only reached for the five listed codes; remaining is PE |
| `crates/core/src/vendor.rs:1283` | BENIGN-MATCH | unknown Groww type refused Malformed |
| `crates/core/src/vendor.rs:1292` | BENIGN-MATCH | unknown option side refused |
| `crates/core/src/vendor.rs:1296` | BENIGN-MATCH | unknown Dhan type refused |
| `crates/core/src/vendor.rs:1311` | BENIGN-MATCH | unknown Zerodha type refused |
| `crates/core/src/vendor.rs:1485` | BENIGN-MATCH | not an alias -> None |
| `crates/core/src/vendor.rs:1511` | BENIGN-MATCH | not an alias -> None |
| `crates/core/src/vendor.rs:1590` | BENIGN-OPTION | declined row's ISIN is evidence only, parsed leniently (documented 1584-1586) |
| `crates/core/src/vendor.rs:1613` | SURFACED | unrecognised exchange -> Decoded::Skipped(UnrecognisedExchange) with reason (distinct from ForeignExchange, D-3310) |
| `crates/core/src/vendor.rs:1817` | BENIGN-MATCH | option rows: expiry/strike validated then declined LiveContract |
| `crates/core/src/vendor.rs:1866` | BENIGN-OPTION | documented: vendor with no ISIN column / non-equity -> None (1857-1863) |
| `crates/core/src/vendor.rs:1980` | BENIGN-MATCH | non-positive or malformed strike refused (D-1311) |
| `crates/telemetry/src/clock.rs:53` | BENIGN-INFALLIBLE | documented saturation ~292M years out (47-51) |
| `crates/telemetry/src/clock.rs:136` | BENIGN-MATCH | digit dispatch; value is n%10 |
| `crates/telemetry/src/encode.rs:171` | BENIGN-INFALLIBLE | Utf8Sink never errs; asserted rather than discarded (167-170) |
| `crates/telemetry/src/encode.rs:174` | BENIGN-INFALLIBLE | start <= out.len() |
| `crates/telemetry/src/encode.rs:221` | BENIGN-INFALLIBLE | end on a char boundary <= len |
| `crates/telemetry/src/event.rs:191` | BENIGN-INFALLIBLE | len <= MAX_FIELDS == items capacity |
| `crates/telemetry/src/json.rs:148` | BENIGN-MATCH | escape dispatch: every other byte copied verbatim (documented 146-147) |
| `crates/telemetry/src/json.rs:171` | BENIGN-MATCH | hex digit dispatch for nibble 15 |
| `crates/telemetry/src/json.rs:241` | BENIGN-MATCH | string scanner: plain byte arm |
| `crates/telemetry/src/json.rs:310` | BENIGN-MATCH | non-hex escape refused LineFault::BadEscape |
| `crates/telemetry/src/json.rs:334` | BENIGN-INFALLIBLE | start <= at <= len; empty refused 335-336 |
| `crates/telemetry/src/level.rs:69` | BENIGN-MATCH | unknown rank -> None, callers refuse |
| `crates/telemetry/src/lib.rs:200` | BENIGN-INFALLIBLE | poison recovery on an install-serialising unit lock |
| `crates/telemetry/src/record.rs:156` | BENIGN-MATCH | forward-compatible: unknown key stepped over whole (comment 155) |
| `crates/telemetry/src/record.rs:176` | BENIGN-INFALLIBLE | !at_end so peek is Some; value only names the refusal |
| `crates/telemetry/src/record.rs:337` | BENIGN-MATCH | non-finite/unparseable number refused BadNumber (331-334) |
| `crates/telemetry/src/sink.rs:535` | BENIGN-INFALLIBLE | iterator starts with once(global), never empty |
| `crates/telemetry/src/sink.rs:977` | BENIGN-INFALLIBLE | poison recovery (policy 1363-1368); audit open failure itself reported 979-980 |
| `crates/telemetry/src/sink.rs:1142` | BENIGN-INFALLIBLE | documented backstop: only crate-written ranks stored (1136-1138) |
| `crates/telemetry/src/sink.rs:1151` | BENIGN-INFALLIBLE | as 1142 |
| `crates/telemetry/src/sink.rs:1250` | BENIGN-OPTION | CAS outcome is the answer (claimed or not) |
| `crates/telemetry/src/sink.rs:1291` | BENIGN-OPTION | exhaustion -> None refusal, documented (1282-1284) |
| `crates/telemetry/src/sink.rs:1369` | BENIGN-INFALLIBLE | poison recovery documented (1363-1368) |
| `crates/telemetry/src/sink.rs:1424` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/telemetry/src/sink.rs:1462` | BENIGN-INFALLIBLE | own_from <= buf.len() |
| `crates/telemetry/src/sink.rs:1465` | BENIGN-MATCH | non-audited event: no audit copy |
| `crates/telemetry/src/sink.rs:1542` | BENIGN-INFALLIBLE | poison recovery; sync failure returned (1545) |
| `crates/telemetry/src/sink.rs:1562` | BENIGN-INFALLIBLE | poison recovery on a read of counters |
| `crates/telemetry/src/sink.rs:1569` | BENIGN-INFALLIBLE | as 1562 |
| `crates/telemetry/src/sink.rs:1575` | BENIGN-INFALLIBLE | as 1562 |
| `crates/telemetry/src/sink.rs:1639` | BENIGN-INFALLIBLE | poison recovery while recording a failure |
| `crates/telemetry/src/sink.rs:1643` | BENIGN-OPTION | CAS gates the one-time notice; count kept in health() |
| `crates/telemetry/src/sink.rs:1649` | SURFACED | this IS the surfacing: one stderr notice + last_error + dropped count in health() |
| `crates/telemetry/src/sink.rs:1729` | SWALLOWED | unreadable newest log tail -> Resumed::default(): seq/run floor restart at 0, no runtime notice; telemetry |
| `crates/telemetry/src/sink.rs:1749` | SWALLOWED | read_at I/O error dropped by .ok()? -> feeds the silent restart at 1729 |
| `crates/telemetry/src/sink.rs:1753` | BENIGN-OPTION | skipping undecodable (torn) lines while seeking the last good one is the design |
| `crates/telemetry/src/sink.rs:2195` | BENIGN-INFALLIBLE | nibble < 16 |
| `crates/telemetry/src/sink.rs:2270` | SURFACED | malformed pointer -> None -> counted out.malformed (2757) |
| `crates/telemetry/src/sink.rs:2280` | SURFACED | as 2270 |
| `crates/telemetry/src/sink.rs:2394` | BENIGN-OPTION | negative run field is not a run id -> None |
| `crates/telemetry/src/sink.rs:2396` | BENIGN-MATCH | non-numeric run value -> None |
| `crates/telemetry/src/sink.rs:2427` | BENIGN-OPTION | non-canonical number is not a run id |
| `crates/telemetry/src/sink.rs:2512` | SURFACED | unreadable dims file -> None -> listed in out.uncovered (2719-2721); I/O cause conflated w/ pre-dimension |
| `crates/telemetry/src/sink.rs:2567` | BENIGN-OPTION | rolled past keep -> None, documented (2559) |
| `crates/telemetry/src/sink.rs:2598` | BENIGN-OPTION | line without trailing newline used as-is; seq checked 2600 |
| `crates/telemetry/src/sink.rs:2736` | BENIGN-MATCH | no cursor or older generation: read all |
| `crates/telemetry/src/sink.rs:2739` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/telemetry/src/sink.rs:2753` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/telemetry/src/sink.rs:2883` | BENIGN-INFALLIBLE | retired job path always has a parent |
| `crates/telemetry/src/tail.rs:322` | BENIGN-INFALLIBLE | u64->usize lossless; read_exact errors propagate |
| `crates/telemetry/src/tail.rs:543` | BENIGN-INFALLIBLE | at+1..end within work |
| `crates/telemetry/src/tail.rs:560` | BENIGN-INFALLIBLE | end <= work.len() |
| `crates/telemetry/src/tail.rs:588` | BENIGN-INFALLIBLE | as 322 |
| `crates/telemetry/src/value.rs:70` | BENIGN-INFALLIBLE | documented backstop (61-67) |
| `crates/telemetry/src/value.rs:78` | BENIGN-MATCH | accessor: not text |
| `crates/telemetry/src/value.rs:154` | BENIGN-MATCH | accessor |
| `crates/telemetry/src/value.rs:167` | BENIGN-OPTION | documented: past i64::MAX is None not negative (160-162) |
| `crates/telemetry/src/value.rs:168` | BENIGN-MATCH | accessor |
| `crates/telemetry/src/value.rs:177` | BENIGN-OPTION | negative is not a u64 |
| `crates/telemetry/src/value.rs:178` | BENIGN-MATCH | accessor |
| `crates/telemetry/src/value.rs:187` | BENIGN-MATCH | accessor |
| `crates/telemetry/src/value.rs:196` | BENIGN-MATCH | accessor |
| `crates/telemetry/src/value.rs:229` | BENIGN-MATCH | equality: different variants unequal |
| `crates/vocab/src/expression.rs:105` | BENIGN-MATCH | three-valued logic: any Unknown without a False is Unknown |
| `crates/vocab/src/expression.rs:115` | BENIGN-MATCH | three-valued logic: any Unknown without a True is Unknown |
| `crates/vocab/src/expression.rs:166` | BENIGN-INFALLIBLE | 2-byte slice -> [u8;2]; None refused Refusal::Encoding (168) |
| `crates/vocab/src/expression.rs:224` | BENIGN-MATCH | unknown opcode refused Refusal::Encoding |
| `crates/vocab/src/expression.rs:314` | BENIGN-OPTION | malformed stack -> Unknown, which only refuses a signal (documented 282-283) |
| `crates/vocab/src/expression.rs:329` | BENIGN-OPTION | as 314 |
| `crates/vocab/src/expression.rs:355` | BENIGN-INFALLIBLE | len <= MAX_INSTRUCTIONS fits u16 |
| `crates/vocab/src/expression.rs:635` | SURFACED | non-UTF-8 token -> Refusal::Syntax (636) |
| `crates/vocab/src/expression.rs:639` | BENIGN-OPTION | not a number -> try a name; neither -> Refusal::UnavailableBit (641) |
| `crates/vocab/src/expression_search.rs:96` | BENIGN-INFALLIBLE | Debug output; count bounded by decode |
| `crates/vocab/src/expression_search.rs:106` | BENIGN-INFALLIBLE | Debug output; length bounded by decode |
| `crates/vocab/src/expression_search.rs:245` | BENIGN-OPTION | None = grammar prefix not admissible (documented contract) |
| `crates/vocab/src/expression_search.rs:253` | BENIGN-OPTION | as 245 |
| `crates/vocab/src/expression_search.rs:255` | BENIGN-MATCH | inadmissible instruction at this depth -> None (prefix pruned) |
| `crates/vocab/src/expression_search.rs:271` | BENIGN-MATCH | rank past the alphabet -> None |
| `crates/vocab/src/expression_search.rs:320` | BENIGN-INFALLIBLE | 2-byte slice; None refused Refusal::Cursor |
| `crates/vocab/src/expression_search.rs:330` | BENIGN-MATCH | noncanonical flag refused Refusal::Cursor |
| `crates/vocab/src/implication.rs:150` | BENIGN-MATCH | position not on the pivot chain -> None |
| `crates/vocab/src/implication.rs:175` | BENIGN-MATCH | documented: a band is never implied (173-174) |
| `crates/vocab/src/implication.rs:195` | BENIGN-MATCH | compatible by default unless provably impossible (190-194); keeps candidates, never prunes wrongly |
| `crates/vocab/src/mask.rs:84` | BENIGN-MATCH | documented: position >= BITS ignored, index validated once at table::set_exact (67-71) |
| `crates/vocab/src/mask.rs:101` | BENIGN-MATCH | as 84 |
| `crates/vocab/src/mask.rs:117` | BENIGN-MATCH | bit past width is not set |
| `crates/engine/src/column.rs:44` | BENIGN-INFALLIBLE | word index 0..6 fits u32 |
| `crates/engine/src/column.rs:95` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/engine/src/keep.rs:224` | BENIGN-MATCH | documented: missing slot does not rank below (217-220); stops a sift only |
| `crates/engine/src/lib.rs:1274` | SURFACED | allocation failure -> refused_on_memory outcome (Halt Breach::Memory), not an empty success |
| `crates/engine/src/lib.rs:1404` | SURFACED | as 1274 (refused_streamed_on_memory) |
| `crates/engine/src/lib.rs:1477` | SURFACED | allocation failure -> Tail with halted Some(Breach::Memory) (1481) |
| `crates/engine/src/lib.rs:1495` | BENIGN-INFALLIBLE | Infallible sink error: match never {} |
| `crates/engine/src/lib.rs:1774` | SURFACED | allocation failure -> Halt Breach::Memory (1776) |
| `crates/engine/src/lib.rs:2158` | BENIGN-INFALLIBLE | positions < 384 fit u16 |
| `crates/engine/src/lib.rs:2159` | BENIGN-INFALLIBLE | positions < 384 fit u16 |
| `crates/engine/src/lib.rs:2262` | BENIGN-INFALLIBLE | word index 0..6 fits u32 |
| `crates/engine/src/lib.rs:2276` | BENIGN-INFALLIBLE | word index 0..6 fits u32 |
| `crates/engine/src/lib.rs:2440` | BENIGN-INFALLIBLE | popcount <= 384 fits usize |
| `crates/engine/src/lib.rs:2481` | BENIGN-INFALLIBLE | usize->u64 lossless (documented 2477-2479) |
| `crates/engine/src/resume.rs:53` | BENIGN-MATCH | Error::source dispatch |
| `crates/engine/src/resume.rs:415` | BENIGN-OPTION | NotLive carries no support; encoded 0, decoder requires hits==0 for NotLive (723) |
| `crates/engine/src/resume.rs:697` | BENIGN-MATCH | unknown halt reason refused |
| `crates/engine/src/resume.rs:726` | BENIGN-MATCH | unknown exclusion refused |
| `crates/indicators/src/anchored.rs:335` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/indicators/src/anchored.rs:911` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/indicators/src/anchored.rs:913` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/indicators/src/column.rs:718` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/indicators/src/column.rs:1101` | BENIGN-OPTION | documented: out-of-range or unknown acceptance -> false (1092-1094), conservative |
| `crates/indicators/src/column.rs:1226` | BENIGN-INFALLIBLE | w in 0..6 over a 6-word array |
| `crates/indicators/src/daily.rs:616` | BENIGN-INFALLIBLE | set_exact refuses only static position faults, pinned by the_plan_agrees_with_the_vocabulary (608-615) |
| `crates/indicators/src/daily.rs:640` | BENIGN-INFALLIBLE | as 616 (set_near refusals are static: vocab table.rs:1797-1838) |
| `crates/indicators/src/daily.rs:653` | BENIGN-INFALLIBLE | as 616 |
| `crates/indicators/src/evaluator.rs:139` | BENIGN-OPTION | index only names a WrongBand refusal that is returned (140) |
| `crates/indicators/src/evaluator.rs:181` | BENIGN-INFALLIBLE | static position; set_exact refusals are table faults pinned by tests |
| `crates/indicators/src/evaluator.rs:182` | BENIGN-INFALLIBLE | as 181 |
| `crates/indicators/src/evaluator.rs:552` | BENIGN-INFALLIBLE | calendar len <= 9 fits u64 |
| `crates/indicators/src/evaluator.rs:568` | BENIGN-INFALLIBLE | fixed-length chain (Calendar days [i64;9]) equals EVALUATION_SPEC_V1_LEN=163; debug_assert 569 |
| `crates/indicators/src/evaluator.rs:1024` | BENIGN-INFALLIBLE | as 181 |
| `crates/indicators/src/evaluator.rs:1069` | BENIGN-INFALLIBLE | as 181 |
| `crates/indicators/src/evaluator.rs:1191` | BENIGN-INFALLIBLE | as 181 (comment 1182-1185) |
| `crates/indicators/src/evaluator.rs:1209` | BENIGN-MATCH | third-plus crossing ordinal; documented saturation (1206-1208) |
| `crates/indicators/src/evaluator.rs:1211` | BENIGN-INFALLIBLE | as 181 |
| `crates/indicators/src/evaluator.rs:1282` | SURFACED | bad prior session -> yesterday None; absence shown by has_yesterday/warmed_up; cause unnamed (1273-1276) |
| `crates/indicators/src/evaluator.rs:1518` | BENIGN-INFALLIBLE | const rung count fits u16 |
| `crates/indicators/src/fib.rs:168` | BENIGN-MATCH | wide-division fallback computes the same Euclidean result (documented 159-162), not an error default |
| `crates/indicators/src/fib.rs:195` | BENIGN-OPTION | unrepresentable rung -> None -> rung absent (not known), never clamped onto i64::MIN (§7) |
| `crates/indicators/src/fib.rs:231` | BENIGN-INFALLIBLE | static set_near refusal pinned by the_positions_agree_with_the_vocabulary (222-230) |
| `crates/indicators/src/fib.rs:253` | BENIGN-INFALLIBLE | as 231 (known mask) |
| `crates/indicators/src/fib.rs:385` | BENIGN-MATCH | ring slot 4; next is kept in 0..5 by modulo (388) |
| `crates/indicators/src/gap.rs:207` | BENIGN-OPTION | unrepresentable level -> None (documented 199-201) |
| `crates/indicators/src/gap.rs:390` | BENIGN-MATCH | new bucket arm of a fold state machine |
| `crates/indicators/src/gap.rs:472` | BENIGN-INFALLIBLE | i < 11 fits u16 |
| `crates/indicators/src/lib.rs:188` | BENIGN-MATCH | weekend day has no weekday bit by design (D-0694, 186-187) |
| `crates/indicators/src/lib.rs:549` | BENIGN-OPTION | unrepresentable level -> None (rung absent) |
| `crates/indicators/src/lib.rs:584` | BENIGN-INFALLIBLE | i < CURDAY_RUNGS.len() fits u16 |
| `crates/indicators/src/lib.rs:621` | BENIGN-INFALLIBLE | as 584 |
| `crates/indicators/src/lib.rs:623` | BENIGN-INFALLIBLE | static set_near refusal pinned by test |
| `crates/indicators/src/lib.rs:665` | BENIGN-OPTION | documented: not clamped onto the OI sentinel (662-664) |
| `crates/indicators/src/orb.rs:351` | BENIGN-INFALLIBLE | static position; documented and pinned by orb::the_positions_agree_with_the_vocabulary (347-349) |
| `crates/indicators/src/orb.rs:364` | BENIGN-INFALLIBLE | as 351 |
| `crates/indicators/src/pattern.rs:274` | BENIGN-MATCH | lookback table; remaining positions need 5 bars |
| `crates/indicators/src/pattern.rs:348` | BENIGN-MATCH | depth >= LOOKBACK certifies every position (documented 339-340) |
| `crates/indicators/src/pattern.rs:942` | BENIGN-INFALLIBLE | static position pinned by every_position_is_a_live_plain_pattern |
| `crates/indicators/src/session.rs:247` | BENIGN-INFALLIBLE | static position 68 pinned by test (488-493) |
| `crates/indicators/src/session.rs:494` | BENIGN-INFALLIBLE | as 247 (documented 487-493) |
| `crates/indicators/src/session.rs:502` | BENIGN-INFALLIBLE | static position pinned by test |
| `crates/indicators/src/trend.rs:241` | BENIGN-OPTION | unrepresentable EMA -> None (absent) |
| `crates/indicators/src/trend.rs:349` | BENIGN-OPTION | as 241 |
| `crates/indicators/src/trend.rs:439` | BENIGN-OPTION | unrepresentable stop -> absent, reseeds next candle (documented 462-464) |
| `crates/indicators/src/trend.rs:465` | BENIGN-OPTION | as 439 |
| `crates/indicators/src/trend.rs:906` | BENIGN-INFALLIBLE | static set_near refusal pinned by the_positions_match_the_vocabulary |
| `crates/indicators/src/trend.rs:1029` | BENIGN-INFALLIBLE | as 906 (documented 1023-1027) |
| `crates/indicators/src/vwap.rs:298` | BENIGN-OPTION | unrepresentable VWAP -> None (absent) |
| `crates/indicators/src/vwap.rs:382` | BENIGN-OPTION | as 298 |
| `crates/indicators/src/vwap.rs:603` | BENIGN-OPTION | band overflow -> None for both truth and availability (601) |
| `crates/indicators/src/vwap.rs:633` | BENIGN-INFALLIBLE | static position pinned by test |
| `crates/indicators/src/vwap.rs:644` | BENIGN-INFALLIBLE | as 633 |
| `crates/costs/src/dated.rs:141` | BENIGN-OPTION | flatten over fixed Option row slots (unused slots); LOCKED file, nothing to fix |
| `crates/costs/src/day.rs:258` | BENIGN-OPTION | const constructor: invalid date -> None (caller-checked consts); LOCKED file |
| `crates/costs/src/day.rs:321` | BENIGN-MATCH | weekday dispatch, rem_euclid(7)==6 is Sunday; LOCKED file |
| `crates/costs/src/expiry.rs:452` | BENIGN-OPTION | flatten over fixed Option row slots; LOCKED file |
| `crates/costs/src/lot.rs:429` | BENIGN-OPTION | flatten over fixed Option row slots; LOCKED file |
| `crates/costs/src/lot.rs:510` | BENIGN-OPTION | window edge for the text of an Unverified refusal that is returned (511); LOCKED file |
| `crates/costs/src/regime.rs:176` | BENIGN-OPTION | flatten over fixed Option row slots; LOCKED file |
| `crates/costs/src/regime.rs:208` | BENIGN-OPTION | flatten skips unused slots (documented 206); LOCKED file |
| `crates/greeks/src/moneyness.rs:131` | BENIGN-MATCH | remaining (kind, sign) pairs are in-the-money; exhaustive classification |
| `crates/runner/src/admission.rs:2931` | BENIGN-MATCH | non-measured observation; recorded as unmeasured/refused by record_observation (2868-2869) |
| `crates/runner/src/admission.rs:3050` | BENIGN-INFALLIBLE | numerator <= denominator so <= PPM (documented 3058) |
| `crates/runner/src/admission.rs:3063` | BENIGN-INFALLIBLE | as 3050 |
| `crates/runner/src/admission.rs:5818` | BENIGN-INFALLIBLE | length checked 5807-5813; mismatch refused |
| `crates/runner/src/admission.rs:5825` | BENIGN-INFALLIBLE | as 5818 |
| `crates/runner/src/audit.rs:237` | BENIGN-INFALLIBLE | ppm of overnight move; saturates by sign, display audit |
| `crates/runner/src/bootstrap.rs:114` | BENIGN-INFALLIBLE | x % n < n fits usize |
| `crates/runner/src/bootstrap.rs:207` | BENIGN-MATCH | calibration label for >=300 periods; exhaustive range dispatch |
| `crates/runner/src/bootstrap.rs:926` | BENIGN-OPTION | usize->u64 cannot fail on 64-bit; None propagates as no-verdict (913 `?`) |
| `crates/runner/src/bootstrap.rs:928` | BENIGN-OPTION | as 926 |
| `crates/runner/src/bootstrap.rs:929` | BENIGN-OPTION | as 926 |
| `crates/runner/src/bootstrap.rs:931` | BENIGN-OPTION | as 926 |
| `crates/runner/src/bootstrap.rs:932` | BENIGN-OPTION | as 926 |
| `crates/runner/src/bootstrap.rs:1362` | BENIGN-INFALLIBLE | observed has one entry per return series; arm unreachable, deterministic order anyway |
| `crates/runner/src/bootstrap.rs:1451` | BENIGN-OPTION | as 926 (1428 `?`) |
| `crates/runner/src/bootstrap.rs:1453` | BENIGN-OPTION | as 926 |
| `crates/runner/src/bootstrap.rs:1454` | BENIGN-OPTION | as 926 |
| `crates/runner/src/bootstrap.rs:1457` | BENIGN-OPTION | as 926 |
| `crates/runner/src/bootstrap.rs:1458` | BENIGN-OPTION | as 926 |
| `crates/runner/src/bootstrap.rs:1531` | BENIGN-INFALLIBLE | value <= draws+1 fits usize |
| `crates/runner/src/bootstrap.rs:1547` | BENIGN-INFALLIBLE | as 1362 |
| `crates/runner/src/bootstrap.rs:1838` | BENIGN-OPTION | empty series -> magnitude 0 |
| `crates/runner/src/bootstrap.rs:1839` | BENIGN-OPTION | None = not provably exact -> caller keeps the slow fold (documented 1830-1831) |
| `crates/runner/src/bootstrap.rs:1847` | BENIGN-OPTION | documented zero padding mirrors the fold's own padding (1815-1820) |
| `crates/runner/src/bootstrap.rs:1858` | BENIGN-OPTION | None -> slow exact fold (documented 1853-1856) |
| `crates/runner/src/bootstrap.rs:1912` | BENIGN-MATCH | run-length grouping: start a new run |
| `crates/runner/src/closed.rs:92` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/closed.rs:167` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/closed.rs:183` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/excursion.rs:172` | BENIGN-OPTION | rung i*step overflow dropped only for absurd steps (>i64::MAX/count ppm); ladder then validated by new() |
| `crates/runner/src/excursion.rs:307` | BENIGN-INFALLIBLE | documented: >64 halvings means tightest observation (308-310) |
| `crates/runner/src/excursion.rs:548` | BENIGN-OPTION | no crossing for rung -> NEVER (documented contract) |
| `crates/runner/src/excursion.rs:554` | BENIGN-OPTION | as 548 |
| `crates/runner/src/excursion.rs:569` | BENIGN-OPTION | as 548 |
| `crates/runner/src/excursion.rs:660` | BENIGN-OPTION | as 548 |
| `crates/runner/src/excursion.rs:1013` | BENIGN-OPTION | missing verdict -> refused bar (conservative); path marked refused |
| `crates/runner/src/excursion.rs:1501` | BENIGN-INFALLIBLE | saturating ppm narrow; documented i128 width (1493-1495) |
| `crates/runner/src/excursion.rs:1517` | BENIGN-INFALLIBLE | as 1501 |
| `crates/runner/src/exit_grid_policy.rs:306` | BENIGN-OPTION | single-series mode: execution series is the signal series |
| `crates/runner/src/exit_grid_policy.rs:1299` | BENIGN-MATCH | unsupported underlying refused |
| `crates/runner/src/exit_grid_policy.rs:3692` | BENIGN-INFALLIBLE | u64 -> usize lossless on 64-bit; reserve hint |
| `crates/runner/src/exit_grid_policy.rs:3718` | BENIGN-INFALLIBLE | usize -> u64 lossless; limit refused 3719 |
| `crates/runner/src/exit_grid_policy.rs:3799` | BENIGN-OPTION | None -> caller refuses (checked count); usize->u128 cannot fail |
| `crates/runner/src/exit_grid_policy.rs:3800` | BENIGN-OPTION | as 3799 |
| `crates/runner/src/exit_grid_policy.rs:3801` | BENIGN-OPTION | as 3799 |
| `crates/runner/src/exit_grid_policy.rs:3820` | BENIGN-OPTION | as 3799 |
| `crates/runner/src/exit_grid_policy.rs:3828` | BENIGN-OPTION | overflow -> None -> caller refuses |
| `crates/runner/src/exit_grid_policy.rs:3934` | BENIGN-OPTION | disposition hash includes terminal tag; None only for policy-refused (asserted 3926-3928) |
| `crates/runner/src/exit_grid_policy.rs:4129` | BENIGN-INFALLIBLE | usize->u64 inside a refusal |
| `crates/runner/src/exit_grid_policy.rs:4134` | BENIGN-INFALLIBLE | usize->u64 lossless; refused if nonzero |
| `crates/runner/src/exit_grid_policy.rs:4139` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/exit_grid_policy.rs:4174` | BENIGN-OPTION | inside is_some_and(previous) branch; names the refusal |
| `crates/runner/src/exit_grid_policy.rs:4537` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/grid.rs:664` | BENIGN-INFALLIBLE | i128 floor beyond i64 unreachable (bounded paisa x trades); saturates by sign |
| `crates/runner/src/grid.rs:1742` | BENIGN-OPTION | from_excursions None = nothing observed/all zero (documented excursion.rs:205-207): no derived stop ladder |
| `crates/runner/src/grid.rs:1763` | BENIGN-INFALLIBLE | sorted+deduped non-empty set always valid (comment 1760-1762) |
| `crates/runner/src/grid.rs:1844` | SWALLOWED | costs::fill::Bar::new refusal discarded by .ok()?; feeds exit_fill None -> trade booked flat at 5026 |
| `crates/runner/src/grid.rs:1851` | SWALLOWED | fills_at refusal (e.g. sub-tick printed low, costs fill.rs:452-457) discarded; -> flat trade at 5026 |
| `crates/runner/src/grid.rs:2128` | BENIGN-INFALLIBLE | usize count -> u64 lossless |
| `crates/runner/src/grid.rs:2211` | BENIGN-OPTION | stepped None only for rungs==0 (step<=0 refused by levels_refusal 2512); then no ladder either way |
| `crates/runner/src/grid.rs:2212` | BENIGN-OPTION | as 1742: no observation -> no derived ladder |
| `crates/runner/src/grid.rs:2214` | BENIGN-OPTION | as 1742 |
| `crates/runner/src/grid.rs:2225` | BENIGN-INFALLIBLE | invalid caller ladder already refused by levels_refusal (2506-2511, 2135) |
| `crates/runner/src/grid.rs:2247` | BENIGN-INFALLIBLE | deliberately inert legacy parameter, documented D-1140 (2227-2246) |
| `crates/runner/src/grid.rs:2289` | BENIGN-OPTION | documented: only when nothing ran favourable, older ladder is the honest fallback (2286-2288) |
| `crates/runner/src/grid.rs:2412` | BENIGN-INFALLIBLE | usize count -> u64 lossless |
| `crates/runner/src/grid.rs:2495` | BENIGN-INFALLIBLE | usize -> i128 always fits |
| `crates/runner/src/grid.rs:2524` | BENIGN-INFALLIBLE | usize -> u64 lossless |
| `crates/runner/src/grid.rs:2559` | BENIGN-OPTION | ratio bitmap lookup in range by construction; absent = not admitted (conservative) |
| `crates/runner/src/grid.rs:2588` | BENIGN-OPTION | no TSL -> armed trail may use every rung; enumeration bound, not a failure |
| `crates/runner/src/grid.rs:3141` | BENIGN-INFALLIBLE | usize -> u64 lossless |
| `crates/runner/src/grid.rs:3866` | BENIGN-INFALLIBLE | usize -> u64 lossless (envelope refusal counted as refused_paths) |
| `crates/runner/src/grid.rs:3974` | BENIGN-INFALLIBLE | usize -> u64 lossless |
| `crates/runner/src/grid.rs:4200` | BENIGN-OPTION | Time is always reachable; default only when no attribution set, which is the time exit |
| `crates/runner/src/grid.rs:4810` | BENIGN-INFALLIBLE | u64 wins -> i64 cannot exceed i64::MAX in practice; guarded wins>0 |
| `crates/runner/src/grid.rs:4815` | BENIGN-INFALLIBLE | as 4810 |
| `crates/runner/src/grid.rs:5024` | BENIGN-MATCH | market-order arm (time exit / trail without peak); exhaustive dispatch |
| `crates/runner/src/grid.rs:5026` | SWALLOWED | unpriceable pessimistic exit fill booked flat (exit=entry charged), no counter; only doc-commented 1830-1832 |
| `crates/runner/src/grid.rs:5092` | BENIGN-INFALLIBLE | tsl rung index from the same ladder |
| `crates/runner/src/grid.rs:5110` | BENIGN-INFALLIBLE | trail rung index from the same ladder |
| `crates/runner/src/grid.rs:5291` | BENIGN-OPTION | flatten over Option<Ended> attribution slots, no Result |
| `crates/runner/src/grid.rs:5355` | BENIGN-INFALLIBLE | ppm x paisa / 1e6 within i64 for real prices; documented saturating narrow (validate.rs:1824-1827) |
| `crates/runner/src/grid.rs:5611` | BENIGN-INFALLIBLE | ppm of bounded move; saturating narrow |
| `crates/runner/src/grid.rs:10964` | BENIGN-INFALLIBLE | step_by of a valid ascending ladder stays valid & non-empty |
| `crates/runner/src/identity.rs:212` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/identity.rs:251` | BENIGN-INFALLIBLE | blake3 digest is 32 bytes |
| `crates/runner/src/identity.rs:665` | BENIGN-INFALLIBLE | term lengths are small fixed/str sizes << 4 GiB |
| `crates/runner/src/identity.rs:736` | BENIGN-INFALLIBLE | instrument strings short |
| `crates/runner/src/identity.rs:768` | BENIGN-INFALLIBLE | kind bytes short |
| `crates/runner/src/identity.rs:794` | BENIGN-INFALLIBLE | descriptors < 4 KiB (comment 791) |
| `crates/runner/src/lib.rs:466` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/lib.rs:477` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/lib.rs:541` | BENIGN-OPTION | no separate scoring column -> score on the swept column (documented API) |
| `crates/runner/src/lib.rs:581` | BENIGN-INFALLIBLE | usize->u64 lossless; mismatch refused |
| `crates/runner/src/lib.rs:601` | BENIGN-OPTION | as 541 |
| `crates/runner/src/lib.rs:623` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/lib.rs:917` | SURFACED | only Err w/ always-Ok reporter is support-column alloc -> Auto{affordable:false, refused_on_memory} 920-927 |
| `crates/runner/src/lib.rs:1019` | BENIGN-INFALLIBLE | division by constant 2 never None |
| `crates/runner/src/outcome.rs:136` | BENIGN-INFALLIBLE | steps has one entry per bar (prefix_median_steps_over); index < bars.len() |
| `crates/runner/src/outcome.rs:172` | BENIGN-OPTION | verdict missing for an index -> not accepted -> bar never fillable (conservative) |
| `crates/runner/src/outcome.rs:216` | BENIGN-OPTION | filter_map over a HashMap of day facts; selects proved days, no Result dropped |
| `crates/runner/src/outcome.rs:241` | BENIGN-MATCH | new-day arm of session state machine |
| `crates/runner/src/outcome.rs:250` | BENIGN-OPTION | unrepresentable square-off -> None -> bar not fillable (refuses rather than saturates, comment 243-244) |
| `crates/runner/src/outcome.rs:314` | BENIGN-MATCH | documented: out-of-range indices share no day (308-309) |
| `crates/runner/src/outcome.rs:514` | BENIGN-OPTION | Option<Option> flatten; None = tail or refused, refusal tracked in `refused` lane |
| `crates/runner/src/outcome.rs:523` | BENIGN-OPTION | as 514 |
| `crates/runner/src/outcome.rs:529` | BENIGN-OPTION | as 514 |
| `crates/runner/src/outcome.rs:536` | BENIGN-OPTION | as 514 |
| `crates/runner/src/outcome.rs:794` | BENIGN-INFALLIBLE | chunks() never yields an empty chunk |
| `crates/runner/src/outcome.rs:795` | BENIGN-INFALLIBLE | chunks() never yields an empty chunk |
| `crates/runner/src/outcome.rs:834` | BENIGN-INFALLIBLE | ilog2 of usize fits usize |
| `crates/runner/src/outcome.rs:958` | BENIGN-OPTION | no square-off = forced-exit bar: no entry by charter rule (comment 953-957), recorded Unpriced::No |
| `crates/runner/src/outcome.rs:966` | BENIGN-INFALLIBLE | horizon usize -> i64 fits; saturation only lengthens deadline then capped by forced close |
| `crates/runner/src/outcome.rs:1146` | BENIGN-OPTION | as 172: missing verdict = not accepted (conservative) |
| `crates/runner/src/outcome.rs:2024` | BENIGN-INFALLIBLE | usize -> i128 always fits |
| `crates/runner/src/outcome.rs:2025` | BENIGN-INFALLIBLE | usize -> i128 always fits |
| `crates/runner/src/outcome.rs:2079` | BENIGN-INFALLIBLE | slot = x % wheel.len() always in range |
| `crates/runner/src/outcome.rs:2106` | BENIGN-INFALLIBLE | slot = death % wheel.len() always in range |
| `crates/runner/src/outcome.rs:2140` | BENIGN-OPTION | Welford shift unset before the first sample; sum is then 0 terms |
| `crates/runner/src/outcome.rs:2362` | BENIGN-INFALLIBLE | inside Some(r)=at(); measured() has exit>i so excursions Some (1075-1084). Untyped invariant: would undercount |
| `crates/runner/src/outcome.rs:2364` | BENIGN-INFALLIBLE | as 2362 |
| `crates/runner/src/outcome.rs:2371` | BENIGN-INFALLIBLE | exit_at Some wherever at Some (documented 2367-2370) |
| `crates/runner/src/pbo.rs:97` | BENIGN-OPTION | None -> fold counted unrankable (pbo.rs:178) |
| `crates/runner/src/pbo.rs:98` | BENIGN-OPTION | as 97 |
| `crates/runner/src/pbo.rs:100` | BENIGN-OPTION | as 97 |
| `crates/runner/src/pbo.rs:134` | BENIGN-OPTION | None -> rate unavailable (refused, not 0) |
| `crates/runner/src/pbo.rs:135` | BENIGN-OPTION | as 134 |
| `crates/runner/src/pbo.rs:137` | BENIGN-OPTION | as 134 |
| `crates/runner/src/pbo.rs:261` | BENIGN-OPTION | documented refusal over a wrapping cast (257-260); None -> unrankable |
| `crates/runner/src/pbo.rs:262` | BENIGN-OPTION | as 261 |
| `crates/runner/src/pbo.rs:318` | BENIGN-OPTION | None = no probability (documented 310-312) |
| `crates/runner/src/pbo.rs:319` | BENIGN-OPTION | as 318 |
| `crates/runner/src/pbo.rs:385` | BENIGN-INFALLIBLE | mid < n |
| `crates/runner/src/pbo.rs:387` | BENIGN-INFALLIBLE | mid-1 < n |
| `crates/runner/src/pbo.rs:388` | BENIGN-INFALLIBLE | mid < n |
| `crates/runner/src/portfolio.rs:254` | BENIGN-OPTION | flatten over a fixed Option slot array |
| `crates/runner/src/portfolio.rs:486` | BENIGN-OPTION | as 254 |
| `crates/runner/src/portfolio.rs:558` | BENIGN-OPTION | as 254 |
| `crates/runner/src/portfolio.rs:675` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/rank.rs:256` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/rank.rs:264` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/rank.rs:708` | BENIGN-OPTION | flatten Vec<Vec<_>> of per-chunk tops, no Result |
| `crates/runner/src/report.rs:103` | BENIGN-OPTION | display: unnamed bit rendered '?<pos>', never omitted |
| `crates/runner/src/report.rs:160` | BENIGN-OPTION | display name lookup |
| `crates/runner/src/report.rs:162` | BENIGN-OPTION | display '?' for unnamed bit |
| `crates/runner/src/report.rs:214` | BENIGN-OPTION | display filter for vwap note |
| `crates/runner/src/research_family.rs:128` | BENIGN-MATCH | non-index family -> None (documented exact mapping) |
| `crates/runner/src/research_family.rs:160` | BENIGN-OPTION | no NUL terminator = name fills field; then UTF-8 & identity checked |
| `crates/runner/src/research_family.rs:165` | BENIGN-MATCH | unknown tag refused InvalidEncoding |
| `crates/runner/src/research_family.rs:188` | BENIGN-MATCH | unknown index refused |
| `crates/runner/src/research_family.rs:194` | BENIGN-MATCH | out-of-scope instrument refused |
| `crates/runner/src/research_family.rs:306` | BENIGN-OPTION | position(): out-of-scope instrument -> None by contract |
| `crates/runner/src/research_family.rs:307` | BENIGN-OPTION | as 306 |
| `crates/runner/src/resolved_grid_view.rs:272` | BENIGN-OPTION | absent axis encoded as the canonical 'no setting' slot = len (enumeration at 340) |
| `crates/runner/src/resolved_grid_view.rs:273` | BENIGN-OPTION | as 272 |
| `crates/runner/src/resolved_grid_view.rs:283` | BENIGN-OPTION | coordinate not in grid -> Ok(None) (caller refuses) |
| `crates/runner/src/resolved_grid_view.rs:286` | BENIGN-OPTION | as 272 |
| `crates/runner/src/resolved_grid_view.rs:340` | BENIGN-OPTION | as grid.rs:2588 enumeration bound |
| `crates/runner/src/resolved_grid_view.rs:372` | BENIGN-OPTION | out-of-range pair -> not admitted (conservative) |
| `crates/runner/src/signal_candle_stop.rs:191` | BENIGN-MATCH | unknown code refused Error::Encoding |
| `crates/runner/src/signal_candle_stop.rs:340` | BENIGN-MATCH | unknown exit reason refused Error::Encoding |
| `crates/runner/src/signal_candle_stop.rs:386` | BENIGN-MATCH | row failing the forced-exit shape refused Error::Encoding |
| `crates/runner/src/signal_candle_stop.rs:472` | BENIGN-OPTION | encoded with a presence flag (471) first; decoder option() refuses flag/value mismatch (1728-1733) |
| `crates/runner/src/signal_candle_stop.rs:473` | BENIGN-OPTION | as 472 (presence at 471) |
| `crates/runner/src/signal_candle_stop.rs:475` | BENIGN-OPTION | as 472 (presence at 474) |
| `crates/runner/src/signal_candle_stop.rs:477` | BENIGN-OPTION | as 472 (presence at 476) |
| `crates/runner/src/signal_candle_stop.rs:1014` | SURFACED | out-of-scope day -> Error::Scope (1015) |
| `crates/runner/src/signal_candle_stop.rs:1165` | SURFACED | no aligned entry minute -> event keeps reason Unreachable/TooLate (D-0603, 1166-1183) |
| `crates/runner/src/signal_candle_stop.rs:1226` | BENIGN-OPTION | unknown duplicate status -> treated as duplicate -> refused (conservative) |
| `crates/runner/src/signal_candle_stop.rs:1286` | BENIGN-OPTION | no proven real exit -> Reason::ClosingMinute refusal (1289) |
| `crates/runner/src/signal_candle_stop.rs:1450` | SURFACED | overflow -> Error::Arithmetic (1451) |
| `crates/runner/src/signal_candle_stop.rs:1493` | BENIGN-OPTION | as 1226: unknown -> not accepted, counted in period stats |
| `crates/runner/src/signal_candle_stop.rs:1512` | BENIGN-OPTION | no exit fact -> close not verified (conservative) |
| `crates/runner/src/signal_candle_stop.rs:1654` | BENIGN-MATCH | unknown timeframe refused Error::Scope |
| `crates/runner/src/signal_candle_stop.rs:1668` | BENIGN-MATCH | unknown timeframe refused Error::Scope |
| `crates/runner/src/signal_candle_stop.rs:1732` | BENIGN-MATCH | malformed presence flag refused Error::Encoding |
| `crates/runner/src/significance.rs:78` | BENIGN-INFALLIBLE | usize->u64 lossless |
| `crates/runner/src/split.rs:214` | BENIGN-INFALLIBLE | horizon bars fit usize |
| `crates/runner/src/synthetic.rs:59` | BENIGN-INFALLIBLE | synthetic generator; minute < 375 |
| `crates/runner/src/synthetic.rs:100` | BENIGN-OPTION | negative count clamped to 0 sessions (generator) |
| `crates/runner/src/synthetic.rs:213` | BENIGN-INFALLIBLE | as 59 |
| `crates/runner/src/synthetic.rs:285` | BENIGN-OPTION | as 100 |
| `crates/runner/src/topn.rs:422` | BENIGN-INFALLIBLE | all-zero weights refused at Lens construction (topn.rs:352-354) |
| `crates/runner/src/topn.rs:423` | BENIGN-INFALLIBLE | normalised score <= SCORE_SCALE |
| `crates/runner/src/topn.rs:460` | BENIGN-OPTION | zero-width population range -> neutral score for every candidate (no information) |
| `crates/runner/src/topn.rs:593` | BENIGN-INFALLIBLE | end <= len |
| `crates/runner/src/topn.rs:734` | BENIGN-INFALLIBLE | drops the displaced worst row of a full bounded heap; intended |
| `crates/runner/src/trade.rs:334` | BENIGN-INFALLIBLE | usize -> u64 lossless |
| `crates/runner/src/trade.rs:503` | BENIGN-OPTION | documented '0 outside a prefix table' (499); a 0 cadence is refused downstream (trade.rs:1546) |
| `crates/runner/src/trade.rs:592` | BENIGN-OPTION | diagnostic only, empty slice -> 0 (documented 586-587) |
| `crates/runner/src/trade.rs:614` | BENIGN-OPTION | no verdict -> refuse the row (conservative) |
| `crates/runner/src/trade.rs:624` | BENIGN-OPTION | documented: empty slice makes every lookup refuse (619-621; excursion.rs:1013) |
| `crates/runner/src/trade.rs:654` | BENIGN-INFALLIBLE | from <= to <= last checked 649-653; prefix has len+1 entries |
| `crates/runner/src/trade.rs:659` | BENIGN-INFALLIBLE | to+1 <= last+1 in range |
| `crates/runner/src/trade.rs:984` | BENIGN-OPTION | first_row past the column -> nothing to walk (callers pass first_live <= len) |
| `crates/runner/src/trade.rs:986` | BENIGN-OPTION | as 984 |
| `crates/runner/src/trade.rs:1106` | SURFACED | no forced exit for the entry -> counted too_late/while_open (1107-1111); conflation documented 324-326 |
| `crates/runner/src/trade.rs:1123` | SURFACED | foreign/mismatched clock refused and counted (1126-1131) |
| `crates/runner/src/trade.rs:1207` | BENIGN-INFALLIBLE | horizon usize -> i64 fits |
| `crates/runner/src/trade.rs:1411` | SURFACED | unpriceable round trip -> None -> occupancy priceable=false (1286-1291) -> grid refused_paths; too_late count |
| `crates/runner/src/trade.rs:1412` | SURFACED | as 1411 |
| `crates/runner/src/trade.rs:1413` | SURFACED | as 1411 |
| `crates/runner/src/trade.rs:1420` | SURFACED | as 1411 |
| `crates/runner/src/trade.rs:1452` | BENIGN-OPTION | is_ok as an explicit priceability predicate, the answer is the point |
| `crates/runner/src/trade.rs:1453` | BENIGN-OPTION | as 1452 |
| `crates/runner/src/trade.rs:1550` | BENIGN-INFALLIBLE | horizon usize -> i64 fits |
| `crates/runner/src/trade.rs:1634` | BENIGN-INFALLIBLE | steps has one entry per bar |
| `crates/runner/src/validate.rs:103` | BENIGN-INFALLIBLE | usize count -> u64 lossless |
| `crates/runner/src/validate.rs:1750` | BENIGN-MATCH | refusal arm: bad/zero BRUTEX_GRID_RUNGS -> Err naming value (D-1547); unset default recorded resolved_rungs |
| `crates/runner/src/validate.rs:1832` | BENIGN-INFALLIBLE | usize -> u128 always fits |
| `crates/runner/src/validate.rs:1833` | BENIGN-INFALLIBLE | usize -> u128 always fits |
| `crates/runner/src/validate.rs:1839` | BENIGN-INFALLIBLE | whole_len > 0 (whole==0 returned at 1829), checked_div never None |
| `crates/runner/src/validate.rs:1840` | BENIGN-OPTION | documented saturating narrow (1824-1827); a support above u64::MAX finds nothing, floored at 1 |
| `crates/runner/src/validate.rs:3018` | BENIGN-MATCH | unknown signal length -> None; caller refuses |
| `crates/runner/src/validate.rs:3216` | BENIGN-OPTION | hash of optional with a presence byte first (3215); absent != zero digest |
| `crates/runner/src/validate.rs:4212` | BENIGN-INFALLIBLE | usize -> u64 lossless |
| `crates/runner/src/validate.rs:4270` | BENIGN-INFALLIBLE | usize -> u64 lossless |
| `crates/runner/src/validate.rs:4390` | BENIGN-INFALLIBLE | usize -> u64 lossless |
| `crates/runner/src/validate.rs:4391` | BENIGN-INFALLIBLE | usize -> u64 lossless |
| `crates/runner/src/validate.rs:4420` | BENIGN-INFALLIBLE | usize -> u64 lossless; presence byte hashed first |
| `crates/runner/src/validate.rs:4421` | BENIGN-OPTION | absent value hashed as presence 0 + zeros; distinguishable |
| `crates/runner/src/validate.rs:4431` | BENIGN-OPTION | presence byte hashed first (4430) |
| `crates/runner/src/validate.rs:4574` | BENIGN-INFALLIBLE | usize -> u64 lossless |
| `crates/runner/src/validate.rs:4657` | BENIGN-OPTION | no execution projection (single-series mode) -> the signal column is the trade column |
| `crates/runner/src/validate.rs:4903` | BENIGN-INFALLIBLE | usize -> u64 lossless |
| `crates/runner/src/validate.rs:4910` | BENIGN-OPTION | None = candidate with no grid cell / zero training trades (4847-4850): not a failure; priced counts all visited |
| `crates/runner/src/validate.rs:4978` | BENIGN-INFALLIBLE | fold.test.end <= bars.len() by Shape::folds construction |
| `crates/runner/src/validate.rs:5023` | BENIGN-OPTION | as 4657 |
| `crates/runner/src/validate.rs:5089` | BENIGN-OPTION | no live row -> start at end: walk visits nothing, correct for a column with no set bit |
| `crates/runner/src/validate.rs:5140` | SWALLOWED | OOS replay None (no trade OR all paths refused, grid.rs:3994) -> 0 in out_of_sample_all (PBO); refusal lost |
| `crates/pull/src/alerts.rs:353` | BENIGN-OPTION | text shorter than TEXT_MAX is kept whole; longer is pre-cut at a char boundary (alerts.rs:470-474) |
| `crates/pull/src/alerts.rs:354` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/alerts.rs:370` | BENIGN-INFALLIBLE | fixed-width field decode inside a length- and CRC-checked 128-byte record |
| `crates/pull/src/alerts.rs:380` | BENIGN-INFALLIBLE | fixed-width field decode inside a length- and CRC-checked 128-byte record |
| `crates/pull/src/alerts.rs:381` | BENIGN-INFALLIBLE | fixed-width field decode inside a length- and CRC-checked 128-byte record |
| `crates/pull/src/alerts.rs:390` | BENIGN-INFALLIBLE | fixed-width field decode inside a length- and CRC-checked 128-byte record |
| `crates/pull/src/alerts.rs:396` | BENIGN-INFALLIBLE | fixed-width field decode inside a length- and CRC-checked 128-byte record |
| `crates/pull/src/alerts.rs:403` | BENIGN-INFALLIBLE | fixed-width field decode inside a length- and CRC-checked 128-byte record |
| `crates/pull/src/alerts.rs:410` | BENIGN-INFALLIBLE | fixed-width field decode inside a length- and CRC-checked 128-byte record |
| `crates/pull/src/alerts.rs:435` | BENIGN-INFALLIBLE | first 16 bytes of a 32-byte BLAKE3 hash always exist |
| `crates/pull/src/alerts.rs:436` | BENIGN-INFALLIBLE | first 16 bytes of a 32-byte BLAKE3 hash always exist |
| `crates/pull/src/alerts.rs:474` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/alerts.rs:511` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/alerts.rs:516` | BENIGN-OPTION | an ordinal outside the span answers None; caller names it |
| `crates/pull/src/alerts.rs:589` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/alerts.rs:705` | BENIGN-MATCH | reserve file absent/short is simply recreated below |
| `crates/pull/src/alerts.rs:707` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/alerts.rs:759` | SURFACED | reserve spent logged at error with freed flag (alerts.rs:1930-1938); write retried and its error returned |
| `crates/pull/src/alerts.rs:1137` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/alerts.rs:1365` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/alerts.rs:1407` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/alerts.rs:1422` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/alerts.rs:1429` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/alerts.rs:1465` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/alerts.rs:1466` | BENIGN-OPTION | empty-collection default (max/min/first of nothing) is the correct answer |
| `crates/pull/src/alerts.rs:1520` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/alerts.rs:1577` | SURFACED | header shape failure returned as KeptShape.header_ok (alerts.rs:1597-1601) |
| `crates/pull/src/alerts.rs:1589` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/alerts.rs:1752` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/alerts.rs:1864` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/alerts.rs:1868` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/archive.rs:326` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/archive.rs:495` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/archive.rs:865` | BENIGN-INFALLIBLE | capacity hint only, documented (archive.rs:862-864) |
| `crates/pull/src/calendar.rs:465` | BENIGN-INFALLIBLE | LOCKED file - compile-time assert TRADED.len()==DAYS.div_ceil(8) (calendar.rs:429) makes get infallible |
| `crates/pull/src/calendar.rs:476` | BENIGN-MATCH | LOCKED file - bounded table walk arm |
| `crates/pull/src/calendar.rs:483` | BENIGN-MATCH | LOCKED file - bounded table walk arm |
| `crates/pull/src/calendar.rs:1495` | BENIGN-OPTION | LOCKED file - missing hours fail closed to Phase::Unverified (documented calendar.rs:1490-1493) |
| `crates/pull/src/calendar.rs:1502` | BENIGN-INFALLIBLE | LOCKED file - minute < 1440 fits u16 |
| `crates/pull/src/calendar.rs:1535` | BENIGN-OPTION | LOCKED file - day outside Day range -> None; callers answer Unverified |
| `crates/pull/src/calendar.rs:1536` | BENIGN-OPTION | LOCKED file - day outside Day range -> None; callers answer Unverified |
| `crates/pull/src/calendar.rs:1578` | BENIGN-INFALLIBLE | LOCKED file - count <= MAX_BANDS by construction |
| `crates/pull/src/calendar.rs:1655` | BENIGN-OPTION | LOCKED file - no hours -> no cut; phase_at answers Unverified for that day |
| `crates/pull/src/calendar.rs:1666` | BENIGN-OPTION | LOCKED file - Option flatten of optional reopening window |
| `crates/pull/src/calendar.rs:1690` | BENIGN-OPTION | LOCKED file - no later cut means end of day |
| `crates/pull/src/calendar.rs:1754` | BENIGN-MATCH | LOCKED file - exhaustive mod-7 weekday dispatch |
| `crates/pull/src/calendar.rs:1913` | BENIGN-MATCH | LOCKED file - dispatch arm |
| `crates/pull/src/calendar.rs:3038` | BENIGN-OPTION | LOCKED file - no observation selects the reason text |
| `crates/pull/src/calendar.rs:3045` | BENIGN-MATCH | LOCKED file - reason-text arm |
| `crates/pull/src/calendar.rs:3279` | BENIGN-INFALLIBLE | LOCKED file - min was Some so max is Some |
| `crates/pull/src/calendar.rs:3291` | BENIGN-OPTION | LOCKED file - documented D-0953: span that does not fit -> first day only, rest Unmeasured |
| `crates/pull/src/calendar.rs:3292` | BENIGN-OPTION | LOCKED file - documented D-0953: span that does not fit -> first day only, rest Unmeasured |
| `crates/pull/src/calendar.rs:3299` | BENIGN-OPTION | LOCKED file - entry outside span skipped; it reads Unmeasured |
| `crates/pull/src/calendar.rs:3320` | BENIGN-OPTION | LOCKED file - SEBI outage day outside the observed span is not applied |
| `crates/pull/src/calendar.rs:3332` | BENIGN-INFALLIBLE | LOCKED file - saturating count |
| `crates/pull/src/calendar.rs:3363` | BENIGN-INFALLIBLE | LOCKED file - saturating length, documented D-1390 |
| `crates/pull/src/calendar.rs:3366` | BENIGN-MATCH | LOCKED file - empty-calendar arm |
| `crates/pull/src/calendar.rs:3389` | BENIGN-OPTION | LOCKED file - unindexable day -> Unmeasured (safe direction, D-0953) |
| `crates/pull/src/calendar.rs:3393` | BENIGN-OPTION | LOCKED file - day past the span -> Unmeasured (safe direction) |
| `crates/pull/src/capture.rs:318` | BENIGN-OPTION | per-slot diagnostic capture cap reached -> no capture; diagnostic only |
| `crates/pull/src/capture.rs:330` | BENIGN-INFALLIBLE | fmt::Write into a String cannot fail |
| `crates/pull/src/capture.rs:426` | BENIGN-OPTION | per-feed diagnostic capture cap reached -> no capture; diagnostic only |
| `crates/pull/src/capture.rs:435` | BENIGN-INFALLIBLE | fmt::Write into a String cannot fail |
| `crates/pull/src/cash_auction.rs:97` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/cash_auction.rs:108` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/cash_auction.rs:218` | BENIGN-MATCH | absent/empty/zero date recorded as an Unspecified issue, not a default |
| `crates/pull/src/cash_auction.rs:262` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/cash_auction.rs:357` | BENIGN-INFALLIBLE | constant literal date |
| `crates/pull/src/cash_auction.rs:358` | BENIGN-INFALLIBLE | constant literal date |
| `crates/pull/src/cash_auction.rs:390` | BENIGN-OPTION | optional BOM strip |
| `crates/pull/src/cash_auction.rs:533` | BENIGN-INFALLIBLE | documented compile-time backstop (cash_auction.rs:529-532) |
| `crates/pull/src/cash_auction.rs:584` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/cash_auction.rs:712` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/cash_session_cache.rs:302` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/cash_session_cache.rs:494` | BENIGN-INFALLIBLE | bool ignored; the error is propagated by ? |
| `crates/pull/src/cash_session_cache.rs:609` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/census_store.rs:341` | BENIGN-OPTION | exists flag after read_census succeeded; single-file publish never reads it (census_store.rs:943) |
| `crates/pull/src/census_store.rs:449` | BENIGN-INFALLIBLE | byte count ignored; read error propagated by ? |
| `crates/pull/src/census_store.rs:453` | BENIGN-OPTION | read cap only; open_partition validates the header itself |
| `crates/pull/src/census_store.rs:461` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:462` | BENIGN-INFALLIBLE | byte count ignored; read error propagated by ? |
| `crates/pull/src/census_store.rs:466` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:483` | BENIGN-OPTION | documented: a slot that does not decode bounds nothing (census_store.rs:477) |
| `crates/pull/src/census_store.rs:546` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:721` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/census_store.rs:1031` | SWALLOWED | lstat error other than NotFound read as 'epoch absent' before write_whole renames over the path (low) |
| `crates/pull/src/census_store.rs:1197` | BENIGN-INFALLIBLE | first's own commit offset is `older`, so find always hits |
| `crates/pull/src/census_store.rs:1263` | BENIGN-OPTION | only segments with rows form a group |
| `crates/pull/src/census_store.rs:1506` | BENIGN-OPTION | lstat error read as absent, but the final rename onto an existing dir fails loudly |
| `crates/pull/src/census_store.rs:1513` | BENIGN-OPTION | lstat error read as absent, but staging creation/rename fails loudly |
| `crates/pull/src/census_store.rs:1523` | BENIGN-OPTION | legacy_bytes 0 makes later drift report loud, not silent |
| `crates/pull/src/census_store.rs:1679` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:1742` | SWALLOWED | unreadable migration receipt (I/O, not UTF-8) -> None = 'no drift'; /store and recount warnings vanish |
| `crates/pull/src/census_store.rs:1746` | SWALLOWED | receipt without parseable legacy_bytes -> None = 'no drift' |
| `crates/pull/src/census_store.rs:1916` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:1935` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/census_store.rs:1995` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/census_store.rs:2110` | BENIGN-OPTION | unreadable stub = not sealed: drift is reported / seal path re-validates by name |
| `crates/pull/src/census_store.rs:2197` | SURFACED | ok_or_else names the missing legacy_bytes (census_store.rs:2198) |
| `crates/pull/src/census_store.rs:2214` | BENIGN-OPTION | fallthrough re-lstats legacy and hard_link refuses by name |
| `crates/pull/src/census_store.rs:2272` | BENIGN-INFALLIBLE | leftover removal; create_new below fails loudly if it remains |
| `crates/pull/src/census_store.rs:2351` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/census_store.rs:2464` | BENIGN-INFALLIBLE | leftover removal; create_new below fails loudly if it remains |
| `crates/pull/src/census_store.rs:2508` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:2529` | BENIGN-INFALLIBLE | year_month unreachable for a validated Day (session.rs:428-432) |
| `crates/pull/src/census_store.rs:2530` | BENIGN-INFALLIBLE | year_month unreachable for a validated Day (session.rs:428-432) |
| `crates/pull/src/census_store.rs:2654` | SURFACED | non-symbol directory counted as a stray |
| `crates/pull/src/census_store.rs:2686` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:2688` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:2798` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/census_store.rs:2880` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/census_store.rs:2948` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:2952` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:2956` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:2960` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:3117` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/census_store.rs:3120` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/census_store.rs:3172` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/census_store.rs:3180` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/census_store.rs:3182` | BENIGN-INFALLIBLE | closed keys always carry definition=; one without is refused at census_store.rs:3243 |
| `crates/pull/src/census_store.rs:3240` | SURFACED | ok_or_else names the kind (census_store.rs:3242) |
| `crates/pull/src/census_store.rs:3241` | SURFACED | ok_or_else names the kind (census_store.rs:3242) |
| `crates/pull/src/census_store.rs:3274` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/census_store.rs:3285` | BENIGN-OPTION | display-only files= count (cli/src/census_recount.rs:343) |
| `crates/pull/src/census_store.rs:3307` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/census_store.rs:3334` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/census_store.rs:3345` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/census_store.rs:3347` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/census_store.rs:3349` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/chain.rs:432` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/chain.rs:433` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/chain.rs:434` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/chain.rs:435` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/config.rs:272` | SURFACED | error event 'refused' and the ConfigError is returned to the caller |
| `crates/pull/src/config.rs:1106` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/config.rs:1145` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/config.rs:1171` | BENIGN-MATCH | documented unreachable arm after assign_top matched the three names |
| `crates/pull/src/csv.rs:397` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/csv.rs:402` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/csv.rs:406` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/csv.rs:410` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/csv.rs:411` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/csv.rs:428` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/csv.rs:481` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/csv.rs:483` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/csv.rs:603` | SURFACED | stderr half of the warn 'file decoded' event (csv.rs:635-659) |
| `crates/pull/src/csv.rs:798` | SURFACED | unreadable OI counted (unreadable_oi) and stored absent; warn event + stderr |
| `crates/pull/src/csv.rs:822` | BENIGN-MATCH | shape declares no header |
| `crates/pull/src/csv.rs:884` | BENIGN-INFALLIBLE | fields_of guarantees the field count, so get is Some |
| `crates/pull/src/csv.rs:892` | BENIGN-INFALLIBLE | fields_of guarantees the field count, so get is Some |
| `crates/pull/src/csv.rs:898` | BENIGN-INFALLIBLE | fields_of guarantees the field count, so get is Some |
| `crates/pull/src/csv.rs:987` | BENIGN-INFALLIBLE | fields_of guarantees the field count, so get is Some |
| `crates/pull/src/csv.rs:1005` | BENIGN-INFALLIBLE | fields_of guarantees the field count, so get is Some |
| `crates/pull/src/csv.rs:1018` | SURFACED | documented substitution counted in Tally.unreadable_volume; warn event (csv.rs:652-655) + stderr |
| `crates/pull/src/dhan.rs:226` | BENIGN-MATCH | unpublished code -> None, documented 'not a default' |
| `crates/pull/src/expiry_alias.rs:586` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/expiry_alias.rs:842` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/expiry_alias.rs:852` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/expiry_alias.rs:853` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/fetch.rs:781` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/fetch.rs:1164` | BENIGN-OPTION | absent OI -> i64::MIN null sentinel (CLAUDE.md 7) |
| `crates/pull/src/fno.rs:196` | BENIGN-OPTION | envelope optional; the field itself is required (fno.rs:197-200) |
| `crates/pull/src/fno.rs:272` | BENIGN-INFALLIBLE | nibble < 16 always has a digit |
| `crates/pull/src/fno.rs:1189` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/fno.rs:1236` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/fno.rs:1242` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/fnowork.rs:462` | BENIGN-INFALLIBLE | day 1 of a valid YearMonth |
| `crates/pull/src/fnowork.rs:463` | BENIGN-INFALLIBLE | a valid Expiry is a valid Day |
| `crates/pull/src/fnowork.rs:480` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/fold.rs:275` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/fold.rs:277` | BENIGN-MATCH | empty input reserves nothing |
| `crates/pull/src/fold.rs:1185` | BENIGN-OPTION | unrepresentable minute makes the bar invalid -> refused |
| `crates/pull/src/fold.rs:1386` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/fold.rs:1387` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/fold.rs:1549` | SURFACED | ok_or_else names the invalid session close (fold.rs:1550) |
| `crates/pull/src/fold.rs:1579` | BENIGN-MATCH | a non-open day expects zero minutes |
| `crates/pull/src/folder.rs:311` | BENIGN-INFALLIBLE | earliest <= latest by construction (folder.rs:442-445) |
| `crates/pull/src/gaps.rs:222` | BENIGN-INFALLIBLE | documented safe direction (gaps.rs:217-221) |
| `crates/pull/src/gaps.rs:351` | BENIGN-OPTION | undatable day -> Unmeasured (safe direction) |
| `crates/pull/src/gaps.rs:352` | BENIGN-OPTION | undatable day -> Unmeasured (safe direction) |
| `crates/pull/src/gaps.rs:363` | BENIGN-OPTION | hours this build cannot state -> Unmeasured, documented (gaps.rs:413-416) |
| `crates/pull/src/gaps.rs:374` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gaps.rs:420` | BENIGN-OPTION | hours this build cannot state -> Unmeasured, documented (gaps.rs:413-416) |
| `crates/pull/src/gaps.rs:501` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_archive.rs:106` | BENIGN-INFALLIBLE | documented: usize::MAX reaches no slice so the access refuses |
| `crates/pull/src/gdfl_archive.rs:244` | BENIGN-INFALLIBLE | empty record fails the signature check and is refused by name |
| `crates/pull/src/gdfl_archive.rs:255` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_archive.rs:316` | BENIGN-INFALLIBLE | month 1..=12 always indexes MONTHS |
| `crates/pull/src/gdfl_archive.rs:327` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_archive.rs:416` | BENIGN-OPTION | empty-collection default (max/min/first of nothing) is the correct answer |
| `crates/pull/src/gdfl_archive.rs:421` | BENIGN-OPTION | empty-collection default (max/min/first of nothing) is the correct answer |
| `crates/pull/src/gdfl_archive.rs:464` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/gdfl_archive.rs:542` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_archive.rs:629` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_archive.rs:674` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_cm.rs:545` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:578` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_cm.rs:597` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_cm.rs:633` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:649` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:663` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:687` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:711` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:732` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:747` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:911` | SURFACED | invalid share symbol refused by name (gdfl_cm.rs:913-916) |
| `crates/pull/src/gdfl_cm.rs:993` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:1004` | BENIGN-OPTION | static index-row key; an unbuildable key files nothing and the entry is counted skipped |
| `crates/pull/src/gdfl_cm.rs:1011` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:1017` | BENIGN-OPTION | static lineage table symbol; unbuildable key files nothing, counted skipped |
| `crates/pull/src/gdfl_cm.rs:1054` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:1100` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:1105` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:1202` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_cm.rs:1227` | BENIGN-OPTION | documented: a name without the prefix is read as the symbol |
| `crates/pull/src/gdfl_cm.rs:1363` | BENIGN-MATCH | odd extension recorded and refused by name at gdfl_cm.rs:1405 |
| `crates/pull/src/gdfl_cm.rs:1407` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:1477` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:1929` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_cm.rs:1949` | BENIGN-INFALLIBLE | statistic saturates |
| `crates/pull/src/gdfl_cm.rs:2220` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_cm.rs:2336` | BENIGN-OPTION | empty-collection default (max/min/first of nothing) is the correct answer |
| `crates/pull/src/gdfl_cm.rs:2417` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_cm.rs:2438` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_cm.rs:2463` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_cm.rs:2482` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_cm.rs:2595` | BENIGN-OPTION | strips exactly one CR; others refused by name (D-0808) |
| `crates/pull/src/gdfl_cm.rs:2745` | BENIGN-OPTION | unparseable price refused as RowFault::OffGrid (gdfl_cm.rs:2752) |
| `crates/pull/src/gdfl_cm.rs:2752` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_cm.rs:2772` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_cm.rs:2995` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_cm_auction.rs:184` | BENIGN-INFALLIBLE | fixed-width field decode inside a length- and CRC-checked 128-byte record |
| `crates/pull/src/gdfl_cm_auction.rs:185` | BENIGN-INFALLIBLE | fixed-width field decode inside a length- and CRC-checked 128-byte record |
| `crates/pull/src/gdfl_cm_auction.rs:186` | BENIGN-INFALLIBLE | fixed-width field decode inside a length- and CRC-checked 128-byte record |
| `crates/pull/src/gdfl_cm_auction.rs:208` | BENIGN-INFALLIBLE | constant RECORD_LEN fits u16 |
| `crates/pull/src/gdfl_import.rs:645` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_import.rs:1241` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import.rs:1339` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import.rs:1707` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_import.rs:1813` | BENIGN-OPTION | absent OI -> OI_NULL sentinel (CLAUDE.md 7) |
| `crates/pull/src/gdfl_import.rs:1853` | BENIGN-INFALLIBLE | bars are stamped on `day` (micros_at), so the second is 0..86400 |
| `crates/pull/src/gdfl_import.rs:1940` | BENIGN-INFALLIBLE | year_month unreachable for a validated Day (session.rs:428-432) |
| `crates/pull/src/gdfl_import.rs:2134` | SURFACED | fail-closed: unreadable dir counts as held -> UnderlyingSplit refusal (doc 2106-2107); I/O cause unnamed |
| `crates/pull/src/gdfl_import.rs:2146` | BENIGN-OPTION | unspellable table symbol has no store directory to split (comment 2160-2161) |
| `crates/pull/src/gdfl_import.rs:2162` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_import.rs:2290` | BENIGN-OPTION | parser None is read as auction era -> re-read, never passed (2268-2271) |
| `crates/pull/src/gdfl_import.rs:2629` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import.rs:2632` | BENIGN-OPTION | no newline means the whole text is the torn tail |
| `crates/pull/src/gdfl_import.rs:2668` | BENIGN-INFALLIBLE | open keys are always '<kind> <day> <only>' |
| `crates/pull/src/gdfl_import.rs:2747` | BENIGN-OPTION | a whole-tree close with no scope= field |
| `crates/pull/src/gdfl_import.rs:2823` | BENIGN-MATCH | done/recounted arm of a validated verb set |
| `crates/pull/src/gdfl_import.rs:2854` | BENIGN-OPTION | absent vendor_refused means 0; a malformed one is refused by ? |
| `crates/pull/src/gdfl_import.rs:2980` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_import.rs:3032` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import.rs:3061` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import.rs:3120` | BENIGN-INFALLIBLE | every day between two valid Days is a Day |
| `crates/pull/src/gdfl_import.rs:3494` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/gdfl_import.rs:3522` | SURFACED | warn code=scope_unrecorded (3512-3520); old scope >= swept, so reading it as swept can only over-owe |
| `crates/pull/src/gdfl_import.rs:3712` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import.rs:3713` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import.rs:3725` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_import.rs:3881` | BENIGN-INFALLIBLE | month 1..=12 always indexes MONTHS |
| `crates/pull/src/gdfl_import.rs:3934` | BENIGN-OPTION | documented: 0 before 1970 is refused by name downstream (3929) |
| `crates/pull/src/gdfl_import.rs:3998` | SURFACED | ok_or_else names the committed/held counts (gdfl_import.rs:4000) |
| `crates/pull/src/gdfl_import.rs:4083` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/gdfl_import.rs:4107` | SURFACED | the begin-line refusal is returned as Err |
| `crates/pull/src/gdfl_import.rs:4114` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/gdfl_import.rs:4142` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/gdfl_import.rs:4241` | BENIGN-INFALLIBLE | every index is taken once by a joined worker; a panic is resumed (4234) |
| `crates/pull/src/gdfl_import.rs:4434` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import.rs:4756` | BENIGN-INFALLIBLE | a filer exists only for a day admit() passed session_gate (gdfl_import.rs:4931) |
| `crates/pull/src/gdfl_import.rs:4830` | SURFACED | single-thread path; a worker spawn failure is warned (gdfl_import.rs:4819-4826) |
| `crates/pull/src/gdfl_import.rs:5174` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/gdfl_import.rs:5296` | SURFACED | room error is carried in census_words (gdfl_import.rs:5338) |
| `crates/pull/src/gdfl_import.rs:5432` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/gdfl_import.rs:5618` | BENIGN-INFALLIBLE | every day between two valid Days is a Day |
| `crates/pull/src/gdfl_import.rs:6494` | SURFACED | non-.csv entry counted into report.files_skipped (gdfl_import.rs:6242) |
| `crates/pull/src/gdfl_import.rs:6566` | BENIGN-OPTION | non-.csv entry is no member file |
| `crates/pull/src/gdfl_import.rs:6675` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import.rs:7038` | BENIGN-OPTION | warm-up filter only; the import of that day refuses an undecodable name (7214) |
| `crates/pull/src/gdfl_import.rs:7075` | SWALLOWED | lineage warm-up drops every file read_file refuses (CRC, decode, strike) with no log; doc 7001 says logged |
| `crates/pull/src/gdfl_import.rs:7077` | SWALLOWED | read_file(..).ok(): warm-up file refusal dropped unlogged; lineage folds without that contract |
| `crates/pull/src/gdfl_import.rs:7112` | SWALLOWED | same as 7077 on the worker path: read_file(..).ok() drops the refusal unlogged |
| `crates/pull/src/gdfl_import.rs:7117` | SWALLOWED | flatten() of the worker results discards the dropped warm-up reads; only a count of survivors is logged |
| `crates/pull/src/gdfl_import.rs:7214` | BENIGN-MATCH | an undecodable name is refused when it could be wanted |
| `crates/pull/src/gdfl_import.rs:7357` | BENIGN-INFALLIBLE | split always yields a first item |
| `crates/pull/src/gdfl_import.rs:7375` | BENIGN-MATCH | non-closing verbs (run/begin) do not decide imported days |
| `crates/pull/src/gdfl_import.rs:7380` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/gdfl_import_lineage.rs:229` | BENIGN-OPTION | no strike shift found -> Relisted, the documented kind |
| `crates/pull/src/gdfl_import_lineage.rs:240` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import_lineage.rs:241` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import_lineage.rs:324` | BENIGN-INFALLIBLE | split always yields a first item |
| `crates/pull/src/gdfl_import_lineage.rs:368` | BENIGN-MATCH | other record kinds; doc 309-310: unparsed lines counted by the serving readers |
| `crates/pull/src/gdfl_import_lineage.rs:485` | BENIGN-INFALLIBLE | static table whose rows its own tests render (documented 482-484) |
| `crates/pull/src/gdfl_import_lineage.rs:545` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_import_lineage.rs:614` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/gdfl_import_lineage.rs:632` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/gdfl_import_lineage.rs:694` | BENIGN-OPTION | walk stops at the calendar floor |
| `crates/pull/src/gdfl_import_lineage.rs:697` | BENIGN-OPTION | a day the import does not admit is not a warm-up session |
| `crates/pull/src/gdfl_import_quality.rs:257` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/gdfl_import_quality.rs:337` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_import_quality.rs:374` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import_quality.rs:375` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import_quality.rs:385` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import_quality.rs:430` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import_quality.rs:535` | BENIGN-OPTION | unkeyed lines are not sampled lines |
| `crates/pull/src/gdfl_import_quality.rs:716` | BENIGN-INFALLIBLE | documented: a gap outside the day is no gap of it (713-715) |
| `crates/pull/src/gdfl_import_quality.rs:717` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import_quality.rs:750` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/gdfl_import_quality.rs:761` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_import_quality.rs:774` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_import_quality.rs:790` | BENIGN-OPTION | member_bytes=0 is paired with member_src=none (documented module doc 28-50) |
| `crates/pull/src/gdfl_import_scope.rs:199` | BENIGN-OPTION | None makes Ledger::load refuse the line by name (gdfl_import_scope.rs:186-191) |
| `crates/pull/src/gdfl_import_scope.rs:215` | BENIGN-OPTION | None makes Ledger::load refuse the line by name (gdfl_import_scope.rs:186-191) |
| `crates/pull/src/gdfl_import_scope.rs:216` | BENIGN-OPTION | None makes Ledger::load refuse the line by name (gdfl_import_scope.rs:186-191) |
| `crates/pull/src/gdfl_import_scope.rs:220` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_lineage.rs:272` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_lineage.rs:364` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:384` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/gdfl_lineage.rs:391` | BENIGN-MATCH | documented: no lot or a tie answers None |
| `crates/pull/src/gdfl_lineage.rs:562` | BENIGN-OPTION | stale until first fresh row after the last stale one, else the last stale |
| `crates/pull/src/gdfl_lineage.rs:648` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:660` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:662` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:690` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:691` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:701` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:702` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:703` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:704` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:792` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:835` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_lineage.rs:866` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:867` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:970` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:1072` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:1136` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_lineage.rs:1165` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:1173` | BENIGN-MATCH | inconsistent why/median -> false -> the record is refused |
| `crates/pull/src/gdfl_lineage.rs:1317` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:1326` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:1327` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:1397` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_lineage.rs:1486` | BENIGN-INFALLIBLE | table symbols always spell; core table test builds every row (1483-1485) |
| `crates/pull/src/gdfl_lineage.rs:1564` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/gdfl_lineage.rs:1577` | BENIGN-OPTION | overflowing prediction answers None (no pairing) |
| `crates/pull/src/gdfl_lineage.rs:1587` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/gdfl_lineage.rs:1592` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_lineage.rs:1638` | BENIGN-OPTION | an unrepresentable contract has no store key to stitch |
| `crates/pull/src/gdfl_lineage.rs:1780` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/gdfl_lineage.rs:1784` | BENIGN-OPTION | empty-collection default (max/min/first of nothing) is the correct answer |
| `crates/pull/src/gdfl_lineage.rs:1788` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_lineage.rs:1806` | BENIGN-OPTION | an unrepresentable contract has no store key to stitch |
| `crates/pull/src/gdfl_nfo.rs:445` | BENIGN-INFALLIBLE | month 1..=12 always indexes MONTHS |
| `crates/pull/src/gdfl_nfo.rs:488` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_nfo.rs:496` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_nfo.rs:505` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_nfo.rs:561` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_nfo.rs:567` | BENIGN-INFALLIBLE | an Open calendar day is a valid Day |
| `crates/pull/src/gdfl_nfo.rs:569` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_nfo.rs:605` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_nfo.rs:638` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_nfo.rs:678` | SURFACED | no reading -> NfoRefusal::TickerUnparsed returned |
| `crates/pull/src/gdfl_nfo.rs:706` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_nfo.rs:713` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/gdfl_nfo.rs:715` | SURFACED | ok_or_else UnderlyingRefused (gdfl_nfo.rs:717) |
| `crates/pull/src/gdfl_nfo.rs:746` | SURFACED | ok_or_else ContractUnrenderable (gdfl_nfo.rs:754) |
| `crates/pull/src/gdfl_nfo.rs:785` | BENIGN-INFALLIBLE | month 1..=12 always indexes MONTHS |
| `crates/pull/src/gdfl_nfo.rs:939` | BENIGN-OPTION | an empty ladder has no median |
| `crates/pull/src/gdfl_nfo.rs:965` | BENIGN-MATCH | no ladder median -> no basis to refuse the strike |
| `crates/pull/src/gdfl_nfo.rs:1119` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/gdfl_nfo.rs:1306` | BENIGN-OPTION | optional final newline |
| `crates/pull/src/gdfl_nfo.rs:1310` | BENIGN-OPTION | optional CR |
| `crates/pull/src/gdfl_nfo.rs:1315` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_nfo.rs:1321` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_nfo.rs:1326` | BENIGN-OPTION | optional CR |
| `crates/pull/src/gdfl_nfo.rs:1358` | BENIGN-OPTION | unparseable ltq -> MalformedRow refusal (gdfl_nfo.rs:1362) |
| `crates/pull/src/gdfl_rekey.rs:137` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_rekey.rs:202` | SWALLOWED | second `superseded` line for one key with a different target silently replaces the first (low) |
| `crates/pull/src/gdfl_rekey.rs:209` | SWALLOWED | second `retired` line for one key with different fields silently replaces the first (low) |
| `crates/pull/src/gdfl_rekey.rs:213` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/gdfl_rekey.rs:381` | BENIGN-INFALLIBLE | listing insert; lossy-name collision only among non-UTF-8 names, which are strays anyway |
| `crates/pull/src/gdfl_rekey.rs:395` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_rekey.rs:424` | BENIGN-INFALLIBLE | next day of a valid in-range day exists |
| `crates/pull/src/gdfl_rekey.rs:436` | BENIGN-INFALLIBLE | next day of a valid in-range day exists |
| `crates/pull/src/gdfl_rekey.rs:459` | SURFACED | non-symbol directory counted as a stray |
| `crates/pull/src/gdfl_rekey.rs:665` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_rekey.rs:700` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/gdfl_rekey.rs:715` | BENIGN-OPTION | documented (707-709): an expiry the settlement refuses is judged day by day |
| `crates/pull/src/gdfl_rekey.rs:719` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/gdfl_rekey.rs:724` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/gdfl_rekey.rs:757` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/gdfl_rekey.rs:761` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_rekey.rs:824` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/gdfl_rekey.rs:836` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/gdfl_rekey.rs:861` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/gdfl_rekey.rs:931` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_rekey.rs:970` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_rekey.rs:1088` | BENIGN-INFALLIBLE | flatten of Vec<Vec<Bar>>, not of Results |
| `crates/pull/src/gdfl_rekey.rs:1098` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/gdfl_rekey.rs:1154` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_rekey.rs:1228` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_rekey.rs:1266` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/gdfl_rekey.rs:1320` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/gdfl_rekey.rs:1327` | BENIGN-INFALLIBLE | ignored bool/Option of a set/map insert or count; no failure involved |
| `crates/pull/src/gdfl_rekey.rs:1354` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_tickstore.rs:116` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_tickstore.rs:138` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_tickstore.rs:159` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_tickstore.rs:175` | BENIGN-INFALLIBLE | take(1) returned exactly one byte |
| `crates/pull/src/gdfl_tickstore.rs:179` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_tickstore.rs:253` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_tickstore.rs:304` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_tickstore.rs:309` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_tickstore.rs:315` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_tickstore.rs:338` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/gdfl_tickstore.rs:347` | BENIGN-MATCH | tag 1 numeric; every other tag refused at gdfl_tickstore.rs:338 |
| `crates/pull/src/gdfl_tickstore.rs:359` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_tickstore.rs:408` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_tickstore.rs:467` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/gdfl_tickstore.rs:476` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/gdfl_tickstore.rs:477` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/gdfl_tickstore.rs:487` | BENIGN-OPTION | out-of-range time refused as TIME_RANGE (gdfl_tickstore.rs:488) |
| `crates/pull/src/gdfl_tickstore.rs:500` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/gdfl_tickstore.rs:503` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/gdfl_tickstore.rs:541` | BENIGN-INFALLIBLE | the name day_zip_name builds always ends .zip |
| `crates/pull/src/gdfl_tickstore.rs:580` | BENIGN-MATCH | names the refusal text for the remaining entry kinds |
| `crates/pull/src/groww.rs:123` | BENIGN-MATCH | unpublished code -> None, documented |
| `crates/pull/src/http.rs:683` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/http.rs:1036` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/http.rs:1041` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/http.rs:1049` | BENIGN-OPTION | duplicate-key scan; malformed JSON is refused by serde elsewhere |
| `crates/pull/src/http.rs:1056` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/http.rs:1239` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:1248` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:1410` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:1411` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:1413` | SURFACED | stderr half of a counted warn telemetry event emitted just above (the loud degrade itself) |
| `crates/pull/src/http.rs:1536` | BENIGN-OPTION | None makes the caller refuse the cell by name (doc 1526-1529) |
| `crates/pull/src/http.rs:1541` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/http.rs:1543` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/http.rs:1544` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/http.rs:1545` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/http.rs:1555` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/http.rs:1559` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/http.rs:1561` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/http.rs:1749` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:1753` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:1756` | SURFACED | stderr half of a counted warn telemetry event emitted just above (the loud degrade itself) |
| `crates/pull/src/http.rs:1784` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:1788` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:1791` | SURFACED | stderr half of a counted warn telemetry event emitted just above (the loud degrade itself) |
| `crates/pull/src/http.rs:1818` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:1822` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:1825` | SURFACED | stderr half of a counted warn telemetry event emitted just above (the loud degrade itself) |
| `crates/pull/src/http.rs:1989` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/http.rs:2020` | SURFACED | stderr half of a counted warn telemetry event emitted just above (the loud degrade itself) |
| `crates/pull/src/http.rs:2359` | BENIGN-MATCH | short column keeps its row; decode names the length disagreement (2327-2329) |
| `crates/pull/src/http.rs:2377` | BENIGN-INFALLIBLE | every column verified the mask's length before the filter (2410-2416) |
| `crates/pull/src/http.rs:2420` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/http.rs:2729` | BENIGN-INFALLIBLE | room < frame.len() in this branch |
| `crates/pull/src/http.rs:2758` | SWALLOWED | a Location header that is not visible ASCII is reported as 'no Location header' (low) |
| `crates/pull/src/http.rs:2782` | SURFACED | documented (2777-2779): refusal still returned with its status; empty detail -> 'vendor answered <status>' |
| `crates/pull/src/http.rs:3005` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/http.rs:3032` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/http.rs:3316` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/http.rs:3481` | SURFACED | the parse Err is returned as not_json (http.rs:3484) |
| `crates/pull/src/http.rs:3553` | BENIGN-OPTION | documented (3550-3551): non-JSON names no refusal; the caller parses the text and refuses |
| `crates/pull/src/http.rs:3580` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/http.rs:3591` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/http.rs:3773` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:3774` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/http.rs:3776` | SURFACED | stderr half of a counted warn telemetry event emitted just above (the loud degrade itself) |
| `crates/pull/src/http.rs:3827` | BENIGN-OPTION | short text is refused by local_seconds' length check (http.rs:3969) |
| `crates/pull/src/http.rs:3878` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/http.rs:3976` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/http.rs:3981` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/http.rs:4000` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/http.rs:4001` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/http.rs:4002` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/http.rs:4004` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/identity_screen.rs:248` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/ingest.rs:235` | BENIGN-INFALLIBLE | year_month unreachable for a validated Day |
| `crates/pull/src/ingest.rs:596` | BENIGN-MATCH | documented: a span outside the store's months names no month (594-595) |
| `crates/pull/src/ingest.rs:610` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/ingest.rs:1347` | BENIGN-MATCH | conflicting duplicate row is refused and noted |
| `crates/pull/src/ingest.rs:1376` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/ingest.rs:1428` | BENIGN-OPTION | day label only (on_stamp_day 1406-1411) |
| `crates/pull/src/ingest.rs:1942` | SURFACED | unreadable stamp counted as a named drop in the DropCensus (doc 1927-1931) |
| `crates/pull/src/ingest.rs:2059` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/ingest.rs:2307` | BENIGN-OPTION | documented (2294-2297): a month that will not open holds nothing; month already refused by name |
| `crates/pull/src/ingest.rs:2308` | BENIGN-OPTION | documented (2294-2297): a month that will not open holds nothing; month already refused by name |
| `crates/pull/src/ingest.rs:2327` | SWALLOWED | time-index error on a held day read as 'not held' -> counted refused_day in the ledger (low) |
| `crates/pull/src/ingest.rs:2329` | SWALLOWED | read_record error on a held day read as 'not held' -> counted refused_day in the ledger (low) |
| `crates/pull/src/ingest.rs:2336` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/ingest.rs:2451` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/ingest.rs:2813` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/ingest.rs:3401` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/ingest.rs:3626` | SURFACED | ok_or_else names the missing venue (ingest.rs:3629) |
| `crates/pull/src/ingest.rs:3627` | SURFACED | ok_or_else names the missing venue (ingest.rs:3629) |
| `crates/pull/src/ingest.rs:3876` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/ingest.rs:3880` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/ingest.rs:3913` | SURFACED | warn 'could not be checked' with the why (ingest.rs:3946) |
| `crates/pull/src/ingest.rs:3921` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/ingest.rs:3943` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/ingest.rs:4230` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/ingest.rs:4270` | SURFACED | time-index error -> held != bar -> the append is refused by name (4274); cause unnamed |
| `crates/pull/src/ingest.rs:4272` | SURFACED | read error -> held != bar -> the append is refused by name (4274); cause unnamed |
| `crates/pull/src/ingest.rs:4584` | BENIGN-INFALLIBLE | byte count ignored; read error propagated by ? |
| `crates/pull/src/ingest.rs:4585` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/ingest.rs:4694` | BENIGN-INFALLIBLE | documented best effort; a failure is install_locked's to report (4689-4692) |
| `crates/pull/src/ingest.rs:4885` | BENIGN-MATCH | documented deferral D-2765: path failure fails the install on the same cause; tests named |
| `crates/pull/src/ingest.rs:5015` | BENIGN-OPTION | metadata failure -> whole install of the image just read under the lock |
| `crates/pull/src/ingest.rs:5094` | BENIGN-OPTION | documented (5090-5093): the fallback path fails loudly on the next line |
| `crates/pull/src/kite.rs:172` | BENIGN-MATCH | unknown exception name -> None |
| `crates/pull/src/lib.rs:108` | BENIGN-INFALLIBLE | documented: Err means a provider is already installed (105-107) |
| `crates/pull/src/manifest.rs:532` | BENIGN-INFALLIBLE | documented saturation with doctests (523-528) |
| `crates/pull/src/manifest.rs:533` | BENIGN-INFALLIBLE | documented saturation with doctests (523-528) |
| `crates/pull/src/manifest.rs:863` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/manifest.rs:1749` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/manifest.rs:1768` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/manifest.rs:2165` | BENIGN-INFALLIBLE | contract text <= CONTRACT_CAPACITY fits u8 |
| `crates/pull/src/manifest.rs:2592` | BENIGN-INFALLIBLE | capacity <= MAX_CAPACITY = u32::MAX (manifest.rs:255) |
| `crates/pull/src/manifest.rs:2674` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/manifest.rs:2803` | SURFACED | no valid header -> Err returned |
| `crates/pull/src/manifest.rs:3041` | BENIGN-OPTION | documented (3034-3038): a short file is handed on whole and refused for its length |
| `crates/pull/src/manifest.rs:3471` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/manifest.rs:3531` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/manifest.rs:3626` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/manifest.rs:4236` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/manifest.rs:4512` | SWALLOWED | CRC-valid but unreadable contract (len>24, non-UTF-8, Contract::parse None) decodes as NO contract |
| `crates/pull/src/manifest.rs:4546` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/manifest.rs:4547` | BENIGN-OPTION | an unterminated field spans its whole width |
| `crates/pull/src/manifest.rs:4548` | BENIGN-OPTION | None is refused by every caller (manifest.rs:2062 SymbolNotUtf8, 2648 UnknownVendor) |
| `crates/pull/src/masters.rs:486` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/masters.rs:491` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/masters.rs:496` | SURFACED | stderr half of the warn 'index entries skipped' event (487-495) |
| `crates/pull/src/masters.rs:710` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/masters.rs:741` | BENIGN-INFALLIBLE | poisoned-mutex recovery: state written whole under the lock (documented); worker panics are resumed |
| `crates/pull/src/masters.rs:746` | BENIGN-INFALLIBLE | split always yields a first item |
| `crates/pull/src/masters.rs:761` | BENIGN-OPTION | no host -> no cookie jar entry |
| `crates/pull/src/masters.rs:794` | BENIGN-OPTION | non-ASCII Set-Cookie not remembered; cookie jar only |
| `crates/pull/src/masters.rs:1098` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/masters.rs:1102` | SURFACED | a copy that was not kept is named in the returned sentence (masters.rs:1117) |
| `crates/pull/src/masters.rs:1103` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/masters.rs:1196` | BENIGN-INFALLIBLE | opener is Some in this branch |
| `crates/pull/src/masters.rs:1203` | BENIGN-OPTION | empty body has no comma and is refused (1204) |
| `crates/pull/src/masters.rs:1250` | BENIGN-OPTION | header already checked by missing_columns (masters.rs:1218-1229) |
| `crates/pull/src/masters.rs:1263` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/masters.rs:1266` | BENIGN-OPTION | empty-collection default (max/min/first of nothing) is the correct answer |
| `crates/pull/src/masters.rs:1293` | BENIGN-INFALLIBLE | missing_columns refused absent columns; width gate 1281 guarantees the field |
| `crates/pull/src/masters.rs:1384` | SURFACED | documented D-2374: a refused touch falls to the full replacement whose outcome is reported |
| `crates/pull/src/masters.rs:1460` | BENIGN-OPTION | a bare file name syncs the current directory |
| `crates/pull/src/masters.rs:1545` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/masters.rs:1594` | BENIGN-MATCH | saturating backoff, documented and tested |
| `crates/pull/src/nse.rs:406` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/nse.rs:452` | BENIGN-MATCH | unquoted attribute skipped, documented (448-451) |
| `crates/pull/src/nse.rs:462` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/nse.rs:590` | BENIGN-MATCH | unquoted attribute skipped |
| `crates/pull/src/nse.rs:610` | BENIGN-INFALLIBLE | slice/get inside bounds established just above; cannot fail |
| `crates/pull/src/nse.rs:638` | BENIGN-OPTION | an empty body fails the header comparison and is refused |
| `crates/pull/src/nse.rs:829` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/nseindex.rs:126` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/pricing.rs:114` | BENIGN-OPTION | a symbol that is not a swept slot -> None |
| `crates/pull/src/pricing.rs:130` | BENIGN-OPTION | documented: outside the cost calendar's window -> None |
| `crates/pull/src/pricing.rs:552` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/rate.rs:794` | BENIGN-MATCH | documented floor branch (rate.rs:786-790, docs/06-limits.md) |
| `crates/pull/src/rate.rs:888` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/rate.rs:904` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/rate.rs:983` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/rate.rs:997` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/rate.rs:1017` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/rate.rs:1028` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/rate.rs:1048` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/rate.rs:1125` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/rate.rs:1316` | BENIGN-INFALLIBLE | the closure always returns Some, so fetch_update cannot fail |
| `crates/pull/src/rate.rs:1346` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/refusal.rs:405` | BENIGN-MATCH | documented status table |
| `crates/pull/src/refusal.rs:576` | BENIGN-OPTION | non-JSON body names no error |
| `crates/pull/src/refusal.rs:621` | BENIGN-OPTION | non-JSON body names no disposition |
| `crates/pull/src/refusal.rs:647` | BENIGN-OPTION | documented (641-644): absent sentence reads empty so the code still answers |
| `crates/pull/src/repair_plan.rs:118` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/repair_plan.rs:161` | BENIGN-OPTION | parts with no store path list nothing; one() refuses the same StorePath by name (207-212) |
| `crates/pull/src/repair_plan.rs:171` | SWALLOWED | held_months: dir-entry errors dropped, and read_dir errors (167) answer 'no months held' |
| `crates/pull/src/repair_plan.rs:172` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/repair_plan.rs:178` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/repair_plan.rs:179` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/repair_plan.rs:181` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/repair_plan.rs:247` | SWALLOWED | time-index error on a held day -> day planned 'missing' for a repair (repair_plan.rs:256-263) |
| `crates/pull/src/repair_plan.rs:249` | SWALLOWED | read_record error on a held day -> day planned 'missing' for a repair |
| `crates/pull/src/request_minutes.rs:285` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/request_minutes.rs:403` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/request_minutes.rs:535` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/request_minutes.rs:565` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/request_minutes.rs:566` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/request_minutes.rs:579` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/request_minutes.rs:654` | BENIGN-INFALLIBLE | constant SOURCED_LAST_DAY is a valid day |
| `crates/pull/src/request_minutes.rs:739` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/resolve.rs:229` | BENIGN-OPTION | default only fills a label/telemetry/display field; no data decision |
| `crates/pull/src/resolve.rs:676` | BENIGN-OPTION | hint left empty for a non-ASCII Location; the status itself is still named |
| `crates/pull/src/riskfree.rs:280` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/riskfree.rs:288` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/riskfree.rs:309` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/riskfree.rs:455` | BENIGN-INFALLIBLE | documented: index + 1 fits (453-454) |
| `crates/pull/src/riskfree.rs:494` | SURFACED | missing slot -> LookupError::Stale (riskfree.rs:502) |
| `crates/pull/src/riskfree.rs:497` | SURFACED | missing slot -> LookupError::Stale (riskfree.rs:502) |
| `crates/pull/src/riskfree.rs:500` | SURFACED | missing slot -> LookupError::Stale (riskfree.rs:502) |
| `crates/pull/src/rolling.rs:120` | SURFACED | ok_or RollingError::NoExpiry named (rolling.rs:121) |
| `crates/pull/src/rolling.rs:238` | SURFACED | costs error refused as RollingError::NoExpiry (reason text assumes before-regime) |
| `crates/pull/src/rolling.rs:671` | BENIGN-OPTION | envelope optional, documented (669-670) |
| `crates/pull/src/rolling.rs:1024` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/rolling.rs:1034` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/rolling.rs:1050` | BENIGN-OPTION | strict parser answers None/Err; the caller refuses or labels it |
| `crates/pull/src/series_identity.rs:255` | BENIGN-OPTION | binary-search hit test |
| `crates/pull/src/session.rs:328` | BENIGN-MATCH | documented: February plus values Day::new already refused |
| `crates/pull/src/session.rs:670` | BENIGN-MATCH | compile-time assertion arm |
| `crates/pull/src/session.rs:674` | BENIGN-MATCH | compile-time assertion arm |
| `crates/pull/src/session.rs:1282` | BENIGN-INFALLIBLE | documented: minute < 1440; saturation lands after close (1279-1281) |
| `crates/pull/src/ssm.rs:245` | BENIGN-OPTION | AWS identity, not the broker credential (ssm.rs:228-236); non-Unicode refused first (244), CE-69 |
| `crates/pull/src/ssm.rs:310` | BENIGN-OPTION | AWS identity discovery; non-Unicode refused first (309); half-set env refused (D-1534) |
| `crates/pull/src/ssm.rs:353` | BENIGN-OPTION | D-1534: AWS_PROFILE unset/empty names the default profile; documented, not the parameter path |
| `crates/pull/src/ssm.rs:472` | BENIGN-MATCH | other credentials-file keys ignored |
| `crates/pull/src/ssm.rs:482` | BENIGN-MATCH | refusing arm: returns a named Err/None that the caller refuses |
| `crates/pull/src/ssm.rs:1117` | BENIGN-INFALLIBLE | integer width/saturating conversion that cannot fail at this bound (or saturates a reported count) |
| `crates/pull/src/totp.rs:297` | BENIGN-INFALLIBLE | masked to 8 bits, documented (296) |
| `crates/pull/src/universe.rs:523` | BENIGN-OPTION | a published name absent from the master covers 0 rows |
| `crates/pull/src/vendor.rs:124` | BENIGN-MATCH | exhaustive dispatch / match arm that is not a default on a failure |
| `crates/pull/src/vendor.rs:157` | BENIGN-INFALLIBLE | const fn; every built date has a compile-time round-trip assertion (149-153) |
| `crates/pull/src/vendor.rs:630` | BENIGN-MATCH | compile-time assertion arm |
| `crates/pull/src/vendor.rs:1233` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/vendor.rs:1262` | BENIGN-OPTION | Option flatten/filter over Options; absence is legitimate |
| `crates/pull/src/vendor.rs:1667` | BENIGN-MATCH | compile-time assertion arm |
| `crates/pull/src/vendor.rs:3279` | BENIGN-OPTION | a rung without its own range_end uses the feed's, documented (3274) |
| `crates/api/src/aliases.rs:62` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/aliases.rs:178` | BENIGN-OPTION | validity probe: an invalid symbol renders store_key null, which is the true answer |
| `crates/api/src/assets.rs:113` | BENIGN-OPTION | BRUTEX_WEB unset -> documented default web/ dir (front end, not a credential path) |
| `crates/api/src/assets.rs:185` | BENIGN-MATCH | exhaustive status / content-type dispatch |
| `crates/api/src/assets.rs:197` | BENIGN-MATCH | hex digit parser; None refuses the percent-decode |
| `crates/api/src/assets.rs:318` | BENIGN-MATCH | exhaustive status / content-type dispatch |
| `crates/api/src/assets.rs:417` | SURFACED | unreadable entry counted, emitted as `unreadable` in api.assets startup log (assets.rs:494) |
| `crates/api/src/assets.rs:588` | BENIGN-MATCH | documented: Err is the shell newer than every source |
| `crates/api/src/assets.rs:648` | SWALLOWED | staleness walk drops unreadable DirEntry silently; banner then says 'serving' (low, display only) |
| `crates/api/src/assets.rs:703` | SURFACED | canonicalize failure -> Build::Missing, Warn log + every page 503 (assets.rs:476-488); io reason not named |
| `crates/api/src/assets.rs:757` | SURFACED | canonicalize Err -> 404 counted by note_missing (assets.rs:972-996); EACCES and ENOENT collapse (low) |
| `crates/api/src/audit.rs:327` | BENIGN-MATCH | unknown stored code refused as None (documented at audit.rs:317-321) |
| `crates/api/src/audit.rs:373` | BENIGN-MATCH | unknown stored code refused as None (documented at audit.rs:317-321) |
| `crates/api/src/audit.rs:476` | BENIGN-MATCH | unknown stored code refused as None (documented at audit.rs:317-321) |
| `crates/api/src/audit.rs:605` | BENIGN-MATCH | dispatch on kind/reason |
| `crates/api/src/audit.rs:634` | BENIGN-MATCH | documented unreachable arm of a foreign #[non_exhaustive] enum; recorded() gives the honest None |
| `crates/api/src/audit.rs:1035` | BENIGN-MATCH | dispatch on kind/reason |
| `crates/api/src/audit.rs:1057` | BENIGN-OPTION | note offset or counted by record kind (version 3 layout) |
| `crates/api/src/audit.rs:1090` | BENIGN-OPTION | writer side: new records always Some (audit.rs:595); None only from a v1 decode, never re-imaged |
| `crates/api/src/audit.rs:1933` | BENIGN-OPTION | decode Err is kept in Entry.decoded; only the whole-note lookup is skipped |
| `crates/api/src/audit.rs:2122` | BENIGN-OPTION | saturating length field; kept text capped at MAX_WHOLE_NOTE |
| `crates/api/src/audit.rs:2123` | BENIGN-OPTION | saturating length field; kept text capped at MAX_WHOLE_NOTE |
| `crates/api/src/audit.rs:2146` | BENIGN-INFALLIBLE | constant offsets inside a fixed-size array |
| `crates/api/src/audit.rs:2165` | BENIGN-INFALLIBLE | u32->usize cannot fail on 64-bit; over-capacity folds into a named Err |
| `crates/api/src/audit.rs:2265` | BENIGN-INFALLIBLE | all 16 hex digits verified before from_str_radix |
| `crates/api/src/audit.rs:2266` | BENIGN-INFALLIBLE | all 16 hex digits verified before from_str_radix |
| `crates/api/src/audit.rs:2280` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/audit.rs:2285` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/audit.rs:2305` | BENIGN-INFALLIBLE | cut on a char boundary computed just above |
| `crates/api/src/audit.rs:2310` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/audit.rs:2319` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/audit.rs:2333` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/audit.rs:2337` | BENIGN-INFALLIBLE | constant offsets inside a fixed-size array |
| `crates/api/src/audit.rs:2359` | BENIGN-INFALLIBLE | constant offsets inside a fixed-size array |
| `crates/api/src/audit.rs:2563` | BENIGN-OPTION | no vendor -> code 0, documented at audit.rs:2598 |
| `crates/api/src/audit.rs:2564` | BENIGN-OPTION | no vendor -> code 0, documented at audit.rs:2598 |
| `crates/api/src/audit.rs:2578` | BENIGN-INFALLIBLE | constant offsets inside a fixed-size array |
| `crates/api/src/audit.rs:2590` | BENIGN-INFALLIBLE | exact-width slice after the length check |
| `crates/api/src/audit.rs:2594` | BENIGN-INFALLIBLE | exact-width slice after the length check |
| `crates/api/src/audit.rs:2620` | BENIGN-INFALLIBLE | constant offsets inside a fixed-size array |
| `crates/api/src/audit.rs:2632` | BENIGN-INFALLIBLE | exact-width slice after the length check |
| `crates/api/src/audit.rs:2637` | BENIGN-INFALLIBLE | exact-width slice after the length check |
| `crates/api/src/audit.rs:2658` | BENIGN-INFALLIBLE | nibble is < 16 |
| `crates/api/src/audit.rs:2682` | SURFACED | undecodable record filed as a fault entry and counted (audit.rs:2683-2685; logged 3121) |
| `crates/api/src/audit.rs:2846` | BENIGN-MATCH | unknown index vendor code -> re-reads the authoritative journal record |
| `crates/api/src/audit.rs:2851` | SWALLOWED | counted into Indexed.malformed which no code reads (audit_json.rs:714-721); run vanishes from attribution |
| `crates/api/src/audit.rs:2865` | SWALLOWED | corrupt fault-index entry dropped uncounted; `undecodable` on /audit.json understated |
| `crates/api/src/audit.rs:2889` | SWALLOWED | corrupt refusal-index entry dropped uncounted; /dayflags refusal history omits that record |
| `crates/api/src/audit.rs:3001` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/audit.rs:3055` | BENIGN-MATCH | remove_dir_all Ok/NotFound; every other error returned at 3052-3053 |
| `crates/api/src/audit.rs:3106` | SWALLOWED | audit-index thread spawn error discarded; index never built and nothing logs why (low) |
| `crates/api/src/audit.rs:3125` | BENIGN-OPTION | telemetry text for an absent why |
| `crates/api/src/audit_json.rs:104` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/audit_json.rs:239` | SURFACED | unparseable page -> newest page + Warn 'page ignored' (audit_json.rs:242-252); response body cannot tell |
| `crates/api/src/audit_json.rs:350` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/audit_json.rs:355` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/audit_json.rs:482` | BENIGN-MATCH | find_map arm over descriptors |
| `crates/api/src/audit_json.rs:554` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/audit_json.rs:646` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/audit_json.rs:652` | SURFACED | undecodable records counted -> feed_scope.undecodable (audit_json.rs:359) |
| `crates/api/src/audit_json.rs:655` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/audit_json.rs:703` | BENIGN-OPTION | metadata only as cache identity; File::open at 626 surfaces the real error |
| `crates/api/src/audit_json.rs:707` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/audit_json.rs:732` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/audit_json.rs:819` | BENIGN-MATCH | Option<Result> split into whole_note and whole_note_error columns |
| `crates/api/src/audit_json.rs:823` | BENIGN-MATCH | Option<Result> split into whole_note and whole_note_error columns |
| `crates/api/src/audit_json.rs:870` | BENIGN-INFALLIBLE | every held key has an entry by construction (census.rs:579-593) |
| `crates/api/src/audit_json.rs:945` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/audit_json.rs:956` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/audit_json.rs:1113` | BENIGN-OPTION | documented: None printed as 'unknown', never 'never' |
| `crates/api/src/autopilot.rs:319` | BENIGN-OPTION | label for a window with no representable month |
| `crates/api/src/autopilot.rs:373` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/autopilot.rs:374` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/autopilot.rs:380` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/autopilot.rs:436` | BENIGN-OPTION | documented month-label fallback chain (autopilot.rs:426-428) |
| `crates/api/src/autopilot.rs:440` | BENIGN-OPTION | documented month-label fallback chain (autopilot.rs:426-428) |
| `crates/api/src/autopilot.rs:445` | BENIGN-OPTION | documented month-label fallback chain (autopilot.rs:426-428) |
| `crates/api/src/autopilot.rs:847` | BENIGN-OPTION | corrupt ts -> key treated unheld -> month re-offered (conservative; documented 840-843) |
| `crates/api/src/autopilot.rs:861` | BENIGN-INFALLIBLE | day 1 of a valid YearMonth |
| `crates/api/src/autopilot.rs:944` | BENIGN-MATCH | min selection |
| `crates/api/src/autopilot.rs:950` | BENIGN-INFALLIBLE | start <= to by construction (autopilot.rs:916-946) |
| `crates/api/src/autopilot.rs:1590` | BENIGN-OPTION | absent reason rendered as 'none was recorded' |
| `crates/api/src/autopilot.rs:1603` | BENIGN-OPTION | absent reason rendered as 'none was recorded' |
| `crates/api/src/autopilot.rs:2002` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/autopilot.rs:2427` | SWALLOWED | poisoned status -> Stop says no hand pull in flight (server.rs:9912); inspect doc 2487-2493 forbids (unwind builds) |
| `crates/api/src/autopilot.rs:2471` | BENIGN-OPTION | CAS failure means the seat is held, not an error |
| `crates/api/src/autopilot.rs:2499` | BENIGN-OPTION | poison -> None, documented as a refusal every caller must name |
| `crates/api/src/autopilot.rs:2730` | BENIGN-OPTION | None outside the store's month range (documented) |
| `crates/api/src/autopilot.rs:2731` | BENIGN-OPTION | None outside the store's month range (documented) |
| `crates/api/src/autopilot.rs:2732` | BENIGN-OPTION | None outside the store's month range (documented) |
| `crates/api/src/autopilot.rs:2778` | BENIGN-OPTION | a valid Day always has a YearMonth |
| `crates/api/src/autopilot.rs:2830` | BENIGN-INFALLIBLE | epoch day 0 / epoch..yesterday / a valid Day are representable |
| `crates/api/src/autopilot.rs:2831` | BENIGN-INFALLIBLE | epoch day 0 / epoch..yesterday / a valid Day are representable |
| `crates/api/src/autopilot.rs:2836` | SWALLOWED | clock failure -> floor None -> feed silently dropped by drivable() (low, defence in depth) |
| `crates/api/src/autopilot.rs:2838` | SWALLOWED | clamp_to_floor's named Err discarded; feed silently leaves the backfill loop (low) |
| `crates/api/src/autopilot.rs:2852` | SWALLOWED | feed with no floor skipped; autopilot status never names it (low) |
| `crates/api/src/autopilot.rs:2855` | BENIGN-INFALLIBLE | epoch day 0 / epoch..yesterday / a valid Day are representable |
| `crates/api/src/autopilot.rs:2867` | BENIGN-OPTION | None on an unusable clock; caller idles (autopilot.rs:3553) |
| `crates/api/src/autopilot.rs:2868` | BENIGN-OPTION | None on an unusable clock; caller idles (autopilot.rs:3553) |
| `crates/api/src/autopilot.rs:2987` | BENIGN-OPTION | display minimum over floors |
| `crates/api/src/autopilot.rs:3083` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/autopilot.rs:3098` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/autopilot.rs:3166` | BENIGN-OPTION | Option<String> rendered empty = none |
| `crates/api/src/autopilot.rs:3167` | BENIGN-OPTION | Option<String> rendered empty = none |
| `crates/api/src/autopilot.rs:3304` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/autopilot.rs:3494` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/autopilot.rs:3543` | BENIGN-INFALLIBLE | documented poison read of a flag (3536-3539); release panic=abort |
| `crates/api/src/autopilot.rs:3674` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/autopilot.rs:3679` | BENIGN-OPTION | Option<String> rendered empty = none |
| `crates/api/src/autopilot.rs:3704` | BENIGN-OPTION | Option<String> rendered empty = none |
| `crates/api/src/autopilot.rs:3714` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/autopilot.rs:3839` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/autopilot.rs:3871` | BENIGN-OPTION | named default reason when the refused list is empty |
| `crates/api/src/autopilot.rs:3940` | BENIGN-OPTION | unknown completeness -> not complete (conservative: retries) |
| `crates/api/src/autopilot.rs:4143` | SURFACED | poisoned lock -> named REFUSED admission |
| `crates/api/src/backtest.rs:547` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:551` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:552` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:553` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:554` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:555` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:556` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:557` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:558` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:559` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:560` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:561` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:562` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:563` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:564` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:565` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:566` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:567` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:568` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:569` | BENIGN-INFALLIBLE | exact-width take() from a fixed array; try_into cannot fail |
| `crates/api/src/backtest.rs:724` | BENIGN-OPTION | no NUL padding -> whole field (documented lossy text) |
| `crates/api/src/backtest.rs:725` | BENIGN-INFALLIBLE | in-bounds slice / fixed header / saturating conversion |
| `crates/api/src/backtest.rs:1124` | BENIGN-INFALLIBLE | in-bounds slice / fixed header / saturating conversion |
| `crates/api/src/backtest.rs:1125` | BENIGN-INFALLIBLE | in-bounds slice / fixed header / saturating conversion |
| `crates/api/src/backtest.rs:1155` | BENIGN-INFALLIBLE | in-bounds slice / fixed header / saturating conversion |
| `crates/api/src/backtest.rs:1159` | BENIGN-INFALLIBLE | in-bounds slice / fixed header / saturating conversion |
| `crates/api/src/backtest.rs:1240` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/backtest.rs:1310` | SURFACED | unparseable limit -> default + Warn 'limit ignored' (backtest.rs:1297-1306) |
| `crates/api/src/bars.rs:228` | BENIGN-OPTION | documented '—' for an unrepresentable timestamp |
| `crates/api/src/bars.rs:229` | BENIGN-OPTION | documented '—' for an unrepresentable timestamp |
| `crates/api/src/bars.rs:549` | SURFACED | refused records returned in FaultLog and logged (bars.rs:550-555, 593) |
| `crates/api/src/bars.rs:587` | SURFACED | refused records returned in FaultLog and logged (bars.rs:550-555, 593) |
| `crates/api/src/bars.rs:614` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars.rs:619` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars.rs:816` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses (bars_route.rs:510) |
| `crates/api/src/bars.rs:977` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars.rs:1066` | BENIGN-MATCH | only BaseNotPositive remains; rendered as a named code |
| `crates/api/src/bars.rs:1108` | BENIGN-INFALLIBLE | absent is Some whenever behind is not a Bar |
| `crates/api/src/bars.rs:1227` | BENIGN-OPTION | a key whose symbol is not a Symbol has no GDFL re-key |
| `crates/api/src/bars.rs:1431` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars.rs:1438` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars.rs:1439` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars.rs:1440` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars.rs:1564` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars.rs:1815` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars.rs:1816` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars.rs:1878` | SURFACED | Behind::Unreadable -> named 'previous_unreadable' (bars.rs:1070) |
| `crates/api/src/bars.rs:1901` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars.rs:1941` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars.rs:1996` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars.rs:1998` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars.rs:2001` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars.rs:2003` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars.rs:2035` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars_change.rs:173` | BENIGN-OPTION | segment text that does not parse names no store directory, so there is no bar to judge |
| `crates/api/src/bars_change.rs:193` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars_change.rs:312` | BENIGN-MATCH | unknown phase -> not a phase change |
| `crates/api/src/bars_change.rs:325` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars_change.rs:334` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars_change.rs:361` | BENIGN-OPTION | documented: Unknown, never NotEligible (bars_change.rs:350-352) |
| `crates/api/src/bars_change.rs:362` | BENIGN-OPTION | documented: Unknown, never NotEligible (bars_change.rs:350-352) |
| `crates/api/src/bars_change.rs:382` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars_change.rs:474` | BENIGN-MATCH | cache miss path; an unreadable ledger is named (477-486) |
| `crates/api/src/bars_change.rs:532` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars_change.rs:570` | BENIGN-OPTION | HashMap::insert's displaced value discarded; last ledger record for a key wins |
| `crates/api/src/bars_change.rs:580` | BENIGN-OPTION | HashMap::insert's displaced value discarded; last ledger record for a key wins |
| `crates/api/src/bars_route.rs:219` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/bars_route.rs:321` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars_route.rs:344` | BENIGN-OPTION | bisection failure -> full-month read, same answer (documented 318-342) |
| `crates/api/src/bars_route.rs:345` | BENIGN-OPTION | bisection failure -> full-month read, same answer (documented 318-342) |
| `crates/api/src/bars_route.rs:347` | BENIGN-OPTION | bisection failure -> full-month read, same answer (documented 318-342) |
| `crates/api/src/bars_route.rs:362` | BENIGN-OPTION | bisection failure -> full-month read, same answer (documented 318-342) |
| `crates/api/src/bars_route.rs:363` | BENIGN-OPTION | bisection failure -> full-month read, same answer (documented 318-342) |
| `crates/api/src/bars_route.rs:368` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars_route.rs:473` | SURFACED | refused at bars_route.rs:486 |
| `crates/api/src/bars_route.rs:484` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/bars_route.rs:502` | SURFACED | refused at bars_route.rs:504 |
| `crates/api/src/bars_route.rs:566` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars_route.rs:647` | SURFACED | refused at bars_route.rs:650 |
| `crates/api/src/bars_route.rs:788` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/bars_route.rs:909` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars_route.rs:987` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/bars_route.rs:1062` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars_series.rs:174` | BENIGN-INFALLIBLE | index in range by construction |
| `crates/api/src/bars_series.rs:180` | BENIGN-INFALLIBLE | index in range by construction |
| `crates/api/src/bars_series.rs:202` | BENIGN-MATCH | exhaustive shape / bisection arm |
| `crates/api/src/bars_series.rs:282` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/bars_series.rs:424` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/bars_series.rs:455` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/bars_series.rs:726` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/bars_series.rs:850` | BENIGN-INFALLIBLE | index in range by construction |
| `crates/api/src/bars_series.rs:921` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/bars_series.rs:930` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/bars_series.rs:931` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/bars_series.rs:962` | BENIGN-INFALLIBLE | index in range by construction |
| `crates/api/src/bars_series.rs:963` | BENIGN-INFALLIBLE | index in range by construction |
| `crates/api/src/bars_series.rs:1019` | BENIGN-INFALLIBLE | index in range by construction |
| `crates/api/src/bars_series.rs:1026` | BENIGN-INFALLIBLE | index in range by construction |
| `crates/api/src/bars_series.rs:1113` | BENIGN-MATCH | exhaustive shape / bisection arm |
| `crates/api/src/bars_series.rs:1242` | BENIGN-MATCH | pushes a new UnreadMonth entry: this is the surfacing |
| `crates/api/src/bars_series.rs:1377` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/bars_sort.rs:294` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/bars_sort.rs:728` | SURFACED | .ok() immediately converted to a named Err |
| `crates/api/src/bars_sort.rs:744` | SURFACED | .ok() immediately converted to a named Err |
| `crates/api/src/bars_sort.rs:770` | SURFACED | .ok() immediately converted to a named Err |
| `crates/api/src/bars_sort.rs:1032` | SURFACED | .ok() immediately converted to a named Err |
| `crates/api/src/bars_sort.rs:1114` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars_sort.rs:1178` | BENIGN-MATCH | cursor / bisection arm |
| `crates/api/src/bars_sort.rs:1600` | BENIGN-MATCH | cursor / bisection arm |
| `crates/api/src/bars_sort.rs:2034` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars_sort.rs:2050` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars_sort.rs:2188` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars_sort.rs:2317` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars_sort.rs:2324` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/bars_sort.rs:2354` | SURFACED | falls back to Attempt::Refused(why), named |
| `crates/api/src/bars_sort.rs:2495` | BENIGN-INFALLIBLE | Piece::Indexed is only built from an Ok view (bars_sort.rs:540-560) |
| `crates/api/src/bars_sort.rs:2628` | BENIGN-OPTION | display text |
| `crates/api/src/booleancampaignjson.rs:63` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/booleancampaignjson.rs:158` | BENIGN-OPTION | only completed catalogs; expected_catalogs beside it lists all with null completion |
| `crates/api/src/booleanevidencejson.rs:82` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleanevidencejson.rs:84` | BENIGN-OPTION | absent offset/limit default; malformed refused by integer()? |
| `crates/api/src/booleanevidencejson.rs:86` | BENIGN-OPTION | absent offset/limit default; malformed refused by integer()? |
| `crates/api/src/booleanevidencejson.rs:96` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleanevidencejson.rs:201` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/booleanevidencejson.rs:300` | BENIGN-OPTION | revalidation probe; a failed check forces a fresh authenticated open (detail.rs:1098) |
| `crates/api/src/booleanevidencejson.rs:470` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleanjson.rs:68` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleanjson.rs:71` | BENIGN-OPTION | absent offset/limit default; malformed refused by integer()? |
| `crates/api/src/booleanjson.rs:84` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleanjson.rs:90` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleanjson.rs:130` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/booleanjson.rs:197` | BENIGN-OPTION | revalidation probe; a failed check forces a fresh authenticated open (detail.rs:1098) |
| `crates/api/src/booleanjson.rs:274` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/booleanjson.rs:286` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleanjson.rs:351` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/booleanjson.rs:408` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleanjson.rs:416` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleanlaunch.rs:188` | BENIGN-OPTION | strip optional prefix |
| `crates/api/src/booleanlaunch.rs:478` | SURFACED | None refused at booleanlaunch.rs:468 |
| `crates/api/src/booleanoosjson.rs:49` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleanoosjson.rs:53` | BENIGN-OPTION | absent offset/limit default; malformed refused by integer()? |
| `crates/api/src/booleanoosjson.rs:95` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/booleanoosjson.rs:186` | BENIGN-OPTION | no candidate asked -> null |
| `crates/api/src/booleanqualification_projection.rs:160` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleanqualification_projection.rs:167` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleansearchjson.rs:48` | BENIGN-OPTION | absent offset/limit default; malformed refused by integer()? |
| `crates/api/src/booleansearchjson.rs:50` | BENIGN-OPTION | absent offset/limit default; malformed refused by integer()? |
| `crates/api/src/booleansearchjson.rs:66` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/booleansearchjson.rs:100` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/booleansearchjson.rs:191` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/calendar.rs:197` | BENIGN-MATCH | documented unreachable (month always 1..=12) |
| `crates/api/src/calendar.rs:215` | BENIGN-INFALLIBLE | validity probe for month length |
| `crates/api/src/calendar.rs:218` | BENIGN-INFALLIBLE | validity probe for month length |
| `crates/api/src/calendar.rs:221` | BENIGN-INFALLIBLE | validity probe for month length |
| `crates/api/src/calendar_build.rs:177` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:194` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:232` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:248` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:250` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:261` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:343` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/calendar_build.rs:400` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:425` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:445` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:462` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:470` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:538` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:548` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:562` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:620` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:636` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:722` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_build.rs:731` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:197` | BENIGN-OPTION | documented zero past the month |
| `crates/api/src/calendar_of.rs:252` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:275` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:284` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:294` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:389` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:396` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:667` | SWALLOWED | unrepresentable first day -> withhold_unproved returns: unproved months keep 'Closed' days (fail-open, low) |
| `crates/api/src/calendar_of.rs:668` | SWALLOWED | unrepresentable first day -> withhold_unproved returns: unproved months keep 'Closed' days (fail-open, low) |
| `crates/api/src/calendar_of.rs:669` | BENIGN-INFALLIBLE | day 1 of a valid month |
| `crates/api/src/calendar_of.rs:771` | SURFACED | unreadable records counted and the whole month refused (calendar_of.rs:775-841) |
| `crates/api/src/calendar_of.rs:828` | BENIGN-MATCH | run builder |
| `crates/api/src/calendar_of.rs:831` | SURFACED | unreadable records counted and the whole month refused (calendar_of.rs:775-841) |
| `crates/api/src/calendar_of.rs:851` | BENIGN-INFALLIBLE | bounded arithmetic / own JSON always ends in '}' |
| `crates/api/src/calendar_of.rs:2476` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:2495` | BENIGN-OPTION | kept answer or derive |
| `crates/api/src/calendar_of.rs:2508` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:2513` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:2537` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:2587` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/calendar_of.rs:2922` | BENIGN-INFALLIBLE | bounded arithmetic / own JSON always ends in '}' |
| `crates/api/src/calendar_record.rs:127` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:128` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:268` | BENIGN-INFALLIBLE | bounded inputs / synthetic sane bars / saturating conversion |
| `crates/api/src/calendar_record.rs:276` | BENIGN-INFALLIBLE | bounded inputs / synthetic sane bars / saturating conversion |
| `crates/api/src/calendar_record.rs:289` | BENIGN-INFALLIBLE | bounded inputs / synthetic sane bars / saturating conversion |
| `crates/api/src/calendar_record.rs:290` | BENIGN-INFALLIBLE | bounded inputs / synthetic sane bars / saturating conversion |
| `crates/api/src/calendar_record.rs:361` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:371` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:373` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:384` | BENIGN-INFALLIBLE | bounded inputs / synthetic sane bars / saturating conversion |
| `crates/api/src/calendar_record.rs:386` | BENIGN-INFALLIBLE | bounded inputs / synthetic sane bars / saturating conversion |
| `crates/api/src/calendar_record.rs:551` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:645` | BENIGN-INFALLIBLE | bounded inputs / synthetic sane bars / saturating conversion |
| `crates/api/src/calendar_record.rs:655` | BENIGN-INFALLIBLE | bounded inputs / synthetic sane bars / saturating conversion |
| `crates/api/src/calendar_record.rs:659` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:728` | BENIGN-INFALLIBLE | bounded inputs / synthetic sane bars / saturating conversion |
| `crates/api/src/calendar_record.rs:775` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:884` | BENIGN-INFALLIBLE | parses this server's own serialized answer |
| `crates/api/src/calendar_record.rs:900` | BENIGN-OPTION | every ok answer carries rung (calendar_record.rs:815) |
| `crates/api/src/calendar_record.rs:903` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:907` | BENIGN-INFALLIBLE | bounded inputs / synthetic sane bars / saturating conversion |
| `crates/api/src/calendar_record.rs:934` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:989` | BENIGN-INFALLIBLE | parses this server's own serialized answer |
| `crates/api/src/calendar_record.rs:993` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:1019` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:1024` | BENIGN-OPTION | Option default with no failure behind it (documented where noted) |
| `crates/api/src/calendar_record.rs:1036` | BENIGN-INFALLIBLE | bounded inputs / synthetic sane bars / saturating conversion |
| `crates/api/src/calendar_route.rs:247` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/calendar_route.rs:265` | BENIGN-OPTION | malformed rung is refused by Asked::of (calendar_route.rs:412) |
| `crates/api/src/calendar_route.rs:403` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/calendar_route.rs:456` | BENIGN-INFALLIBLE | status was stored from a StatusCode / saturating ms |
| `crates/api/src/calendar_route.rs:477` | BENIGN-INFALLIBLE | status was stored from a StatusCode / saturating ms |
| `crates/api/src/calendar_route.rs:911` | BENIGN-OPTION | unheld target -> named 'withheld' answer (calendar_record.rs:781-798) |
| `crates/api/src/calendar_route.rs:994` | BENIGN-OPTION | documented: a reference not holding the rung attests nothing (980-981) |
| `crates/api/src/calendar_route.rs:1073` | BENIGN-INFALLIBLE | status was stored from a StatusCode / saturating ms |
| `crates/api/src/calendar_route.rs:1126` | BENIGN-OPTION | feeds without a store vendor are not offered (D-3338) |
| `crates/api/src/calendar_route.rs:1312` | SURFACED | panicked journal walk named as Err |
| `crates/api/src/candidatejson.rs:58` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/candidatejson.rs:64` | BENIGN-OPTION | absent param default; malformed refused by integer()? |
| `crates/api/src/candidatejson.rs:70` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/candidatejson.rs:72` | BENIGN-OPTION | absent param default; malformed refused by integer()? |
| `crates/api/src/candidatejson.rs:73` | BENIGN-OPTION | absent param default; malformed refused by integer()? |
| `crates/api/src/candidatejson.rs:129` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/candidatejson.rs:238` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/catalog.rs:118` | SWALLOWED | unknown sort= silently -> Default order, 200 (documented stale-bookmark rule; display, low) |
| `crates/api/src/catalog.rs:279` | SWALLOWED | unknown pill silently -> Every; doc 256-259 admits ?u=n50 reads as a filter that did nothing (low) |
| `crates/api/src/catalog.rs:411` | BENIGN-OPTION | empty -> 0 |
| `crates/api/src/catalog.rs:456` | BENIGN-INFALLIBLE | fixed-size array / in-range window by construction |
| `crates/api/src/catalog.rs:466` | BENIGN-INFALLIBLE | fixed-size array / in-range window by construction |
| `crates/api/src/catalog.rs:499` | BENIGN-INFALLIBLE | fixed-size array / in-range window by construction |
| `crates/api/src/catalog.rs:501` | BENIGN-INFALLIBLE | fixed-size array / in-range window by construction |
| `crates/api/src/catalog.rs:562` | BENIGN-INFALLIBLE | fixed-size array / in-range window by construction |
| `crates/api/src/census.rs:430` | BENIGN-INFALLIBLE | in-range by construction / saturating / constant valid symbols |
| `crates/api/src/census.rs:435` | BENIGN-INFALLIBLE | in-range by construction / saturating / constant valid symbols |
| `crates/api/src/census.rs:490` | BENIGN-INFALLIBLE | in-range by construction / saturating / constant valid symbols |
| `crates/api/src/census.rs:770` | BENIGN-MATCH | exhaustive comparison / no filter |
| `crates/api/src/census.rs:1156` | SURFACED | short file handed to the decoder, which refuses it by name (census.rs:1149-1153) |
| `crates/api/src/census.rs:1336` | BENIGN-INFALLIBLE | byte count discarded; the io error is propagated by ? |
| `crates/api/src/census.rs:1337` | BENIGN-OPTION | bound only; an undecodable header is refused by Manifest::open |
| `crates/api/src/census.rs:1346` | BENIGN-INFALLIBLE | in-range by construction / saturating / constant valid symbols |
| `crates/api/src/census.rs:1347` | BENIGN-INFALLIBLE | byte count discarded; the io error is propagated by ? |
| `crates/api/src/census.rs:1350` | BENIGN-INFALLIBLE | in-range by construction / saturating / constant valid symbols |
| `crates/api/src/census.rs:1374` | BENIGN-OPTION | None -> no first/last entry -> extend refuses -> whole read |
| `crates/api/src/census.rs:1377` | BENIGN-OPTION | None -> no first/last entry -> extend refuses -> whole read |
| `crates/api/src/census.rs:1378` | BENIGN-OPTION | None -> no first/last entry -> extend refuses -> whole read |
| `crates/api/src/census.rs:1379` | BENIGN-OPTION | None -> no first/last entry -> extend refuses -> whole read |
| `crates/api/src/census.rs:1388` | BENIGN-OPTION | bound only; an undecodable header is refused by Manifest::open |
| `crates/api/src/census.rs:1404` | BENIGN-OPTION | documented: None sends the caller to a whole read that names the damaged slot (1394-1397) |
| `crates/api/src/census.rs:1438` | BENIGN-OPTION | extend() refusal is never an answer: caller does a whole read (census.rs:1134) |
| `crates/api/src/census.rs:1439` | BENIGN-OPTION | extend() refusal is never an answer: caller does a whole read (census.rs:1134) |
| `crates/api/src/census.rs:1444` | BENIGN-OPTION | extend() refusal is never an answer: caller does a whole read (census.rs:1134) |
| `crates/api/src/census.rs:1446` | BENIGN-OPTION | extend() refusal is never an answer: caller does a whole read (census.rs:1134) |
| `crates/api/src/census.rs:1481` | BENIGN-OPTION | extend() refusal is never an answer: caller does a whole read (census.rs:1134) |
| `crates/api/src/census.rs:1493` | BENIGN-MATCH | inconsistent first/last pair -> refuse extend |
| `crates/api/src/census.rs:1502` | BENIGN-INFALLIBLE | in-range by construction / saturating / constant valid symbols |
| `crates/api/src/census.rs:1506` | BENIGN-OPTION | extend() refusal is never an answer: caller does a whole read (census.rs:1134) |
| `crates/api/src/census.rs:1507` | BENIGN-OPTION | extend() refusal is never an answer: caller does a whole read (census.rs:1134) |
| `crates/api/src/census.rs:1509` | BENIGN-OPTION | extend() refusal is never an answer: caller does a whole read (census.rs:1134) |
| `crates/api/src/census.rs:1514` | BENIGN-OPTION | extend() refusal is never an answer: caller does a whole read (census.rs:1134) |
| `crates/api/src/census.rs:1519` | BENIGN-OPTION | extend() refusal is never an answer: caller does a whole read (census.rs:1134) |
| `crates/api/src/census.rs:1631` | BENIGN-INFALLIBLE | byte count discarded; the io error is propagated by ? |
| `crates/api/src/census.rs:1634` | BENIGN-INFALLIBLE | in-range by construction / saturating / constant valid symbols |
| `crates/api/src/census.rs:1688` | BENIGN-INFALLIBLE | silences an unused binding off unix |
| `crates/api/src/census.rs:1778` | BENIGN-OPTION | documented None before 1970 |
| `crates/api/src/census.rs:1792` | BENIGN-OPTION | documented None before 1970 |
| `crates/api/src/census.rs:2183` | BENIGN-MATCH | exhaustive comparison / no filter |
| `crates/api/src/census.rs:2288` | BENIGN-INFALLIBLE | in-range by construction / saturating / constant valid symbols |
| `crates/api/src/census_cache.rs:132` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/census_cache.rs:291` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/census_cache.rs:313` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/census_cache.rs:323` | BENIGN-MATCH | exhaustive comparison arm |
| `crates/api/src/census_cache.rs:393` | BENIGN-INFALLIBLE | array indexed by vendor discriminant |
| `crates/api/src/census_cache.rs:532` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/census_cache.rs:588` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/census_cache.rs:794` | SURFACED | stat error kept as SegmentStamp::Faulted(kind) and compared |
| `crates/api/src/census_cache.rs:798` | BENIGN-OPTION | stamp fallback; ctime still moves on change |
| `crates/api/src/census_cache.rs:816` | BENIGN-OPTION | stamp fallback; ctime still moves on change |
| `crates/api/src/census_cache.rs:916` | BENIGN-MATCH | exhaustive comparison arm |
| `crates/api/src/census_cache.rs:1104` | BENIGN-MATCH | exhaustive comparison arm |
| `crates/api/src/chain.rs:199` | SURFACED | None refused by caller (chain.rs:219) |
| `crates/api/src/chain.rs:238` | SURFACED | named refusal / fault pushed |
| `crates/api/src/chain.rs:500` | SURFACED | half-parsed lineage line counted as unread (chain.rs:526-530) |
| `crates/api/src/chain.rs:633` | BENIGN-OPTION | Option default with no failure behind it (954: no options named at 1003-1007) |
| `crates/api/src/chain.rs:660` | BENIGN-INFALLIBLE | valid expiry -> valid YearMonth / bounded |
| `crates/api/src/chain.rs:751` | BENIGN-OPTION | Option default with no failure behind it (954: no options named at 1003-1007) |
| `crates/api/src/chain.rs:752` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/chain.rs:823` | SURFACED | named refusal / fault pushed |
| `crates/api/src/chain.rs:954` | BENIGN-OPTION | Option default with no failure behind it (954: no options named at 1003-1007) |
| `crates/api/src/chain.rs:1017` | BENIGN-OPTION | Option default with no failure behind it (954: no options named at 1003-1007) |
| `crates/api/src/chain.rs:1107` | BENIGN-INFALLIBLE | valid expiry -> valid YearMonth / bounded |
| `crates/api/src/chain.rs:1108` | BENIGN-INFALLIBLE | valid expiry -> valid YearMonth / bounded |
| `crates/api/src/chain.rs:1178` | BENIGN-OPTION | Option default with no failure behind it (954: no options named at 1003-1007) |
| `crates/api/src/constituents.rs:508` | BENIGN-INFALLIBLE | array indexed by vendor/tier discriminant |
| `crates/api/src/constituents.rs:623` | BENIGN-INFALLIBLE | array indexed by vendor/tier discriminant |
| `crates/api/src/constituents.rs:647` | BENIGN-OPTION | no claim -> empty; caller names 'lacks' (900-905) |
| `crates/api/src/constituents.rs:651` | BENIGN-MATCH | absent or ambiguous -> None, documented 628-633 |
| `crates/api/src/constituents.rs:897` | BENIGN-OPTION | no claim -> empty; caller names 'lacks' (900-905) |
| `crates/api/src/coverage.rs:262` | BENIGN-INFALLIBLE | inside `if let Some(tier)`: ids_for is Some (constituents.rs:679) |
| `crates/api/src/coverage.rs:385` | BENIGN-INFALLIBLE | pinned by every_target_indexes_its_own_slot |
| `crates/api/src/coverage.rs:535` | BENIGN-OPTION | absent id pushed to `missing` |
| `crates/api/src/credential_law.rs:103` | BENIGN-MATCH | every non-transport SecretError is configuration (documented 96-98) |
| `crates/api/src/dayflags.rs:261` | SURFACED | converted to a named refusal [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:323` | SURFACED | converted to a named refusal [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:403` | SURFACED | None falls to the refusal at dayflags.rs:409 [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:426` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:474` | BENIGN-OPTION | page bound always inside from..=to [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:482` | BENIGN-MATCH | feed is GDFL or Zerodha only (feed_of, dayflags.rs:276-288) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:490` | BENIGN-MATCH | feed is GDFL or Zerodha only (feed_of, dayflags.rs:276-288) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:525` | SURFACED | source error named under sources (dayflags.rs:589) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:532` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:543` | SURFACED | source error named under sources (dayflags.rs:589) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:544` | SURFACED | source error named under sources (dayflags.rs:589) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:545` | SURFACED | source error named under sources (dayflags.rs:589) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:590` | SURFACED | source error named under sources (dayflags.rs:589) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:600` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:740` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:750` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:788` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:793` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:836` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:837` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:844` | BENIGN-OPTION | documented (825): run id is its start second (pull/src/gdfl_import.rs:2984-2993) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:845` | BENIGN-OPTION | documented (825): run id is its start second (pull/src/gdfl_import.rs:2984-2993) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:879` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:888` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:897` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:912` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:914` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:951` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1034` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1040` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1104` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1242` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1263` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1266` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1274` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1291` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1314` | SURFACED | converted to a named refusal [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1505` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1513` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1521` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1587` | SURFACED | missing word becomes '' which the next parse refuses by name [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1589` | SURFACED | missing word becomes '' which the next parse refuses by name [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1592` | SURFACED | missing word becomes '' which the next parse refuses by name [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1621` | BENIGN-MATCH | documented forward-compatible field skip (1636-1638) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1629` | SURFACED | converted to a named refusal [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1645` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1806` | SURFACED | missing word becomes '' which the next parse refuses by name [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1823` | SURFACED | missing field '' refused by the version/clock checks that follow [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1834` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1871` | SURFACED | missing word becomes '' which the next parse refuses by name [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1875` | SURFACED | missing word becomes '' which the next parse refuses by name [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:1879` | SURFACED | missing field '' refused by the version/clock checks that follow [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2014` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2099` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2103` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2168` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2239` | BENIGN-OPTION | prefilter only; the note is decoded after [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2245` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2253` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2273` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2364` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2389` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2499` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2527` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2585` | BENIGN-OPTION | index probe; Err just means walk [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2590` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2594` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2623` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2625` | SURFACED | walk error returned at dayflags.rs:2635 [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2636` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2647` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2726` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2786` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2787` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2789` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2853` | SWALLOWED | refused open (2848) or unreadable first 1day record -> 'none_held', no note [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2855` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2889` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:2987` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3026` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3192` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3193` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3206` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3227` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3228` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3237` | SWALLOWED | last bar before page unreadable (or open Refused, 3231) -> prev_close None, no note [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3252` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3367` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3373` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3376` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3377` | BENIGN-INFALLIBLE | bounded / in-range by construction [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3379` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3485` | BENIGN-INFALLIBLE | required ledger fields, validated at parse (dayflags.rs:1378-1420) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3486` | BENIGN-INFALLIBLE | required ledger fields, validated at parse (dayflags.rs:1378-1420) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3658` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3669` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3674` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3721` | BENIGN-INFALLIBLE | required ledger fields, validated at parse (dayflags.rs:1378-1420) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3768` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3803` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3826` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3848` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3915` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:3932` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4070` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4106` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4107` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4113` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4206` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4207` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4214` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4215` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4221` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4233` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4242` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4255` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4273` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4298` | BENIGN-OPTION | field of this server's own /imports.json or record JSON [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4317` | BENIGN-OPTION | Option default with no failure behind it [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4322` | BENIGN-INFALLIBLE | required ledger fields, validated at parse (dayflags.rs:1378-1420) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4341` | BENIGN-INFALLIBLE | required ledger fields, validated at parse (dayflags.rs:1378-1420) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4359` | BENIGN-INFALLIBLE | required ledger fields, validated at parse (dayflags.rs:1378-1420) [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4406` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4434` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4488` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4506` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4610` | BENIGN-MATCH | exhaustive/phrasing arm over a closed set [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4629` | SURFACED | ledger read errors noted at dayflags.rs:4607/4619 [LOCKED dayflags.rs - finding only] |
| `crates/api/src/dayflags.rs:4634` | SURFACED | ledger read errors noted at dayflags.rs:4607/4619 [LOCKED dayflags.rs - finding only] |
| `crates/api/src/detail.rs:163` | SWALLOWED | clock thread spawn error dropped and None cached forever; waits refused as 'Saturated' (misnamed, low) |
| `crates/api/src/detail.rs:191` | BENIGN-OPTION | CAS failure means full, answered Saturated |
| `crates/api/src/detail.rs:216` | BENIGN-OPTION | CAS failure means full, answered Saturated |
| `crates/api/src/detail.rs:493` | BENIGN-OPTION | absent page/limit default; malformed refused by integer_param()? |
| `crates/api/src/detail.rs:494` | BENIGN-OPTION | absent page/limit default; malformed refused by integer_param()? |
| `crates/api/src/detail.rs:505` | BENIGN-INFALLIBLE | product discarded; overflow refused by ? |
| `crates/api/src/detail.rs:562` | SURFACED | pair without '=' has empty value and is refused at 563-569 |
| `crates/api/src/detail.rs:594` | SURFACED | refused at detail.rs:597 |
| `crates/api/src/detail.rs:656` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/detail.rs:695` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/detail.rs:717` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/detail.rs:718` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/detail.rs:874` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/detail.rs:891` | BENIGN-MATCH | slot for another root -> none |
| `crates/api/src/detail.rs:929` | BENIGN-OPTION | refused refresh -> fresh open, which surfaces its own error (933-936) |
| `crates/api/src/detail.rs:1125` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/detail.rs:1152` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/disk_route.rs:110` | SURFACED | refused at disk_route.rs:112 |
| `crates/api/src/disk_route.rs:121` | BENIGN-OPTION | mount text absent -> empty |
| `crates/api/src/disk_route.rs:163` | SWALLOWED | non-NotFound stat error read as 'not sealed' -> role mislabelled on /disk (low) |
| `crates/api/src/disk_route.rs:194` | SWALLOWED | DirEntry errors dropped: epoch files vanish from /disk byte totals (low) |
| `crates/api/src/disk_route.rs:195` | BENIGN-OPTION | names that are not epoch files are not census files |
| `crates/api/src/disk_route.rs:197` | SWALLOWED | metadata failure drops an epoch file from /disk silently (low) |
| `crates/api/src/disk_route.rs:278` | SURFACED | render failure -> recorded:false with why |
| `crates/api/src/disk_route.rs:330` | BENIGN-INFALLIBLE | slice ends at a found newline |
| `crates/api/src/disk_route.rs:380` | BENIGN-MATCH | date-shape / month-length dispatch |
| `crates/api/src/disk_route.rs:399` | BENIGN-MATCH | date-shape / month-length dispatch |
| `crates/api/src/disk_route.rs:539` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/expressionsearchjson.rs:46` | BENIGN-OPTION | absent limit default; malformed refused by integer()? |
| `crates/api/src/expressionsearchjson.rs:65` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/expressionsearchjson.rs:83` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/facets.rs:175` | SURFACED | unparseable contract counted as unread_contracts (facets.rs:301-304, rendered 897/1296) |
| `crates/api/src/facets.rs:176` | SURFACED | unparseable contract counted as unread_contracts (facets.rs:301-304, rendered 897/1296) |
| `crates/api/src/facets.rs:177` | SURFACED | unparseable contract counted as unread_contracts (facets.rs:301-304, rendered 897/1296) |
| `crates/api/src/facets.rs:178` | SURFACED | unparseable contract counted as unread_contracts (facets.rs:301-304, rendered 897/1296) |
| `crates/api/src/facets.rs:186` | SURFACED | unparseable contract counted as unread_contracts (facets.rs:301-304, rendered 897/1296) |
| `crates/api/src/facets.rs:188` | SURFACED | unparseable contract counted as unread_contracts (facets.rs:301-304, rendered 897/1296) |
| `crates/api/src/facets.rs:272` | BENIGN-OPTION | futures carry no strike |
| `crates/api/src/facets.rs:348` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/facets.rs:351` | BENIGN-INFALLIBLE | PoisonError::into_inner on a cache/flag; release panic=abort (Cargo.toml:194) makes poisoning unreachable |
| `crates/api/src/facets.rs:439` | SURFACED | refused at facets.rs:441 |
| `crates/api/src/facets.rs:558` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/facets.rs:613` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/facets.rs:789` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/facets.rs:1224` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/facets.rs:1316` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/facets.rs:1357` | BENIGN-OPTION | futures carry no strike |
| `crates/api/src/facets.rs:1463` | BENIGN-INFALLIBLE | rem_euclid(7) in range / HashSet::insert bool |
| `crates/api/src/facets.rs:1465` | BENIGN-INFALLIBLE | rem_euclid(7) in range / HashSet::insert bool |
| `crates/api/src/facets.rs:1481` | BENIGN-INFALLIBLE | rem_euclid(7) in range / HashSet::insert bool |
| `crates/api/src/facets.rs:1509` | BENIGN-OPTION | same skip as the index, which counts it (facets.rs:301-304) |
| `crates/api/src/facets.rs:1519` | BENIGN-OPTION | same skip as the index, which counts it (facets.rs:301-304) |
| `crates/api/src/fault_log.rs:137` | BENIGN-MATCH | cause classification / span text |
| `crates/api/src/fault_log.rs:323` | BENIGN-MATCH | cause classification / span text |
| `crates/api/src/feeds_route.rs:58` | BENIGN-MATCH | exhaustive rendering arm |
| `crates/api/src/feeds_route.rs:253` | BENIGN-OPTION | documented (250-252): unusable clock renders a null day beside a rolling kind |
| `crates/api/src/feeds_route.rs:748` | BENIGN-MATCH | exhaustive rendering arm |
| `crates/api/src/folder.rs:78` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/folder.rs:256` | BENIGN-MATCH | non-matching arm returns a named refusal / None that the caller refuses |
| `crates/api/src/frontierjson.rs:81` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/frontierjson.rs:233` | BENIGN-INFALLIBLE | fmt::Write into a String cannot fail |
| `crates/api/src/frontierjson.rs:269` | BENIGN-INFALLIBLE | documented saturating conversion; cannot fail on 64-bit / bounded input |
| `crates/api/src/frontierjson.rs:396` | BENIGN-INFALLIBLE | fmt::Write into a String cannot fail |
| `crates/api/src/frontierjson.rs:484` | BENIGN-INFALLIBLE | fmt::Write into a String cannot fail |
| `crates/api/src/frontierjson.rs:501` | BENIGN-INFALLIBLE | fmt::Write into a String cannot fail |
| `crates/api/src/gap_peer_route_tests.rs:159` | BENIGN-INFALLIBLE | test-only file (#![cfg(test)] at line 1); misfiled as non-test |
| `crates/api/src/gap_peer_route_tests.rs:411` | BENIGN-INFALLIBLE | test-only file (#![cfg(test)] at line 1); misfiled as non-test |
| `crates/api/src/gap_peer_route_tests.rs:413` | BENIGN-INFALLIBLE | test-only file (#![cfg(test)] at line 1); misfiled as non-test |
| `crates/api/src/gaps_route.rs:80` | BENIGN-OPTION | Uri::query() None = no query string = empty query; params then validated |
| `crates/api/src/gaps_route.rs:404` | SURFACED | refused at gaps_route.rs:406 |
| `crates/api/src/gaps_route.rs:467` | BENIGN-INFALLIBLE | valid month day 1 / stored StatusCode / saturating |
| `crates/api/src/gaps_route.rs:533` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:536` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:573` | BENIGN-MATCH | run builder / phrasing / non-session counted into `outside` |
| `crates/api/src/gaps_route.rs:614` | SURFACED | schedule error kept as evidence_error (gaps_route.rs:617) |
| `crates/api/src/gaps_route.rs:665` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:784` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:928` | BENIGN-INFALLIBLE | valid month day 1 / stored StatusCode / saturating |
| `crates/api/src/gaps_route.rs:1005` | BENIGN-INFALLIBLE | valid month day 1 / stored StatusCode / saturating |
| `crates/api/src/gaps_route.rs:1016` | BENIGN-MATCH | run builder / phrasing / non-session counted into `outside` |
| `crates/api/src/gaps_route.rs:1090` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:1092` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:1214` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:1215` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:1216` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:1218` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:1284` | BENIGN-INFALLIBLE | valid month day 1 / stored StatusCode / saturating |
| `crates/api/src/gaps_route.rs:1415` | BENIGN-INFALLIBLE | valid month day 1 / stored StatusCode / saturating |
| `crates/api/src/gaps_route.rs:1577` | BENIGN-INFALLIBLE | valid month day 1 / stored StatusCode / saturating |
| `crates/api/src/gaps_route.rs:1609` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:1687` | BENIGN-OPTION | Option default with no failure behind it |
| `crates/api/src/gaps_route.rs:1698` | BENIGN-MATCH | run builder / phrasing / non-session counted into `outside` |
| `crates/api/src/import_journal.rs:217` | BENIGN-INFALLIBLE | position of a kind in ImportKind::ALL always found; u8::MAX unreachable |
| `crates/api/src/import_journal.rs:218` | BENIGN-INFALLIBLE | same: unreachable default |
| `crates/api/src/import_journal.rs:287` | BENIGN-INFALLIBLE | u32 index -> usize always fits; points inside runs by construction |
| `crates/api/src/import_journal.rs:297` | BENIGN-INFALLIBLE | u32 -> usize always fits |
| `crates/api/src/import_journal.rs:305` | BENIGN-OPTION | position past u32::MAX is unindexed -> None (no cursor), documented 345-347 |
| `crates/api/src/import_journal.rs:350` | SWALLOWED | runs_unindexed is counted but nothing reads it (only import_journal.rs); doc claims never silently missing |
| `crates/api/src/import_journal.rs:381` | SURFACED | unparsed line -> fold None -> foreign_count + sample (325-330), shown imports.rs:1161,1181 |
| `crates/api/src/import_journal.rs:434` | BENIGN-MATCH | verb already restricted by the pattern; `_` can only be 'refused' |
| `crates/api/src/import_journal.rs:504` | BENIGN-MATCH | unknown close field kept out of a display row, documented forward-compat (501-503) |
| `crates/api/src/import_journal.rs:521` | BENIGN-INFALLIBLE | end moved to a char boundary first; get(..end) always Some |
| `crates/api/src/import_journal.rs:527` | SURFACED | bad count -> line foreign, counted and named (325-330) |
| `crates/api/src/import_journal.rs:534` | SURFACED | bad kind/day -> line foreign, counted and named (325-330) |
| `crates/api/src/import_journal.rs:540` | SURFACED | bad year -> None -> foreign line counted; other callers refuse |
| `crates/api/src/import_journal.rs:541` | SURFACED | bad month -> None -> foreign line counted; other callers refuse |
| `crates/api/src/import_journal.rs:542` | SURFACED | bad day -> None -> foreign line counted; other callers refuse |
| `crates/api/src/import_journal.rs:543` | SURFACED | invalid date -> None -> foreign line counted; other callers refuse |
| `crates/api/src/import_journal.rs:565` | SURFACED | non-UTF-8 decoded value -> None -> foreign line counted |
| `crates/api/src/import_journal.rs:574` | SURFACED | bad hex digit -> None -> foreign line counted |
| `crates/api/src/import_journal.rs:646` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/import_journal.rs:671` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/import_journal.rs:673` | BENIGN-INFALLIBLE | last < len by rposition; get(..last) always Some |
| `crates/api/src/import_journal.rs:682` | SWALLOWED | non-UTF-8 line folded lossily; if it still parses it is an ordinary row, not counted (comment says Counted) |
| `crates/api/src/import_journal.rs:687` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/imports.rs:204` | SURFACED | bad page refused 400 by name (206) |
| `crates/api/src/imports.rs:237` | BENIGN-OPTION | a cursor without .nth means nth 0, documented 222-226 |
| `crates/api/src/imports.rs:240` | SURFACED | bad run id refused by name (242) |
| `crates/api/src/imports.rs:245` | SURFACED | bad nth refused by name (247) |
| `crates/api/src/imports.rs:271` | SURFACED | bad run refused 400 (273-275) |
| `crates/api/src/imports.rs:314` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/imports.rs:413` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/imports.rs:424` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/imports.rs:470` | BENIGN-OPTION | an outcome with no row has no newest row |
| `crates/api/src/imports.rs:794` | BENIGN-INFALLIBLE | usize -> u64 always fits |
| `crates/api/src/imports.rs:1120` | BENIGN-INFALLIBLE | end <= runs.len() by construction (1094-1116) |
| `crates/api/src/imports.rs:1137` | BENIGN-OPTION | Option<Option> cursor flatten; None = no older page |
| `crates/api/src/imports.rs:1218` | BENIGN-OPTION | absent from/to default to the journal's span, documented 1215-1216 and echoed |
| `crates/api/src/index_consistency_projection.rs:143` | BENIGN-MATCH | unknown period refused Err |
| `crates/api/src/indexmap.rs:89` | BENIGN-INFALLIBLE | usize -> u64 fits; read capped at MAX+1 then oversize refused |
| `crates/api/src/indexmap.rs:278` | BENIGN-OPTION | alias row only when the feed keys aliases |
| `crates/api/src/indexmap.rs:583` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build (577-579) |
| `crates/api/src/indexmap.rs:595` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/indexstopcandlesjson.rs:66` | BENIGN-MATCH | invalid kind/selector combination refused Err |
| `crates/api/src/indexstopcandlesjson.rs:109` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/indexstopcandlesjson.rs:229` | BENIGN-OPTION | kind=source asks no trade -> no candles; window errors propagate via transpose()? (214) |
| `crates/api/src/indexstopjson.rs:44` | BENIGN-MATCH | unknown page kind refused |
| `crates/api/src/indexstopjson.rs:47` | BENIGN-OPTION | absent offset/limit default; garbage refused by candidatejson::integer (candidatejson.rs:101-113) |
| `crates/api/src/indexstopjson.rs:91` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/indexstopjson.rs:217` | BENIGN-MATCH | unsupported kind refused |
| `crates/api/src/indexstoplaunch.rs:114` | BENIGN-OPTION | Option<Option> qualification link; None = not linked yet, rendered null |
| `crates/api/src/indexstoplaunch.rs:116` | BENIGN-OPTION | absent stage rendered null |
| `crates/api/src/indexstoplaunch.rs:121` | BENIGN-OPTION | only linked rungs listed |
| `crates/api/src/indexstoplaunch.rs:122` | BENIGN-OPTION | only linked rungs listed |
| `crates/api/src/indexstoplaunch.rs:209` | BENIGN-OPTION | stage None is checked against the selection; mismatch refused (210-221) |
| `crates/api/src/indexstoplaunch.rs:225` | BENIGN-OPTION | no prior stage = nothing to regress from |
| `crates/api/src/indexstoplaunch.rs:391` | SURFACED | digits pre-validated; overflow -> None -> callers refuse (metadata.rs:150-155,169-170) |
| `crates/api/src/indexstoplaunch_metadata.rs:175` | SURFACED | unset setting stored Null then refused 'not configured; set ENV' (22-28) |
| `crates/api/src/indexstopqualificationjson.rs:53` | BENIGN-MATCH | unknown page kind refused |
| `crates/api/src/indexstopqualificationjson.rs:58` | BENIGN-MATCH | unknown assessment period refused |
| `crates/api/src/indexstopqualificationjson.rs:61` | BENIGN-OPTION | absent offset/limit default; garbage refused by candidatejson::integer |
| `crates/api/src/indexstopqualificationjson.rs:106` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/indexstopqualificationjson.rs:381` | BENIGN-MATCH | unknown period refused |
| `crates/api/src/indexstoprankingjson.rs:74` | BENIGN-MATCH | sequence and completion must come together; otherwise refused |
| `crates/api/src/indexstoprankingjson.rs:82` | BENIGN-MATCH | unknown filter refused |
| `crates/api/src/indexstoprankingjson.rs:84` | BENIGN-OPTION | absent offset defaults to 0; garbage refused by candidatejson::integer |
| `crates/api/src/indexstoprankingjson.rs:86` | BENIGN-OPTION | absent limit defaults to 16; garbage refused; range checked 88-94 |
| `crates/api/src/indexstoprankingjson.rs:124` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/indexstopvixjson.rs:68` | BENIGN-OPTION | absent offset defaults; garbage refused by candidatejson::integer |
| `crates/api/src/indexstopvixjson.rs:69` | BENIGN-OPTION | absent limit defaults; range checked at 70-72 |
| `crates/api/src/indexstopvixjson.rs:99` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/ingest.rs:854` | BENIGN-MATCH | Display arm of a refusal; documented unreachable and still says REFUSED |
| `crates/api/src/ingest.rs:1055` | BENIGN-MATCH | unknown or repeated cash_identity refused InvalidCashIdentity |
| `crates/api/src/ingest.rs:1853` | BENIGN-INFALLIBLE | documented saturation for a clock 292e9 years out (1849-1852) |
| `crates/api/src/ingest.rs:2041` | SURFACED | poisoned autopilot status -> 503 with reason (2042-2050; IC-03) |
| `crates/api/src/ingest.rs:2105` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/ingest.rs:4724` | BENIGN-OPTION | no named expiry -> the window end's month, documented 4721-4723 |
| `crates/api/src/ingest.rs:4727` | BENIGN-OPTION | same documented default |
| `crates/api/src/ingest_refusals.rs:88` | BENIGN-INFALLIBLE | split('=').next() is always Some |
| `crates/api/src/ingest_refusals.rs:192` | BENIGN-OPTION | newest is set whenever an entry exists (187); Null unreachable |
| `crates/api/src/ingest_refusals.rs:278` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/instruments_route.rs:75` | BENIGN-OPTION | no query = empty; absent feed -> Dhan, documented default (ingest.rs:1599-1622) |
| `crates/api/src/instruments_route.rs:217` | BENIGN-OPTION | underlying with options and no spot listed under FNO, documented 215-216 |
| `crates/api/src/instruments_route.rs:239` | BENIGN-OPTION | Option<Option> vendor id; absence = feed does not list it |
| `crates/api/src/instruments_route.rs:247` | BENIGN-OPTION | no held listing for the key = nothing held |
| `crates/api/src/instruments_route.rs:400` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/ladder.rs:360` | BENIGN-OPTION | None at the store's floor month, documented 352-355 |
| `crates/api/src/ladder.rs:361` | BENIGN-OPTION | same |
| `crates/api/src/ladder.rs:362` | BENIGN-OPTION | same |
| `crates/api/src/ladder.rs:525` | BENIGN-OPTION | non-FNO segment is its own spot; a miss refuses Gate::LegFirst loudly |
| `crates/api/src/livejson.rs:131` | BENIGN-INFALLIBLE | poison read-through (release panic=abort, Cargo.toml:194); refresh errors still 503 (135) |
| `crates/api/src/livejson.rs:153` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/livejson.rs:176` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/livejson.rs:198` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/livejson.rs:203` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/logs.rs:188` | BENIGN-OPTION | documented view clamp (logs.rs:168-170); the applied limit is echoed in JSON (864) and the form |
| `crates/api/src/logs.rs:199` | SURFACED | bad run is named under ignored + Warn (206-207, 249-258) |
| `crates/api/src/logs.rs:201` | SURFACED | bad run is named under ignored + Warn (206-207, 249-258) |
| `crates/api/src/logs.rs:215` | SURFACED | bad flag named under ignored + Warn (216, 249-258) |
| `crates/api/src/logs.rs:358` | BENIGN-INFALLIBLE | store_dir errs only on an unset env and serve refuses to start without it (server.rs:19201-19206) |
| `crates/api/src/logs.rs:366` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/logs.rs:460` | BENIGN-INFALLIBLE | store_dir errs only on an unset env; serve refuses to start without it (server.rs:19201-19206) |
| `crates/api/src/logs.rs:585` | BENIGN-OPTION | store halves listed only when a store root exists |
| `crates/api/src/logs.rs:634` | BENIGN-INFALLIBLE | body is a JSON object ending in '}' |
| `crates/api/src/logs.rs:970` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/logs.rs:982` | BENIGN-INFALLIBLE | store_dir errs only on an unset env; serve refuses to start without it (server.rs:19201-19206) |
| `crates/api/src/logs.rs:1563` | BENIGN-OPTION | form echo of an absent filter |
| `crates/api/src/logs.rs:1575` | BENIGN-OPTION | form echo of an absent filter |
| `crates/api/src/logs.rs:1576` | BENIGN-OPTION | form echo of an absent filter |
| `crates/api/src/logs.rs:1615` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/logs.rs:1619` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/logs.rs:1620` | BENIGN-OPTION | pre-epoch clock -> 0 for a request-ration window only |
| `crates/api/src/logs.rs:1630` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/logs.rs:1676` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/logs.rs:1822` | BENIGN-OPTION | only the access keys a request names are kept |
| `crates/api/src/logs.rs:1924` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/logs.rs:2247` | BENIGN-MATCH | grammar: prefix is an exchange or part of the symbol |
| `crates/api/src/logs.rs:2250` | BENIGN-MATCH | grammar: segment present or part of the symbol |
| `crates/api/src/logs.rs:2253` | BENIGN-MATCH | grammar fallthrough; symbol still validated at 2257 |
| `crates/api/src/logs.rs:2257` | SURFACED | unfileable symbol -> None -> named under ignored (logs.rs:304-306) |
| `crates/api/src/logs.rs:2292` | BENIGN-MATCH | exhaustive widest-scope logic |
| `crates/api/src/logs.rs:2328` | BENIGN-OPTION | record not a GDFL kind is not about a file (log filter only) |
| `crates/api/src/logs.rs:2330` | BENIGN-OPTION | absent about -> '' -> WholeDay/Unfiled (display classification) |
| `crates/api/src/logs.rs:2346` | BENIGN-MATCH | non-swept CM resolution classified Unfiled (display) |
| `crates/api/src/logs.rs:2360` | BENIGN-MATCH | refused ticker classified Unfiled, matched by text, documented 2356-2358 |
| `crates/api/src/logs.rs:2491` | BENIGN-INFALLIBLE | Day displays YYYY-MM-DD; get(..7) always Some |
| `crates/api/src/logs.rs:2571` | BENIGN-MATCH | cursor grammar arm |
| `crates/api/src/logs.rs:2574` | SURFACED | bad cursor -> None -> cursor_refused named, no page read (2714-2722) |
| `crates/api/src/logs.rs:2698` | BENIGN-INFALLIBLE | fixed holds one slot per cursor slot by construction |
| `crates/api/src/logs.rs:2756` | BENIGN-INFALLIBLE | slot count checked == wanted (2715) |
| `crates/api/src/logs.rs:2839` | BENIGN-OPTION | streams with no head skipped; capped set when a head is unread (2829-2834) |
| `crates/api/src/logs.rs:3006` | BENIGN-INFALLIBLE | body is a JSON object ending in '}' |
| `crates/api/src/logs.rs:3080` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/logs.rs:3338` | BENIGN-OPTION | display of an absent command field |
| `crates/api/src/logs.rs:3351` | BENIGN-MATCH | other lifecycle messages ignored by design |
| `crates/api/src/logs.rs:3453` | BENIGN-INFALLIBLE | rank < len for a non-empty gap list |
| `crates/api/src/logs.rs:3495` | BENIGN-OPTION | no mtime -> None; len and inode still compared |
| `crates/api/src/logs.rs:3670` | SURFACED | this is the surfacing: absent/unreadable dirs named (D-4862) |
| `crates/api/src/logs.rs:3727` | BENIGN-OPTION | k-way merge of heads |
| `crates/api/src/logs.rs:3748` | BENIGN-OPTION | a dir with no imported stamps contributes none |
| `crates/api/src/logs.rs:3826` | BENIGN-OPTION | k-way merge of heads |
| `crates/api/src/logs.rs:3901` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/logs.rs:3947` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/logs.rs:3996` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/logs.rs:4050` | BENIGN-INFALLIBLE | last < len by rposition |
| `crates/api/src/logs.rs:4057` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/logs.rs:4067` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/logs.rs:4182` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only; feeds the capped flag |
| `crates/api/src/main.rs:45` | SURFACED | runtime failure printed and FAILED exit code returned |
| `crates/api/src/main.rs:57` | SURFACED | argument refusal printed and MISUSED exit code returned |
| `crates/api/src/main.rs:205` | BENIGN-MATCH | unknown exit code named as such, at Error |
| `crates/api/src/master.rs:129` | BENIGN-OPTION | None = vendor does not list it; callers refuse (135-137) |
| `crates/api/src/master.rs:351` | BENIGN-INFALLIBLE | byte count discarded; the error is propagated by map_err(..)? |
| `crates/api/src/master.rs:457` | BENIGN-INFALLIBLE | row width checked > widest above (446-455) |
| `crates/api/src/mastersrun.rs:135` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs |
| `crates/api/src/mastersrun.rs:180` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs |
| `crates/api/src/mastersrun.rs:381` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs; the Err is also returned |
| `crates/api/src/mastersrun.rs:382` | SURFACED | level chosen from the Result; the Err is returned to the caller |
| `crates/api/src/mastersrun.rs:390` | SURFACED | failure message plus notes emitted and returned |
| `crates/api/src/mastersrun.rs:608` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs |
| `crates/api/src/mastersrun.rs:655` | BENIGN-MATCH | filters attempted sources; any refusal still makes the call 502 (674-678) |
| `crates/api/src/mastersrun.rs:674` | SURFACED | any refusal, missing master or failed reload -> 502 |
| `crates/api/src/mastersrun.rs:686` | SURFACED | reloaded flag and notes in the body |
| `crates/api/src/mastersrun.rs:728` | SWALLOWED | metadata error other than NotFound reported as present:false bytes:0, reason dropped |
| `crates/api/src/mastersrun.rs:735` | BENIGN-OPTION | no mtime -> not newer; documented null at 741-742 |
| `crates/api/src/mastersrun.rs:745` | BENIGN-OPTION | documented null where no file or no mtime (741-742) |
| `crates/api/src/mastersrun.rs:746` | BENIGN-OPTION | pre-1970 mtime -> null, documented |
| `crates/api/src/mastersrun.rs:814` | BENIGN-MATCH | display owner label |
| `crates/api/src/mastersrun.rs:817` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/merge.rs:111` | BENIGN-OPTION | documented: None = this vendor gave no unambiguous ISIN |
| `crates/api/src/merge.rs:226` | BENIGN-OPTION | no entry = NativeLookup::Absent, a named state |
| `crates/api/src/merge.rs:377` | BENIGN-MATCH | non-reciprocal or ambiguous id -> Contested, a named state |
| `crates/api/src/merge.rs:608` | BENIGN-OPTION | flatten of Vec<Vec<String>> disputes |
| `crates/api/src/merge.rs:698` | BENIGN-OPTION | Option ISIN comparison; mismatch pushes a conflict |
| `crates/api/src/merge.rs:729` | BENIGN-MATCH | equal or absent ISIN is not a conflict |
| `crates/api/src/merge.rs:743` | BENIGN-MATCH | documented: unconfirmed candidate keeps the vendor key (733-739) |
| `crates/api/src/month_change.rs:338` | BENIGN-OPTION | None below the 1970 floor, documented 333-335 |
| `crates/api/src/nse_identity.rs:275` | BENIGN-INFALLIBLE | const usize -> u64 fits |
| `crates/api/src/operation_audit.rs:102` | BENIGN-MATCH | uncommon method labelled OTHER |
| `crates/api/src/operation_audit.rs:278` | BENIGN-MATCH | unknown query field refused |
| `crates/api/src/operation_audit.rs:314` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/optiongreeks.rs:146` | BENIGN-MATCH | unset or empty env -> Unset -> every cell refused 'rate_unset' (581) |
| `crates/api/src/optiongreeks.rs:172` | BENIGN-OPTION | no mtime -> no stamp -> table reloaded each request (correct, slower) |
| `crates/api/src/optiongreeks.rs:186` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/optiongreeks.rs:301` | SURFACED | non-option -> None -> refused 400 (374-379) |
| `crates/api/src/optiongreeks.rs:307` | SURFACED | bad strike -> None -> refused 400 (374-379) |
| `crates/api/src/optiongreeks.rs:317` | SURFACED | bad expiry -> None -> refused 400 (374-379) |
| `crates/api/src/optiongreeks.rs:523` | SURFACED | spot read error pushed to faults (206 per 832-833); cell refused |
| `crates/api/src/optiongreeks.rs:554` | BENIGN-INFALLIBLE | documented: why() is None only for At, excluded by let-else |
| `crates/api/src/optiongreeks.rs:590` | SURFACED | bad stamp -> cell refused tenor_refused |
| `crates/api/src/optiongreeks.rs:840` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/provenance.rs:221` | BENIGN-INFALLIBLE | day 1 of a valid YearMonth is always valid |
| `crates/api/src/provenance.rs:326` | BENIGN-MATCH | the remaining valid segment/contract combinations |
| `crates/api/src/provenance.rs:337` | SURFACED | bad t refused 400 |
| `crates/api/src/provenance.rs:367` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/provenance.rs:568` | BENIGN-OPTION | revision mtime unknown -> null shown; no run is attributed when served>0 (1697) |
| `crates/api/src/provenance.rs:615` | BENIGN-OPTION | display '?' for an unknown original path |
| `crates/api/src/provenance.rs:646` | BENIGN-MATCH | dispatch by feed |
| `crates/api/src/provenance.rs:695` | BENIGN-INFALLIBLE | json! object literal; as_object always Some |
| `crates/api/src/provenance.rs:754` | BENIGN-INFALLIBLE | masked to 32 bits first |
| `crates/api/src/provenance.rs:761` | BENIGN-OPTION | documented 'when the host says' (757) |
| `crates/api/src/provenance.rs:763` | BENIGN-OPTION | pre-epoch mtime -> None |
| `crates/api/src/provenance.rs:764` | BENIGN-OPTION | overflow -> None |
| `crates/api/src/provenance.rs:787` | BENIGN-MATCH | a non-present sidecar has no records; its state is shown by json() |
| `crates/api/src/provenance.rs:796` | BENIGN-MATCH | same |
| `crates/api/src/provenance.rs:947` | SURFACED | invalid day named as an error in the JSON (950-953) |
| `crates/api/src/provenance.rs:1101` | BENIGN-INFALLIBLE | usize -> u64 fits |
| `crates/api/src/provenance.rs:1208` | BENIGN-MATCH | memo continuation logic |
| `crates/api/src/provenance.rs:1213` | BENIGN-MATCH | lookup errors mapped to Err with the day |
| `crates/api/src/provenance.rs:1232` | SURFACED | lookup errors counted, shown as bar_lookup_errors (1099) |
| `crates/api/src/provenance.rs:1268` | BENIGN-OPTION | display stem; asked symbol already validated; null = not derivable |
| `crates/api/src/provenance.rs:1269` | BENIGN-OPTION | display stem; asked symbol already validated; null = not derivable |
| `crates/api/src/provenance.rs:1293` | BENIGN-OPTION | display stem; asked symbol already validated |
| `crates/api/src/provenance.rs:1294` | BENIGN-OPTION | display stem; asked symbol already validated |
| `crates/api/src/provenance.rs:1354` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/provenance.rs:1397` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/provenance.rs:1455` | SURFACED | unindexed runs counted and shown (runs_walked_on_every_request, 1714) |
| `crates/api/src/provenance.rs:1464` | BENIGN-OPTION | invalid day -> None -> run filed as unindexed (counted) |
| `crates/api/src/provenance.rs:1540` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/provenance.rs:1570` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/provenance.rs:1591` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/provenance.rs:1616` | SURFACED | undecodable record counted index.faults, shown 'undecodable' (1715) |
| `crates/api/src/provenance.rs:1702` | BENIGN-MATCH | no unique attribution -> null (conservative) |
| `crates/api/src/provenance.rs:1893` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/provenance.rs:1897` | BENIGN-OPTION | display of an out-of-calendar day as null |
| `crates/api/src/provenance.rs:1898` | BENIGN-OPTION | display of an out-of-calendar day as null |
| `crates/api/src/pullrun.rs:143` | BENIGN-MATCH | unknown route -> None, refused by name (documented 135-138) |
| `crates/api/src/pullrun.rs:175` | BENIGN-OPTION | documented: an unknown rung sorts last (166-168) |
| `crates/api/src/pullrun.rs:485` | BENIGN-OPTION | pre-epoch clock -> started_at null |
| `crates/api/src/pullrun.rs:486` | BENIGN-OPTION | overflow -> null |
| `crates/api/src/pullrun.rs:1057` | BENIGN-OPTION | no restatement prefix = a fresh open; the note is the server's own sealed text |
| `crates/api/src/pullrun.rs:1058` | BENIGN-OPTION | same |
| `crates/api/src/pullrun.rs:1385` | BENIGN-INFALLIBLE | guard requires unread.is_some() |
| `crates/api/src/pullrun.rs:1421` | BENIGN-OPTION | telemetry display of absent text |
| `crates/api/src/pullrun.rs:1637` | BENIGN-INFALLIBLE | range within len by partition_point |
| `crates/api/src/pullrun.rs:1651` | BENIGN-INFALLIBLE | range within len by partition_point |
| `crates/api/src/pullrun.rs:1703` | BENIGN-INFALLIBLE | calendar horizon day is a small positive day number |
| `crates/api/src/pullrun.rs:2018` | SWALLOWED | Unreadable vendor census silently adds 0 to rowsAtStart/rowsNow (counters() None; also recovery.rs:1045,1788) |
| `crates/api/src/pullrun.rs:2048` | SWALLOWED | same for the run's own vendors: an unreadable census reads as zero rows, not unknown |
| `crates/api/src/pullrun.rs:2098` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/pullrun.rs:2122` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/pullrun.rs:2147` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/pullrun.rs:2273` | BENIGN-MATCH | other badges fall through to further classification |
| `crates/api/src/pullrun.rs:2344` | BENIGN-MATCH | message for the halting outcomes |
| `crates/api/src/pullrun.rs:2390` | BENIGN-MATCH | default class is retry and the leg stays owed |
| `crates/api/src/pullrun.rs:2450` | SURFACED | unstated count -> None -> STORED figure becomes unknown (2454-2462) |
| `crates/api/src/pullrun.rs:2472` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/pullrun.rs:2560` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/pullrun.rs:2607` | BENIGN-OPTION | per-feed halted flags; absent = not halted |
| `crates/api/src/pullrun.rs:2720` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/pullrun.rs:2845` | BENIGN-OPTION | generic text when no first error was kept |
| `crates/api/src/pullrun.rs:2873` | BENIGN-OPTION | generic text when no first error was kept |
| `crates/api/src/recovery.rs:387` | BENIGN-MATCH | repeated or empty field refused |
| `crates/api/src/recovery.rs:419` | BENIGN-MATCH | incomplete successor pair refused |
| `crates/api/src/recovery.rs:554` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/recovery.rs:564` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/recovery.rs:581` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/recovery.rs:672` | BENIGN-OPTION | Ok-but-stopping arm named 'stopped during durable activation' |
| `crates/api/src/recovery.rs:889` | BENIGN-OPTION | first seed creates the control row; an unseeded resume is refused (882-884) |
| `crates/api/src/recovery.rs:1011` | BENIGN-INFALLIBLE | keys are taken from journal.latest (1046-1057) |
| `crates/api/src/recovery.rs:1036` | SURFACED | lifecycle error journaled 'lifecycle UNVERIFIED: why' (1021-1035); counted at 1657-1659 |
| `crates/api/src/recovery.rs:1065` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/recovery.rs:1138` | BENIGN-MATCH | every other status summed as missing/blocked in the summary |
| `crates/api/src/recovery.rs:1148` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs |
| `crates/api/src/recovery.rs:1542` | BENIGN-MATCH | closed/outside-window gaps are not owed |
| `crates/api/src/recovery.rs:1596` | BENIGN-MATCH | minute outside the audited shape refused |
| `crates/api/src/recovery.rs:1629` | SURFACED | inspect failure -> no proof -> evidence_issues+1 and retries cleared (1657-1660); reason not kept |
| `crates/api/src/recovery.rs:1721` | BENIGN-OPTION | no earlier attempt -> a fresh unit |
| `crates/api/src/recovery.rs:1887` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/recovery.rs:1888` | BENIGN-OPTION | no run in this process -> default progress; page says none is running |
| `crates/api/src/recovery.rs:1925` | BENIGN-OPTION | summary text chosen from state |
| `crates/api/src/recovery_control.rs:29` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/recovery_control.rs:38` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/recovery_journal.rs:136` | BENIGN-MATCH | unknown status code refused InvalidData |
| `crates/api/src/recovery_journal.rs:450` | SURFACED | length overflow -> io::Error refusal |
| `crates/api/src/recovery_journal.rs:750` | SURFACED | conversion failure refused InvalidData |
| `crates/api/src/render.rs:378` | BENIGN-MATCH | percent-encoder arm |
| `crates/api/src/render.rs:405` | BENIGN-MATCH | HTML escaper arm |
| `crates/api/src/render.rs:447` | BENIGN-OPTION | link override defaults to the current view |
| `crates/api/src/render.rs:448` | BENIGN-OPTION | link override defaults to the current view |
| `crates/api/src/render.rs:453` | BENIGN-OPTION | link override defaults to the current view |
| `crates/api/src/render.rs:666` | BENIGN-MATCH | no sort mark |
| `crates/api/src/render.rs:863` | BENIGN-INFALLIBLE | note longer than MAX has at least one char |
| `crates/api/src/render.rs:864` | BENIGN-OPTION | no ', ' boundary -> cut at the char boundary |
| `crates/api/src/render.rs:975` | BENIGN-OPTION | non-numeric figures excluded from the bar scale (display) |
| `crates/api/src/render.rs:977` | BENIGN-OPTION | no numeric figure -> scale 1 (display) |
| `crates/api/src/render.rs:1300` | BENIGN-INFALLIBLE | value clamped to 1..=4 first |
| `crates/api/src/render.rs:1442` | BENIGN-OPTION | documented: day itself at the epoch (1434) |
| `crates/api/src/render.rs:1443` | BENIGN-OPTION | documented: day itself at the epoch (1434) |
| `crates/api/src/render.rs:2290` | BENIGN-OPTION | form echo of an absent filter |
| `crates/api/src/render.rs:3196` | SURFACED | non-UTF-8 knob -> U+FFFD -> knob refusal shown on the page (3138-3143) |
| `crates/api/src/render.rs:3271` | SWALLOWED | read_dir entry errors dropped from the CSV-folder suggestions (convenience list only) |
| `crates/api/src/repair_route.rs:83` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/repair_route.rs:88` | SURFACED | join failure -> 500 with the reason |
| `crates/api/src/repair_route.rs:183` | BENIGN-OPTION | display '-' for an out-of-calendar stamp |
| `crates/api/src/repair_route.rs:184` | BENIGN-OPTION | display '-' for an out-of-calendar stamp |
| `crates/api/src/scrub_route.rs:411` | BENIGN-OPTION | run line without a readable pass is not indexed; its findings file under pass 0 'not told' (336-338) |
| `crates/api/src/scrub_route.rs:425` | BENIGN-MATCH | cursor and other line kinds skipped by design (400-401) |
| `crates/api/src/scrub_route.rs:435` | BENIGN-OPTION | no why= -> empty reason |
| `crates/api/src/scrub_route.rs:436` | BENIGN-OPTION | field parse helper |
| `crates/api/src/scrub_route.rs:437` | BENIGN-OPTION | unknown run -> 0 = 'not told', counted as untold (711, 773) |
| `crates/api/src/scrub_route.rs:440` | BENIGN-OPTION | block only for bad lines carrying records |
| `crates/api/src/scrub_route.rs:442` | BENIGN-OPTION | pass 0 = not told, documented 336-338 |
| `crates/api/src/scrub_route.rs:553` | BENIGN-OPTION | unknown keeper run -> 0 |
| `crates/api/src/scrub_route.rs:711` | BENIGN-OPTION | no untold findings -> 0 |
| `crates/api/src/scrub_route.rs:773` | BENIGN-OPTION | no untold busy months -> 0 |
| `crates/api/src/scrub_route.rs:829` | BENIGN-OPTION | k-way merge of heads |
| `crates/api/src/scrub_route.rs:855` | BENIGN-INFALLIBLE | end moved to a char boundary first |
| `crates/api/src/scrub_route.rs:941` | BENIGN-INFALLIBLE | usize -> u64 fits |
| `crates/api/src/scrub_route.rs:1054` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/scrub_route.rs:1071` | BENIGN-OPTION | mtime part of the stamp; identity and length also compared |
| `crates/api/src/scrub_route.rs:1137` | BENIGN-INFALLIBLE | 2000-01 is a valid month |
| `crates/api/src/scrub_route.rs:1148` | SURFACED | None -> refused 'names no store path' (1178-1179) |
| `crates/api/src/scrub_route.rs:1317` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/selectionv6json.rs:120` | BENIGN-MATCH | unknown key -> slot MAX -> refused (122-131) |
| `crates/api/src/selectionv6json.rs:141` | BENIGN-MATCH | only slot 2 reaches this arm |
| `crates/api/src/selectionv6json.rs:163` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/server.rs:117` | SURFACED | returns whether written; the caller shouts to the fallback stream (105-111) |
| `crates/api/src/server.rs:839` | BENIGN-OPTION | exact-key filter |
| `crates/api/src/server.rs:852` | BENIGN-OPTION | documented view-only default (845-849); audit_json names it (audit_json.rs:132) |
| `crates/api/src/server.rs:876` | BENIGN-MATCH | decoder arm |
| `crates/api/src/server.rs:897` | BENIGN-MATCH | non-hex -> the escape is kept literally (872-874) |
| `crates/api/src/server.rs:1094` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/server.rs:1236` | BENIGN-INFALLIBLE | day 0 is valid |
| `crates/api/src/server.rs:1237` | BENIGN-INFALLIBLE | epoch..=today is a valid window |
| `crates/api/src/server.rs:1240` | BENIGN-OPTION | invalid floor date -> no day, tested (feeds_route.rs:1089-1099); the bars path refuses (7665) |
| `crates/api/src/server.rs:1524` | BENIGN-INFALLIBLE | days inside a valid window |
| `crates/api/src/server.rs:1874` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/server.rs:2094` | BENIGN-INFALLIBLE | a budget Governor::new refuses also refuses HttpSource construction (pull/src/http.rs:534-551) |
| `crates/api/src/server.rs:2142` | SWALLOWED | poisoned budgets lock -> None -> .sharing(None) strips the source's own governor (pull http.rs:602-613) |
| `crates/api/src/server.rs:2372` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build (2361-2368) |
| `crates/api/src/server.rs:2456` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build (2361-2368) |
| `crates/api/src/server.rs:2596` | SWALLOWED | page() read failure -> None -> /pull says 'No pull has been recorded' (2857-2863) |
| `crates/api/src/server.rs:2599` | SWALLOWED | undecodable newest record -> None -> the same false 'no pull recorded' sentence |
| `crates/api/src/server.rs:2812` | BENIGN-MATCH | out-of-calendar day numbers named as such |
| `crates/api/src/server.rs:2822` | BENIGN-INFALLIBLE | targets computed per SpotTarget::ALL, same length |
| `crates/api/src/server.rs:2838` | BENIGN-MATCH | no usable window -> no capture; the sentence says so (2857-2859) |
| `crates/api/src/server.rs:2839` | BENIGN-MATCH | same |
| `crates/api/src/server.rs:2855` | BENIGN-MATCH | same |
| `crates/api/src/server.rs:3135` | BENIGN-INFALLIBLE | asked.target is a member of SpotTarget::ALL |
| `crates/api/src/server.rs:3136` | BENIGN-INFALLIBLE | targets has one slot per target |
| `crates/api/src/server.rs:3321` | BENIGN-OPTION | filter of held entries to the spot series |
| `crates/api/src/server.rs:4339` | BENIGN-MATCH | receipt sentence when nothing was refused |
| `crates/api/src/server.rs:4459` | BENIGN-INFALLIBLE | day 1 of a valid month |
| `crates/api/src/server.rs:4473` | BENIGN-OPTION | falls back to the run's window (wider, conservative) for the failure record |
| `crates/api/src/server.rs:4475` | BENIGN-OPTION | same |
| `crates/api/src/server.rs:4482` | BENIGN-MATCH | multi-month window -> None |
| `crates/api/src/server.rs:4655` | BENIGN-OPTION | bool.then(Option).flatten |
| `crates/api/src/server.rs:4713` | BENIGN-OPTION | display of a day number |
| `crates/api/src/server.rs:4714` | BENIGN-OPTION | display of a day number |
| `crates/api/src/server.rs:5248` | BENIGN-OPTION | only owed targets get a reason line |
| `crates/api/src/server.rs:5477` | BENIGN-OPTION | documented: archive feed has no master (5470-5474) |
| `crates/api/src/server.rs:5796` | BENIGN-MATCH | runtime flavour dispatch |
| `crates/api/src/server.rs:5897` | BENIGN-OPTION | banner label when no grounds |
| `crates/api/src/server.rs:5905` | BENIGN-OPTION | telemetry label when no plan in flight |
| `crates/api/src/server.rs:5962` | SURFACED | no vendor id -> refused by name (5963-5969) |
| `crates/api/src/server.rs:6037` | BENIGN-OPTION | Option comparison inside a corroboration check that refuses on false |
| `crates/api/src/server.rs:6294` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/server.rs:6368` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/server.rs:6681` | SURFACED | no venue close -> settled_today refuses 'states no close' (6766-6772) |
| `crates/api/src/server.rs:6688` | SURFACED | an unsettled today is refused with its reason (6698-6701) |
| `crates/api/src/server.rs:6700` | BENIGN-MATCH | refusal text arm |
| `crates/api/src/server.rs:6805` | BENIGN-OPTION | display of a bound as its number |
| `crates/api/src/server.rs:6806` | BENIGN-OPTION | display of a bound as its number |
| `crates/api/src/server.rs:7160` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/server.rs:7242` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/server.rs:7597` | BENIGN-OPTION | documented: 0 when no chunk carried a row (7586-7592) |
| `crates/api/src/server.rs:7741` | BENIGN-INFALLIBLE | leading <= len by construction |
| `crates/api/src/server.rs:8314` | BENIGN-MATCH | status/name only for VendorRefused |
| `crates/api/src/server.rs:8439` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/server.rs:9003` | BENIGN-OPTION | flatten drops lanes with no target (broker_lane None); Results kept |
| `crates/api/src/server.rs:9005` | SURFACED | no lane answered -> a named Err |
| `crates/api/src/server.rs:9148` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/server.rs:9155` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/server.rs:9375` | BENIGN-OPTION | the ledger's opening source is preferred, documented 9368-9371 |
| `crates/api/src/server.rs:9441` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/server.rs:9822` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/server.rs:9867` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/server.rs:9871` | BENIGN-OPTION | unusable clock -> calendar null, documented 9868-9869 |
| `crates/api/src/server.rs:9900` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/server.rs:9906` | BENIGN-MATCH | no running run -> not stopping |
| `crates/api/src/server.rs:10124` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/server.rs:10161` | BENIGN-OPTION | Option<Option> cell flatten |
| `crates/api/src/server.rs:10567` | BENIGN-INFALLIBLE | the only split refusal (zero cap) is pre-checked and refused at 10649-10661 |
| `crates/api/src/server.rs:10587` | BENIGN-INFALLIBLE | documented unreachable: owed guarantees from<=through (10581-10585) |
| `crates/api/src/server.rs:10700` | BENIGN-OPTION | rotated credential preferred when present |
| `crates/api/src/server.rs:11449` | BENIGN-OPTION | documented: expiry month else window end (11441-11445) |
| `crates/api/src/server.rs:11629` | BENIGN-INFALLIBLE | errs only for today in 1970-01 (ingest.rs:1929-1943); comment 11624-27 contradicts the None path |
| `crates/api/src/server.rs:11630` | BENIGN-INFALLIBLE | same |
| `crates/api/src/server.rs:11733` | BENIGN-OPTION | first refusal returned, else NoExpiry |
| `crates/api/src/server.rs:11971` | SWALLOWED | rate supplied but no swept slot -> greeks silently not computed; receipt gives no reason (12068-12125) |
| `crates/api/src/server.rs:12281` | BENIGN-INFALLIBLE | capacity hint only |
| `crates/api/src/server.rs:12390` | BENIGN-MATCH | documented: an unnamed row ends the group, refused on the next call (12385-12388, D-2650) |
| `crates/api/src/server.rs:12916` | SURFACED | no id -> refused by name (12919-12924) |
| `crates/api/src/server.rs:13392` | BENIGN-OPTION | rotated credential preferred when present |
| `crates/api/src/server.rs:14227` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own; page view default documented (845-849) |
| `crates/api/src/server.rs:14247` | BENIGN-INFALLIBLE | const PAGE_ROWS fits |
| `crates/api/src/server.rs:14248` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/server.rs:14250` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/server.rs:14384` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/server.rs:14601` | BENIGN-OPTION | documented: malformed narrowing shown as All; controls render the applied filter (14589-14597) |
| `crates/api/src/server.rs:14638` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/server.rs:14703` | BENIGN-MATCH | feed preferred, else vendor; disagreement refused at 14701 |
| `crates/api/src/server.rs:14823` | SURFACED | non-contract segment refused NoContractSegment |
| `crates/api/src/server.rs:14837` | BENIGN-OPTION | own is Some in this arm; note carried into the AskedUnreadable refusal |
| `crates/api/src/server.rs:14856` | SURFACED | unfileable symbol -> no paths -> refused 400 (14885-14889) |
| `crates/api/src/server.rs:14870` | SURFACED | same: an empty tried list is refused 400 |
| `crates/api/src/server.rs:14881` | SURFACED | same |
| `crates/api/src/server.rs:14975` | BENIGN-OPTION | feed/vendor disagreement is refused later by bars_vendor (15242-15250) |
| `crates/api/src/server.rs:15093` | BENIGN-OPTION | documented: an invalid symbol is returned unchanged and matches nothing (15087-15089) |
| `crates/api/src/server.rs:15356` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/server.rs:15702` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/server.rs:15818` | BENIGN-MATCH | default form bound |
| `crates/api/src/server.rs:15974` | BENIGN-OPTION | documented 'unstamped'; the unstamped gate refuses writes (main.rs:50-52) |
| `crates/api/src/server.rs:15982` | BENIGN-INFALLIBLE | 40-hex or 'unstamped' is always valid header text (15977-15979) |
| `crates/api/src/server.rs:16744` | BENIGN-OPTION | cross-origin refusal issued regardless; malformed value labelled |
| `crates/api/src/server.rs:16745` | BENIGN-OPTION | same |
| `crates/api/src/server.rs:16746` | BENIGN-OPTION | same |
| `crates/api/src/server.rs:16862` | BENIGN-OPTION | strip brackets of an IPv6 literal when present |
| `crates/api/src/server.rs:17103` | BENIGN-INFALLIBLE | the CAS outcome is the answer (slot taken or not) |
| `crates/api/src/server.rs:17251` | BENIGN-MATCH | head-terminator state machine |
| `crates/api/src/server.rs:17445` | BENIGN-INFALLIBLE | documented: a signal error still means stop (17442-17444) |
| `crates/api/src/server.rs:18455` | BENIGN-OPTION | holder stamp is detail inside a refusal that is issued regardless |
| `crates/api/src/server.rs:18456` | BENIGN-OPTION | same |
| `crates/api/src/server.rs:18493` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/server.rs:18704` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/server.rs:18725` | BENIGN-OPTION | strip one trailing newline when present |
| `crates/api/src/server.rs:19058` | BENIGN-OPTION | no cwd -> logs beneath the store, documented 19051-19054; banner prints the path |
| `crates/api/src/server.rs:19107` | BENIGN-MATCH | default log location |
| `crates/api/src/server.rs:34085` | BENIGN-OPTION | rows only for vendor-listed ids |
| `crates/api/src/server.rs:34086` | BENIGN-OPTION | same |
| `crates/api/src/server.rs:34285` | BENIGN-MATCH | verdict rendered as a bool; the Err reason is written next (34287-34289) |
| `crates/api/src/server.rs:34290` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/server.rs:34300` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/server.rs:34316` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/server.rs:34325` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/server.rs:34802` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/server.rs:34855` | BENIGN-OPTION | index symbols the feed lists |
| `crates/api/src/sources.rs:299` | BENIGN-MATCH | the valid segment/contract combinations |
| `crates/api/src/sources.rs:349` | BENIGN-OPTION | documented: None where the census cannot key the symbol |
| `crates/api/src/sources.rs:433` | SWALLOWED | any metadata error -> None, the same key as 'no file' in the DAYS memo (see 565-566) |
| `crates/api/src/sources.rs:450` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/sources.rs:515` | BENIGN-OPTION | a .bin not named YYYY-MM is not a month file |
| `crates/api/src/sources.rs:521` | BENIGN-OPTION | None past the last addressable year |
| `crates/api/src/sources.rs:523` | BENIGN-OPTION | None past the last addressable month |
| `crates/api/src/sources.rs:565` | SWALLOWED | StorePath refusal -> None -> DayKey may hit facts cached while the month was absent |
| `crates/api/src/sources.rs:566` | SWALLOWED | served_bars_path refusal (store says serving original would be a hidden fallback) -> None -> same stale key |
| `crates/api/src/sources.rs:858` | BENIGN-INFALLIBLE | u32 -> usize fits |
| `crates/api/src/sources.rs:962` | BENIGN-MATCH | documented: regular open and false, which the answer names (950-953) |
| `crates/api/src/sources.rs:987` | BENIGN-OPTION | display '?' for an out-of-calendar stamp |
| `crates/api/src/sources.rs:988` | BENIGN-OPTION | display '?' for an out-of-calendar stamp |
| `crates/api/src/sources.rs:1018` | SURFACED | bad field -> refused by name (bad()) |
| `crates/api/src/sources.rs:1022` | SURFACED | bad month -> refused by name |
| `crates/api/src/sources.rs:1031` | SURFACED | bad day -> refused by name |
| `crates/api/src/sources.rs:1035` | BENIGN-MATCH | wrong field count refused |
| `crates/api/src/sources.rs:1044` | BENIGN-OPTION | only offered feeds with a store prefix |
| `crates/api/src/sources.rs:1209` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/sources.rs:1480` | SURFACED | unparsable body -> status and why shown; rung marked sessions_unchecked (1737-1745) |
| `crates/api/src/sources.rs:1481` | BENIGN-OPTION | absent field -> null |
| `crates/api/src/sources.rs:1492` | SURFACED | calendar refusal carried with its status; default sentence when no reason |
| `crates/api/src/sources.rs:1503` | BENIGN-OPTION | entries without a date skipped |
| `crates/api/src/sources.rs:1506` | BENIGN-OPTION | missing list -> not whole (1516) |
| `crates/api/src/sources.rs:1513` | BENIGN-OPTION | missing flag read as truncated (conservative) |
| `crates/api/src/sources.rs:1522` | BENIGN-OPTION | no list -> no named days |
| `crates/api/src/sources.rs:1548` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/sources.rs:1589` | SWALLOWED | edge month open error -> span null with no reason (row already labelled uncounted) |
| `crates/api/src/sources.rs:1734` | BENIGN-OPTION | Option<Option> behind flatten |
| `crates/api/src/sources.rs:1814` | BENIGN-OPTION | display '?' for an out-of-calendar stamp |
| `crates/api/src/sources.rs:1815` | BENIGN-OPTION | display '?' for an out-of-calendar stamp |
| `crates/api/src/sources.rs:1889` | SURFACED | bad limit/offset refused 400 |
| `crates/api/src/sources.rs:1899` | BENIGN-OPTION | absent limit defaults; bounds checked 1900-1905 |
| `crates/api/src/sources.rs:1906` | BENIGN-OPTION | absent offset defaults; bound checked 1907-1912 |
| `crates/api/src/sources.rs:1920` | SURFACED | bad cursor refused 400 |
| `crates/api/src/sources.rs:1945` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/sources.rs:2405` | BENIGN-OPTION | bool.then(Option).flatten |
| `crates/api/src/sources.rs:2484` | BENIGN-MATCH | name is one of o/h/l/c |
| `crates/api/src/sources.rs:2517` | BENIGN-OPTION | factor null where none is computable; factor_of names the pair |
| `crates/api/src/sources.rs:2533` | BENIGN-MATCH | 'unverified' basis label |
| `crates/api/src/sources.rs:2658` | SURFACED | bad page refused 400 |
| `crates/api/src/sources.rs:2687` | BENIGN-INFALLIBLE | first/last lie inside the validated from..=to |
| `crates/api/src/sources.rs:2697` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/sources.rs:2854` | BENIGN-INFALLIBLE | a Day's year/month is a valid YearMonth |
| `crates/api/src/sources.rs:3021` | BENIGN-INFALLIBLE | days inside a validated span |
| `crates/api/src/sources.rs:3064` | BENIGN-OPTION | no flag record -> null; dayflags failures carried in notes (dayflags.rs:494-496; sources.rs:3095) |
| `crates/api/src/sources.rs:3119` | BENIGN-OPTION | absent field -> null |
| `crates/api/src/spot.rs:191` | BENIGN-MATCH | accessor: no close outside At |
| `crates/api/src/spot.rs:200` | BENIGN-MATCH | accessor: no stamp outside At |
| `crates/api/src/spot.rs:240` | BENIGN-OPTION | documented: None for an instant no calendar can place (235-236) |
| `crates/api/src/spot.rs:241` | BENIGN-OPTION | same |
| `crates/api/src/spot.rs:384` | BENIGN-MATCH | accessor |
| `crates/api/src/spot.rs:411` | SURFACED | bad instant refused by name |
| `crates/api/src/spot.rs:539` | SURFACED | read error pushed to faults -> 206 (675-677); answer Unreadable |
| `crates/api/src/spot.rs:683` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/store_route.rs:196` | BENIGN-OPTION | no degradation -> empty header; an unreadable census still answers 503 (815-819) |
| `crates/api/src/store_route.rs:233` | BENIGN-OPTION | no legacy drift -> empty header |
| `crates/api/src/store_route.rs:701` | BENIGN-OPTION | non-ASCII If-None-Match -> full answer served |
| `crates/api/src/store_route.rs:771` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/store_route.rs:798` | BENIGN-MATCH | unknown encoding refused 400 |
| `crates/api/src/store_route.rs:844` | BENIGN-OPTION | no census row -> empty array; the note header says so (186-189) |
| `crates/api/src/store_route.rs:940` | SURFACED | non-canonical generation -> None -> refused bad_request (1167-1176) |
| `crates/api/src/store_route.rs:941` | SURFACED | same |
| `crates/api/src/store_route.rs:949` | SURFACED | bad segment -> None -> refused bad_request (1167-1176) |
| `crates/api/src/store_route.rs:983` | BENIGN-OPTION | None past the last addressable month |
| `crates/api/src/store_route.rs:1013` | BENIGN-OPTION | the guard proves every part is named (1003-1009) |
| `crates/api/src/store_route.rs:1098` | BENIGN-INFALLIBLE | HashSet::insert returns a bool |
| `crates/api/src/sweepevidence.rs:47` | BENIGN-MATCH | unknown kind refused |
| `crates/api/src/sweepevidence.rs:76` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/sweeprun.rs:215` | BENIGN-OPTION | a running search measures to now |
| `crates/api/src/sweeprun.rs:217` | BENIGN-OPTION | negative or overflowing elapsed -> null |
| `crates/api/src/sweeprun.rs:326` | BENIGN-MATCH | the remaining refusal variants -> 400 |
| `crates/api/src/sweeprun.rs:661` | BENIGN-MATCH | field accessor; unknown name None |
| `crates/api/src/sweeprun.rs:688` | BENIGN-MATCH | field accessor; unknown name None |
| `crates/api/src/sweeprun.rs:1002` | BENIGN-MATCH | field accessor; unknown name None |
| `crates/api/src/sweeprun.rs:1067` | SURFACED | missing or non-numeric field refused Malformed |
| `crates/api/src/sweeprun.rs:1129` | BENIGN-INFALLIBLE | month validated 1..=12 above |
| `crates/api/src/sweeprun.rs:1281` | SURFACED | bad max_points refused Malformed |
| `crates/api/src/sweeprun.rs:1297` | SURFACED | bad top refused Malformed |
| `crates/api/src/sweeprun.rs:1459` | BENIGN-MATCH | unknown completion outcome audited as Failed (conservative) |
| `crates/api/src/sweeprun.rs:1471` | BENIGN-OPTION | text when no report existed |
| `crates/api/src/sweeprun.rs:1480` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/sweeprun.rs:1503` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/sweeprun.rs:1685` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/sweeprun.rs:1824` | BENIGN-OPTION | documented: pre-epoch clock -> 0, page shows 'not stamped' (1815-1819) |
| `crates/api/src/sweeprun.rs:1825` | BENIGN-OPTION | same |
| `crates/api/src/sweeprun.rs:1826` | BENIGN-OPTION | same |
| `crates/api/src/sweeprun.rs:1935` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/sweeprun.rs:1939` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/sweeprun.rs:1949` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/sweeprun.rs:2069` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs; the refusal is returned |
| `crates/api/src/sweeprun.rs:2085` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs; the refusal is returned |
| `crates/api/src/sweeprun.rs:2108` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs; Busy is returned |
| `crates/api/src/sweeprun.rs:2144` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs |
| `crates/api/src/sweeprun.rs:2237` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs; the refusal is returned |
| `crates/api/src/sweeprun.rs:2291` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs |
| `crates/api/src/sweeprun.rs:2345` | BENIGN-INFALLIBLE | poison read-through; release panic=abort (Cargo.toml:194) means no poison in the shipped build |
| `crates/api/src/sweeprun.rs:2472` | BENIGN-OPTION | running -> started stamp |
| `crates/api/src/sweeprun.rs:2500` | BENIGN-MATCH | only a quiet log with no local run can claim 'no sweep has run' |
| `crates/api/src/sweeprun.rs:2502` | BENIGN-OPTION | otherwise the observed status |
| `crates/api/src/sweeprun.rs:2571` | BENIGN-MATCH | record naming no command -> None |
| `crates/api/src/sweeprun.rs:2642` | BENIGN-MATCH | other lifecycle messages ignored |
| `crates/api/src/sweeprun.rs:2686` | BENIGN-MATCH | results ledger present or unreadable -> no 'no sweep' claim (conservative) |
| `crates/api/src/sweeprun.rs:2904` | BENIGN-MATCH | documented: a malformed marker is retained so status becomes unknown (2901-2903) |
| `crates/api/src/sweeprun.rs:2974` | BENIGN-OPTION | documented: no newer activity -> the marker (2966-2970) |
| `crates/api/src/sweeprun.rs:2977` | BENIGN-MATCH | phase unknown |
| `crates/api/src/sweeprun.rs:2992` | BENIGN-MATCH | status 'unknown' with reason: silence is not completion |
| `crates/api/src/sweeprun.rs:3043` | BENIGN-MATCH | not an unterminated durable sweep marker |
| `crates/api/src/sweeprun.rs:3074` | BENIGN-MATCH | audit read failure -> not proven ended -> not admitted (documented 3062-3065) |
| `crates/api/src/sweeprun.rs:3296` | SURFACED | missing or bad field refused Malformed |
| `crates/api/src/sweeprun.rs:3503` | BENIGN-MATCH | other words refused by command_from_wire |
| `crates/api/src/sweeprun.rs:3724` | BENIGN-MATCH | commands that need no prepared launch |
| `crates/api/src/sweeprun.rs:3758` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs |
| `crates/api/src/sweeprun.rs:3783` | BENIGN-MATCH | ordinary command dispatch |
| `crates/api/src/sweeprun.rs:3842` | BENIGN-OPTION | text when no report existed |
| `crates/api/src/sweeprun.rs:3886` | SURFACED | emit result: a drop is kept in Sink::health (telemetry sink.rs:222), shown on /logs |
| `crates/api/src/sweeprun.rs:3937` | BENIGN-MATCH | strict configuration for other commands refused |
| `crates/api/src/topjson.rs:78` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/topjson.rs:122` | BENIGN-MATCH | one of feed/underlying without the other refused |
| `crates/api/src/trades.rs:60` | BENIGN-OPTION | no query string = empty query; each param is parsed or refused on its own |
| `crates/api/src/trades.rs:188` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/trades.rs:205` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/trades.rs:237` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/trades.rs:256` | BENIGN-INFALLIBLE | conversion saturates on an unreachable overflow; count/duration only |
| `crates/api/src/trades.rs:296` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/trades.rs:301` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/trades.rs:372` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/trades.rs:408` | BENIGN-INFALLIBLE | fmt::Write into a String is infallible |
| `crates/api/src/trades.rs:528` | BENIGN-MATCH | non-hex digit -> None -> identity refused |
| `crates/api/src/verification_route_tests.rs:90` | BENIGN-INFALLIBLE | test-only file (#![cfg(test)] at line 1): fixture cleanup |
| `crates/cli/src/admission_join.rs:486` | BENIGN-INFALLIBLE | nibble index <16; display only |
| `crates/cli/src/admission_join.rs:492` | BENIGN-INFALLIBLE | nibble index <16; display only |
| `crates/cli/src/admission_store.rs:1653` | BENIGN-INFALLIBLE | fixed header array slice of 4; fallback 0 would be refused by version check 1655 |
| `crates/cli/src/alerts_command.rs:64` | SURFACED | bad ordinal => named refusal 66 |
| `crates/cli/src/alerts_command.rs:87` | BENIGN-OPTION | display-only date; u64 secs/86400 fits u32 for any real clock |
| `crates/cli/src/alerts_command.rs:359` | SURFACED | refusal printed "ALERTS REFUSED: {why}" and FAILED exit |
| `crates/cli/src/all_rung_population_v5.rs:181` | BENIGN-MATCH | index outside 0..8 => None; caller must handle (rung index lookup) |
| `crates/cli/src/all_rung_population_v5.rs:1106` | BENIGN-MATCH | family topology violation => Err |
| `crates/cli/src/all_rung_selection_v5.rs:377` | SURFACED | conversion failure => "rank overflowed" refusal 379 |
| `crates/cli/src/all_rung_selection_v5.rs:583` | BENIGN-INFALLIBLE | error propagated by `?` at 591; only Ok value discarded |
| `crates/cli/src/all_rung_selection_v5.rs:791` | SURFACED | conversion failure => "winner rank overflowed" refusal 793 |
| `crates/cli/src/all_rung_selection_v6.rs:84` | SURFACED | conversion failure => None != Some => topology refused 86 |
| `crates/cli/src/anchored_search_lineage_v2.rs:148` | BENIGN-MATCH | unknown family tag => Err |
| `crates/cli/src/anchored_search_lineage_v2.rs:727` | BENIGN-MATCH | unreceipted trailing members => Err |
| `crates/cli/src/anchored_search_lineage_v2.rs:1708` | BENIGN-INFALLIBLE | nibble index <16 into 16-byte table; display only |
| `crates/cli/src/anchored_search_lineage_v2.rs:1711` | BENIGN-INFALLIBLE | nibble index <16 into 16-byte table; display only |
| `crates/cli/src/anchored_search_lineage_v3.rs:149` | BENIGN-MATCH | unknown family tag => Err |
| `crates/cli/src/anchored_search_lineage_v3.rs:593` | BENIGN-INFALLIBLE | cfg(not(unix)) only; not compiled on Linux |
| `crates/cli/src/anchored_search_lineage_v3.rs:782` | BENIGN-MATCH | unreceipted trailing members => Err |
| `crates/cli/src/anchored_search_lineage_v3.rs:1803` | BENIGN-INFALLIBLE | nibble index <16; display only |
| `crates/cli/src/anchored_search_lineage_v3.rs:1806` | BENIGN-INFALLIBLE | nibble index <16; display only |
| `crates/cli/src/anchored_search_lineage_v4.rs:148` | BENIGN-MATCH | unknown family tag => Err |
| `crates/cli/src/anchored_search_lineage_v4.rs:648` | BENIGN-INFALLIBLE | cfg(not(unix)) only; not compiled on Linux |
| `crates/cli/src/anchored_search_lineage_v4.rs:873` | BENIGN-MATCH | unreceipted trailing members => Err |
| `crates/cli/src/anchored_search_lineage_v4.rs:1924` | BENIGN-INFALLIBLE | nibble index <16; display only |
| `crates/cli/src/anchored_search_lineage_v4.rs:1927` | BENIGN-INFALLIBLE | nibble index <16; display only |
| `crates/cli/src/and_checkpoint.rs:273` | BENIGN-MATCH | unsupported entry => Err |
| `crates/cli/src/and_checkpoint.rs:559` | BENIGN-OPTION | flatten of Vec<Vec<Piece>> (levels), not Results; overflow refused 564 |
| `crates/cli/src/and_checkpoint.rs:615` | SURFACED | conversion failure => None != Some => refused 618 |
| `crates/cli/src/and_checkpoint.rs:616` | SURFACED | conversion failure => None != Some => refused 618 |
| `crates/cli/src/and_checkpoint.rs:637` | BENIGN-INFALLIBLE | at<=chunk.len() by construction (load resets at=CHUNK_HEADER after header check) |
| `crates/cli/src/audited_range_command.rs:155` | SURFACED | bad bound => "{name} must be a positive u64" refusal 157 |
| `crates/cli/src/batch.rs:207` | BENIGN-INFALLIBLE | usize->u64 on 64-bit target |
| `crates/cli/src/batch.rs:331` | BENIGN-INFALLIBLE | usize->u64 on 64-bit target |
| `crates/cli/src/batch.rs:788` | SURFACED | Attempt::level remembers failure (sweep_evidence.rs:622,719); attempt.check() refuses filing at batch.rs:864 |
| `crates/cli/src/batch.rs:799` | BENIGN-INFALLIBLE | u64->usize on 64-bit target |
| `crates/cli/src/batch.rs:814` | BENIGN-INFALLIBLE | usize->u64 telemetry field, 64-bit |
| `crates/cli/src/batch.rs:815` | BENIGN-INFALLIBLE | usize->u64 telemetry field, 64-bit |
| `crates/cli/src/boolean_admission_reader.rs:129` | BENIGN-OPTION | filter_map over Option romano field (rows without Romano family); not Results |
| `crates/cli/src/boolean_campaign.rs:240` | BENIGN-OPTION | flatten of Vec<Vec<Fingerprint>>; every fingerprint checked, errors propagate via ? |
| `crates/cli/src/boolean_campaign_codec.rs:149` | BENIGN-OPTION | None completion encoded zeros; decode 221 maps back, 296 refuses Some(zeros) |
| `crates/cli/src/boolean_campaign_codec.rs:162` | BENIGN-MATCH | unknown magic => Err |
| `crates/cli/src/boolean_campaign_codec.rs:375` | BENIGN-OPTION | None link encoded zeros; decode 431-437 maps back / refuses partial |
| `crates/cli/src/boolean_campaign_codec.rs:390` | BENIGN-MATCH | invalid status => Err |
| `crates/cli/src/boolean_campaign_codec.rs:437` | BENIGN-MATCH | partial link => Err |
| `crates/cli/src/boolean_candidate_grid.rs:260` | BENIGN-MATCH | unknown encoding => Err |
| `crates/cli/src/boolean_candidate_grid.rs:265` | BENIGN-MATCH | unknown side => Err |
| `crates/cli/src/boolean_candidate_grid.rs:280` | BENIGN-MATCH | unknown selector => Err |
| `crates/cli/src/boolean_candidate_grid.rs:289` | BENIGN-MATCH | unknown forced-stop => Err |
| `crates/cli/src/boolean_candidate_reader.rs:301` | BENIGN-MATCH | reordered/incomplete sequence => Err |
| `crates/cli/src/boolean_candidate_reader.rs:333` | BENIGN-MATCH | invalid side => Err |
| `crates/cli/src/boolean_candidate_reader.rs:517` | BENIGN-MATCH | half TTP => Err |
| `crates/cli/src/boolean_candidate_v1.rs:205` | BENIGN-MATCH | capability mismatch => Err |
| `crates/cli/src/boolean_candidate_v1.rs:1037` | SURFACED | Option<Option> flatten; None refused "trade day was not accepted" 1038 |
| `crates/cli/src/boolean_catalog_command.rs:84` | SURFACED | bad MAX_POINTS => named refusal 86 |
| `crates/cli/src/boolean_catalog_command.rs:149` | BENIGN-INFALLIBLE | split().next() is always Some |
| `crates/cli/src/boolean_catalog_command.rs:195` | SURFACED | render skips Err rows, but collect_families refuses any Err at boolean_catalog_command.rs:225-231 |
| `crates/cli/src/boolean_catalog_prepared.rs:225` | SURFACED | Err rows skipped only for require_current; collect_families 229 refuses them (boolean_catalog_command.rs:228) |
| `crates/cli/src/boolean_grammar_batch.rs:194` | SURFACED | .ok() feeds ok_or_else("grammar batch field is missing") 195 |
| `crates/cli/src/boolean_grammar_campaign.rs:102` | SURFACED | bad count => "positive integer required" refusal 104 |
| `crates/cli/src/boolean_grammar_campaign.rs:429` | BENIGN-MATCH | unknown record => Err |
| `crates/cli/src/boolean_grammar_campaign.rs:486` | BENIGN-MATCH | invalid predecessor => Err |
| `crates/cli/src/boolean_grammar_campaign.rs:516` | SURFACED | .ok() feeds ok_or_else("grammar checkpoint field missing") 517 |
| `crates/cli/src/boolean_observation_file.rs:115` | SURFACED | overflow => named refusal 116 |
| `crates/cli/src/boolean_oos_command.rs:74` | SURFACED | bad MAX_POINTS => refusal 76 |
| `crates/cli/src/boolean_qualification_plan.rs:307` | BENIGN-MATCH | unknown version magic => Err |
| `crates/cli/src/boolean_qualification_plan.rs:620` | BENIGN-MATCH | version is computed 1\|2\|3 at 610-616; arm is 3 |
| `crates/cli/src/boolean_qualification_v1.rs:250` | SURFACED | overflow/conversion => "consistency civil span overflow" refusal 251 |
| `crates/cli/src/boolean_qualification_wire.rs:72` | BENIGN-OPTION | None shared digest encoded zeros; decode 211 maps back; 424 cross-checks |
| `crates/cli/src/boolean_qualification_wire.rs:73` | BENIGN-INFALLIBLE | flatten of fixed [[u64;5];2] array, not Results |
| `crates/cli/src/boolean_qualification_wire.rs:115` | BENIGN-MATCH | wrong proof width => Err |
| `crates/cli/src/boolean_qualification_wire.rs:291` | BENIGN-MATCH | bad tag/padding => Err |
| `crates/cli/src/boolean_qualified_command.rs:68` | SURFACED | bad MAX_POINTS => refusal 70 |
| `crates/cli/src/boolean_qualified_journal.rs:402` | BENIGN-OPTION | None predecessor encoded (0,zeros); decode 446-450 maps back/refuses |
| `crates/cli/src/boolean_qualified_journal.rs:414` | BENIGN-OPTION | incomplete slot encoded as zero link; decode 467-471 refuses half link |
| `crates/cli/src/boolean_qualified_journal.rs:449` | BENIGN-MATCH | invalid predecessor => Err |
| `crates/cli/src/boolean_qualified_journal.rs:463` | BENIGN-MATCH | invalid state tag => Err |
| `crates/cli/src/boolean_qualified_journal.rs:470` | BENIGN-MATCH | half completion => Err |
| `crates/cli/src/boolean_qualified_journal.rs:531` | BENIGN-MATCH | unknown format => Err |
| `crates/cli/src/boolean_qualified_journal.rs:536` | SURFACED | .ok() feeds ok_or_else("qualified campaign field missing") 537 |
| `crates/cli/src/boolean_rung_scope.rs:67` | BENIGN-OPTION | filter_map selects scoped rung labels; not Results |
| `crates/cli/src/boolean_search_command.rs:48` | BENIGN-MATCH | no trailing timeframe list => ALL scope; other arities refused by the slice pattern at 50 |
| `crates/cli/src/boolean_search_command.rs:118` | SURFACED | non-canonical allowance => named refusal 120 |
| `crates/cli/src/boolean_search_record.rs:62` | BENIGN-MATCH | unknown magic => Err |
| `crates/cli/src/boolean_search_record.rs:71` | BENIGN-MATCH | unknown magic => Err |
| `crates/cli/src/boolean_search_record.rs:79` | BENIGN-MATCH | const width table; V1/V2 have no extra spec bytes |
| `crates/cli/src/boolean_search_record.rs:356` | BENIGN-OPTION | None predecessor encoded (0,zeros); decode maps back and refuses partial 410-426 |
| `crates/cli/src/boolean_search_record.rs:412` | BENIGN-MATCH | partial predecessor => Err |
| `crates/cli/src/boolean_search_record.rs:426` | BENIGN-MATCH | absent pin => Err |
| `crates/cli/src/boolean_search_record.rs:519` | SURFACED | .ok() feeds ok_or_else("search field is truncated") 520 |
| `crates/cli/src/boolean_statistics_reader.rs:497` | BENIGN-MATCH | invalid side => Err |
| `crates/cli/src/boolean_statistics_reader.rs:575` | BENIGN-MATCH | invalid flag => Err |
| `crates/cli/src/boolean_statistics_reader.rs:650` | BENIGN-OPTION | ranks all Some iff romano available (508, 626, unique-rank check 634); else all None |
| `crates/cli/src/candidate_trades.rs:1153` | BENIGN-OPTION | absent cell must reconcile to zero trades (strict check refuses otherwise 1166) |
| `crates/cli/src/candidate_trades/codec.rs:66` | BENIGN-MATCH | unknown flag => Err |
| `crates/cli/src/candidate_trades/codec.rs:102` | BENIGN-MATCH | unknown direction => Err |
| `crates/cli/src/candidate_trades/codec.rs:141` | BENIGN-OPTION | presence flag written at 140 before the placeholder value |
| `crates/cli/src/candidate_trades/codec.rs:143` | BENIGN-OPTION | presence flag written at 142 before the placeholder value |
| `crates/cli/src/candidate_trades/codec.rs:223` | BENIGN-MATCH | half TTP => Err |
| `crates/cli/src/candidate_trades/codec.rs:273` | BENIGN-OPTION | presence flag written at 272 before default cell bytes |
| `crates/cli/src/candidate_universe.rs:2053` | BENIGN-OPTION | ttp None encoded as explicit NONE_U32 sentinel pair; decode validates 5868-5870 |
| `crates/cli/src/candidate_universe.rs:4375` | BENIGN-INFALLIBLE | const usize->u64 on 64-bit target |
| `crates/cli/src/candidate_universe.rs:4379` | BENIGN-INFALLIBLE | const table len usize->u64 on 64-bit target |
| `crates/cli/src/candidate_universe.rs:4897` | BENIGN-MATCH | unknown rung => Err |
| `crates/cli/src/candidate_universe.rs:5372` | BENIGN-INFALLIBLE | usize->u64 on 64-bit target |
| `crates/cli/src/candidate_universe.rs:5762` | BENIGN-OPTION | ttp None hashed as explicit NONE sentinel pair (identity framing) |
| `crates/cli/src/candidate_universe.rs:5875` | BENIGN-OPTION | inside is_some_and branch => always Some; only formats an error message |
| `crates/cli/src/candidate_universe.rs:5881` | BENIGN-OPTION | inside is_some_and branch => always Some; only formats an error message |
| `crates/cli/src/candidate_universe.rs:5953` | BENIGN-MATCH | unknown byte => Err |
| `crates/cli/src/candidate_universe.rs:5970` | BENIGN-MATCH | unknown byte => Err |
| `crates/cli/src/candidate_universe.rs:5997` | BENIGN-MATCH | non-0/1 bool byte => Err |
| `crates/cli/src/candidate_universe.rs:6607` | BENIGN-INFALLIBLE | cfg(not(unix)) only; Linux uses dev/ino/ctime generation 6599 |
| `crates/cli/src/census_recount.rs:47` | SURFACED | digit-checked parse; None refused at 55 |
| `crates/cli/src/census_recount.rs:48` | SURFACED | flatten of same Option; None refused at 55 |
| `crates/cli/src/census_recount.rs:53` | BENIGN-MATCH | wrong part count => None => refused 55 |
| `crates/cli/src/census_recount.rs:56` | BENIGN-INFALLIBLE | 2-digit u16 fits u8; Day::new refuses 0 |
| `crates/cli/src/census_recount.rs:167` | BENIGN-MATCH | bad argv => refused + MISUSED |
| `crates/cli/src/census_recount.rs:334` | BENIGN-MATCH | display: no closed days => no range suffix (count printed) |
| `crates/cli/src/columns.rs:109` | BENIGN-OPTION | short refusal rows have fewer cells by design (doc 100-101); width only |
| `crates/cli/src/columns.rs:112` | BENIGN-OPTION | no rows => width 0, display only |
| `crates/cli/src/columns.rs:171` | BENIGN-INFALLIBLE | header row always pushed at 166; next() is Some |
| `crates/cli/src/execution_capability.rs:2364` | BENIGN-INFALLIBLE | const usize->u32 conversion; constant small; mismatch refused 2365 |
| `crates/cli/src/execution_capability.rs:2368` | BENIGN-INFALLIBLE | stride constant ->u32; mismatch refused 2369-2372 |
| `crates/cli/src/execution_capability.rs:2571` | SURFACED | .ok() feeds ok_or_else refusal "payload is absent" at 2572; raw is fixed array anyway |
| `crates/cli/src/execution_capability.rs:2670` | SURFACED | .ok() feeds ok_or_else refusal "short fixed record" at 2671 |
| `crates/cli/src/execution_capability.rs:2733` | BENIGN-INFALLIBLE | validation-only call; error propagated by `?`, only Ok value discarded |
| `crates/cli/src/execution_capability.rs:2801` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_capability.rs:2816` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_capability.rs:2873` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_capability.rs:3474` | BENIGN-MATCH | noncanonical tag/seconds => Err |
| `crates/cli/src/execution_capability.rs:3491` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_capability.rs:3513` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_capability.rs:3530` | BENIGN-MATCH | noncanonical forced-stop => Err |
| `crates/cli/src/execution_disposition_v2.rs:120` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_disposition_v2.rs:811` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_disposition_v2.rs:1755` | BENIGN-INFALLIBLE | validation-only; error propagated by `?` |
| `crates/cli/src/execution_disposition_v2.rs:1801` | BENIGN-MATCH | half TTP coordinate => Err |
| `crates/cli/src/execution_disposition_v2.rs:3265` | SURFACED | .ok() feeds ok_or_else refusal at 3266 |
| `crates/cli/src/execution_disposition_v2.rs:3355` | SURFACED | .ok() feeds ok_or_else refusal "short fixed record" 3356 |
| `crates/cli/src/execution_v3.rs:425` | BENIGN-MATCH | unknown forced-stop tag arm returns Err (refusal) |
| `crates/cli/src/execution_v3.rs:2219` | BENIGN-OPTION | no trailing partial block => empty prefix at current record counts; prefix then verified |
| `crates/cli/src/execution_v3.rs:3749` | BENIGN-INFALLIBLE | cfg(not(unix)) only; not compiled on Linux target |
| `crates/cli/src/execution_v3.rs:3763` | BENIGN-INFALLIBLE | cfg(not(unix)) only; not compiled on Linux target |
| `crates/cli/src/execution_v3.rs:3881` | BENIGN-INFALLIBLE | cfg(not(unix)) unused-variable suppressor |
| `crates/cli/src/execution_v3.rs:3936` | BENIGN-INFALLIBLE | mtime always available on Linux; one of several change guards (len, dev/ino, double hash) |
| `crates/cli/src/execution_v3.rs:3947` | BENIGN-INFALLIBLE | same as 3936 |
| `crates/cli/src/execution_v3.rs:3966` | BENIGN-INFALLIBLE | same as 3936; hash first!=second also refuses |
| `crates/cli/src/execution_v3.rs:4121` | BENIGN-OPTION | None encoded as explicit OPTIONAL_U32_NONE sentinel |
| `crates/cli/src/execution_v3.rs:4132` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_v3.rs:4140` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_v3.rs:4150` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_v3.rs:4160` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_v3.rs:4169` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_v3.rs:4238` | BENIGN-MATCH | hex_digit of a nibble; display only |
| `crates/cli/src/execution_v4.rs:451` | BENIGN-MATCH | unknown forced-stop tag arm returns Err (refusal) |
| `crates/cli/src/execution_v4.rs:2750` | BENIGN-OPTION | no trailing partial block => empty prefix at current record counts; prefix then verified 2758-2759 |
| `crates/cli/src/execution_v4.rs:3893` | BENIGN-OPTION | flatten over [Option;4] counts expected topology slots; not Results |
| `crates/cli/src/execution_v4.rs:3929` | BENIGN-MATCH | incomplete-parameter-block classification; incomplete case refused/handled at 3931- |
| `crates/cli/src/execution_v4.rs:4651` | BENIGN-INFALLIBLE | cfg(not(unix)) only; not compiled on Linux target; dev/ino used instead |
| `crates/cli/src/execution_v4.rs:4665` | BENIGN-INFALLIBLE | cfg(not(unix)) only; not compiled on Linux target |
| `crates/cli/src/execution_v4.rs:4783` | BENIGN-INFALLIBLE | cfg(not(unix)) unused-variable suppressor |
| `crates/cli/src/execution_v4.rs:4838` | BENIGN-INFALLIBLE | mtime always available on Linux; one of several change guards (len, dev/ino, double hash) |
| `crates/cli/src/execution_v4.rs:4849` | BENIGN-INFALLIBLE | same as 4838; len/identity/hash checks refuse independently |
| `crates/cli/src/execution_v4.rs:4868` | BENIGN-INFALLIBLE | same as 4838; first!=second hash also refuses |
| `crates/cli/src/execution_v4.rs:5055` | BENIGN-OPTION | None encoded as explicit OPTIONAL_U32_NONE sentinel; decode_optional_u32 inverts |
| `crates/cli/src/execution_v4.rs:5066` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_v4.rs:5074` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_v4.rs:5084` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_v4.rs:5094` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_v4.rs:5104` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_v4.rs:5113` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/execution_v4.rs:5182` | BENIGN-MATCH | hex_digit of a nibble; arm unreachable for 0..=15 callers (display only) |
| `crates/cli/src/expression.rs:366` | SURFACED | conversion None => "expression evidence size overflow" refusal 367 |
| `crates/cli/src/expression.rs:481` | SURFACED | .ok() feeds ok_or_else refusal 482 |
| `crates/cli/src/expression.rs:626` | BENIGN-MATCH | invalid truth byte => Err |
| `crates/cli/src/expression_pricing.rs:21` | SURFACED | bad HORIZON => refusal 23 |
| `crates/cli/src/expression_pricing.rs:26` | SURFACED | bad GRID_RUNGS => refusal 28 |
| `crates/cli/src/expression_pricing.rs:31` | SURFACED | bad GRID_STEP_PPM => refusal 33 |
| `crates/cli/src/expression_pricing.rs:35` | SURFACED | conversion None => is_none => grid admission refusal 40 |
| `crates/cli/src/expression_search.rs:52` | SURFACED | parse failure => "{name} must be a positive integer" refusal 54 |
| `crates/cli/src/expression_search.rs:59` | SURFACED | parse failure => "MONTH must be 1..=12" refusal 61 |
| `crates/cli/src/expression_search.rs:406` | BENIGN-OPTION | absent last candidate encoded as zeros + presence flag; decode checks 496-501 |
| `crates/cli/src/expression_search.rs:450` | SURFACED | .ok() feeds ok_or("cursor width mismatch")? 451 |
| `crates/cli/src/expression_search.rs:458` | SURFACED | .ok() feeds ok_or("checkpoint number missing")? 460 |
| `crates/cli/src/expression_search.rs:471` | SURFACED | .ok() feeds ok_or("previous checkpoint seal missing")? 472 |
| `crates/cli/src/expression_search.rs:476` | SURFACED | .ok() feeds ok_or("candidate identity missing")? 477 |
| `crates/cli/src/fixed_tail.rs:222` | BENIGN-INFALLIBLE | poison recovered with into_inner so failed-barrier set is still recorded (fail-closed) |
| `crates/cli/src/fixed_tail.rs:234` | BENIGN-INFALLIBLE | poison recovered; refusal still enforced 237 |
| `crates/cli/src/fixed_tail.rs:376` | BENIGN-OPTION | empty file / complete header / non-usize len => no torn header to heal (Ok(None)) |
| `crates/cli/src/fold_audit.rs:241` | BENIGN-INFALLIBLE | usize->u64 on 64-bit target |
| `crates/cli/src/fold_audit.rs:256` | BENIGN-INFALLIBLE | usize->u64 on 64-bit target |
| `crates/cli/src/fold_audit.rs:280` | BENIGN-MATCH | pairing arm of the merge walk; field disagreements recorded |
| `crates/cli/src/fold_audit.rs:326` | BENIGN-INFALLIBLE | usize->u64 on 64-bit target |
| `crates/cli/src/fold_audit.rs:327` | BENIGN-INFALLIBLE | usize->u64 on 64-bit target |
| `crates/cli/src/fold_audit.rs:330` | BENIGN-INFALLIBLE | usize->u64 on 64-bit target |
| `crates/cli/src/fold_audit.rs:353` | BENIGN-MATCH | find_map predicate; ordered pair => None, unordered => refused 356 |
| `crates/cli/src/fold_audit.rs:423` | BENIGN-INFALLIBLE | u64 record count -> usize on 64-bit; 0 only a reserve hint |
| `crates/cli/src/frontier.rs:474` | BENIGN-INFALLIBLE | i32 saturation of fill-headroom; value range-validated before encode (D-1623, comment 464-465) |
| `crates/cli/src/frontier.rs:493` | BENIGN-INFALLIBLE | usize->u64 cannot fail on 64-bit target; saturation documented 488-490 |
| `crates/cli/src/frontier.rs:543` | BENIGN-INFALLIBLE | take(2) of fixed &[u8;STRIDE_BYTES] yields exactly 2 bytes; try_into cannot fail |
| `crates/cli/src/frontier.rs:548` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail (take panics, never shortens) |
| `crates/cli/src/frontier.rs:549` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:550` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:551` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:552` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:553` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:554` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:555` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:556` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:557` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:558` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:559` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:560` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:561` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:563` | BENIGN-INFALLIBLE | take(1).first() on fixed array is Some; unknown byte refused at 567-571 |
| `crates/cli/src/frontier.rs:577` | BENIGN-INFALLIBLE | take(1).first() on fixed array is Some; byte >1 refused at 581 |
| `crates/cli/src/frontier.rs:597` | BENIGN-INFALLIBLE | take(4) of fixed-size row array; negative headroom refused 598 |
| `crates/cli/src/frontier.rs:606` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:607` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:608` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:609` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:610` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:611` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:615` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:623` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:625` | BENIGN-INFALLIBLE | u64->usize cannot fail on 64-bit; saturation mirrors write 620-621 |
| `crates/cli/src/frontier.rs:630` | BENIGN-INFALLIBLE | take(8) of fixed-size row array; try_into cannot fail |
| `crates/cli/src/frontier.rs:728` | BENIGN-OPTION | unpriced row has no side; falls back to evidence side, documented 718-726 (trades==0 tells apart) |
| `crates/cli/src/frontier.rs:1639` | SURFACED | first schema damage kept in `damaged`; refused at 1651 (require_parent) else returned 1668 to of_run caller |
| `crates/cli/src/frontier.rs:1650` | BENIGN-INFALLIBLE | usize->u64 cannot fail on 64-bit |
| `crates/cli/src/frontier.rs:1719` | BENIGN-OPTION | HashMap capacity hint only; no semantic effect |
| `crates/cli/src/frontier.rs:1772` | BENIGN-INFALLIBLE | raw is &[u8;STRIDE_BYTES]; get(..32) is always Some |
| `crates/cli/src/frontier.rs:1858` | BENIGN-INFALLIBLE | header is [u8;HEADER_BYTES]; get(8..12) always 4 bytes; 0 would be refused by version check 1861 |
| `crates/cli/src/frontier.rs:1859` | BENIGN-INFALLIBLE | same as 1858 |
| `crates/cli/src/frontier.rs:1902` | BENIGN-INFALLIBLE | header fixed array, HEADER_RESERVED constant in range; get never None |
| `crates/cli/src/gdfl_dayflags.rs:72` | BENIGN-MATCH | bad trailing args => USAGE refusal |
| `crates/cli/src/gdfl_import_command.rs:139` | BENIGN-MATCH | Indices/Options need no share scope; empty CmScope is the correct scope, errors still propagate via ? |
| `crates/cli/src/gdfl_import_command.rs:155` | BENIGN-MATCH | exhaustive dispatch over ImportKind (Stocks/Options share list) |
| `crates/cli/src/gdfl_import_command.rs:218` | SURFACED | digit-checked parse; None refused by name at 225 |
| `crates/cli/src/gdfl_import_command.rs:219` | SURFACED | Option flatten of the same parse; None refused at 225 |
| `crates/cli/src/gdfl_import_command.rs:223` | BENIGN-MATCH | wrong part count => None => refused at 225 |
| `crates/cli/src/gdfl_import_command.rs:226` | BENIGN-INFALLIBLE | 2-digit u16 (<=99) always fits u8; 0 refused by Day::new 227 |
| `crates/cli/src/gdfl_import_command.rs:239` | SURFACED | unparsable BRUTEX_GDFL_WORKERS refused by name at 240-244 |
| `crates/cli/src/gdfl_import_command.rs:284` | BENIGN-MATCH | unset/empty root var => Err, no default |
| `crates/cli/src/gdfl_import_command.rs:402` | BENIGN-MATCH | source already validated to tickstore\|zip at 346-357; arm is (zip, non-Options) |
| `crates/cli/src/gdfl_rekey.rs:93` | SURFACED | invalid symbol => named refusal 95-97 |
| `crates/cli/src/gdfl_rekey.rs:98` | BENIGN-INFALLIBLE | BTreeSet::insert bool (duplicate) discarded; set semantics intended |
| `crates/cli/src/global_replay.rs:729` | BENIGN-OPTION | filter_map over Option cursors (exhausted streams); not Results |
| `crates/cli/src/global_replay.rs:2395` | BENIGN-MATCH | mismatched disposition pair => false => refused by caller 2352 |
| `crates/cli/src/global_replay.rs:3488` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/global_replay.rs:3507` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/global_replay.rs:3522` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/global_replay.rs:3539` | BENIGN-MATCH | unknown tag => Err |
| `crates/cli/src/global_replay_v2.rs:592` | SURFACED | u32->usize failure compares None!=Some => refused 593 |
| `crates/cli/src/global_replay_v2.rs:3014` | BENIGN-MATCH | noncanonical disposition => Err |
| `crates/cli/src/global_replay_v2.rs:3024` | BENIGN-MATCH | unknown path tag => Err |
| `crates/cli/src/global_replay_v2.rs:3034` | BENIGN-MATCH | unknown family tag => Err |
| `crates/cli/src/global_replay_v2.rs:3042` | BENIGN-MATCH | unknown direction tag => Err |
| `crates/cli/src/global_replay_v2.rs:3455` | BENIGN-OPTION | filter_map over Option cursors (exhausted streams have no next entry); not Results |
| `crates/cli/src/global_replay_v3.rs:1779` | BENIGN-MATCH | contradictory disposition => Err |
| `crates/cli/src/global_replay_v3.rs:2062` | BENIGN-MATCH | unknown vendor tag => Err |
| `crates/cli/src/global_replay_v3.rs:2072` | BENIGN-MATCH | unknown family tag => Err |
| `crates/cli/src/global_replay_v3.rs:2082` | BENIGN-MATCH | unknown direction tag => Err |
| `crates/cli/src/global_replay_v3.rs:2092` | BENIGN-MATCH | unknown path tag => Err |
| `crates/cli/src/global_replay_v3.rs:2264` | BENIGN-MATCH | payload contradicts tag => Err |
| `crates/cli/src/global_replay_v4.rs:701` | SURFACED | conversion/lookup failure => named refusal 703 |
| `crates/cli/src/global_replay_v4_codec.rs:67` | BENIGN-INFALLIBLE | Record is [u8;STRIDE]; get(8..16) always Some; MAX would be refused as unknown kind |
| `crates/cli/src/global_replay_v4_codec.rs:171` | BENIGN-OPTION | presence word written 170 before placeholder value |
| `crates/cli/src/global_replay_v4_lifecycle.rs:52` | BENIGN-INFALLIBLE | is_ok only selects refusal; result returned via combine 64 |
| `crates/cli/src/global_replay_v4_lifecycle.rs:58` | BENIGN-INFALLIBLE | is_ok only selects terminal status; result returned via combine 64 |
| `crates/cli/src/index_consistency.rs:505` | BENIGN-MATCH | unknown tag => DecodeError::Format |
| `crates/cli/src/index_consistency.rs:761` | BENIGN-MATCH | unknown tag => DecodeError::Format |
| `crates/cli/src/index_consistency.rs:1012` | BENIGN-OPTION | presence word 1011 precedes placeholder 0 |
| `crates/cli/src/index_stop.rs:495` | SURFACED | overflow => "complete row count overflow" refusal 496 |
| `crates/cli/src/index_stop_qualification.rs:548` | BENIGN-OPTION | no replay limits => original facts; limits applied only when supplied |
| `crates/cli/src/index_stop_qualification_codec.rs:92` | BENIGN-OPTION | presence word 91 precedes placeholder |
| `crates/cli/src/index_stop_qualification_codec.rs:94` | BENIGN-OPTION | presence word 93 precedes placeholder |
| `crates/cli/src/index_stop_qualification_codec.rs:347` | BENIGN-MATCH | bad bool tag => Err |
| `crates/cli/src/index_stop_qualification_metrics.rs:181` | BENIGN-OPTION | no periods => max trades 0; ceiling of 0 over trades |
| `crates/cli/src/index_stop_qualification_numeric.rs:193` | SURFACED | overflow => "single-stop span overflow" refusal 194 |
| `crates/cli/src/index_stop_qualification_numeric.rs:267` | BENIGN-OPTION | no CSCV layout for the geometry is a deterministic data-shape outcome mirroring writer 413-414; PBO stays Unmeasured |
| `crates/cli/src/index_stop_qualification_numeric.rs:682` | SURFACED | overflow => "fold span overflow" refusal 683 |
| `crates/cli/src/index_stop_search.rs:504` | BENIGN-OPTION | label for an error message only; refusal still returned 503-507 |
| `crates/cli/src/index_stop_search_checkpoint.rs:81` | SURFACED | flatten None => named refusal 82 |
| `crates/cli/src/index_stop_search_checkpoint.rs:236` | BENIGN-OPTION | no batch => empty bytes; batch length word encoded at 263 |
| `crates/cli/src/index_stop_search_checkpoint.rs:255` | BENIGN-OPTION | presence flag at 249; decode refuses zero pin 310 |
| `crates/cli/src/index_stop_search_checkpoint.rs:269` | BENIGN-OPTION | None link encoded zeros; decode refuses half pair 330-331 |
| `crates/cli/src/index_stop_search_progress.rs:99` | BENIGN-INFALLIBLE | is_ok only selects final stage; result itself matched at 105 |
| `crates/cli/src/index_stop_search_progress.rs:198` | BENIGN-OPTION | flatten of Vec<Vec<(usize,R)>>; unfilled slots refused 208 |
| `crates/cli/src/index_stop_search_reader.rs:75` | BENIGN-MATCH | unsupported version => Err |
| `crates/cli/src/index_stop_search_reader.rs:210` | BENIGN-OPTION | Option<Option> flatten returned as Option; callers refuse None (index_stop_search_checkpoint.rs:82) |
| `crates/cli/src/index_stop_search_reader.rs:219` | BENIGN-OPTION | legacy declarations have no context link (doc 213); returned as Option |
| `crates/cli/src/index_stop_search_reader.rs:519` | BENIGN-OPTION | frames without this rung's link contributed no child; presence enforced by transition() checkpoint.rs:544-548 |
| `crates/cli/src/index_stop_source_context_codec.rs:319` | BENIGN-MATCH | invalid execution alias => Err |
| `crates/cli/src/index_stop_source_context_codec.rs:357` | BENIGN-OPTION | None execution is the validated 1min alias of signal (317) |
| `crates/cli/src/index_stop_store.rs:347` | BENIGN-MATCH | unsupported direction => Err |
| `crates/cli/src/index_stop_store.rs:366` | SURFACED | conversion None => "record admission overflow" refusal 368 |
| `crates/cli/src/index_stop_store.rs:367` | SURFACED | conversion None => "record admission overflow" refusal 368 |
| `crates/cli/src/index_stop_vix.rs:625` | BENIGN-MATCH | no trades => month extent 0 (correct empty case) |
| `crates/cli/src/index_stop_vix_codec.rs:94` | BENIGN-OPTION | presence word written at 93; decode refuses contradictions 365 |
| `crates/cli/src/index_stop_vix_codec.rs:98` | BENIGN-OPTION | absent digest encoded zeros alongside presence word; decode validates 365 |
| `crates/cli/src/index_stop_vix_codec.rs:99` | BENIGN-OPTION | absent reason encoded length 0; decode validates provenance 365 |
| `crates/cli/src/index_stop_vix_codec.rs:365` | BENIGN-MATCH | contradictory provenance => Err |
| `crates/cli/src/index_stop_vix_codec.rs:438` | BENIGN-MATCH | bad stamp tag => Err |
| `crates/cli/src/institutional_evidence.rs:1823` | BENIGN-MATCH | foreign cell => Err |
| `crates/cli/src/institutional_evidence.rs:1938` | BENIGN-MATCH | partial durable identity => Err |
| `crates/cli/src/institutional_evidence.rs:2157` | SURFACED | conversion/overflow => named refusal 2160 |
| `crates/cli/src/institutional_statistics.rs:1601` | BENIGN-INFALLIBLE | numerator<=denominator so quotient<=PPM; documented conservative saturation 1594-1596 (D-0930) |
| `crates/cli/src/knobs.rs:185` | BENIGN-OPTION | pure parser; callers refuse/record None (knobs.rs:150, strict_range_knobs.rs) |
| `crates/cli/src/knobs.rs:191` | BENIGN-OPTION | pure parser; count_usize_within records refusal 178; lib.rs:14513 caller |
| `crates/cli/src/knobs.rs:196` | BENIGN-OPTION | pure parser; policy_floor/strict_range_knobs callers decide refusal |
| `crates/cli/src/knobs.rs:220` | BENIGN-MATCH | other floor names have no extra bound; Some(value) |
| `crates/cli/src/knobs.rs:227` | BENIGN-OPTION | pure parser; callers ok_or refusal (boolean_oos_command.rs:71, boolean_grammar_campaign.rs:91) |
| `crates/cli/src/knobs.rs:234` | BENIGN-OPTION | pure parser; callers (lib.rs:12629, strict_range_knobs.rs:72) decide |
| `crates/cli/src/knobs.rs:256` | BENIGN-INFALLIBLE | RwLock poison unreachable (write bodies 124-128/379-385 cannot panic); documented 122-123 |
| `crates/cli/src/knobs.rs:312` | BENIGN-INFALLIBLE | poison unreachable; documented fallback-to-env rationale 296-302 |
| `crates/cli/src/ledger_all.rs:339` | BENIGN-OPTION | unset knob => next source; unparsable knob recorded missing 349-355 |
| `crates/cli/src/ledger_all.rs:978` | BENIGN-MATCH | rung-group dispatch for rendering |
| `crates/cli/src/ledger_all.rs:1420` | SURFACED | refusal first line noted (1422) and returned |
| `crates/cli/src/lib.rs:1338` | SURFACED | None => callers return the original refusal unchanged (lib.rs:1057-1058, 1251-1252) |
| `crates/cli/src/lib.rs:1808` | BENIGN-OPTION | documented fallback (1794-1798) reached only when no positive price exists, so nothing can be priced |
| `crates/cli/src/lib.rs:1900` | SURFACED | bad month => Err(()) => refused by the match at 1904- |
| `crates/cli/src/lib.rs:1968` | SURFACED | bad month => "MONTH must be 1..=12" refusal 1970 |
| `crates/cli/src/lib.rs:2337` | SURFACED | bad month => named refusal 2339 |
| `crates/cli/src/lib.rs:2348` | SURFACED | bad MAX_POINTS => named refusal 2350 |
| `crates/cli/src/lib.rs:2553` | BENIGN-OPTION | documented: zero means uncorrelated, never an ambient run (2551-2552); telemetry only |
| `crates/cli/src/lib.rs:2966` | SURFACED | parse failure => named refusal |
| `crates/cli/src/lib.rs:2996` | SURFACED | parse failure => named refusal |
| `crates/cli/src/lib.rs:3044` | SURFACED | parse failure => named refusal |
| `crates/cli/src/lib.rs:3052` | SURFACED | None widths => "the pinned tolerances are not valid" refusal (lib.rs:3100) |
| `crates/cli/src/lib.rs:3083` | SURFACED | None widths => refusal at lib.rs:3100 |
| `crates/cli/src/lib.rs:3558` | BENIGN-MATCH | non-canonical build stamp => None; documented, recording refuses without a stamp (3549-3553) |
| `crates/cli/src/lib.rs:3748` | SURFACED | no resolvable dir => UNLOGGED banner naming the variables (3752-3757) |
| `crates/cli/src/lib.rs:3759` | BENIGN-OPTION | banner text only |
| `crates/cli/src/lib.rs:3854` | SURFACED | Dropped events counted in Sink::health (telemetry/src/sink.rs:222); NotInstalled reported once by install_log (3849-3... |
| `crates/cli/src/lib.rs:3894` | BENIGN-INFALLIBLE | usize->u64 telemetry field |
| `crates/cli/src/lib.rs:4093` | BENIGN-OPTION | folded is Some only when days were withheld (doc 4084-4086) |
| `crates/cli/src/lib.rs:4313` | BENIGN-OPTION | same as 4093 |
| `crates/cli/src/lib.rs:4475` | BENIGN-INFALLIBLE | usize->u64 telemetry field |
| `crates/cli/src/lib.rs:4547` | BENIGN-INFALLIBLE | usize->u64 telemetry field |
| `crates/cli/src/lib.rs:4552` | BENIGN-INFALLIBLE | usize->u64 telemetry field |
| `crates/cli/src/lib.rs:4558` | BENIGN-INFALLIBLE | usize->u64 telemetry field |
| `crates/cli/src/lib.rs:4926` | BENIGN-INFALLIBLE | usize->u64 telemetry field |
| `crates/cli/src/lib.rs:5125` | BENIGN-INFALLIBLE | constant BOOTSTRAP_ALPHA_PPM conversion |
| `crates/cli/src/lib.rs:5126` | BENIGN-INFALLIBLE | divisor is a nonzero constant |
| `crates/cli/src/lib.rs:5426` | BENIGN-INFALLIBLE | PAISA_PER_POINT nonzero constant divisor |
| `crates/cli/src/lib.rs:5493` | BENIGN-INFALLIBLE | small usize rung count ->i64 |
| `crates/cli/src/lib.rs:5767` | BENIGN-OPTION | percentile index; callers pass constant nonzero denominators |
| `crates/cli/src/lib.rs:5796` | BENIGN-OPTION | percentile index; callers pass constant nonzero denominators |
| `crates/cli/src/lib.rs:5880` | BENIGN-OPTION | no measurable travel => 1.00x widening floor, documented 5876-5877 |
| `crates/cli/src/lib.rs:5888` | BENIGN-INFALLIBLE | range ppm of positive close fits i64 |
| `crates/cli/src/lib.rs:5940` | BENIGN-INFALLIBLE | checked_div(100) cannot fail |
| `crates/cli/src/lib.rs:5942` | BENIGN-INFALLIBLE | resolution >0 enforced (lib.rs:5927 filter) or default 20 |
| `crates/cli/src/lib.rs:6171` | BENIGN-INFALLIBLE | saturating display count, documented |
| `crates/cli/src/lib.rs:6258` | BENIGN-INFALLIBLE | cap/step non-negative; result clamped 6263 |
| `crates/cli/src/lib.rs:6425` | BENIGN-INFALLIBLE | nonzero constant/thread divisors |
| `crates/cli/src/lib.rs:6489` | BENIGN-INFALLIBLE | usize->u64 |
| `crates/cli/src/lib.rs:6867` | SWALLOWED | out-of-order day: eprintln then return a TRUNCATED session index; bootstrap/pool_oos run on it unlabelled (LOCKED) |
| `crates/cli/src/lib.rs:6874` | BENIGN-MATCH | new-day arm of the dedup walk |
| `crates/cli/src/lib.rs:7126` | BENIGN-INFALLIBLE | usize->u64 telemetry field |
| `crates/cli/src/lib.rs:7180` | BENIGN-INFALLIBLE | usize->u64 |
| `crates/cli/src/lib.rs:8337` | BENIGN-INFALLIBLE | exit_rungs is [i16;5] (results.rs:351); get always Some; -1 is the none sentinel |
| `crates/cli/src/lib.rs:8338` | BENIGN-INFALLIBLE | same as 8337 |
| `crates/cli/src/lib.rs:8339` | BENIGN-INFALLIBLE | same as 8337 |
| `crates/cli/src/lib.rs:8340` | BENIGN-INFALLIBLE | same as 8337 |
| `crates/cli/src/lib.rs:8341` | BENIGN-INFALLIBLE | same as 8337 |
| `crates/cli/src/lib.rs:8781` | BENIGN-INFALLIBLE | first line of a refusal (held:false already reported) |
| `crates/cli/src/lib.rs:8968` | SWALLOWED | verify scratch dir remove_dir_all failure discarded; debris left silently; check result unaffected (low, LOCKED) |
| `crates/cli/src/lib.rs:10419` | SURFACED | Rules::stated records unusable value via refuse_value (lib.rs:10701-10715); fallback documented 10414-10417 |
| `crates/cli/src/lib.rs:10435` | BENIGN-INFALLIBLE | stated() admits only >=0 (policy_floor/nonnegative_floor), so i64->u64 cannot fail |
| `crates/cli/src/lib.rs:10473` | SURFACED | unusable value recorded by stated(); unset/invalid keeps the rule ON (conservative) |
| `crates/cli/src/lib.rs:10962` | BENIGN-INFALLIBLE | trade count fits i64 |
| `crates/cli/src/lib.rs:11001` | BENIGN-INFALLIBLE | small usize rung index -> i64 |
| `crates/cli/src/lib.rs:11297` | BENIGN-MATCH | tier label for ranks >=80 |
| `crates/cli/src/lib.rs:11375` | SURFACED | unusable BRUTEX_SCREEN_CAP recorded (knobs.rs:178) and printed in KNOB REFUSED block |
| `crates/cli/src/lib.rs:11602` | BENIGN-INFALLIBLE | elapsed_nanos>0 checked at 11593 |
| `crates/cli/src/lib.rs:11603` | BENIGN-INFALLIBLE | saturation of a fit count, clamped to offered 11615 |
| `crates/cli/src/lib.rs:11610` | BENIGN-INFALLIBLE | ilog2 < bit width so shl cannot overflow |
| `crates/cli/src/lib.rs:11642` | SURFACED | unusable BRUTEX_VALIDATE named via refuse_value; stack stays ON |
| `crates/cli/src/lib.rs:11646` | BENIGN-OPTION | raw absent only affects the recorded text |
| `crates/cli/src/lib.rs:11669` | SURFACED | None => named refusal at 11642-11647 |
| `crates/cli/src/lib.rs:11743` | BENIGN-INFALLIBLE | rules non-negative by construction; saturation documented 11741-11742 |
| `crates/cli/src/lib.rs:11783` | BENIGN-INFALLIBLE | usize->u64 |
| `crates/cli/src/lib.rs:11815` | BENIGN-INFALLIBLE | rule non-negative by construction |
| `crates/cli/src/lib.rs:11816` | BENIGN-INFALLIBLE | rule non-negative by construction |
| `crates/cli/src/lib.rs:11817` | BENIGN-INFALLIBLE | derived non-negative floor |
| `crates/cli/src/lib.rs:11818` | BENIGN-INFALLIBLE | rule non-negative by construction |
| `crates/cli/src/lib.rs:11819` | BENIGN-INFALLIBLE | rule non-negative by construction |
| `crates/cli/src/lib.rs:11836` | BENIGN-INFALLIBLE | grid_step_ppm returns >=1 |
| `crates/cli/src/lib.rs:11885` | BENIGN-OPTION | budget refused on every recorded run (11881-11883, D-0685) so term is 0 |
| `crates/cli/src/lib.rs:11895` | BENIGN-INFALLIBLE | usize->u64 |
| `crates/cli/src/lib.rs:12200` | BENIGN-INFALLIBLE | pinned-constant evaluator; caller 10555 documents the early return to operator rules |
| `crates/cli/src/lib.rs:12506` | BENIGN-INFALLIBLE | u64->usize on 64-bit |
| `crates/cli/src/lib.rs:12535` | BENIGN-INFALLIBLE | first line of a refusal that is itself reported |
| `crates/cli/src/lib.rs:12540` | BENIGN-OPTION | no refusal => empty reason with ok=true |
| `crates/cli/src/lib.rs:13369` | BENIGN-OPTION | documented zero past the grain list (13367) |
| `crates/cli/src/lib.rs:13379` | BENIGN-OPTION | shares array one per GRAIN, never empty |
| `crates/cli/src/lib.rs:13442` | BENIGN-MATCH | grains with no extra field |
| `crates/cli/src/lib.rs:14513` | SURFACED | unusable BRUTEX_TOP named via refuse_value then 25 |
| `crates/cli/src/lib.rs:14700` | BENIGN-INFALLIBLE | pure value discarded (dead computation); no failure involved |
| `crates/cli/src/lib.rs:15122` | BENIGN-OPTION | telemetry support ppm, zero bars => 0 |
| `crates/cli/src/lib.rs:15290` | BENIGN-INFALLIBLE | nonzero constant divisor |
| `crates/cli/src/lib.rs:15381` | BENIGN-INFALLIBLE | first line of a refusal |
| `crates/cli/src/lib.rs:15430` | BENIGN-OPTION | in-process cache key only; commit absence refused where a run is recorded (15295) |
| `crates/cli/src/lib.rs:16087` | BENIGN-INFALLIBLE | usize->u64; zero refused 16088 |
| `crates/cli/src/lib.rs:16296` | SURFACED | documented 16293-16294: a refused load keeps the count and every step refuses with that reason |
| `crates/cli/src/lib.rs:16340` | BENIGN-INFALLIBLE | stderr progress by design (16337-16339); not a failure |
| `crates/cli/src/lib.rs:16924` | SURFACED | refusal first line kept in the refusals column |
| `crates/cli/src/lib.rs:16972` | SURFACED | REFUSED line printed with reason |
| `crates/cli/src/lib.rs:17194` | BENIGN-INFALLIBLE | stderr progress by design (17191-17193) |
| `crates/cli/src/lib.rs:17505` | SURFACED | refusal reason printed "REFUSED: {why}" at lib.rs:17548 |
| `crates/cli/src/lib.rs:18198` | SURFACED | None only sizes the floor; same failure refuses every step (doc 16293-16294) |
| `crates/cli/src/lib.rs:18219` | SURFACED | as 18198; parse_vendor also refuses at 15409 |
| `crates/cli/src/lib.rs:18220` | SURFACED | as 18198; budget refusal also returned at 15384 |
| `crates/cli/src/lib.rs:18221` | SURFACED | as 18198 |
| `crates/cli/src/lib.rs:18790` | BENIGN-OPTION | indices from enumerate(folded) so in range; empty window refused downstream (doc 18756-18757) |
| `crates/cli/src/lib.rs:19269` | SURFACED | zero/unparsable BRUTEX_CEILING => named refusal |
| `crates/cli/src/lib.rs:19299` | BENIGN-INFALLIBLE | divisor max(1) |
| `crates/cli/src/lib.rs:19479` | BENIGN-OPTION | failed refresh discards cached handle; fresh open re-validates and errors propagate 19484 |
| `crates/cli/src/lib.rs:19492` | BENIGN-INFALLIBLE | is_ok selects caching; result returned 19495 |
| `crates/cli/src/lib.rs:19513` | BENIGN-OPTION | unreadable identity => no reuse, fresh open |
| `crates/cli/src/lib.rs:19518` | BENIGN-INFALLIBLE | cfg(not(unix)) only |
| `crates/cli/src/lib.rs:19664` | BENIGN-INFALLIBLE | trade seq bounded by bars << u32::MAX |
| `crates/cli/src/lib.rs:19666` | BENIGN-INFALLIBLE | usize->u64 |
| `crates/cli/src/lib.rs:19667` | BENIGN-INFALLIBLE | usize->u64 |
| `crates/cli/src/lib.rs:19668` | BENIGN-INFALLIBLE | usize->u64 |
| `crates/cli/src/lib.rs:19690` | BENIGN-INFALLIBLE | usize->u64 |
| `crates/cli/src/lib.rs:19818` | SURFACED | rank past u16 dropped and counted: "{written} of {kept}" (lib.rs:19838-19839), doc 19819-19823 |
| `crates/cli/src/lib.rs:19824` | SURFACED | same as 19818 |
| `crates/cli/src/lib.rs:19849` | BENIGN-INFALLIBLE | usize->u64 |
| `crates/cli/src/lib.rs:20146` | BENIGN-INFALLIBLE | None => documented -1 sentinel; rung index bounded by the grid cell budget far below i16::MAX |
| `crates/cli/src/lib.rs:20164` | BENIGN-INFALLIBLE | depth <= vocabulary width |
| `crates/cli/src/lib.rs:20413` | BENIGN-OPTION | report sections absent => omitted |
| `crates/cli/src/lib.rs:20430` | BENIGN-OPTION | report text slicing |
| `crates/cli/src/lib.rs:20440` | BENIGN-INFALLIBLE | split().next() is Some |
| `crates/cli/src/lib.rs:20452` | BENIGN-INFALLIBLE | lines().next() fallback to rest |
| `crates/cli/src/lib.rs:20500` | SURFACED | overflow => "ranked evidence ordinal overflowed" refusal 20502 |
| `crates/cli/src/lib.rs:20525` | SURFACED | Attempt::level remembers failure; caller checks attempt.check() => NOT_RECORDED (lib.rs:22102-22106) |
| `crates/cli/src/lib.rs:20583` | BENIGN-MATCH | not recording => no live view; recording implies id (refused otherwise lib.rs:21964) |
| `crates/cli/src/lib.rs:20631` | BENIGN-INFALLIBLE | take(STORED_KEEP=25) so rank fits u16 |
| `crates/cli/src/lib.rs:20757` | BENIGN-INFALLIBLE | usize->u64 telemetry |
| `crates/cli/src/lib.rs:20759` | BENIGN-INFALLIBLE | usize->u64 telemetry |
| `crates/cli/src/lib.rs:20901` | BENIGN-INFALLIBLE | dead binding, no failure |
| `crates/cli/src/lib.rs:21017` | SWALLOWED | HORIZON=rung with underivable spacing => Horizon::DEFAULT, no refuse_value/note (low, LOCKED) |
| `crates/cli/src/lib.rs:21069` | BENIGN-INFALLIBLE | minutes between bars fit u32; feeds 21017 |
| `crates/cli/src/lib.rs:21114` | BENIGN-INFALLIBLE | usize->u64 telemetry |
| `crates/cli/src/lib.rs:21115` | BENIGN-INFALLIBLE | usize->u64 telemetry |
| `crates/cli/src/lib.rs:21116` | BENIGN-INFALLIBLE | usize->u64 telemetry |
| `crates/cli/src/lib.rs:21201` | BENIGN-INFALLIBLE | progress counter poison recovered; display only |
| `crates/cli/src/lib.rs:21264` | BENIGN-INFALLIBLE | usize->u64 telemetry |
| `crates/cli/src/lib.rs:21265` | BENIGN-INFALLIBLE | usize->u64 telemetry |
| `crates/cli/src/lib.rs:21291` | BENIGN-INFALLIBLE | usize->u64 telemetry |
| `crates/cli/src/lib.rs:21292` | BENIGN-INFALLIBLE | usize->u64 telemetry |
| `crates/cli/src/lib.rs:21293` | BENIGN-INFALLIBLE | usize->u64 telemetry |
| `crates/cli/src/lib.rs:22289` | BENIGN-MATCH | no recording => no capture; recording implies attempt (lib.rs:21953-21965) |
| `crates/cli/src/lib.rs:22516` | BENIGN-OPTION | documented unreachable except caller defect; coarse w/o execution refused earlier (22513-22515) |
| `crates/cli/src/lib.rs:22581` | BENIGN-OPTION | validate off or empty family => None, rendered as explicit absence |
| `crates/cli/src/live.rs:268` | BENIGN-INFALLIBLE | fixed [u8;24] array, constant offsets 0/8/16; get() never misses |
| `crates/cli/src/live.rs:385` | BENIGN-INFALLIBLE | top-N row count (keep=25) to u32; count is re-checked against file length on read |
| `crates/cli/src/live.rs:406` | BENIGN-INFALLIBLE | temp cleanup on failed write; primary error returned at live.rs:407 |
| `crates/cli/src/live.rs:410` | BENIGN-INFALLIBLE | temp cleanup on failed rename; primary error returned at live.rs:411-416 |
| `crates/cli/src/live.rs:647` | SURFACED | unlistable live dir -> Census.listed=false (documented live.rs:608-610); io reason text dropped |
| `crates/cli/src/live.rs:659` | SWALLOWED | ReadDir entry errors dropped, not counted in skipped/strays; census_at only (prod CensusCache refuses at :937) |
| `crates/cli/src/live.rs:710` | SURFACED | no mtime -> Freshness::Unknown, counted in Census.undated (live.rs:676) |
| `crates/cli/src/live.rs:711` | SURFACED | clock-moved file -> Freshness::Unknown, counted undated (live.rs:676); documented :704-707 |
| `crates/cli/src/live.rs:743` | SURFACED | read_one None -> counted skipped (live.rs:681) / CensusCache refuses (live.rs:969-971) |
| `crates/cli/src/live.rs:745` | SURFACED | short header -> None -> skipped count (live.rs:681) or CensusCache refusal (:969-971) |
| `crates/cli/src/live.rs:755` | SURFACED | u32->usize infallible on 64-bit; any None -> skipped/refused |
| `crates/cli/src/live.rs:758` | SURFACED | short summary -> None -> skipped count (live.rs:681) or refusal (:969-971) |
| `crates/cli/src/live.rs:769` | SURFACED | metadata failure -> None -> skipped count (live.rs:681) or refusal (:969-971) |
| `crates/cli/src/live.rs:772` | SURFACED | u64->usize infallible on 64-bit; None -> skipped/refused |
| `crates/cli/src/live.rs:775` | SURFACED | strict-length check overflow -> None -> CensusCache refusal (live.rs:969-971) |
| `crates/cli/src/live.rs:875` | BENIGN-OPTION | created() unsupported -> None, used only as a change-detection stamp field |
| `crates/cli/src/live.rs:956` | BENIGN-INFALLIBLE | usize->u64 of a small constant bound |
| `crates/cli/src/live.rs:964` | BENIGN-OPTION | Option<Option<&>> cache lookup; miss means re-read the file |
| `crates/cli/src/live.rs:1061` | BENIGN-INFALLIBLE | stem already checked lowercase-hex ASCII at live.rs:1056 |
| `crates/cli/src/minute_gaps.rs:150` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/minute_gaps.rs:259` | BENIGN-OPTION | display: unconvertible day prints its raw number (documented :249-252), never dropped |
| `crates/cli/src/minute_gaps.rs:260` | BENIGN-OPTION | display: unconvertible day prints its raw number (documented :249-252), never dropped |
| `crates/cli/src/operation_audit.rs:71` | BENIGN-MATCH | decoder refusing arm for unknown origin code |
| `crates/cli/src/operation_audit.rs:128` | BENIGN-MATCH | decoder refusing arm for unknown phase code |
| `crates/cli/src/operation_audit.rs:163` | SURFACED | fixed-layout slice; failure becomes Err at operation_audit.rs:164 |
| `crates/cli/src/operation_audit.rs:395` | BENIGN-OPTION | wall clock unavailable -> 0, documented "zero means unstamped" (operation_audit.rs:138) |
| `crates/cli/src/operation_audit.rs:396` | BENIGN-INFALLIBLE | u128 micros -> u64 saturates only after ~584k years |
| `crates/cli/src/operation_audit.rs:428` | BENIGN-INFALLIBLE | poisoned lock read of immutable id field only |
| `crates/cli/src/operation_audit.rs:481` | SURFACED | Drop path: telemetry error event + stderr; record stays "unconfirmed" (Phase::label) |
| `crates/cli/src/operation_audit.rs:504` | BENIGN-INFALLIBLE | poisoned lock read of immutable id field only |
| `crates/cli/src/operation_audit.rs:524` | SURFACED | telemetry error + stderr; append failure poisons State.failure so finish() refuses (:384-385,:404) |
| `crates/cli/src/operation_audit.rs:570` | BENIGN-OPTION | wall clock unavailable -> 0, documented "zero means unstamped" (operation_audit.rs:138) |
| `crates/cli/src/ordered.rs:85` | BENIGN-INFALLIBLE | scheduling counters; a lane panic is resumed at ordered.rs:166 |
| `crates/cli/src/ordered.rs:216` | BENIGN-INFALLIBLE | scheduling counters; a lane panic is resumed at ordered.rs:166 |
| `crates/cli/src/pool.rs:374` | SURFACED | counts screened vs refused into telemetry (pool.rs:377-378); refusals rendered at :822 |
| `crates/cli/src/pool.rs:539` | BENIGN-INFALLIBLE | usize->u64 for a telemetry field |
| `crates/cli/src/pool.rs:566` | SURFACED | all-refused pass 1 becomes a named refusal (pool.rs:569-581) |
| `crates/cli/src/pool.rs:764` | SURFACED | Err rows appended after Ok rows (pool.rs:774) and printed REFUSED with reason (:822) |
| `crates/cli/src/pool.rs:802` | SURFACED | refused row gets symbol cell; reason printed at pool.rs:821-822 |
| `crates/cli/src/pool.rs:867` | SURFACED | frontier open failure named per instrument into unread, printed at pool.rs:385-395 |
| `crates/cli/src/pool.rs:884` | SURFACED | parent ledger open failure named per instrument into unread, printed at pool.rs:385-395 |
| `crates/cli/src/pool.rs:987` | BENIGN-INFALLIBLE | horizon bars -> usize on 64-bit |
| `crates/cli/src/pool.rs:1199` | SURFACED | collects pass-2 refusals; printed NOT PRICED at pool.rs:1243-1252 |
| `crates/cli/src/pool.rs:1220` | SURFACED | Err entries printed in place at pool.rs:1227-1228 |
| `crates/cli/src/pool_oos.rs:195` | SURFACED | overflow becomes Err via ok_or at pool_oos.rs:196 |
| `crates/cli/src/pool_oos.rs:386` | BENIGN-OPTION | usize->i128 infallible; zero sessions -> 0 documented at :382-383 |
| `crates/cli/src/pool_oos.rs:457` | BENIGN-INFALLIBLE | usize->u64 length compare |
| `crates/cli/src/pool_oos.rs:551` | SURFACED | early-exit precheck only; create_new at write time refuses and page says refused (:707-708) |
| `crates/cli/src/pool_oos.rs:686` | BENIGN-INFALLIBLE | row.candidate comes from enumerate over union-length tallies (pool_oos.rs:352-356) |
| `crates/cli/src/population.rs:169` | BENIGN-MATCH | decoder refusing arm (unknown family byte) |
| `crates/cli/src/population.rs:347` | BENIGN-MATCH | decoder refusing arm (unknown direction byte) |
| `crates/cli/src/population.rs:379` | BENIGN-MATCH | decoder refusing arm (unknown closure byte) |
| `crates/cli/src/population.rs:415` | BENIGN-MATCH | decoder refusing arm (unknown admission-status byte) |
| `crates/cli/src/population.rs:466` | BENIGN-OPTION | inside error text after is_some_and(true); None arm unreachable |
| `crates/cli/src/population.rs:472` | BENIGN-OPTION | inside error text after is_some_and(true); None arm unreachable |
| `crates/cli/src/population.rs:798` | BENIGN-MATCH | half-present TTP coordinate refused |
| `crates/cli/src/population.rs:3393` | SURFACED | unreceipted trailing block not exact -> cut with warn telemetry (fixed_tail.rs:340-350); read-error reason lost |
| `crates/cli/src/population.rs:4128` | BENIGN-INFALLIBLE | usize->u64 sequence ordinal |
| `crates/cli/src/population.rs:4155` | BENIGN-INFALLIBLE | usize->u64 sequence ordinal |
| `crates/cli/src/population.rs:4327` | BENIGN-OPTION | absent direction-cell count means zero rows in that direction |
| `crates/cli/src/population.rs:4332` | BENIGN-OPTION | absent direction-cell count means zero rows in that direction |
| `crates/cli/src/population.rs:4447` | BENIGN-OPTION | no row block == empty facts, compared against receipt (mismatch refuses :4448) |
| `crates/cli/src/population.rs:4979` | BENIGN-INFALLIBLE | fixed header slice; fallback 0 can only refuse at version check :4981 |
| `crates/cli/src/population.rs:5293` | BENIGN-INFALLIBLE | ppm of part<=total fits u64; total==0 handled at :5289 |
| `crates/cli/src/population.rs:5312` | BENIGN-OPTION | None encoded as reserved u32::MAX sentinel; Some(u32::MAX) refused by validate (:448) |
| `crates/cli/src/population.rs:5348` | BENIGN-MATCH | unknown option tag refused |
| `crates/cli/src/population.rs:5358` | BENIGN-MATCH | unknown bool byte refused |
| `crates/cli/src/population.rs:5405` | BENIGN-INFALLIBLE | take<=8 slice of an 8-byte zero array |
| `crates/cli/src/population.rs:5490` | BENIGN-INFALLIBLE | bytes::<1>() is a [u8;1]; first() always Some |
| `crates/cli/src/population_admission_v2.rs:208` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/population_admission_v2.rs:239` | BENIGN-MATCH | decoder refusing arm (unknown status) |
| `crates/cli/src/population_admission_v3.rs:236` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/population_admission_v3.rs:261` | BENIGN-MATCH | decoder refusing arm (unknown status) |
| `crates/cli/src/population_admission_v3.rs:3482` | BENIGN-OPTION | cfg(not(unix)) generation stamp; not compiled on unix |
| `crates/cli/src/population_admission_v4.rs:164` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/population_admission_v4.rs:211` | BENIGN-MATCH | decoder refusing arm (unknown family terminal) |
| `crates/cli/src/population_admission_v4.rs:246` | BENIGN-MATCH | decoder refusing arm (unknown decision status) |
| `crates/cli/src/population_admission_v4.rs:583` | SURFACED | cardinality mismatch refused at population_admission_v4.rs:584 |
| `crates/cli/src/population_admission_v4.rs:774` | SURFACED | cardinality mismatch refused at population_admission_v4.rs:775 |
| `crates/cli/src/population_admission_v4.rs:1710` | BENIGN-OPTION | len>=header means not a torn header; header validated elsewhere |
| `crates/cli/src/population_admission_v4.rs:2742` | BENIGN-OPTION | exact-retry comparison; mismatch treated as scratch prefix (D-1905); read errors propagate via ? |
| `crates/cli/src/population_admission_writer.rs:1376` | BENIGN-INFALLIBLE | checked_div by nonzero total (zero handled at :1370) |
| `crates/cli/src/population_admission_writer.rs:1377` | BENIGN-INFALLIBLE | ppm of part<=total fits u64 |
| `crates/cli/src/population_base_evidence_ledger_v2.rs:1744` | BENIGN-MATCH | any probe result but NotFound refuses (incl. probe errors) |
| `crates/cli/src/population_base_evidence_ledger_v2.rs:1843` | BENIGN-OPTION | cfg(not(unix)) generation stamp; not compiled on unix |
| `crates/cli/src/population_base_evidence_ledger_v2.rs:1896` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/population_base_evidence_v2.rs:562` | BENIGN-INFALLIBLE | min over fixed [i64; GRAIN_COUNT] array is always Some |
| `crates/cli/src/population_finalization_v2.rs:2298` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/population_finalization_v3.rs:1736` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/population_finalization_v3.rs:1746` | BENIGN-MATCH | decoder refusing arm (unknown status) |
| `crates/cli/src/population_finalization_v3.rs:1882` | BENIGN-OPTION | cfg(not(unix)) generation stamp; not compiled on unix |
| `crates/cli/src/population_finalization_v3.rs:2468` | SURFACED | invalid missing count refused via ok_or_else at :2469 |
| `crates/cli/src/population_finalization_v4.rs:1358` | BENIGN-OPTION | len>=header means not a torn header; header validated elsewhere |
| `crates/cli/src/population_finalization_v4.rs:1397` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/population_finalization_v4.rs:1408` | BENIGN-MATCH | decoder refusing arm (unknown terminal) |
| `crates/cli/src/population_finalization_v4.rs:1418` | BENIGN-MATCH | decoder refusing arm (unknown decision status) |
| `crates/cli/src/population_finalization_v4.rs:2466` | BENIGN-OPTION | exact-retry comparison; mismatch treated as scratch prefix (D-1905); read errors propagate via ? |
| `crates/cli/src/population_observations_v1.rs:920` | SURFACED | Option<Option<usize>> flatten then refused by ok_or_else at :921-925 |
| `crates/cli/src/population_observations_v1.rs:1181` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/population_observations_v1.rs:1189` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/population_observations_v1.rs:1478` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/population_observations_v1.rs:2414` | BENIGN-MATCH | sync error still returned; arm only skips the snapshot refresh |
| `crates/cli/src/population_observations_v1.rs:2663` | BENIGN-INFALLIBLE | u64->usize on 64-bit, then min(record_count/2) capacity hint |
| `crates/cli/src/population_observations_v1.rs:2809` | BENIGN-OPTION | cfg(not(unix)) generation stamp; not compiled on unix |
| `crates/cli/src/population_observations_v1.rs:2870` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/population_observations_v1.rs:2896` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/population_observations_v1.rs:2970` | BENIGN-MATCH | decoder refusing arm (unknown disposition) |
| `crates/cli/src/population_observations_v1.rs:3682` | BENIGN-MATCH | sync error still returned; arm only skips the snapshot refresh |
| `crates/cli/src/population_observations_v1.rs:3730` | BENIGN-INFALLIBLE | constant record size to u32 |
| `crates/cli/src/population_observations_v1.rs:3831` | BENIGN-MATCH | foreign record kind refused |
| `crates/cli/src/population_statistics_v2.rs:146` | BENIGN-MATCH | decoder refusing arm (unknown record kind) |
| `crates/cli/src/population_statistics_v2.rs:5787` | BENIGN-OPTION | cfg(not(unix)) generation stamp; not compiled on unix |
| `crates/cli/src/population_statistics_v2.rs:5899` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/population_statistics_v2.rs:6017` | BENIGN-MATCH | nibble is 0..=15; '?' arm unreachable |
| `crates/cli/src/population_statistics_v3.rs:123` | BENIGN-MATCH | decoder refusing arm (unknown terminal) |
| `crates/cli/src/population_statistics_v3.rs:325` | BENIGN-MATCH | PBO/terminal contradiction refused |
| `crates/cli/src/population_statistics_v3.rs:753` | SURFACED | cardinality mismatch refused at population_statistics_v3.rs:754 |
| `crates/cli/src/population_statistics_v3.rs:1619` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/population_statistics_v3.rs:1647` | BENIGN-MATCH | decoder refusing arm (unknown record kind) |
| `crates/cli/src/population_statistics_v3.rs:1794` | BENIGN-OPTION | absent Romano-Wolf digest -> zeros with explicit presence flag bit 2 (:1784-1788) |
| `crates/cli/src/population_statistics_v3.rs:2041` | SURFACED | overflow refused via ok_or_else at :2042 |
| `crates/cli/src/population_statistics_v3.rs:2497` | SURFACED | overflow refused via ok_or_else at :2498 |
| `crates/cli/src/population_statistics_v3.rs:3115` | BENIGN-OPTION | cfg(not(unix)) generation stamp; not compiled on unix |
| `crates/cli/src/population_v5.rs:3362` | BENIGN-OPTION | cfg(not(unix)) identity stamp; not compiled on unix |
| `crates/cli/src/population_v5.rs:3376` | BENIGN-OPTION | cfg(not(unix)) identity hash; not compiled on unix |
| `crates/cli/src/population_v5.rs:3445` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/population_v5.rs:3455` | BENIGN-MATCH | decoder refusing arm (unknown status) |
| `crates/cli/src/population_v5.rs:3624` | BENIGN-INFALLIBLE | cfg(not(unix)) unused-binding silencer |
| `crates/cli/src/population_v5.rs:3679` | BENIGN-OPTION | mtime stamp for change detection; content also hashed twice (:3680,:3698) |
| `crates/cli/src/population_v5.rs:3690` | BENIGN-OPTION | mtime stamp for change detection; content also hashed twice |
| `crates/cli/src/population_v5.rs:3709` | BENIGN-OPTION | mtime stamp for change detection; content also hashed twice |
| `crates/cli/src/population_v6.rs:1404` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/population_v6.rs:1497` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/population_v6.rs:1506` | BENIGN-MATCH | decoder refusing arm (unknown terminal) |
| `crates/cli/src/population_v6.rs:1516` | BENIGN-MATCH | decoder refusing arm (unknown status) |
| `crates/cli/src/population_v6.rs:1885` | BENIGN-OPTION | cfg(not(unix)) identity stamp; not compiled on unix |
| `crates/cli/src/population_v6.rs:2674` | BENIGN-OPTION | per-family Option: a family is evaluated XOR extinct (step3_orchestrator.rs:4316-4330) |
| `crates/cli/src/population_v6.rs:2909` | SURFACED | row-count mismatch refused at population_v6.rs:2912 |
| `crates/cli/src/population_v6.rs:3298` | BENIGN-MATCH | family without retained source refused |
| `crates/cli/src/population_v6.rs:3402` | BENIGN-MATCH | topology change refused |
| `crates/cli/src/pre_admission_data.rs:312` | BENIGN-MATCH | non-canonical eligibility byte refused |
| `crates/cli/src/pre_admission_data.rs:853` | BENIGN-MATCH | decoder refusing arm (unknown record kind) |
| `crates/cli/src/pre_admission_data.rs:2176` | BENIGN-MATCH | decoder refusing arm (unknown V2 record kind) |
| `crates/cli/src/pre_admission_data.rs:3385` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/pre_admission_data.rs:3696` | BENIGN-OPTION | cfg(not(unix)) generation stamp; not compiled on unix |
| `crates/cli/src/research.rs:200` | BENIGN-OPTION | inventory count: absent key means zero files found |
| `crates/cli/src/research_policy.rs:165` | BENIGN-MATCH | non-boolean value refused |
| `crates/cli/src/research_policy.rs:222` | SURFACED | parse failure refused via ok_or_else at :224 |
| `crates/cli/src/result_set.rs:88` | BENIGN-INFALLIBLE | constant range of fixed [u8;STRIDE] array (a miss would panic, not default) |
| `crates/cli/src/result_set.rs:91` | BENIGN-INFALLIBLE | constant range of fixed [u8;STRIDE] array |
| `crates/cli/src/result_set.rs:94` | BENIGN-INFALLIBLE | constant range of fixed [u8;STRIDE] array |
| `crates/cli/src/result_set.rs:104` | BENIGN-INFALLIBLE | constant range of fixed [u8;STRIDE] array |
| `crates/cli/src/result_set.rs:111` | BENIGN-INFALLIBLE | constant range of fixed [u8;STRIDE] array |
| `crates/cli/src/result_set.rs:122` | BENIGN-INFALLIBLE | constant 8-byte range of fixed array; try_into cannot fail |
| `crates/cli/src/result_set.rs:123` | BENIGN-INFALLIBLE | constant 8-byte range of fixed array; try_into cannot fail |
| `crates/cli/src/result_set.rs:127` | BENIGN-INFALLIBLE | constant 8-byte range of fixed array; try_into cannot fail |
| `crates/cli/src/result_set.rs:128` | BENIGN-INFALLIBLE | constant 8-byte range of fixed array; try_into cannot fail |
| `crates/cli/src/result_set.rs:130` | BENIGN-INFALLIBLE | fixed index 48; fallback 0 would be refused by direction_of anyway |
| `crates/cli/src/result_set.rs:131` | BENIGN-INFALLIBLE | fixed index 49; fallback 0 would be refused by policy_of anyway |
| `crates/cli/src/result_set.rs:147` | BENIGN-MATCH | unknown direction byte refused |
| `crates/cli/src/result_set.rs:162` | BENIGN-MATCH | unknown trade-policy byte refused |
| `crates/cli/src/result_set.rs:170` | BENIGN-INFALLIBLE | constant range of fixed array |
| `crates/cli/src/result_set.rs:173` | BENIGN-INFALLIBLE | 32-byte digest prefix of 8 bytes |
| `crates/cli/src/result_set.rs:372` | BENIGN-INFALLIBLE | HashMap capacity hint only |
| `crates/cli/src/result_set.rs:659` | BENIGN-INFALLIBLE | usize->u64 byte count |
| `crates/cli/src/result_set.rs:706` | BENIGN-INFALLIBLE | read<=buffer.len() |
| `crates/cli/src/result_set.rs:707` | BENIGN-INFALLIBLE | usize->u64 byte count |
| `crates/cli/src/result_set.rs:1045` | BENIGN-INFALLIBLE | constant range of fixed header array |
| `crates/cli/src/result_set.rs:1049` | BENIGN-INFALLIBLE | constant range of fixed header array |
| `crates/cli/src/result_set.rs:1085` | BENIGN-INFALLIBLE | fixed header slice; fallback 0 can only refuse at :1088 |
| `crates/cli/src/result_set.rs:1086` | BENIGN-INFALLIBLE | fixed header slice; fallback 0 can only refuse at :1088 |
| `crates/cli/src/results.rs:494` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes (indexing); try_into cannot fail |
| `crates/cli/src/results.rs:501` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:502` | BENIGN-INFALLIBLE | take(1) non-empty; first() always Some |
| `crates/cli/src/results.rs:503` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:504` | BENIGN-INFALLIBLE | take(1) non-empty; first() always Some |
| `crates/cli/src/results.rs:505` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:506` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:507` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:508` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:509` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:510` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:511` | BENIGN-INFALLIBLE | take(1) non-empty; first() always Some |
| `crates/cli/src/results.rs:512` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:513` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:514` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:515` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:516` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:517` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:518` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:519` | BENIGN-INFALLIBLE | take(n) returns exactly n bytes; try_into cannot fail |
| `crates/cli/src/results.rs:589` | BENIGN-OPTION | no NUL pad means the 16-byte field is full |
| `crates/cli/src/results.rs:590` | BENIGN-INFALLIBLE | end<=raw.len(); seal-covered display field |
| `crates/cli/src/results.rs:687` | SURFACED | header read-back failure refused at results.rs:688-698 |
| `crates/cli/src/results.rs:989` | BENIGN-INFALLIBLE | fixed header slice; fallback 0 can only refuse at version check |
| `crates/cli/src/results.rs:990` | BENIGN-INFALLIBLE | fixed header slice; fallback 0 can only refuse at version check |
| `crates/cli/src/results.rs:1077` | BENIGN-INFALLIBLE | HashMap capacity hint only |
| `crates/cli/src/scrub_blocks.rs:293` | BENIGN-OPTION | no newline means no whole line |
| `crates/cli/src/scrub_blocks.rs:336` | BENIGN-MATCH | strict parser: not a cursor line; resume refuses on None (scrub_blocks.rs:417-421) |
| `crates/cli/src/scrub_blocks.rs:342` | SURFACED | digits-only u64 overflow -> None -> resume refuses (scrub_blocks.rs:417-421) |
| `crates/cli/src/scrub_blocks.rs:374` | BENIGN-OPTION | no " why=" suffix means the whole line is the head |
| `crates/cli/src/scrub_blocks.rs:377` | BENIGN-OPTION | run-id scan for id allocation; clock seconds also bound the id (scrub_blocks.rs:717-725) |
| `crates/cli/src/scrub_blocks.rs:379` | BENIGN-OPTION | run-id scan for id allocation; clock seconds also bound the id (scrub_blocks.rs:717-725) |
| `crates/cli/src/scrub_blocks.rs:572` | BENIGN-INFALLIBLE | usize->u64 byte count |
| `crates/cli/src/scrub_blocks.rs:579` | BENIGN-OPTION | lines without a run id are not run lines |
| `crates/cli/src/scrub_blocks.rs:581` | BENIGN-OPTION | empty ledger tail -> newest run 0 (documented :575) |
| `crates/cli/src/scrub_blocks.rs:640` | SURFACED | unaddressable census entry counted and named in Unaddressed (scrub_blocks.rs:643-659) |
| `crates/cli/src/scrub_blocks.rs:732` | BENIGN-OPTION | missing commit printed as explicit "unstamped" |
| `crates/cli/src/scrub_blocks.rs:734` | BENIGN-OPTION | None cursor written as documented "-" token (scrub_blocks.rs:313-314) |
| `crates/cli/src/scrub_blocks.rs:881` | BENIGN-OPTION | None cursor written as documented "-" token |
| `crates/cli/src/scrub_blocks.rs:1012` | BENIGN-OPTION | None cursor written as documented "-" token |
| `crates/cli/src/scrub_blocks.rs:1023` | BENIGN-INFALLIBLE | throttle sleep bound saturates |
| `crates/cli/src/scrub_blocks.rs:1051` | SURFACED | unparsable flag value -> None -> refused MISUSED at scrub_blocks.rs:1055-1057 |
| `crates/cli/src/scrub_blocks.rs:1055` | BENIGN-MATCH | unknown/malformed flag refused MISUSED |
| `crates/cli/src/search_checkpoint.rs:595` | SURFACED | fixed header slice; failure refused via ok_or at :597 |
| `crates/cli/src/selection.rs:1071` | BENIGN-INFALLIBLE | end<=len by min(); get never misses |
| `crates/cli/src/selection.rs:1123` | BENIGN-INFALLIBLE | u64->usize on 64-bit then min(MAX_TOP); mismatch refuses |
| `crates/cli/src/selection.rs:1479` | BENIGN-INFALLIBLE | end<=len by min(); get never misses |
| `crates/cli/src/selection.rs:1532` | BENIGN-INFALLIBLE | u64->usize on 64-bit then min(MAX_TOP); mismatch refuses |
| `crates/cli/src/selection.rs:2535` | BENIGN-INFALLIBLE | u64->usize on 64-bit then min(MAX_TOP); mismatch refuses |
| `crates/cli/src/selection.rs:3400` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/selection.rs:3445` | BENIGN-INFALLIBLE | nibble index 0..=15 into 16-digit table |
| `crates/cli/src/selection.rs:3451` | BENIGN-INFALLIBLE | nibble index 0..=15 into 16-digit table |
| `crates/cli/src/selection_v3.rs:524` | BENIGN-INFALLIBLE | end<=len by min(); get never misses |
| `crates/cli/src/selection_v3.rs:579` | BENIGN-INFALLIBLE | u64->usize on 64-bit then min(MAX_TOP); mismatch refuses |
| `crates/cli/src/selection_v3.rs:960` | BENIGN-INFALLIBLE | u64->usize on 64-bit then min(MAX_TOP); mismatch refuses |
| `crates/cli/src/selection_v3.rs:1829` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/selection_v3.rs:1875` | BENIGN-INFALLIBLE | nibble index 0..=15 into 16-digit table |
| `crates/cli/src/selection_v3.rs:1881` | BENIGN-INFALLIBLE | nibble index 0..=15 into 16-digit table |
| `crates/cli/src/selection_v4.rs:158` | BENIGN-MATCH | by_index only called with 0..4 (selection_v4.rs:573-574); 0 arm unreachable |
| `crates/cli/src/selection_v4.rs:184` | BENIGN-MATCH | by_index only called with 0..2 (selection_v4.rs:580-581); 0 arm unreachable |
| `crates/cli/src/selection_v4.rs:495` | BENIGN-INFALLIBLE | exhaustive index fns (selection_v4.rs:2351-2365) into fixed [[u64;2];4] |
| `crates/cli/src/selection_v4.rs:515` | BENIGN-INFALLIBLE | flatten over fixed [[u64;2];4] array, not Results |
| `crates/cli/src/selection_v4.rs:537` | BENIGN-INFALLIBLE | index 0..2 into [u64;2] rows |
| `crates/cli/src/selection_v4.rs:591` | BENIGN-INFALLIBLE | flatten over fixed [[u64;2];4] array, not Results |
| `crates/cli/src/selection_v4.rs:1025` | BENIGN-INFALLIBLE | end<=len by min(); get never misses |
| `crates/cli/src/selection_v4.rs:1083` | BENIGN-INFALLIBLE | u64->usize on 64-bit then min(MAX_TOP); mismatch refuses |
| `crates/cli/src/selection_v4.rs:1524` | BENIGN-INFALLIBLE | u64->usize on 64-bit then min(MAX_TOP); mismatch refuses |
| `crates/cli/src/selection_v4.rs:2394` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/selection_v4.rs:2442` | BENIGN-INFALLIBLE | nibble index 0..=15 into 16-digit table |
| `crates/cli/src/selection_v4.rs:2448` | BENIGN-INFALLIBLE | nibble index 0..=15 into 16-digit table |
| `crates/cli/src/selection_v4_authority.rs:599` | BENIGN-INFALLIBLE | nibble index 0..=15 into 16-digit table |
| `crates/cli/src/selection_v4_authority.rs:605` | BENIGN-INFALLIBLE | nibble index 0..=15 into 16-digit table |
| `crates/cli/src/selection_v5.rs:450` | SURFACED | rank out of range refused at selection_v5.rs:457-459 |
| `crates/cli/src/selection_v5.rs:1055` | SURFACED | rank/order mismatch refused at selection_v5.rs:1059-1061 |
| `crates/cli/src/selection_v5.rs:1730` | BENIGN-INFALLIBLE | fixed [u64;16] matrix (TERMINAL_MATRIX_CELLS=16, :85) |
| `crates/cli/src/selection_v5.rs:1739` | BENIGN-INFALLIBLE | fixed [u64;16] matrix (TERMINAL_MATRIX_CELLS=16, :85) |
| `crates/cli/src/selection_v5.rs:1752` | BENIGN-INFALLIBLE | fixed [u64;16] matrix (TERMINAL_MATRIX_CELLS=16, :85) |
| `crates/cli/src/selection_v5.rs:2100` | BENIGN-OPTION | no trailing prefix == empty trailing block at current end |
| `crates/cli/src/selection_v5.rs:2644` | SURFACED | rank/order mismatch refused (same predicate block) |
| `crates/cli/src/selection_v5.rs:2683` | SURFACED | rank/order mismatch refused (same predicate block) |
| `crates/cli/src/selection_v5.rs:3311` | BENIGN-MATCH | unknown optional tag refused |
| `crates/cli/src/selection_v5.rs:3341` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/selection_v5.rs:3356` | BENIGN-MATCH | decoder refusing arm (unknown direction) |
| `crates/cli/src/selection_v5.rs:3373` | BENIGN-MATCH | decoder refusing arm (unknown admission) |
| `crates/cli/src/selection_v5.rs:3390` | BENIGN-MATCH | decoder refusing arm (unknown terminal) |
| `crates/cli/src/selection_v6.rs:405` | SURFACED | length mismatch -> None -> caller refuses at selection_v6.rs:436-440 |
| `crates/cli/src/selection_v6.rs:409` | BENIGN-INFALLIBLE | read error propagated by ? at :412; only the byte count is discarded |
| `crates/cli/src/selection_v6_read.rs:331` | BENIGN-MATCH | invalid definedness word refused |
| `crates/cli/src/selection_v6_source.rs:292` | BENIGN-MATCH | contradictory terminal refused |
| `crates/cli/src/selection_v6_source.rs:410` | BENIGN-MATCH | unmeasured profit allowed only for non-admitted rows (admitted refused :407-408) |
| `crates/cli/src/selection_v6_source.rs:435` | BENIGN-MATCH | unmeasured metric allowed only for non-admitted rows (admitted refused :432-433) |
| `crates/cli/src/selection_v6_source.rs:538` | BENIGN-OPTION | ratio encoded as (defined flag, value); None is (0,0) as decoder expects |
| `crates/cli/src/series_identity.rs:125` | BENIGN-INFALLIBLE | constant YearMonth::new(2000,1) |
| `crates/cli/src/series_identity.rs:154` | BENIGN-MATCH | display "none" for an empty month list |
| `crates/cli/src/series_identity.rs:160` | BENIGN-OPTION | display: unconvertible day printed as "IST day N" |
| `crates/cli/src/series_identity.rs:161` | BENIGN-OPTION | display: unconvertible day printed as "IST day N" |
| `crates/cli/src/series_identity.rs:271` | SWALLOWED | ReadDir entry errors dropped; EVERY-SERIES audit table omits entry and can exit OK |
| `crates/cli/src/series_identity.rs:273` | BENIGN-OPTION | non-Symbol directory names cannot be a loadable series (Symbol::new uppercases) |
| `crates/cli/src/srt_builder.rs:260` | BENIGN-INFALLIBLE | 4/2 ASCII digits always parse as u16; None refused at :266-271 |
| `crates/cli/src/srt_builder.rs:261` | BENIGN-INFALLIBLE | 4/2 ASCII digits always parse as u16; None refused at :266-271 |
| `crates/cli/src/srt_builder.rs:272` | BENIGN-INFALLIBLE | 2-digit month <=99 fits u8; YearMonth::new validates at :273 |
| `crates/cli/src/srt_builder.rs:306` | BENIGN-MATCH | malformed instrument path refused |
| `crates/cli/src/srt_builder.rs:327` | BENIGN-INFALLIBLE | endpoints validated by YearMonth::new; interior months 1..=12 from months_between |
| `crates/cli/src/srt_builder.rs:502` | SWALLOWED | dry run: non-NotFound lstat error read as "name absent"; can print would-build where build refuses |
| `crates/cli/src/srt_builder.rs:545` | BENIGN-INFALLIBLE | writeln! to String |
| `crates/cli/src/srt_builder.rs:617` | BENIGN-OPTION | months non-empty (from<=to validated); "" unreachable |
| `crates/cli/src/srt_builder.rs:621` | BENIGN-OPTION | months non-empty (from<=to validated); "" unreachable |
| `crates/cli/src/stability.rs:142` | BENIGN-MATCH | inner match: only Year reaches this arm |
| `crates/cli/src/stability.rs:165` | BENIGN-INFALLIBLE | civil month is 1..=12 by construction |
| `crates/cli/src/stability.rs:226` | BENIGN-INFALLIBLE | usize->i64 bucket count |
| `crates/cli/src/stability.rs:231` | BENIGN-INFALLIBLE | usize->i64 bucket count |
| `crates/cli/src/stability.rs:239` | BENIGN-OPTION | empty = no evidence, tested (stability.rs:529-535); sole caller returns None on empty (lib.rs:13426) |
| `crates/cli/src/stability.rs:253` | BENIGN-OPTION | empty = no evidence, tested (stability.rs:529-535) |
| `crates/cli/src/step3_orchestrator.rs:787` | SURFACED | row-count mismatch refused at step3_orchestrator.rs:788 |
| `crates/cli/src/step3_orchestrator.rs:1970` | SURFACED | cardinality mismatch refused at step3_orchestrator.rs:1971 |
| `crates/cli/src/step3_orchestrator.rs:2024` | SURFACED | cardinality mismatch refused at step3_orchestrator.rs:2025 |
| `crates/cli/src/stored.rs:143` | BENIGN-INFALLIBLE | count <= charter day list length |
| `crates/cli/src/stored.rs:168` | BENIGN-OPTION | keeps only days flagged removed |
| `crates/cli/src/stored.rs:192` | BENIGN-OPTION | display: raw day number on conversion failure, documented stored.rs:181-185 |
| `crates/cli/src/stored.rs:193` | BENIGN-OPTION | display: raw day number on conversion failure, documented stored.rs:181-185 |
| `crates/cli/src/stored.rs:594` | SURFACED | missing CAS flag recorded in unverified + digest (stored.rs:730-735); None withholds/refuses day (:3490-3497) |
| `crates/cli/src/stored.rs:596` | BENIGN-MATCH | non-full session on a CAS day answers None (refused, documented :585-588) |
| `crates/cli/src/stored.rs:667` | BENIGN-OPTION | unconvertible day is not a CAS-era day; calendar close applies |
| `crates/cli/src/stored.rs:668` | BENIGN-OPTION | unconvertible day is not a CAS-era day; calendar close applies |
| `crates/cli/src/stored.rs:709` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/stored.rs:732` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/stored.rs:1311` | BENIGN-OPTION | intervals within count proven present by finish() (stored.rs:1286-1288) |
| `crates/cli/src/stored.rs:1436` | BENIGN-MATCH | non-canonical rung refused |
| `crates/cli/src/stored.rs:1623` | BENIGN-OPTION | intervals within count proven present by finish() (stored.rs:1491) |
| `crates/cli/src/stored.rs:2415` | BENIGN-OPTION | display: unconvertible day printed as "IST day N" |
| `crates/cli/src/stored.rs:2416` | BENIGN-OPTION | display: unconvertible day printed as "IST day N" |
| `crates/cli/src/stored.rs:3139` | BENIGN-INFALLIBLE | month count bounded by MAX_SPAN_MONTHS |
| `crates/cli/src/stored.rs:3493` | SURFACED | text inside a refusal; reason fallback only when no unverified reason exists |
| `crates/cli/src/stored_data_completeness.rs:93` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/stored_data_completeness.rs:1036` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/stored_data_completeness.rs:1044` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/stored_data_completeness.rs:1062` | BENIGN-MATCH | decoder refusing arm (unknown family) |
| `crates/cli/src/stored_data_completeness.rs:1184` | BENIGN-INFALLIBLE | constant usize->u64 |
| `crates/cli/src/stored_data_completeness.rs:1185` | BENIGN-INFALLIBLE | constant usize->u64 |
| `crates/cli/src/stored_post_training_oos.rs:634` | BENIGN-INFALLIBLE | usize->u64 length prefix for a hash |
| `crates/cli/src/strict_range_config.rs:96` | BENIGN-MATCH | any missing/invalid setting refused with names |
| `crates/cli/src/strict_range_config.rs:140` | SURFACED | non-UTF-8 value -> pushed to invalid (strict_range_config.rs:143-145), no default |
| `crates/cli/src/strict_range_config.rs:141` | SURFACED | unparsable value -> pushed to invalid (strict_range_config.rs:143-145), no default |
| `crates/cli/src/strict_range_knobs.rs:64` | SURFACED | invalid switch -> false -> named in KnobRefusal (strict_range_knobs.rs:150-158) |
| `crates/cli/src/strict_range_knobs.rs:88` | BENIGN-MATCH | unknown/never-usable knob -> invalid -> refused (documented D-0685) |
| `crates/cli/src/strict_v6_inputs.rs:27` | BENIGN-OPTION | None = ordinary (non-strict) source with no guard to recheck (step3_orchestrator.rs:416,3367) |
| `crates/cli/src/strict_v6_inputs.rs:360` | SURFACED | failed result recorded as Completion::Refused terminal |
| `crates/cli/src/strict_v6_inputs.rs:370` | BENIGN-OPTION | Ok result text inside the terminal-persistence refusal message |
| `crates/cli/src/sweep_evidence.rs:199` | BENIGN-MATCH | decoder refusing arm (unknown operation) |
| `crates/cli/src/sweep_evidence.rs:242` | BENIGN-MATCH | decoder refusing arm (unknown completion) |
| `crates/cli/src/sweep_evidence.rs:282` | BENIGN-INFALLIBLE | usize->u64 count |
| `crates/cli/src/sweep_evidence.rs:283` | BENIGN-INFALLIBLE | usize->u64 count |
| `crates/cli/src/sweep_evidence.rs:649` | BENIGN-INFALLIBLE | usize->u64 index; mismatch refused at :650 |
| `crates/cli/src/sweep_evidence.rs:866` | SURFACED | journal failure propagated at sweep_evidence.rs:877; attempts left unrecorded |
| `crates/cli/src/sweep_evidence.rs:1199` | BENIGN-INFALLIBLE | flushed-path cache; inserts only after a successful barrier (:1188-1190) |
| `crates/cli/src/sweep_evidence.rs:1438` | BENIGN-MATCH | invalid ranked-availability byte refused |
| `crates/cli/src/sweep_evidence.rs:1444` | BENIGN-MATCH | invalid validation-request byte refused |
| `crates/cli/src/sweep_evidence.rs:1502` | BENIGN-INFALLIBLE | usize->u64 count; overflow refused at :1505 |
| `crates/cli/src/sweep_evidence.rs:1615` | BENIGN-INFALLIBLE | usize->u64 page limit |
| `crates/cli/src/tix_rebuilder.rs:144` | BENIGN-INFALLIBLE | 4/2 ASCII digits always parse as u16; None refused at :150-155 |
| `crates/cli/src/tix_rebuilder.rs:145` | BENIGN-INFALLIBLE | 4/2 ASCII digits always parse as u16; None refused at :150-155 |
| `crates/cli/src/tix_rebuilder.rs:156` | BENIGN-INFALLIBLE | 2-digit month <=99 fits u8; YearMonth::new validates at :157 |
| `crates/cli/src/tix_rebuilder.rs:196` | BENIGN-INFALLIBLE | endpoints validated by YearMonth::new; interior months 1..=12 |
| `crates/cli/src/tix_rebuilder.rs:349` | BENIGN-INFALLIBLE | writeln! to String |
| `crates/cli/src/tix_rebuilder.rs:415` | BENIGN-OPTION | months non-empty (from<=to validated); "" unreachable |
| `crates/cli/src/tix_rebuilder.rs:419` | BENIGN-OPTION | months non-empty (from<=to validated); "" unreachable |
| `crates/cli/src/trades.rs:327` | BENIGN-MATCH | unknown direction byte refused |
| `crates/cli/src/trades.rs:341` | BENIGN-INFALLIBLE | constant range of fixed array |
| `crates/cli/src/trades.rs:344` | BENIGN-INFALLIBLE | 8-byte prefix of a 32-byte digest |
| `crates/cli/src/trades.rs:711` | BENIGN-INFALLIBLE | usize->u64 row count |
| `crates/cli/src/trades.rs:714` | BENIGN-INFALLIBLE | usize->u64 byte count |
| `crates/cli/src/trades.rs:715` | BENIGN-INFALLIBLE | usize->u64 row count |
| `crates/cli/src/trades.rs:1076` | BENIGN-INFALLIBLE | Vec capacity hint only |
| `crates/cli/src/trades.rs:1130` | BENIGN-INFALLIBLE | usize->u64 row count; shortfall refused/named at :1131-1140 |
| `crates/cli/src/trades.rs:1180` | BENIGN-INFALLIBLE | fixed header slice; fallback 0 can only refuse at version check :1182 |
| `crates/cli/src/trades.rs:1268` | BENIGN-INFALLIBLE | HashMap capacity hint only |
| `crates/cli/src/trades.rs:1319` | BENIGN-INFALLIBLE | constant range of fixed array (row already seal-checked) |
| `crates/cli/src/trades.rs:1355` | BENIGN-INFALLIBLE | write_fmt to String |
| `crates/cli/src/vix_reference.rs:232` | BENIGN-OPTION | empty slot is the explicit VixStamp::Absent variant; mismatched slot refused :235-239 |
| `crates/cli/src/zerodha_repair.rs:83` | BENIGN-INFALLIBLE | fixed-width ASCII digits always parse as u16; None refused at :91-96 |
| `crates/cli/src/zerodha_repair.rs:84` | BENIGN-INFALLIBLE | fixed-width ASCII digits always parse as u16; None refused at :91-96 |
| `crates/cli/src/zerodha_repair.rs:89` | BENIGN-MATCH | wrong part count -> refused at :91-96 |
| `crates/cli/src/zerodha_repair.rs:99` | BENIGN-INFALLIBLE | 2-digit value <=99 fits u8; Day::new validates at :97-102 |
| `crates/cli/src/zerodha_repair.rs:100` | BENIGN-INFALLIBLE | 2-digit value <=99 fits u8; Day::new validates at :97-102 |
| `crates/cli/src/zerodha_repair.rs:125` | BENIGN-MATCH | unknown segment refused |
| `crates/cli/src/zerodha_repair.rs:179` | BENIGN-MATCH | half-given repair flags refused |
| `crates/cli/src/zerodha_repair.rs:440` | SWALLOWED | ReadDir entry errors dropped; EVERY-SERIES repair plan omits entry and can exit OK |
| `crates/cli/src/zerodha_repair.rs:442` | BENIGN-OPTION | non-Symbol directory names cannot be a loadable series (Symbol::new uppercases) |

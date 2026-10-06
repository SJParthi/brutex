# 02 — Store format, versions 2 and 3

Bytes on disk. This document is the authority; the code follows it.

The version number is in the header. A format change mints a **new version**
and old files stay readable at their own stride. Nothing is ever mutated in
place — `CLAUDE.md` §3 rule 8.

**Version 1 is retired.** §10 says what it was and why it is refused rather
than decoded. Its number is never reused.

**Version 3 is the version written since D-1571.** It is version 2's geometry
byte for byte; what it adds is that block checksums are mandatory (§2.1).
Version 2 files stay readable at their own row and are never rewritten as 3.

---

## 1. File layout

```
byte 0                  32768                                       EOF
  ├──── header region ────┼─── record 0 ─┼─── record 1 ─┼── … ───────┤
   2 slots × 16384 spacing    56 bytes       56 bytes
```

**Address of record *i*:**

```
ptr = base + 32768 + i * 56
```

An add, a multiply, a load. No search, no decode, no allocation.

The two numbers in that formula are not constants on the read path. They come
from `store::layout::Layout`, selected by the file's own `format_version`.

---

## 2. Header region — two slots, one write each

The header region is `slot_count × 16384` bytes. Each slot carries **64 bytes
of fields** at the start of its 16384-byte span; the rest of the span is
reserved and zero.

Commit *g* is written to slot `g % slot_count`, so consecutive commits never
touch the same slot, and a reader takes the valid slot with the highest
generation that the file's length supports.

**Why the slots are 16384 bytes apart and not adjacent.** Storage does not fail
at byte granularity. The smallest unit a device programs or a filesystem
writes back is a block or a page, and a partially programmed unit takes
everything in it. Two slots 64 bytes apart share one such unit, which makes the
redundancy nominal. Measured on the two hosts this repository builds on:

| Host | Device block | Page |
|---|---|---|
| Apple Silicon, macOS, APFS | 4096 | 16384 |
| GitHub CI runner, x86_64 Linux, ext4 | 4096 | 4096 |

16384 is the largest of those. It is a constant rather than a query of the
host: a geometry that differed between the two would make the same bytes verify
differently on each (§3 rule 5).

### One slot — 64 bytes

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | 8 | `magic` | `b"BRUTEXB3"` (version 3) or `b"BRUTEXB2"` (version 2) |
| 8 | 2 | `format_version` | `3`, or `2` for a file written before D-1571 |
| 10 | 2 | `record_stride` | `56`. Read it; never assume it. |
| 12 | 4 | `flags` | bit 0: block checksums present. Every other bit is zero; a set one is refused on read and on commit (D-1354) |
| 16 | 8 | `generation` | which commit this slot holds. Higher wins. |
| 24 | 8 | `n_valid` | **the commit counter.** See §5. |
| 32 | 8 | `first_ts_micros` | of record 0. Meaningful only when `n_valid > 0`. |
| 40 | 8 | `last_ts_micros` | of record `n_valid-1` |
| 48 | 4 | `symbol_id` | resolved from the path; a cross-check, not the index |
| 52 | 4 | `timeframe_secs` | 60 for 1-minute |
| 56 | 4 | `slot_crc` | CRC-32C over bytes 0..56 **and** 60..64 |
| 60 | 4 | reserved | zero; a non-zero value is refused (D-1353) |

**Every integer in the slot is little-endian**, whatever the host: the
encoder writes each field with `to_le_bytes` and the decoder reads it with
`from_le_bytes` (tests-docs-security-pass14 P14-03, D-1959).

The checksum covers every byte of the slot except the four it occupies, so a
flipped bit anywhere in the 64 is detected — there is no window a corruption
can land in and be called clean.

Reserved bytes are zero and stay zero. A future field takes reserved space in
a **new version**, never by reinterpreting version 2.

The 64-byte slot size and the 16384-byte slot spacing are **family** constants:
every version uses them, which is what lets a reader locate the slots before it
knows which version the file is.

### 2.1 Version 3 — block checksums are mandatory (D-1571)

At version 2, bit 0 of `flags` decides whether a month is verified. Clearing it
in both slots and recomputing their CRCs turned verification off for a sealed
month, and the `.crc` beside it was ignored (audit-20261003 attackdata-8);
random rot cannot do it, because the slot CRC covers the flag, but a deliberate
rewrite can.

Version 3 closes that without touching version 2's bytes:

- **Same geometry.** Header region, slot offsets, 56-byte stride, 73-record
  blocks and the `.crc` sidecar are version 2's exactly. Only `magic`
  (`BRUTEXB3`), `format_version` (`3`) and one rule differ.
- **The rule.** A version-3 slot whose `flags` lacks bit 0 is refused on read
  (`FormatError::ChecksumsRequired(3)`) and is never committed. A version-3
  month is therefore always verified, and a sealed month whose `.crc` is
  missing is refused as `ChecksumsMissing`, as at version 2.
- **Written.** `store::layout::Layout::CURRENT` is version 3, so every month
  created from D-1571 on is born at it.
- **Version 2 stays readable.** A `.bin` resolves against both rows; a
  version-2 month is opened, appended to and sealed at version 2 for its whole
  life, flag optional as before. No file is upgraded in place (`CLAUDE.md`
  §3 rule 8).
- **What it does not close.** The CRC is integrity, not authentication. An
  actor who rewrites `magic` and `format_version` back to version 2 in both
  slots and recomputes their CRCs can still present an unsealed version-2
  month; one who can do that can equally rewrite the `.crc`. Stated in
  `docs/06-limits.md`.

---

## 3. Record — 56 bytes, seven `i64`

```rust
#[repr(C)]
pub struct Bar {
    pub ts_micros:      i64,  //  0  microseconds since Unix epoch, UTC
    pub open:           i64,  //  8  paisa
    pub high:           i64,  // 16  paisa
    pub low:            i64,  // 24  paisa
    pub close:          i64,  // 32  paisa
    pub volume:         i64,  // 40  contracts or shares; 0 is a real zero
    pub open_interest:  i64,  // 48  paisa-free count; i64::MIN means null
}

const _: () = assert!(size_of::<Bar>() == 56);
const _: () = assert!(align_of::<Bar>() == 8);
```

**On disk the seven fields are little-endian `i64`s at offsets 0, 8, 16, 24,
32, 40 and 48**, written by `Bar::image` with `to_le_bytes`. The `#[repr(C)]`
above fixes the field *order*, not the byte order: the record is not a
native-endian memory image, and a reader on a big-endian host still decodes it
little-endian (P14-03, D-1959).

**Prices are paisa integers.** Never a float, at any layer, for any reason.
A float price is how a rounding difference becomes a divergent result set six
months later.

**The prices are the vendor's, as served.** The store snaps each price to the
paisa grid at the write boundary and changes nothing else: it applies no split,
bonus, dividend or other corporate-action adjustment, and it records no
adjustment basis. Whether a vendor's cash-equity candles arrive already
adjusted is UNVERIFIED for every feed (`docs/00-charter.md` §4), so a month
pulled before an action and one pulled after it may sit here on different
bases with nothing to tell them apart. numeric-pass16 p16num-2, D-1969.

**`i64::MIN` is the open-interest null sentinel.** Zero means zero. Spot
indices carry no open interest and store the sentinel; conflating that with a
genuine zero would make a derivative series and an index series indistinguish-
able on a field that matters.

**A record has no structure that a lost write violates.** An all-zero record is
a legal flat bar whose open interest is a real zero. Nothing about the record
can tell it apart from an extent that was never written; that is §6's job, and
it is why the checksum flag is not decoration.

---

## 4. Reading — one positional read, no mapping

**No crate maps a bar file.** This section used to say the file was mapped
read-only and that a read was pointer arithmetic over resident pages. No build
has done that: `crates/store/src/lib.rs` carries `#![forbid(unsafe_code)]`,
which a memory mapping needs, and `crates/store/Cargo.toml` does not name
`memmap2`. A read is `BarFile::read_record` → `read_row` → `read_fully` →
`Positional::get`, which is `FileExt::read_at` — one `pread` of one record's
bytes. `store::docs::section_4_names_the_positional_read_and_claims_no_mapping`
reads this paragraph and those source lines together. D-0790.

**Writes never go through a mapping either.** They use positional writes
(`pwrite`), which return an error on a full disk.

This is not a preference. A writable mapping that runs out of space raises
**SIGBUS** — a signal, delivered asynchronously, catchable by no language
construct in any language. Every clean disk-full halt the system has is
disabled the moment a writable mapping is introduced. The predecessor system
halted correctly on a full disk; a naive port to a writable mapping would have
been strictly worse, and that is the single most valuable thing the failure
audit produced.

`Header::decode` copies a slot's 64 bytes **once** before it checks anything, so the checksum covers exactly the bytes the decoded header
carries. A copy that caught a concurrent `pwrite` mid-flight fails that
checksum rather than returning half of each image.

---

## 5. Appending — the commit counter

```
1. pwrite the new records at offset  32768 + n_valid * 56
2. fsync the data, through Commit::durable_through
3. pwrite the affected block checksums into the .crc sidecar, then fsync it
3b. pwrite the time-index entries the batch touches into the .tix, then fsync it
4. pwrite the 64-byte header slot for generation g   <- one write
5. fsync the header
```

Step 3b is D-2329's and §8.1 describes it. Like step 3 it lands before the
slot that commits the bars it describes, so a committed bar is never missing
from the index; a crash between 3b and 5 leaves index entries AHEAD of the
commit, which the next append masks off and overwrites (§8.1). A batch the
index cannot hold — a second daily bar on one IST day — is admitted exactly as
before; the `.tix` is removed before step 1 and the month bisects, loudly
(D-2330).

**This list used to put the sidecar before the data `fsync`.** The writer has
always done it in the order above (`BarFile::append`: records, `sync_all`,
`seal_committed`, header slot, `sync_all`), and the order matters to the next
paragraph. D-0688.

A reader treats records `0 .. n_valid` as the whole file. Bytes past `n_valid`
are, by definition, not there yet.

The writer also `fsync`s the month's directory after it creates the `.crc`
sidecar, which it does only for a stream with no committed records. The
sidecar's own `fsync` makes its bytes durable, not its name, and a sealed month
that loses the name after its first commit is refused forever: a writer may
not recreate proof for existing records (§8). D-0688.

**And every directory the writer creates has its entry flushed in its parent**
(D-1522). Creating a month can make up to six directories below the store root
(vendor .. rung); before D-1522 only the month's own directory was synced, so a
crash could lose a directory whose month had returned `Committed`. **The store
root itself is never created by the writer**: a missing root is refused by
name, so a store on an unmounted volume cannot silently grow a second root on
the parent filesystem (D-1522, the bar-writer half of D-0954).

**A month whose header region is gone is re-initialised only when nothing was
ever committed** (D-1520, D-1521). The writer repairs a file of at most 32,768
bytes whose bytes past the first slot are zero and whose first slot does not
decode — what an interrupted `initialise` leaves, all zeros or a torn genesis
slot — and only when the checksum sidecar beside it is absent or empty. A
sidecar with entries proves an append committed records, so such a file is
refused as `CommittedRecordsLost` instead of being reopened empty.

Consequences:

* **A torn record is never served, and it is not always unobservable.** This
  bullet used to say "unobservable", and that was false for one window. A crash
  between steps 1 and 3 leaves bytes on disk that no reader serves; the next
  append overwrites them. A crash after step 3 has reached the disk and
  before step 5 completes leaves the same bytes AND the tail block's sidecar
  entry sealed over them, so the entry no longer matches the committed
  extent. A reader admits that block only on the positive proof §6
  describes, and logs the interrupted append. Until D-0688 it refused the
  block, and every overlapping re-offer that read it first — a re-pull, or a
  derived rung folded again — was refused with it.
* **A torn header is unobservable.** Step 4 is one write of one self-checked
  64-byte unit, into the slot that does *not* hold the previous commit. A crash
  during it leaves a slot that fails its own checksum, and the reader takes the
  previous generation.
* **A header that outran its data is refused, not served short.** If the slot
  becomes durable and the records do not, the counter names records the length
  cannot support. `Header::read_region` walks back to an older generation, and
  `BarFile::validated` — which every open door goes through — then compares
  that generation's counter with the highest counter any surviving slot still
  decodes, and refuses with `FormatError::CounterExceedsFile` when a slot
  claims more. This bullet used to promise the fallback: that the file would
  open one generation short. `append` makes records durable before it writes
  a slot that names them, so the case needs a truncation or a lost write, and
  opening short would drop committed bars without a word.
  `store::write::a_truncation_back_to_the_header_is_refused_rather_than_silently_accepted`
  and `store::write::a_counter_behind_its_bytes_opens_and_a_counter_ahead_of_them_is_refused`
  assert the refusal. D-0790.

`Commit::durable_through` is the byte offset step 2 must cover. **It states the
offset; `crates/store/src/file.rs` issues the barrier** — `sync_all` on the
records before any slot that names them, and a second one after. This paragraph
read "this repository performs no I/O yet, so it states the offset rather than
issuing the barrier", which was true of the format crate alone and stopped being
true of the repository when `BarFile` learned to write.

**Exactly one writer per file.** Two writers reading the same generation
produce the same slot, the same offset and the same record range. **The writer
takes an advisory `try_lock` on the `.lock` sibling (§8) before step 1** and
refuses loudly when it is held. The clause "nothing in the store crate can
enforce that — an exclusive lock is I/O" stood in front of that sentence and
contradicted it; the lock is real, and what it does *not* provide is
cross-machine exclusion on a network filesystem, which is the honest limit and
the one worth stating.

**A stamp the path rules out is refused before step 1 (D-0915).** The path's
`<yyyy-mm>` and `<tf>` segments bind the records: every offered stamp must lie
in `[00:00 IST on the 1st, 00:00 IST on the 1st of the next month)` and on the
rung's grid — intraday rungs anchored at 09:15 IST as `pull::fold` cuts them,
the `1day` rung any whole second. A refusal (`OutsideMonth`, `OffGrid`) writes
nothing. Session hours are not part of this check; `docs/06-limits.md` records
why. No byte of the format changes, and files written before the check are
read exactly as before.

---

## 6. Integrity — one checksum per block of 73 records

A block is **73 whole records = 4088 bytes**, counted in records rather than in
bytes, and anchored at the end of the header region:

```
block_of(i)  =  i / 73
block start  =  32768 + block * 4088
```

Because the block length is a whole multiple of the stride, **a record can
never begin in one block and end in the next**. Verifying a record reads one
checksum, never two, whatever its index.

The earlier byte-addressed 4096-byte block did not divide 56: 4096/56 = 73.14,
so 23 of the first 2,000 records straddled a boundary and verifying one of them
against either block checked part of a record and pronounced the whole of it
good.

**Why not the padded 4096-byte block** this document used to describe ("73
whole records with 8 bytes of slack")? It also never straddles — but a block is
a range of the file, so padding the block pads the record array, and the
address of record *i* stops being `base + 32768 + i·56` and becomes
`base + 32768 + (i/73)·4096 + (i%73)·56`: a division and two multiplies instead
of a multiply and an add. The 8 bytes are wasted either way, so the padded form
buys nothing for the cost.

### The tail block covers the committed prefix, not the nominal block

The last block of a file holds only the records the commit counter covers. Its
checksum is taken over

```
(n_valid mod 73) * 56 bytes  from the block start
```

and never over the nominal 4088 — which for 72 of every 73 file states would
name bytes past EOF. The writer recomputes the tail block's checksum on every
commit that lands in it; the reader verifies against the `n_valid` it read from
the header. Both sides derive the length from the same counter.

### A tail entry sealed past the commit — D-0688

The one exception to "the same counter" is a crash between §5 step 3 and step
5: the sidecar entry for the tail block was sealed at `n_valid + k`, and the
header still says `n_valid`. The format does not change. The reader, on a tail
block whose entry does not match the committed extent, tries the longer
extents: the committed bytes followed by 1, 2, … whole records the file really
holds past `offset_of(n_valid)`, never past the block's nominal end. If one of
them has the stored CRC-32C, the committed records are served and a
`store.block` warning names the interrupted append and the extent matched.
Every other mismatch is refused as before: a full block, a tail with no whole
record past the commit, or records past it that match nothing.

It is a proof and not a fallback, because the committed bytes are inside every
candidate. A CRC-32C detects every error burst of up to 32 bits, so such a
flip in a committed byte cannot match the extent that was really sealed. Any
other candidate matches only by a 32-bit collision. A tail block with `k`
whole records past its commit is compared `k + 1` times instead of once, and
`k` is at most 72 at this geometry, so a random corruption of such a block is
admitted with probability about `k + 1` in 2^32 instead of 1 in 2^32. A block
with nothing past its commit is compared once, as before.
`docs/06-limits.md` states the cost.

Each block's CRC-32C lives in a sidecar at the same **block index**:

```
bars/groww/NSE/INDEX/NIFTY/1min/2024-03.bin
bars/groww/NSE/INDEX/NIFTY/1min/2024-03.crc
```

The sidecar is indexed by `i / 73`, not by `(32768 + i*56) / 4096`.

**The sidecar's bytes.** `.crc`, `.ovl.crc` and `.grk.crc` share one layout,
each at its own family's records per block (73 bars, 170 overlay rows, 51 Greek
rows):

| Property | Value |
|---|---|
| Header, magic, version | none. The file is entries only. |
| Entry for block *b* | 4 bytes at byte offset `4·b` |
| Entry encoding | `u32`, little-endian |
| Entry value | CRC-32C of the block's **covered** bytes: from the block start (`32768 + b · block_len`) for the committed records only, so a partial tail block covers `(n_valid mod records_per_block) · stride` bytes (§6 above) |
| CRC-32C parameters | Castagnoli polynomial, reflected form `0x82F63B78`; register initialised to `0xFFFFFFFF` and the result inverted (`xorout 0xFFFFFFFF`); check value `0xE3069283` for `b"123456789"` (`store::crc::CHECK_VALUE`) |
| Blocks a reader consults | `0 .. blocks_for(n_valid)`, the blocks the header's commit counter reaches. Bytes past the last of them are not read. |

The writer seals each block an append reached with one 4-byte positional write
at `4·b` (`store::file`, "FOUR BYTES AT `block * 4`"); the reader verifies one
block with one 4-byte positional read at the same offset. Until
tests-docs-security-pass14 P14-02 (D-1959) these facts were stated only in
Rust comments, and this document named the checksum and nothing about where an
entry is or how it is encoded.

**Why this is not optional.** A flipped bit in a raw `i64` price produces a
different price — a *plausible* one. There is no structure to violate, no
parse to fail. Without a checksum the corruption is silent and permanent, and
every downstream result derived from it is wrong in a way nobody can detect.
It is also the only thing that can tell a lost write from 73 flat bars.

A file whose `flags` bit 0 is clear carries no sidecar, and a verification
request against it is **refused** rather than answered — "verified" and "there
was nothing to verify against" are different answers.

Verification on the read path is one block CRC per read, not a file scan:
`read_row` calls `verify_block_of` for the record's own block. What C-07
measures is `block::seal` over a block held in memory — sealing one block
costs the same at 1×, 10× and 100× the file's record count. This paragraph used
to cite C-07 as enforcing the per-read bound; C-07 does not read a file, so the
read path's check is the source's shape, and C-28 and C-29 time
`read_record` end to end. D-0790.

**Read "O(1)" precisely.** The *operation* is constant because a block is a
fixed 4,088 bytes; the CRC inside it still reads every one of them, and always
will. Measured on an Apple M4 Pro after D-0032: 1,485.0 ns for `block::seal` end
to end, 20.3 ns amortised over the 73 records a block covers. Before D-0032 the
same seal cost 15,291.0 ns. `docs/06-limits.md` §14 carries the full numbers and
states what is not claimed.

---

## 7. Opening — boundary check

```
if len > offset_of(n_valid) {
    // bytes past the commit: log loudly with the count, open, do NOT truncate
}
```

A file holding bytes past the extent its commit counter covers — a ragged
record, or whole records an append wrote before it died — was interrupted. The
month opens, and the event is logged (`store.open`, "bytes past the commit
counter") with the byte count discarded. **The file is not truncated** (D-0189):
no reader can reach those bytes, and the next append writes at exactly
`offset_of(n_valid)` and overwrites them, so a destructive write at open buys
nothing. Never silently, never by counting the remainder as records. This
section said "truncated" until D-1527 corrected it to what D-0189 decided.

A file shorter than the header region is all tail, and is reported as such.

---

## 8. Sibling files of a month

Overlay fields — implied volatility, greeks, anything computed — do **not**
widen this record. They live in a sibling file with their own version, their
own stride, and their own commit counter. Join record families by timestamp:
an absent overlay or a pricing refusal can leave different row counts.

| Extension | Holds |
|---|---|
| `.bin` | the bar records |
| `.crc` | bar block checksums, one per bar block index (§6) |
| `.ovl` | vendor spot and implied volatility, 24-byte records, version 9 |
| `.ovl.crc` | overlay block checksums, using the overlay's own geometry |
| `.grk` | computed Greeks and their provenance, 80-byte records, version 8 |
| `.grk.crc` | Greek block checksums, using the Greek file's own geometry |
| `.tix` | the bar file's time index, version 1 (§8.1, D-2329) |
| `.lock` | the advisory lock one writer holds for the month (§5, §9) |

```
bars/<vendor>/NSE/FNO/<underlying>/<contract>/1min/2024-03.bin
bars/<vendor>/NSE/FNO/<underlying>/<contract>/1min/2024-03.ovl
bars/<vendor>/NSE/FNO/<underlying>/<contract>/1min/2024-03.grk
```

This keeps the base stride constant forever. A base file written in year one
is readable in year ten by arithmetic that has not changed.

`.ovl` and `.grk` are the bar file's header region and slot (§2) with their own
magic, version and stride: the same 32,768-byte header region, the same
64-byte slot, the same commit counter, and blocks of whole records anchored at
byte 32,768. Their block checksums are optional as at bar version 2 (`flags`
bit 0), and when present live in `.ovl.crc` / `.grk.crc` (§6's sidecar layout).
Both ARE in `store::layout::Layout::KNOWN` (`[V2, V3, OVERLAY, GREEKS]`),
because `Header::decode` resolves a version while it decodes and a geometry
outside that list is `UnknownVersion` at its own first byte. What keeps an
overlay or Greeks header in a `.bin` from being read at its stride is
`store::file`'s per-kind table (`BAR_TABLE`, `[V2, V3]`, chosen by
`table_of` from the file kind), never `KNOWN`. Every integer and float is
little-endian. (This sentence said neither was in `KNOWN`; the code put both
there. Gap-audit #17, D-3687.) Until P14-03 (D-1959) this
section gave only each record's width and version.

### 8.1 `.ovl` — overlay record, 24 bytes

Magic `BRUTEXB9`, version `9`, stride `24`, **170** records per block
(`24 × 170 = 4,080`). Written by `store::format::Overlay::image`.

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | 8 | `ts_micros` | `i64`, the open of the bar it overlays; joined to the bar by value, not by position |
| 8 | 8 | `spot` | `i64` paisa, or `i64::MIN` when the vendor stated none |
| 16 | 8 | `iv_micros` | `i64` millionths of implied volatility (`125000` is 0.125), or `i64::MIN` when absent |

### 8.2 `.grk` — computed Greeks record, 80 bytes

Magic `BRUTEXB8`, version `8`, stride `80`, **51** records per block
(`80 × 51 = 4,080`). Written by `store::format::Greek::image`.

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | 8 | `ts_micros` | `i64`, the open of the bar it prices |
| 8 | 8 | `spot` | `i64` paisa, the underlying level used |
| 16 | 8 | `volatility` | `f64` decimal (`0.1425`, not `14.25`) |
| 24 | 8 | `delta` | `f64`, scale-free |
| 32 | 8 | `gamma` | `f64`, **per paisa** |
| 40 | 8 | `vega` | `f64`, per `1.00` of volatility, **per paisa** |
| 48 | 8 | `theta` | `f64`, per year, **per paisa** |
| 56 | 8 | `rho` | `f64`, per `1.00` of rate, **per paisa** |
| 64 | 8 | `rate` | `f64`, the continuously-compounded rate priced under |
| 72 | 8 | `provenance` | `i64`, packed: bits `0..8` volatility source (`0` solved, `1` vendor); bits `8..16` rate source (`0` charter, `1` operator, `2` solved); bit `16` below the validated band; bits `32..64` signed `i32` moneyness steps (sign-extend when reading) |

The `f64` fields keep full precision (`CLAUDE.md` §7: a greek is a statistic,
not a price); the two `i64` fields that are prices are paisa.

Each record family owns its checksum file (D-0667). Sharing `.crc` between
different strides overwrote the source bars' integrity evidence when an overlay
or Greek file committed. The month-wide `.lock` remains shared, so writers stay
serialized. The record bytes, format versions and bar checksum path do not
change. A reader refuses an existing sealed derived file whose dedicated
checksum sibling is missing; it does not borrow the bar checksum or fabricate
proof. Writers may create a checksum sibling only for a stream with no
committed records; a missing proof for existing sealed records refuses before
creation. This change does not repair or reseal previously damaged history.

### 8.1 The time index — `.tix`, version 1 (D-2329, D-2330)

`BarFile::first_at_or_after` — the first committed bar stamped at or after a
timestamp — was a bisection over the records (D-1434). The header names the
timeframe and the path names the IST month, so a timestamp's SLOT on the
rung's grid is arithmetic; but a month has nights, holidays and missing
minutes, so the slot is not the row, and no field of the bar header recovers
the row from it. The slot-to-row table is therefore this new file at its own
version and stride; the `.bin`, `.crc` and every existing version are
untouched (`CLAUDE.md` §3 rule 8, §4).

**Slots.** One slot is one timeframe. Intraday slots start on the grid §5's
admission uses — `(ts + anchor) mod width == 0`, anchored at 09:15 IST — so
slot 0 starts at the last grid point at or before 00:00 IST on the 1st (that
instant itself for 1s, 1, 3, 5 and 15 minutes; earlier for 2, 10, 30 and 60
minutes), and every admitted intraday stamp is a slot's first instant. Daily
slots are IST days, slot 0 starting 00:00 IST on the 1st. A month has
`slot_count = (until − 1 − origin) / width + 1` slots: 44,640 at one minute
and 2,678,400 at one second in a 31-day month.

**At most one bar per slot, and an intraday bar AT its slot's start.** Every
intraday stamp the writer admits is a grid point (D-0915), so it is the first
instant of its slot and two strictly increasing ones are two slots: every
intraday rung holds this for free, and only a month written before D-0915 can
break it (such a month gets no index; see below). The daily rung
admits any whole second (D-0915), so it can hold a second bar on one IST day.
That bar is still admitted, because D-2329 does not narrow what the writer
accepts. Before any byte of that append is written the `.tix` is removed and
its directory synced, and from then on the month bisects with the `store.tix`
warning. A later writer open retries the rebuild, meets the same bar, and warns
again (D-2330).

**Header, 64 bytes at offset 0, little-endian:**

| Offset | Size | Field |
|---|---|---|
| 0 | 8 | magic `BRUTEXT1` |
| 8 | 2 | version `1` |
| 10 | 2 | entry stride `16` |
| 12 | 4 | timeframe seconds, as the bar header |
| 16 | 8 | origin: the first instant of slot 0, epoch microseconds, `i64` |
| 24 | 8 | slot width, microseconds, `i64` |
| 32 | 8 | slot count, `u64` |
| 40 | 4 | symbol id, as the bar header |
| 44 | 16 | reserved, zero |
| 60 | 4 | CRC-32C over bytes `0..60` |

A reader recomputes all of it from the path and the bar header and refuses
any difference: a header that fails its checksum, a magic that is not
`BRUTEXT1`, a version that is not 1, or a geometry that is not this month's.

**Entry *b*, 16 bytes at `64 + 16·b`,** one per 64 slots `64b .. 64b+63`:

| Offset | Size | Field |
|---|---|---|
| 0 | 8 | occupancy, `u64`: bit `j` set when a bar lies in slot `64b + j` |
| 8 | 4 | before, `u32`: bars in every slot below `64b` |
| 12 | 4 | CRC-32C over bytes `0..12` followed by `b` as 8 little-endian bytes |

The checksum binds an entry to its position, so an entry copied to another
bucket or a run shifted by a lost write fails it. Entries exist from bucket 0
through the bucket of the last committed bar, and the file ends there: an
empty month's `.tix` is the 64-byte header alone; a full one-minute month's
is `64 + 698·16` bytes, a full one-second month's `64 + 41,850·16`.

**The lookup.** For `ts` at or before the header's `first_ts_micros` the
answer is 0, and after its `last_ts_micros` it is `n_valid`, with no read.
Otherwise `ts` lies in slot `q` no later than the last bar's, and one entry
read gives

```
row = before[q/64] + popcount(occupancy[q/64] & ((1 << (q mod 64)) − 1))
```

the row of the first bar in slot `q` or later. When slot `q` is empty, or
`ts` is its first instant, that is the answer. Otherwise `ts` is strictly
inside an occupied slot. On an intraday rung the bar in it IS the slot's first
instant — the index holds no other (below) — so it is before `ts` and the
answer is `row + 1`. On the daily rung the bar may sit anywhere in its day,
and one read of bar `row` decides between `row` and `row + 1`. **At most one
entry read and, on the daily rung only, one bar read, whatever `n_valid` is.** A read
handle decides once, at its first lookup, whether the index is usable: one
open, the 64-byte header, and the two entries holding the bars the header
stamps first and last, which must be rows `0` and `n_valid − 1` in their
slots.

**The writer keeps it in step.** `BarFile::append` computes the entries a
batch touches before writing anything — one entry read, the one holding the
last committed bar, with every bit above that bar cleared — and writes them
at step 3b, cutting the file to end after them. A crash between 3b and 5
leaves entries for bars that never committed; every reader ignores them
(the lookup never consults a slot past the last committed bar), and the next
append clears them by the same masking. Re-offering held bars
(`AlreadyPresent`) writes nothing. The same bars written in any batching give
the same `.tix`, byte for byte.

**A month without a usable index is served loudly, never silently.** A `.tix`
that is absent (a month written before D-2329, or by a writer that does not go
through `BarFile::append`), refused, or not describing the committed bars
leaves the handle on the D-1434 bisection: the same answers, at
`ceil(log2(n_valid + 1))` record reads, with one `store.tix` warning per
handle naming the reason (`BarFile::time_lookup` reports it). A read door
never writes a `.tix`. The writer door — `BarFile::open_or_create` — rebuilds
one from the committed bars when it finds none it can confirm: O(`n_valid`)
verified record reads, once per month, logged as a `store.tix` info line. That
open IS the explicit migration path for existing months. When the bars cannot
be indexed (a block that fails its checksum, two bars in one slot, an intraday
bar off the grid, a bar outside the month) the writer leaves no `.tix`, the
month still opens and appends exactly as before, and its lookups bisect,
saying why.

**What the reader's confirmation does not prove.** It checks the header and
the two entries holding the first and last bars, not every entry between: an
index rewritten by hand, or one left by a writer that replaced the month's
bars with others that happen to start and end in the same slots at the same
rows, would pass it. `docs/06-limits.md` (D-2329) states the residue. A writer
that changes a `.bin` other than through `BarFile::append` must delete the
`.tix` beside it.

Every one of those names is rendered by `store::path::StorePath` and nothing
else. The vendor is the first segment (D-0019); every segment is
case-canonical, because two segments differing only in case are two files on
ext4 and one file on APFS.

---

## 9. What this format does not solve

| Hazard | Position |
|---|---|
| Page fault on a network mount that has gone away | Uninterruptible. Keep the store on local disk. This is an operational rule, not an architectural fix. |
| Two writers on one file | Not supported. One writer per file, enforced by `store::flock`: the writer takes an exclusive `try_lock` (`flock(2)`) on the `.lock` sibling at open and refuses when another holder has it (§5). The lock is a leaf — never held while acquiring another. What it does not cover: it is **advisory**, so a process that writes the month without taking it is not stopped; and it is not relied on across hosts on a network filesystem, where `flock` semantics are the filesystem's and not this crate's. (This row said "Nothing in `crates/store` can check it", which §5 and `BarFile`'s open contradicted; tests-docs-security-pass14 P14-04, D-1959.) |
| A failure coarser than 16384 bytes | Takes both header slots. No arrangement inside one file survives a dead device. |
| A symlink at a path component | Defeats vendor-prefix isolation, which is a **lexical** property of `StorePath`, not a filesystem one. The writer asks every existing component below the store root with `symlink_metadata` before it creates anything, opens the month file, its `.lock` and its `.crc` with `O_NOFOLLOW`, and halts with `StoreError::Symlinked` naming the linked component (CE-62, D-2686). The remaining window: a directory link swapped in between that walk and the open is followed, because per-component `openat` is not available in `std` without `unsafe`. The store root itself may be a link; it is the operator's to place. |
| A wrong value that is well-formed | Out of scope here. Range validation happens at the ingest boundary, before a byte is written. |

---

## 10. Version 1 — retired

Version 1 described:

* a **64-byte** header region holding **one** slot,
* fields at `n_valid` 16, `first_ts_micros` 24, `last_ts_micros` 32,
  `symbol_id` 40, `timeframe_secs` 44,
* an **8-byte** `header_crc` at offset 48 covering bytes 0..48,
* a byte-addressed **4096-byte** checksum block,
* magic `b"BRUTEXB1"`.

Every one of those numbers is different in version 2, and version 2 inserts a
`generation` field at offset 16. Decoding a version-1 file with version 2's
decoder would lift every field from the wrong offset and return plausible
integers.

So version 1 is **refused by number**, naming the reason
(`FormatError::RetiredVersion(1)`), never decoded and never reported as a
damaged header. Its number is not reused: §3 rule 8 makes the version an
append-only identifier, not a slot to be overwritten.

No version-1 file can exist — version 1 never had a reader or a writer in this
repository, only constants. The entry costs one comparison and removes the only
way this build could misread one if that assumption is ever wrong.

---

## 11. The census file — `BRUTEXM`, versions 1, 2 and 3

Everything above is a **bar** file. The per-vendor census at
`manifest/<vendor>.man` is a different format in the same store, and until
D-0067 its only description was a module comment in `crates/pull/src/manifest.rs`
— which made a Rust comment the authority for bytes on a disk. This section is
the authority now.

### 11.1 What is shared with the bar format, and what is not

**Shared, because a reader must locate the header before it knows the version:**
the two-slot header region, `slot_count × 16384`, 64 bytes of fields at the
start of each slot, commit *g* in slot `g % 2`, and the highest valid generation
wins. The 16384-byte spacing is the measured failure-unit argument of §2 and it
is the same argument here.

**Not shared:** the header's *fields* are counters, not a bar file's
`symbol_id`/`timeframe_secs`; the magic is `BRUTEXM`, never `BRUTEXB`, and the
family check is the first thing either decoder does; and the checksum domain is
one contiguous run `0..60` rather than §2's discontiguous one, because this
slot has no reserved field after its checksum.

### 11.2 File layout

```
byte 0                 32768                                       EOF
  ├──── header region ────┼─── entry 0 ──┼─── entry 1 ──┼── … ──────┤
   2 slots × 16384 spacing    64 or 128      64 or 128
```

**Address of entry *i*: `32768 + i·stride`,** where `stride` comes from
`pull::manifest::Layout`, selected by the file's own `format_version`. It is not
a constant on the read path. 32768 divides by both strides, so an entry is
64-byte aligned at either one and never straddles a cache line.

### 11.3 One header slot — 64 bytes, every version

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | 8 | `magic` | `b"BRUTEXM1"` at version 1, `b"BRUTEXM2"` at versions 2 and 3 — the magic names the GEOMETRY, not the version |
| 8 | 2 | `format_version` | `1`, `2` or `3`. **Selects the geometry and the meaning of reserved bytes.** |
| 10 | 2 | `entry_stride` | `64` at version 1, `128` at versions 2 and 3. Read it; never assume it, and check it against what the version declares. |
| 12 | 4 | reserved | written zero |
| 16 | 8 | `generation` | which commit this slot holds. Higher wins. |
| 24 | 8 | `n_valid` | the commit counter: entries readable |
| 32 | 8 | `n_keys` | distinct `(instrument, timeframe, month)` keys among them |
| 40 | 8 | `total_rows` | bars held across every distinct key |
| 48 | 8 | `vendor` | NUL-padded text; a cross-check against the file name. All eight bytes are the vendor's: `truedata` fills them and `zerodha` uses seven |
| 56 | 4 | reserved | written zero |
| 60 | 4 | `crc` | CRC-32C over bytes `0..60`, one contiguous run |

All integers are little-endian. **The two reserved runs are written zero and
are not checked individually**: the decoder reads neither `12..16` nor
`56..60`, so a non-zero byte there is caught only by the slot's `crc`, which
covers both. (The bar slot refuses a non-zero reserved field by name, D-1353;
this one does not.) This table put a reserved row at `54..60` over the last two
bytes of `vendor` and gave no row for `12..16` until tests-docs-security-pass14
P14-01 (D-1959).

The slot layout is **identical across all three versions**: only the magic, the
version and the stride differ, which is why one decoder reads them all and
dispatch is a table lookup rather than a second parser.

A slot whose magic is not the one its version declares (`BRUTEXM1` for 1,
`BRUTEXM2` for 2 and 3) is refused **by version**. There is no way to tell which
of the two is the lie, and the version field is the thing that selects the
geometry. This table said the magic's last byte is the version until P1-16-01
(D-1763); a version-3 slot begins `BRUTEXM2`.

### 11.4 Entry, version 1 — 64 bytes

| Offset | Size | Field |
|---|---|---|
| 0 | 24 | `symbol`, NUL-padded, canonical case |
| 24 | 8 | `rows` — bars in that month's file |
| 32 | 8 | `first_ts_micros` |
| 40 | 8 | `last_ts_micros` |
| 48 | 4 | `timeframe_secs` |
| 52 | 2 | `year` |
| 54 | 1 | `month` |
| 55 | 1 | `exchange` code — NSE 1, BSE 2, append-only |
| 56 | 1 | `segment` code — INDEX 1, CASH 2, FNO 3, append-only |
| 57 | 3 | reserved, zero |
| 60 | 4 | `crc` — CRC-32C over `0..60` |

**Version 1 is not retired.** 43,422 entries across two vendors are on disk in
this geometry and §3 rule 8 does not admit mutating them in place. It is read at
its own stride and never written.

### 11.5 Entry, version 2 — 128 bytes, two 64-byte halves

Version 2 exists because `/store.json` must serve a month's percentage change
without opening a bar file. It is **two** of the 64-byte checksummed units this
format already has:

```
byte 0                        64                            128
  ├──── a version-1 entry ─────┼──── the closes ─────────────┤
       its own CRC at 60             its own CRC at 124
```

Bytes `0..64` are, byte for byte, a version-1 entry image. Bytes `64..128` are:

| Offset | Size | Field | Notes |
|---|---|---|---|
| 64 | 8 | `first_close_paisa` | close of record `0`, or `i64::MIN` |
| 72 | 8 | `last_close_paisa` | close of record `rows − 1`, or `i64::MIN` |
| 80 | 44 | reserved | zero |
| 124 | 4 | `crc` | CRC-32C over bytes `64..124` |

Every one of the 128 bytes is covered by exactly one of the two checksums, so a
flipped bit anywhere in the entry is detected — there is no window a corruption
can land in and be called clean.

**`i64::MIN` is the not-recorded sentinel, and zero means zero.** A close is a
price in paisa, so `i64::MIN` is not a value any tick grid produces; this is the
same convention §3 states for open interest, applied to a second field rather
than a second mechanism that could drift out of step with the first. A month
whose bars really closed at zero paisa records a zero and is told apart from a
month whose closes nobody has read. Every version-1 entry reads back as
not-recorded, because version 1 had nowhere to put a close and nothing ever did.

**Both closes or neither.** They are read from record 0 and record `n_valid − 1`
of one file in one operation, so half of a pair is a state no writer produces
and it is refused rather than half-believed. A negative close that is not the
sentinel is refused for the same reason: the sentinel must be one value, not a
range.

Reserved bytes are zero and stay zero at version 2. A future field takes
reserved space in a **new version**, never by reinterpreting version 2 — §2's
rule, unchanged. Version 3 is that new version.

### 11.5a Entry, version 3 — 128 bytes, the contract in reserved space

Version 3 is version 2's geometry with the derivative contract written into
bytes the closes half reserved. The magic stays `BRUTEXM2`, because the stride
and every version-2 field are unchanged; the version field is what separates
them. A spot row leaves the contract bytes zero, so a version-3 spot row is
byte-identical to its version-2 image, and only option and futures rows differ.

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | 64 | a version-1 entry image | its own CRC at 60 |
| 64 | 8 | `first_close_paisa` | as version 2 |
| 72 | 8 | `last_close_paisa` | as version 2 |
| 80 | 24 | `contract` | ASCII contract text (`core::instrument::Contract`), zero-padded; all zero for spot |
| 104 | 1 | `contract_len` | `0` for spot, otherwise `1..=24` |
| 105 | 19 | reserved | zero |
| 124 | 4 | `crc` | CRC-32C over bytes `64..124` |

A length past 24, or text that does not parse as a contract, is not read as a
shorter or different contract: a truncated contract is a different series.

### 11.6 Old files stay readable, and the upgrade is a rewrite

`pull::manifest::Layout` is the dispatch: `KNOWN` holds one row per version,
`for_version` refuses an unknown version **by number**, and every geometric
question is a method on the resolved row. Reading a version-1 region in 128-byte
steps would decode every second entry as the tail of the one before it and
return plausible integers, which is §9's "a wrong value that is well-formed"
arriving through the front door.

This build **writes** version 3 only (`Layout::CURRENT`; this said version 2
until P1-16-01, D-1763). A census loaded from a version-1 file is published at
version 3, and because the two strides disagree from the first
entry onward, the publish is a whole-file rewrite rather than a positional
append — the same path a repair and a first write already take. It is checked
after the "nothing moved" gate, so a run that records nothing leaves a version-1
file byte for byte as it was, and the conversion is paid once by the first run
that has something to say.

The entries carried across say **not recorded**, because that is the truth about
them: nothing ever read a close for those months. They fill in as they are
re-ingested — the census's equality probe covers the closes, so a month whose
rows have not moved but whose closes have just been read for the first time is a
change and is recorded, and once recorded it compares equal and the file stops
moving.

---

## 12. Result-detail receipt — `results/detail-sets.bin`

This sidecar is the explicit cardinality manifest for the two variable-length
children of one run. `runs.bin` holds aggregate combinations and the chosen
exit-grid trade count; neither is the row count of `frontier.bin` or
`chosen-trades.bin`. The receipt therefore stores both exact child counts,
including zero, rather than inferring completeness from a similarly named
aggregate. It also carries the selected direction and exact trade policy,
because the frequency-run identity's direction term is `Undirected` and a
zero-trade result has no row from which either fact could be recovered.

All integers are unsigned little-endian. The file is append-only and fixed
stride. Version 2 is the only version this build reads or writes. Version 1's
56-byte receipts named only two counts and the level-less trade child; they are
not widened into the policy below.

### 12.1 Header — 16 bytes

| Offset | Size | Field |
|---:|---:|---|
| 0 | 8 | ASCII magic `BRUTEXRC` |
| 8 | 4 | version `2`, `u32` little-endian |
| 12 | 4 | reserved, zero; a non-zero value is refused |

The reader requires the exact magic, version, reserved value, and a total file
length of `16 + n·64`. Magic and version are inspected before version-2
geometry, so an intact `16 + n·56` version-1 file receives an explicit legacy
version refusal instead of a misleading ragged-length error. No stride is
guessed and no tail is padded or truncated.

### 12.2 Receipt — 64 bytes

| Offset | Size | Field |
|---:|---:|---|
| 0 | 32 | nine-term run identity bytes |
| 32 | 8 | exact `frontier.bin` row count for this identity |
| 40 | 8 | exact `chosen-trades.bin` row count for this identity |
| 48 | 1 | selected direction: `1=long`, `2=short`; every other value refuses |
| 49 | 1 | trade policy: `1=chosen-grid-v1`; every other value refuses |
| 50 | 6 | reserved, all zero; a non-zero byte refuses |
| 56 | 8 | first eight bytes of BLAKE3 over bytes `0..56` |

The seal covers every payload byte. A failed seal refuses the receipt file for
result-set visibility; it is never skipped. Two receipts for one identity are
also an ambiguous manifest and are refused at open. The normal append door
takes an exclusive file lock, absorbs rows appended since its handle opened,
and either appends one new sealed stride or byte-verifies the existing receipt.
The same identity with different counts, direction or policy is nondeterminism
and is not rewritten.
A failed partial `write_all` is rolled back only to the byte offset that call
measured before it wrote; a sync failure leaves the complete sealed receipt for
an exact rerun to verify and re-sync.

### 12.3 Commit order and read visibility

One cooperating writer holds `results/write.lock` across the entire sequence:

```
1. append + sync the run's frontier block (or prove an explicit zero)
2. append + sync the run's exact chosen-grid trade block (or prove an explicit zero)
3. append/verify + sync this receipt with both exact counts
4. sync the `results/` directory so all child names are durable
5. append + sync the `runs.bin` parent -- the public commit marker
6. sync the `results/` directory for the marker's name
```

A read-only detail lookup exposes rows only when the identity exists in
`runs.bin`, exists in this receipt file, and the indexed detail-block count
equals the corresponding receipt count. Zero is therefore a proved empty child,
not a missing file guessed empty. If an entire child file is absent, an explicit
zero count is likewise the only committed empty answer; a positive receipt count
makes the missing file loud corruption. A seal failure, foreign row, content
shortfall or count mismatch refuses the whole child; no valid prefix is returned
as the complete answer. A prepared child or receipt without the final ledger
parent is private and can be reused only by an exact rerun that derives and
compares the same bytes. A ledger row with no receipt predates this protocol or
may be a legacy ledger-first partial commit; the two states are indistinguishable
from the old bytes, so both are named `unverifiable` and refused rather than
guessed.

### 12.4 Chosen trades — `results/chosen-trades.bin`, version 1

The current trade child is a new path and magic rather than an in-place change
to `results/trades.bin` version 2. The legacy file records a level-less horizon
walk; relabelling those rows as the selected stop/target/trailing variant would
change their meaning without changing their bytes. If only the legacy path is
present, the reader names version 2 and requires an exact rerun. It never opens
those bytes through this decoder.

The 16-byte header is ASCII `BRUTEXCT` at bytes `0..8`, little-endian version
`1` at `8..12`, and four reserved zero bytes at `12..16`. A non-zero reserve,
unknown magic/version, or length other than `16 + n·136` refuses.

Each sealed row is 136 bytes:

| Offset | Size | Field |
|---:|---:|---|
| 0 | 32 | nine-term frequency-run identity (its direction term is `Undirected`) |
| 32 | 4 | zero-based contiguous sequence, `u32` |
| 36 | 1 | selected direction: `1=long`, `2=short` |
| 37 | 3 | reserved, all zero |
| 40 | 8 | signal bar index, `u64` |
| 48 | 8 | entry bar index, `u64` |
| 56 | 8 | exit bar index, `u64` |
| 64 | 8 | optimistic realised P&L, paisa per unit, `i64` |
| 72 | 8 | pessimistic realised P&L, paisa per unit, `i64` |
| 80 | 8 | entry timestamp, UTC microseconds, `i64` |
| 88 | 8 | exit timestamp, UTC microseconds, `i64` |
| 96 | 8 | maximum adverse excursion, ppm, `i64` |
| 104 | 8 | maximum adverse excursion, paisa per unit, `i64` |
| 112 | 8 | maximum favourable excursion, ppm, `i64` |
| 120 | 8 | maximum favourable excursion, paisa per unit, `i64` |
| 128 | 8 | first eight bytes of BLAKE3 over bytes `0..128` |

Every row in one appended block must have the same identity and direction and
sequences exactly `0..n-1`. A public read additionally requires the receipt's
direction and `chosen-grid-v1` policy and refuses the complete block if any row
disagrees. The selected cell is replayed before these bytes are prepared; its
complete `Cell` and an independent row fold of count, optimistic/pessimistic
sums, worst trade and maximum drawdown must all reconcile. The file stores no
entry/exit price and no exit-cause enum, so neither may be claimed by the API or
browser. Their absence does not weaken the bar indices, timestamps, direction,
realised fills or MAE/MFE that the row does carry.

---

## 13. Ranked frontier — `results/frontier.bin`, version 7

The ranked frontier is append-only and fixed-stride. Its 16-byte header is
`BRUTEXFR` at bytes `0..8`, little-endian version `7` at `8..12`, and four
reserved zero bytes at `12..16`. A non-zero reserved header byte is an unknown
schema and is refused; the reader does not wait for it to affect a later field
before noticing it.

Each row is 280 bytes. All integer fields are little-endian. Versions 4 to 6
are refused by version, never reread under version 7 meanings: version 5 took
byte 195 for the protective-exit rule, version 6 took `196..200` for the
fill-headroom rule, and version 7 widened the row by eight bytes for the
average-payoff floor and moved the seal to `272..280`. This section described
version 4 (272-byte rows, a reserve at `195..200`, the seal at `264`) until
P1-16-02 (D-1763).


| Offset | Size | Field |
|---:|---:|---|
| 0 | 32 | nine-term run identity |
| 32 | 2 | one-based rank, `u16` |
| 34 | 48 | six condition-mask `u64` words |
| 82 | 8 | edge hits |
| 90 | 8 | forward observations |
| 98 | 8 | mean milli-paisa, `i64` |
| 106 | 8 | t-statistic thousandths, `i64` |
| 114 | 8 | payoff basis points, `i64` |
| 122 | 8 | edge wins |
| 130 | 8 | chosen-cell trades |
| 138 | 8 | chosen-cell wins |
| 146 | 8 | pessimistic total paisa, `i64` |
| 154 | 8 | worst trade paisa, `i64` |
| 162 | 8 | maximum drawdown paisa, `i64` |
| 170 | 8 | smallest winning trade paisa, `i64` |
| 178 | 8 | gross winning paisa, `i64` |
| 186 | 8 | gross losing paisa, `i64` |
| 194 | 1 | direction: `0=long`, `1=short`; every other byte is refused |
| 195 | 1 | `require_protective_exits`: `0=off`, `1=required`; every other byte is refused |
| 196 | 4 | `min_fill_headroom_bp`, `i32`; a negative value is refused |
| 200 | 8 | `max_mae_ppm`, `i64` |
| 208 | 8 | `min_rr_bp`, `i64` |
| 216 | 8 | `min_win_rate_bp`, `i64` |
| 224 | 8 | `min_assurance_bp`, `i64` |
| 232 | 8 | `min_weakest_bp`, `i64` |
| 240 | 8 | `min_ret_over_dd_bp`, `i64` |
| 248 | 8 | `min_trades`, `u64` |
| 256 | 8 | `top`, `u64` on disk and saturated to the target `usize` on read |
| 264 | 8 | `min_avg_rr_bp`, `i64` |
| 272 | 8 | first eight BLAKE3 bytes over `0..272` |

The seal detects accidental byte damage; it does not assign meaning. A sealed
row with an unknown direction, an undefined protective-exit byte or a negative
fill headroom is therefore still refused.
Its sealed identity remains usable only as the block-index key, so an interrupted
exact rerun finds the invalid prepared block and refuses instead of appending a
second block around it. No invalid row reaches a public frontier response.

---

## 14. Calendar-authoritative population completion — version 4

`results/population-completions-v4.bin` is a new append-only authority file. It
does not reinterpret the V2 or V3 completion paths. The 16-byte header is ASCII
`BRUTXPV4`, little-endian version `4`, and a zero `u32` reserve. The total length
must be `16 + n·864`; an unknown header, ragged tail, failed seal or duplicate
population identity refuses the file.

Each 864-byte record is the unchanged 760-byte V3 payload, followed by this
96-byte coverage record and an eight-byte BLAKE3 seal over the complete
856-byte payload:

| Payload offset | Size | Field |
|---:|---:|---|
| 760 | 4 | coverage schema version `1` |
| 764 | 4 | exact signal rung in seconds |
| 768 | 4 | execution rung, exactly `60` |
| 772 | 4 | reserved, zero |
| 776 | 8 | first requested IST civil-day identity, inclusive |
| 784 | 8 | last requested IST civil-day identity, inclusive |
| 792 | 32 | complete signal-rung calendar-receipt digest |
| 824 | 32 | complete one-minute calendar-receipt digest |
| 856 | 8 | first eight BLAKE3 bytes over payload `0..856` |

Both calendar digests come only from typed complete V2 calendar receipts. Their
day bounds must be equal and must cover the full requested civil-month span
embedded in V3; observed endpoints cannot shrink it. The signal rung must equal
the population rung and the execution rung must be one minute. The durable
sequence is population rows, V2 audit completion, V3 span audit completion,
then V4 calendar authority. A crash may therefore leave an auditable prefix but
cannot silently bless missing coverage.

## 15. Admission decision sidecars — version 1

Admission is stored beside population rows so no historical row or selection
byte changes meaning. Both files use the same `results/population-write.lock`
as the population ledger.

### 15.1 Decisions — `results/population-admission-v1.bin`

The 16-byte header is `BRUTXAD1`, version `1`, and four reserved zero bytes. The
length is exactly `16 + n·544`. Each record contains a 512-byte payload and a
full 32-byte BLAKE3 payload seal:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 32 | population identity |
| 32 | 8 | zero-based population-row sequence |
| 40 | 32 | exact strategy digest |
| 72 | 32 | domain-separated digest of the canonical 304-byte population-row payload |
| 104 | 352 | canonical `AdmissionEvidenceV1` bytes |
| 456 | 45 | canonical `AdmissionVerdictV1` bytes |
| 501 | 11 | reserved, all zero |
| 512 | 32 | BLAKE3 over payload `0..512` |

The row-payload digest domain is
`brutex-population-row-payload-v1\0`. The population module owns that digest;
the admission module does not duplicate the row codec. Decision records for one
population are contiguous and sequences are exactly `0..n-1`. A decision block
without its completion is a hidden prepared orphan, never ranking authority.

### 15.2 Receipt last — `results/population-admission-completions-v1.bin`

The 16-byte header is `BRUTXAC1`, version `1`, and four reserved zero bytes. The
length is exactly `16 + n·480`. Each record contains a 448-byte payload and a
full 32-byte BLAKE3 payload seal:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 32 | population identity |
| 32 | 32 | exact Population V4 completion digest |
| 64 | 8 | decision count |
| 72 | 8 | admitted count |
| 80 | 8 | rejected count |
| 88 | 8 | unmeasured count |
| 96 | 8 | refused count |
| 104 | 32 | ordered decision-record digest |
| 136 | 310 | canonical `AdmissionPolicyV1` bytes |
| 446 | 2 | reserved, zero |
| 448 | 32 | BLAKE3 over payload `0..448` |

The four terminal counts must sum to the decision count. Reopen decodes the
canonical policy and every evidence/verdict pair, reevaluates the policy, and
requires byte-identical recomputed verdicts before indexing the completion.
Commit syncs decisions first and the receipt last; exact reruns byte-verify and
reuse, while a different same-population block refuses without replacement.

V1 has no genuine CSCV/PBO procedure identity or complete split-family
receipt. Its three PBO-named observation slots therefore cannot contain a
measured value. Construction and reopen now refuse any such record with the
exact field and `UnsupportedMeasuredPboV1`; historical `Unmeasured` and
`Refused` encodings remain readable. Consequently a publicly constructible V1
record cannot produce `Admitted`: genuine measured PBO and an admissible
success path require a successor evidence version, not reinterpretation of
these bytes. This is a semantic refusal with no layout, stride, tag or version
change.

Open and reconciliation are linear in stored records. An indexed completion
probe is average O(1), one decision is a fixed seek/read, and a bounded page is
O(requested rows); none is an end-to-end latency claim.

## 16. Admission-authoritative global selection — version 3

`results/global-selections-v3.bin` is a new append-only ledger. It does not
read or reinterpret either earlier selection path. Its 40-byte header is:

| Offset | Size | Field |
|---:|---:|---|
| 0 | 8 | ASCII magic `BRUTXSL3` |
| 8 | 4 | version `3` |
| 12 | 4 | header length, exactly `40` |
| 16 | 4 | record stride, exactly `3112` |
| 20 | 4 | requested Top-N, exactly `25` |
| 24 | 8 | ranking score scale, exactly the runner V1 scale |
| 32 | 8 | reserved, zero |

The total file length is exactly `40 + n·3112`. Each record is a 3,080-byte
payload followed by a full 32-byte BLAKE3 payload seal:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 32 | selection identity |
| 32 | 4 | exact signal rung in seconds |
| 36 | 4 | requested Top-N, exactly `25` |
| 40 | 144 | NIFTY Population V4/admission reference |
| 184 | 144 | BANKNIFTY Population V4/admission reference |
| 328 | 32 | considered, admitted, refused and unmeasured counts, four `u64`s |
| 360 | 32 | exact ordered population-proof digest |
| 392 | 32 | ranking-policy digest |
| 424 | 440 | canonical shared-cohort V2 identity |
| 864 | 4 | selected row count, `0..=25` |
| 868 | 4 | reserved, zero |
| 872 | 2200 | 25 fixed 88-byte selected-entry slots; unused slots are all zero |
| 3072 | 8 | reserved, zero |
| 3080 | 32 | BLAKE3 over payload `0..3080` |

Each 144-byte population reference stores, in canonical family order, a
one-byte family tag plus seven zero reserve bytes, population identity, row
count, ordered-row digest, exact Population V4 completion digest and exact
admission-completion digest. Construction holds immutable join snapshots and
streams both joined populations three times: proof/extrema, verified bounded
Top-25, then exact selected-row resolution. Admission is derived only from the
recomputed sidecar verdict; the legacy summary in a population row is checked
for agreement but never authorizes ranking. Top 10 is the exact prefix of the
same Top 25.

The identity is
`BLAKE3("brutex-global-selection-id-v3\0" || payload[32..3080])`.
Open refuses a caller bound of zero, a file above that bound, unknown or legacy
magic, noncanonical header fields, ragged records, bad seals, duplicate
identities, invalid reserves or a content-derived identity mismatch. Latest
lookup is keyed only by the exact `(shared-cohort digest, signal rung)` and is
reconstructed in append order. Version 3 names admitted winners; it deliberately
does not invent a replay horizon or decode opaque parameter digests. A separate
append-only execution capability is required before any selected row can become
global replay authority.

## 17. Reconstructible execution capabilities — version 1

Four append-only files share `results/population-write.lock`. Each has the same
24-byte header: eight-byte file-specific magic, `u32` version `1`, `u32` header
width `24`, `u32` fixed stride, then four reserved zero bytes. Every record is
its fixed payload followed by a full 32-byte BLAKE3 seal of that payload.

| Path | Magic | Payload | Stride |
|---|---:|---:|---:|
| `results/execution-parameters-v1.bin` | `BRUTXEP1` | 616 | 648 |
| `results/execution-percentiles-v1.bin` | `BRUTXEG1` | 96 | 128 |
| `results/execution-capabilities-v1.bin` | `BRUTXEC1` | 288 | 320 |
| `results/execution-capability-completions-v1.bin` | `BRUTXEF1` | 384 | 416 |

The 616-byte parameter payload is:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 192 | parameter, population, Population V4, policy, expected-resolution and training digests; six 32-byte values |
| 192 | 163 | exact evaluation-spec fingerprint |
| 355 | 9 | direction, family, execution/range/selector/forced-stop, next-minute-entry and forced-exit tags; one reserved zero byte |
| 364 | 24 | signal rung, horizon, execution seconds and stop/target/trail atom counts; six `u32`s |
| 388 | 2 | entry delay, exactly one minute |
| 390 | 2 | forced-exit IST minute, exactly `910` (15:10) |
| 392 | 40 | four runner parameters and maximum levels per axis; five `u64`s |
| 432 | 16 | minimum and maximum reward/risk hundredths; two `i64`s |
| 448 | 64 | ratio-pair/cell bounds, forced-stop ppm, ambiguity/gap bounds, training count and training endpoints |
| 512 | 96 | ordered-percentile, cost-model and exact-execution-law digests |
| 608 | 8 | reserved, zero |
| 616 | 32 | BLAKE3 seal over payload `0..616` |

**This record grew in place, and that is a recorded breach of §2, not a
second geometry this reader accepts.** The evaluation-spec fingerprint widened
from 155 to 163 bytes when the charter gained its ninth non-regular day, and
the record went from 608/640 to 616/648 bytes while the magic stayed
`BRUTXEP1` and the version stayed `1`. The header's own stride field is what
separates the two, so a file whose header says 640 is refused by name: its
records carry the eight-day calendar's fingerprint and name runs no current
build reproduces. A 648-byte file is read. This table showed the 608/640
layout until P1-16-03 (D-1763).

Dynamic rung arrays are not truncated into the scalar record. Each 96-byte
percentile payload stores parameter id `0..32`, population id `32..64`, axis
tag at `64`, three reserved zero bytes, ordinal `68..72`, reduced numerator
`72..76`, denominator `76..80`, and 16 reserved zero bytes. Its seal occupies
`96..128`. Canonical order is long then short, and within each side stop then
target then trail with contiguous zero-based ordinals.

The 288-byte per-row capability payload is:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 256 | capability, parameter, population, Population V4, row-payload, strategy, training-run and selected-exit digests |
| 256 | 8 | exact population-row sequence |
| 264 | 4 | signal rung in seconds |
| 268 | 1 | direction tag |
| 269 | 1 | instrument-family tag |
| 270 | 18 | reserved, zero |
| 288 | 32 | BLAKE3 seal over payload `0..288` |

The 384-byte completion payload is:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 160 | authority, population, Population V4, long-parameter and short-parameter digests |
| 160 | 72 | first/count pairs for parameter, percentile and capability blocks, then row/long/short counts; nine `u64`s |
| 232 | 128 | ordered parameter, percentile, capability and exact-execution-law digests |
| 360 | 24 | reserved, zero |
| 384 | 32 | BLAKE3 seal over payload `0..384` |

The completion is synced last. Its semantic authority excludes physical block
offsets so a valid orphan tail left by a crash cannot re-key an exact retry.
Reopen validates every referenced block, order digest, side count and exact
Population V4 binding before indexing it. This format makes an execution
authority reconstructible; it does not prove that a production caller has
rebuilt the selected exit from real stored training/OOS bytes.

## 18. Global single-position replay publication — version 1

Four append-only files share `results/global-replay-write.lock` and use the
same 24-byte header and payload-plus-32-byte-seal convention as §17.

| Path | Magic | Payload | Stride |
|---|---:|---:|---:|
| `results/global-replay-streams-v1.bin` | `BRUTXGS1` | 368 | 400 |
| `results/global-replay-decisions-v1.bin` | `BRUTXGD1` | 208 | 240 |
| `results/global-replay-trades-v1.bin` | `BRUTXGT1` | 320 | 352 |
| `results/global-replay-completions-v1.bin` | `BRUTXGC1` | 1,216 | 1,248 |

One 368-byte stream payload stores replay id `0..32`, stream ordinal/rank/rung
`32..40`, family/direction plus six reserved bytes `40..48`, selection and
population identities plus source row sequence `48..120`, seven 32-byte
strategy/execution/training/selection/OOS/universe identities `120..344`,
candidate and pricing-refused counts `344..360`, and eight reserved zero bytes.
The manifest requires exactly `8 × 25 = 200` canonical streams.

One 208-byte decision payload stores replay id and sequence `0..40`,
stream/rank/rung `40..48`, family/direction/disposition/path and four reserved
bytes `48..56`, strategy and candidate digests `56..120`, four signal/entry/
occupancy/disposition timestamps `120..152`, candidate ordinal `152..160`, the
simultaneously admitted strategy digest `160..192`, and 16 reserved zero bytes.

One 320-byte admitted-money payload stores replay id and decision/stream/rank/
rung/family/direction `0..50`, six reserved zero bytes, strategy digest
`56..88`, the exact 88-byte `TradeRow` at `88..176`, exact-or-absent 64-byte
India VIX stamps for entry and exit at `176..304`, then one `u64` each for
ambiguous-bar and gap-fill counts. An absent VIX stamp is tag zero followed by
63 zero bytes; an exact stamp is tag one, seven zero bytes and all seven stored
OHLCV/OI `i64` fields. VIX changes the publication identity but not selection,
P&L or the execution-only replay identity.

The 1,216-byte receipt-last completion payload is:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 128 | publication, replay, manifest and common-cohort digests |
| 128 | 256 | eight canonical Selection V3 identities |
| 384 | 512 | sixteen execution-authority identities in rung then family order |
| 896 | 112 | stream/decision/trade first/count pairs, six scheduler counters and two pricing-refusal counts; fourteen `u64`s |
| 1008 | 160 | ordered stream/decision/trade, execution-law and VIX-policy digests |
| 1168 | 48 | reserved, zero |
| 1216 | 32 | BLAKE3 seal over payload `0..1216` |

Reopen replays and reconciles the committed scheduler decisions before it
indexes a receipt; valid unreferenced tails remain visible crash evidence but
not authority. The fixed files and replay checker are implemented. One direct
public-preparation test now supplies eight selections, sixteen execution
authorities and 200 exact witnesses, reaches the scheduler, and refuses a
missing or foreign authority. Its bars, selections, authorities and VIX month
are controlled fixtures, however; no production stored-run caller or real
Zerodha sweep has crossed this format.

## 19. Stored-data completeness authority — version 1

`results/stored-data-completeness-v1.bin` is one append-only receipt ledger.
It is never inferred from Population V4 alone: preparation recomputes exact
signal, complete one-minute context, evaluated execution and daily-reference
identities, and only a durably appended record can reopen as authority.

The sealed 64-byte header is:

| Offset | Size | Field |
|---:|---:|---|
| 0 | 16 | ASCII magic `BTX-DATA-COMP-V1` |
| 16 | 4 | little-endian format version, `1` |
| 20 | 4 | little-endian record stride, `676` |
| 24 | 8 | reserved, zero |
| 32 | 32 | BLAKE3 seal over bytes `0..32` |

Every receipt is a 644-byte payload followed by a 32-byte BLAKE3 seal over that
payload, for a 676-byte fixed stride:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 4 | receipt version, `1` |
| 4 | 4 | reserved, zero |
| 8 | 32 | Population V4 identity |
| 40 | 32 | Population V4 completion content digest |
| 72 | 1 | instrument family: NIFTY `1`, BANKNIFTY `2` |
| 73 | 3 | reserved, zero |
| 76 | 4 | signal rung seconds |
| 80 | 20 | canonical requested-span identity |
| 100 | 224 | four 56-byte stream facts: signal, complete minute context, evaluated execution and daily reference; each stores count, first timestamp, last timestamp and ordered data digest |
| 324 | 8 | eligibility count; must equal daily count |
| 332 | 32 | ordered eligibility-byte digest |
| 364 | 224 | data, feed, source-commit, calendar-policy, daily-reference-policy, signal-calendar-receipt and execution-calendar-receipt digests |
| 588 | 12 | daily schema, eligibility policy and gap-overlay policy, three `u32`s |
| 600 | 2 | daily and minute integrity-evidence tags; version 1 accepts only explicit unverified tag `0` |
| 602 | 2 | reserved, zero |
| 604 | 8 | excluded IST-day count |
| 612 | 32 | ordered excluded IST-day digest |
| 644 | 32 | BLAKE3 seal over payload `0..644` |

Open validates the complete header, exact whole-record tail, every record seal
and semantic field, unique population identities and a caller-provided receipt
ceiling before indexing. A writable handle holds an exclusive file lock; a
read-only handle holds a shared lock. Before authority lookup or append, the
handle rehashes the full file and refuses any length or same-length generation
change. An exact retry reuses the existing receipt; different bytes for an
existing population refuse. Legacy files have no promotion path. D-0458 and
DC-01 define the authority boundary; §142 records the nonconstant open and
preparation costs.

## 20. Selection-V4/Execution-V2 global replay authority — version 2

Five append-only files share `results/global-replay-write-v2.lock`. They do not
open or reinterpret the V1 files in §18. Every file starts with the same
24-byte envelope: eight-byte magic, little-endian format version `2`, header
width `24`, record stride, then four reserved zero bytes.

| Path | Magic | Payload | Stride |
|---|---:|---:|---:|
| `results/global-replay-authority-manifests-v2.bin` | `BRUX2GMF` | 3,536 | 3,568 |
| `results/global-replay-streams-v2.bin` | `BRUX2STR` | 400 | 432 |
| `results/global-replay-candidates-v2.bin` | `BRUX2CAN` | 104 | 136 |
| `results/global-replay-decisions-v2.bin` | `BRUX2DEC` | 208 | 240 |
| `results/global-replay-completions-v2.bin` | `BRUX2COM` | 384 | 416 |

Every payload is followed by a 32-byte BLAKE3 seal over the complete payload.
The manifest payload also carries its own semantic envelope: inner magic
`BRUTXGM2`, semantic version `2` and payload width `3,536` at `0..16`.
Manifest id and common-cohort digest occupy `16..80`; eight canonical
Selection V4 ids occupy `80..336`. Sixteen population ids, Population V4
completion digests, admission completion digests and Execution V2 completion
ids occupy four 512-byte arrays at `336..2,384`. Thirty-two long/short dynamic
parameter ids occupy `2,384..3,408`; sixteen row counts occupy
`3,408..3,536`.

One 400-byte stream header stores stream id `0..32`; ordinal, one-based rank
and rung `32..40`; family/direction plus six reserved zero bytes `40..48`;
selection id, population id and source row sequence `48..120`; then eight
32-byte strategy, Execution V2 completion, row-disposition, fresh capability,
training-run, selected-exit, OOS-run and candidate-universe digests
`120..376`. Pricing-refused count, absolute candidate first and candidate count
are the three `u64`s at `376..400`.

One 104-byte candidate payload stores candidate digest `0..32`, stream id
`32..64`, stream-local ordinal `64..72`, signal/entry/inclusive-occupancy
timestamps `72..96`, the path tag at `96`, and seven reserved zero bytes. Path
tags distinguish priceable, block-only, crossing-refused and both-refused
occupancy; none contains or implies a money row.

One 208-byte decision payload stores decision and replay ids `0..64`, global
sequence `64..72`, stream ordinal plus six reserved zero bytes `72..80`,
candidate ordinal/digest `80..120`, entry timestamp and strategy digest
`120..160`, disposition/path plus six reserved zero bytes `160..168`, then the
inclusive occupancy timestamp and simultaneous-winner digest `168..208`.
Admitted and prior-occupancy decisions require a zero winner digest;
same-minute blocks require `i64::MIN` in the occupancy slot and an exact
nonzero winner digest.

The receipt-last 384-byte completion payload is:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 256 | completion, replay, manifest, ordered-stream, ordered-candidate, ordered-decision, exact-execution-law and authority-scope digests |
| 256 | 64 | manifest/stream/candidate/decision absolute first/count pairs; eight `u64`s |
| 320 | 48 | offered, admitted, occupied-blocked, simultaneous-blocked, unreachable and refused counters |
| 368 | 16 | pricing-refused candidates and admitted pricing-refused candidates |
| 384 | 32 | BLAKE3 seal over payload `0..384` |

Manifest, candidates, stream headers and decisions are appended and synced
before the completion is appended and synced last. Reopen checks every whole
record and seal, including unreferenced records; completion ranges may skip a
valid orphan but may never overlap or run backwards. It reconstructs exactly
200 canonical streams, requires their candidate subranges to partition the
receipt range, and re-runs the global scheduler before indexing. Exact replay
reuse is idempotent; corruption, sealed reordering, duplicate replay or
completion identity and same-length post-open changes refuse.

The authority-scope digest is deliberately the versioned
`execution-only-no-money-no-vix` domain. Version 2 contains no admitted
`TradeRow` and no exact-or-absent India VIX stamp, so it makes no claim about
either authority and never borrows the V1 trade file. D-0459 and GPR-02 define
this boundary; limits §143 records its nonconstant preparation/open costs.

## 21. Complete Population V4 execution dispositions — version 2

Four append-only files share the existing `results/population-write.lock` with
Population V4 and admission. They never promote or reinterpret the V1
execution-capability files. Every file starts with the same 24-byte envelope:
eight-byte magic, little-endian format version `2`, header width `24`, record
stride, then four reserved zero bytes.

| Path | Magic | Payload | Stride |
|---|---:|---:|---:|
| `results/execution-disposition-parameters-v2.bin` | `BRUX2PR1` | 616 | 648 |
| `results/execution-disposition-percentiles-v2.bin` | `BRUX2PC1` | 96 | 128 |
| `results/execution-row-dispositions-v2.bin` | `BRUTXED2` | 480 | 512 |
| `results/execution-capability-completions-v2.bin` | `BRUTXEF2` | 512 | 544 |

The first two files reuse the exact sealed V1 scalar-parameter and percentile
record codecs, so the parameter file grew from 640 to 648 bytes with them and a
640-byte one is refused by name, as §17 says. One 480-byte row payload stores thirteen 32-byte digests at
`0..416`: disposition, population, Population V4 completion, admission
completion, population-row payload, strategy, sparse capability, dynamic
parameter, runner disposition, training resolution, training run, complete
grid context and condition column. Row sequence and refusal bits occupy
`416..432`; admission and execution tags occupy `432..434`; the five canonical
exit-coordinate fields occupy `434..474`; six reserved zero bytes occupy
`474..480`. A 32-byte BLAKE3 seal follows. The capability digest is nonzero
only for `Authorized`; refusal bits are exact, known and nonempty only for
`PolicyRefused`.

The receipt-last 512-byte completion payload is:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 320 | completion, population, Population V4, admission, long/short parameter, ordered-parameter, ordered-percentile, ordered-disposition and execution-law digests |
| 320 | 72 | parameter/percentile/disposition absolute first/count pairs, row count, authorized count and policy-refused count; nine `u64`s |
| 392 | 64 | exact 4×2 matrix, row-major `Admitted`, `Rejected`, `Unmeasured`, `Refused` by `Authorized`, `PolicyRefused` |
| 456 | 56 | reserved, zero |
| 512 | 32 | BLAKE3 seal over payload `0..512` |

Parameters, percentiles and every terminal row are appended and synced before
the completion is appended and synced last. The semantic completion identity
excludes physical first offsets, so a valid crash orphan remains evidence but
does not re-key an exact retry. Reopen validates every record and reconstructs
the parameter blocks, exact row order, row count, sparse-capability/refusal
partition, 4×2 matrix and ordered digests before indexing. Admission marginals
are proven once, at preparation, where `from_ledgers` holds the admission
ledger (`require_admission_marginals`); reopen is given no admission ledger, so
it proves the matrix consistent with the rows it holds, not with admission
(Z1-slice17-F2, D-1771).
Missing rows, foreign authority, torn or corrupt records, a semantically
resealed matrix redistribution and same-length post-open mutation all refuse.
Pages are limited to 256 rows. Preparation and reopen are O(rows); only an
exact lookup on an already validated unchanged handle is average O(1).
D-0463, ED-01 and limits §144 define the tested boundary.

## 22. Exact institutional family statistics — version 1

Two append-only files share `results/institutional-statistics-write-v1.lock`.
Each starts with a sealed 64-byte header: 16-byte kind magic, little-endian
format version and kind (`u32` each), stride (`u64`), then a 32-byte BLAKE3
seal over those first 32 bytes.

| Path | Magic | Payload | Stride |
|---|---:|---:|---:|
| `results/institutional-statistics-values-v1.bin` | `BTX-ISTATS-D-V1!` | 368 | 400 |
| `results/institutional-statistics-completions-v1.bin` | `BTX-ISTATS-C-V1!` | 256 | 288 |

The 368-byte value payload stores version/reserve at `0..8`; subject,
Population V4, receipt-last selection, ranking-policy, selected-strategy and
ordered Romano--Wolf family digests at `8..200`; then twenty-one `u64`s at
`200..368`: draws, strategies, periods, seed, block, candidate index, canonical
stepdown rank, observed-statistic `f64` bits, strict exceedances, initial,
candidate-adjusted and full-family numerator/denominator pairs, White statistic
and p-value bits, SPA statistic and p-value bits, and the exact-count-derived
full-family/selected-candidate ppm projections. A 32-byte row seal follows.
Prices never enter this format; all statistical `f64` values retain their exact
source bits.

The 256-byte completion payload stores version/reserve at `0..8`, subject at
`8..40`, absolute row index at `40..48`, row-content digest at `48..80`, the
five Population/selection/ranking/strategy/family digests at `80..240`, and the
two ppm projections at `240..256`; a 32-byte seal follows. The value row is
appended and synced before the completion is appended and synced last. A crash
may leave exactly one valid trailing value row; it is not authority and only
its byte-exact retry may append the receipt. Any second/middle/foreign orphan,
duplicate subject, backward/reordered reference, malformed reserve, bad seal,
ragged tail, over-bound file or stale length/same-length content refuses.

The subject is the domain-separated digest of Population, selection receipt,
ranking policy and strategy. A completion additionally binds the complete
Romano--Wolf family and row content. Exact already-committed evidence is reused
without appending; changed evidence under the same subject refuses. This is a
narrow family-statistics authority. It does not contain raw Wilson, PBO or
risk/ratio sources and cannot set the global full-precision completeness field.
D-0461, IS-01 and limits §146 define that boundary.

## 23. Pre-admission candidate universe — version 1

Candidate Universe V1 is the first durable object in the selection-independent
Step-3 successor path. Its codec encodes closed-mask/grid-cell source terms and
raw direct-cell fields. It contains no assurance, admission verdict, ranking
score, selected rank or final Population identity.

**It has a production writer.** `cli::candidate_universe::produce_candidate_universe_v1`
derives the rows from a naturally-extinct sweep, and
`append_and_reopen` → `append_produced_candidate_universe_v1` commits them
receipt-last and reopens them. Both are `pub(crate)` and are reached from
`step3_orchestrator::commit_candidate_family_guarded_v6`, which the operator
command `cli ledger-v6` (`ledger_v6::ledger_v6`) drives. This section called the
format "audit only" with "no public production constructor or writer" until
tests-docs-security-pass14 P14-06 (D-1959); "public" was true and the
conclusion was not.

The three paths are:

| Path | Role |
|---|---|
| `candidate-universe-write-v1.lock` | one writer/shared-reader generation and path-identity lock |
| `candidate-universe-rows-v1.bin` | contiguous canonical candidate rows |
| `candidate-universe-completions-v1.bin` | receipt-last structural completion for audit; not production authority |

Each data file begins with the same sealed 64-byte envelope:

| Offset | Size | Field |
|---:|---:|---|
| 0 | 16 | kind magic: `BTX-CAND-UROW-V1` or `BTX-CAND-UREC-V1` |
| 16 | 4 | little-endian header version, exactly `1` |
| 20 | 4 | kind, row `1` or completion `2` |
| 24 | 8 | fixed stride, row `480` or completion `976` |
| 32 | 32 | domain-separated BLAKE3 seal over bytes `0..32` |

The complete length must be `64 + n*stride`. Unknown kind/version/stride,
ragged tails or a failed header seal refuse before any record is indexed.

### 23.1 Candidate rows — 480 bytes

One row is a 448-byte payload plus a full 32-byte domain-separated BLAKE3
seal. Its payload is:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 4 | row version, `1` |
| 4 | 4 | reserved, zero |
| 8 | 32 | candidate-universe identity |
| 40 | 8 | zero-based canonical row sequence |
| 48 | 32 | semantic candidate digest, before final Population rekey |
| 80 | 48 | six durable condition-mask `u64` words |
| 128 | 3 | direction, instrument family and closure tags; closure is exactly `Closed` |
| 131 | 1 | reserved, zero |
| 132 | 4 | signal rung in seconds, one of the eight canonical rungs |
| 136 | 8 | closed-mask support hits |
| 144 | 20 | stop, target, TSL, TTP-arm and TTP-trail indices; absent is `u32::MAX`, and TTP is either wholly present or wholly absent |
| 164 | 4 | reserved, zero |
| 168 | 32 | exact one-minute execution-run identity |
| 200 | 4 | exact outcome horizon in one-minute bars |
| 204 | 4 | reserved, zero |
| 208 | 32 | complete evaluated-grid digest |
| 240 | 8 | canonical cell ordinal in that grid |
| 248 | 192 | every raw `Cell` fact in fixed order: trades, wins, pessimistic/optimistic money, fill cost, five terminal counts, ambiguous/gapped counts, MAE/MFE values, gross win/loss, best/min-win, bars held, losing/winning streaks, worst trade and maximum drawdown |
| 440 | 8 | reserved, zero |
| 448 | 32 | row seal over payload `0..448` |

Rows are ordered without a set: closed mask order, direction order, then the
canonical exit-coordinate key. Sequence must equal physical position inside
its universe block. A duplicate/reordered coordinate, empty/invalid mask,
foreign universe/grid/run identity, invalid direct-fact arithmetic or nonzero
reserve refuses.

### 23.2 Completion receipts — 976 bytes

One completion is a 944-byte payload plus its full 32-byte content digest/seal:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 8 | receipt and row versions, both `1` |
| 8 | 32 | candidate-universe identity |
| 40 | 8 | committed row count |
| 48 | 32 | digest of the exact ordered row payload sequence |
| 80 | 112 | fourteen `u64` reconciliation counts: sweep/frequent/infrequent/closure partitions, direction counts, long/short cells and total expected/evaluated cells |
| 192 | 12 | extinction depth, signal rung and one-minute horizon |
| 204 | 3 | family, extinction-complete and closure-complete tags |
| 207 | 33 | reserved, zero |
| 240 | 352 | eleven 32-byte identities: data, feed, source commit, vocabulary, evaluation policy, long/short grid policy/resolution, canonical calendar policy and causal prior-day policy |
| 592 | 20 | canonical requested month-span endpoints |
| 612 | 32 | requested-span digest recomputed from those endpoints |
| 644 | 12 | reserved, zero |
| 656 | 96 | complete signal-rung and exact-one-minute calendar coverage |
| 752 | 56 | exact signal stream count/endpoints/digest |
| 808 | 56 | exact requested-span one-minute execution stream count/endpoints/digest |
| 864 | 32 | exact signal-column digest |
| 896 | 32 | exact execution-column digest |
| 928 | 16 | reserved, zero |
| 944 | 32 | completion content digest and record seal over payload `0..944` |

Every expected/evaluated reconciliation count must agree, the frequent
partition must close, and the row count must equal the complete long-plus-short
grid population. Rows are written and synced before the receipt is written and
synced last. A crash may leave one valid trailing row prefix. Only the
byte-exact same universe may append its missing suffix and receipt; a foreign
retry refuses rather than truncating or overwriting history. Reopen validates
every sealed record, canonical order and receipt-to-row digest before indexing.
Exact committed retry reuses bytes.

Opening is O(rows + receipts) time and bounded index space. Open validates every
record seal and semantic field. Later generation checks bind the held
lock/row/receipt paths by metadata; on Unix they bind device and inode plus size
and high-resolution modification/change times, but do not rehash full contents.
Non-Unix generation identity is weaker and is recorded in limits §150. A
validated sequence seek is worst-case O(1) in record count; indexed universe
lookup is average O(1), a page is O(page rows), and open, hashing, durability,
filesystem latency and construction are not O(1). D-0468, CU-01 and CU-02 bind
the tested boundary.

## 24. Pre-Admission Data audit ledger — version 1

`pre-admission-data-v1.bin` and `pre-admission-data-v1.lock` form the second
audit-only format in the selection-independent Step-3 path. The data file starts
with a sealed 64-byte `BTX-PREADMIT-V1\0` header carrying version `1`, kind `1`
and stride `740`. Each logical audit entry occupies two adjacent records: a
`Data` record is appended and synced first, then a byte-equivalent semantic
`Completion` record is appended and synced last. Only the kind differs. One
valid final Data orphan may be completed by the exact retry; every middle,
foreign or second orphan refuses.

Each 740-byte record is a 708-byte payload plus a full 32-byte
domain-separated BLAKE3 seal:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 16 | record version, Data/Completion kind and canonical sequence |
| 16 | 96 | Pre-Admission identity, Candidate Universe identity and Candidate completion digest |
| 112 | 24 | candidate row count, instrument family, signal rung, one-minute horizon and zero reserves |
| 136 | 20 | canonical requested month span |
| 156 | 128 | feed, source commit, measured-calendar policy and daily-reference policy digests |
| 284 | 40 | exact signal count and ordered-bar digest |
| 324 | 56 | complete one-minute-context count, endpoints and ordered-bar digest |
| 380 | 40 | exact evaluated execution-subslice count and ordered-bar digest |
| 420 | 56 | prior-day daily-reference count, endpoints and ordered-bar digest |
| 476 | 88 | eligibility count, eligible count, excluded-day count and both ordered digests |
| 564 | 56 | full requested-span signal calendar rung, bounds and receipt digest |
| 620 | 56 | full requested-span one-minute calendar rung, bounds and receipt digest |
| 676 | 24 | explicit nonzero signal, minute-context and daily stored-load ceilings |
| 700 | 8 | execution subslice start index inside the complete minute context |
| 708 | 32 | seal over payload `0..708` |

The private preparation path measures actual candle slices. It requires the
execution slice to equal one contiguous range of the supplied minute context,
recomputes the exact signal and one-minute full-span calendar receipts, binds
the exact daily records and eligibility bytes, and reconciles every Candidate
completion source term. The public API can only bounded-open, audit and page
existing bytes. It cannot author them, so a self-consistent file is structural
audit evidence rather than production source authority until the typed
Candidate producer owns the private append path.

Opening validates the complete bounded file, both members of every pair and
the cached lock/data generation before indexing. On Unix, generation binds
device/inode, length, high-resolution modification/change times and a complete
content hash, with metadata checked before and after hashing. One validated
fixed-record seek is worst-case O(1) in record count; open is O(file bytes), a
page is O(returned rows), and source preparation, hashing, locking, sync and
filesystem latency remain input/system dependent. D-0471, PAD-01/PAD-02 and
limits §153 define the boundary.

## 25. Population Statistics audit ledger — version 2

`population-statistics-v2-audit.bin` and its lock form the third audit-only
Step-3 format. The data file begins with a sealed 64-byte
`BTX-POPSTATS-V2\0` header carrying version `2`, kind `1` and stride `1,024`.
Every record is a 992-byte payload plus a full 32-byte domain-separated BLAKE3
seal. A logical block is ordered as Data, all Candidate rows, all Period rows
in period-major order, all Split rows in split-major order, then Completion.
The complete prefix is synced before Completion is synced last.

All record payloads start with version, kind, physical record sequence and the
32-byte audit identity in bytes `0..48`. Kind-specific live regions are:

| Kind | Live payload region | Bound facts |
|---|---:|---|
| Data / Completion | `0..864` | logical sequence; rung/horizon/span; CSCV segments, periods and splits; bootstrap draws/seed/block; exact NIFTY and BANKNIFTY Pre-Admission bindings; feed/commit/calendar/daily policies; Wilson/CSCV policies; exact White/SPA/Romano--Wolf family evidence; split-derived PBO counts/fraction/bits; ordered candidate/period/split digests |
| Candidate | `0..280` | canonical global and family sequence, family, semantic and Pre-Admission identities, trade/win totals, Wilson bits, Romano--Wolf statistic/rank/exceedances/exact initial and adjusted fractions, ordered period/split digests |
| Period | `0..152` | candidate and period sequences, candidate identity, return in paisa, trade/win counts and shared aligned-period identity |
| Split | `0..168` | candidate and split sequences, candidate identity, segment count, complementary train/test masks, train/test scores and split identity |

Every unused byte through offset 992 is reserved zero. Data and Completion
repeat identical semantic manifest fields; only kind and physical placement
differ. Open recomputes the exact block from raw rows, including candidate
order/totals, aligned-period and canonical-split digests, CSCV placement/counts
and the stored bootstrap family. Only one valid trailing block prefix is
recoverable, and only its byte-identical retry may append the suffix and
Completion.

The public boundary can bounded-open, verify its exact Pre-Admission pair,
lookup a completed audit and page candidates. Source construction, writable
open and append remain test-private, so the file is not production statistical
authority yet. One validated candidate seek is worst-case O(1) in record count;
open/recomputation and all statistical procedures are input-dependent. D-0472,
PS-01/PS-02 and limits §154 define the boundary.

## 26. Candidate observation companion authority — version 1

`candidate-observation-authorities-v1.bin` and its advisory lock retain the
source and policy authority required to reproduce exact Candidate session
observations. The file begins with a 64-byte header: 16-byte magic
`BTX-OBS-AUTH-H1\0`, `u32` version `1`, `u32` record stride `512`, eight zero
reserved bytes, then a 32-byte BLAKE3 seal over bytes `0..32`.

Each logical authority occupies two adjacent fixed 512-byte records. A Data
record is appended and synced first; its Completion is appended and synced
last. Both records use a 480-byte payload plus a 32-byte domain-separated seal.
The Data live payload is:

| Offset | Size | Field |
|---:|---:|---|
| 0 | 16 | `BTX-OBS-AUTH-D1\0` magic |
| 16 | 4 | file/version `1` |
| 20 | 4 | kind `1` (Data) |
| 24 | 32 | observation authority ID |
| 56 | 32 | complete paired-observation identity |
| 88 | 32 | paired source-only identity |
| 120 | 32 | observation/score-policy identity |
| 152 | 32 | deterministic CSCV-layout identity |
| 184 | 64 | NIFTY then BANKNIFTY Candidate universe IDs |
| 248 | 64 | NIFTY then BANKNIFTY observation-family IDs |
| 312 | 32 | ordered accepted-session digest |
| 344 | 32 | ordered exact-period digest |
| 376 | 32 | ordered canonical split-score digest |
| 408 | 8 | execution rung seconds and horizon bars |
| 416 | 24 | period, NIFTY-candidate and BANKNIFTY-candidate counts |
| 440 | 8 | segment count and observation schema version |
| 448 | 24 | periods per segment, split count and split-score row count |
| 472 | 4 | versioned maximum segment count, exactly `16` |
| 476 | 4 | reserved zero |
| 480 | 32 | Data-record seal |

Completion carries 16-byte `BTX-OBS-AUTH-C1\0` magic, version `1`, kind `2`,
the authority ID, Data-record seal, paired identity and logical record sequence
through payload offset 128. Bytes `128..480` are zero and bytes `480..512`
carry its completion seal. It must be adjacent to and byte-bind the preceding
Data.

This is deliberately a **companion authority, not a raw-observation store**.
No period row and no split-score row is persisted here; existing Population
Statistics V2 bytes remain unchanged. After a crash, the one allowed trailing
Data can be completed only when a rerun rederives the exact opaque observation
pair and produces byte-identical Data. Exact completed reruns reuse without
appending. Open validates explicit file/count ceilings, header and every pair;
cached access also generation-checks the named lock/data paths and hashes the
bounded file. Missing/non-directory roots are refused and never created.

The fixed stride makes one admitted pair position arithmetic constant in
record count. Opening, scanning, hashing, Candidate replay, observation
construction, split derivation, locking and sync remain input/filesystem
dependent. D-0476, CO-01..CO-03 and limits §157 define the semantic and honest
complexity boundary; D-0473/limits §155 still govern physical-volume identity
and hot unplug.

## 27. Run ledger — `results/runs.bin`, versions 2 and 3

`runs.bin` is the ledger §12.3 calls the public commit marker: one fixed-stride
record per finished run. This section was absent until P1-16-04 (D-1940); the
offsets below are counted from `cli::results::Record::to_bytes`, not from its
comments. All integers are little-endian.

The 16-byte header is `BRUTEXRS` at `0..8`, the version as a `u32` at `8..12`,
and four bytes at `12..16` that the writer leaves zero. The reader checks the
magic and the version only; it does not check those four bytes. A fresh ledger
is written at version `3`. Version `2` is READ and never appended to, and every
other version is refused. A length that is not `16 + n*stride` is refused.

Each version-3 record is 261 bytes. Version 2 is the same record without the
mask: 213 bytes, a 205-byte payload and the seal at `205..213`. Every version-2
offset below `205` is the version-3 offset; a version-2 record is checked
against its own seal and then read with six zero mask words.

| Offset | Size | Field |
|---:|---:|---|
| 0 | 32 | nine-term run identity |
| 32 | 8 | finish time, UTC microseconds, `i64` |
| 40 | 16 | feed word, zero-padded |
| 56 | 16 | instrument, zero-padded |
| 72 | 16 | signal rung, zero-padded |
| 88 | 2 | first year, `u16` |
| 90 | 1 | first month |
| 91 | 2 | last year, `u16` |
| 93 | 1 | last month |
| 94 | 4 | months asked, `u32` |
| 98 | 4 | months found, `u32` |
| 102 | 8 | signal bars swept, `u64` |
| 110 | 8 | support threshold, `u64` |
| 118 | 8 | combinations produced, `u64` |
| 126 | 4 | deepest level, `u32` |
| 130 | 1 | halted: `1` when a budget stopped the walk |
| 131 | 8 | trades of the chosen combination, `u64` |
| 139 | 8 | pessimistic total paisa, `i64` |
| 147 | 8 | optimistic total paisa, `i64` |
| 155 | 8 | worst trade paisa, `i64` |
| 163 | 8 | maximum drawdown paisa, `i64` |
| 171 | 8 | winners' adverse excursion ppm, `i64` |
| 179 | 8 | winners' favourable excursion ppm, `i64` |
| 187 | 8 | every trade's adverse excursion ppm, `i64` |
| 195 | 10 | five exit rungs, `i16` each, `-1` for none: stop, target, TSL, TTP arm, TTP trail |
| 205 | 48 | six condition-mask `u64` words (version 3 only) |
| 253 | 8 | first eight BLAKE3 bytes over `0..253` |

The seal is an accident detector, not a security claim. The test
`cli::results::tests::the_store_format_doc_states_the_ledger_this_build_writes`
binds this section's magic, versions, strides and seal row to the constants.

## 28. Population rows — `results/population-v1.bin`, version 1

The population row file holds one fixed-stride row per expanded strategy. Its
receipt files (`population-completions-v2.bin` to `-v4.bin`) are separate; §14
describes the version-4 receipt. Offsets are counted from
`cli::population::PopulationRowV1::payload_bytes` (D-1940). All integers are
little-endian.

The 16-byte header is `BRUTEXPP` at `0..8`, version `1` at `8..12`, and four
reserved bytes at `12..16` that must be zero. A wrong magic, another version,
a non-zero reserved byte or a length that is not `16 + n*312` is refused.

Each row is 312 bytes: a 304-byte payload and an 8-byte seal.

| Offset | Size | Field |
|---:|---:|---|
| 0 | 32 | population identity |
| 32 | 8 | row sequence, `u64` |
| 40 | 32 | strategy digest |
| 72 | 48 | six condition-mask `u64` words; all zero is refused |
| 120 | 1 | direction: `1=long`, `2=short` |
| 121 | 1 | instrument family: `1=NIFTY`, `2=BANKNIFTY` |
| 122 | 1 | closure: `1=closed`, `2=redundant`, `3=unknown` |
| 123 | 1 | admission status: `0=admitted`, `1=rejected`, `2=unmeasured`, `3=refused` |
| 124 | 4 | signal rung seconds, `u32` |
| 128 | 8 | support hits, `u64` |
| 136 | 20 | stop, target, TSL, TTP-arm, TTP-trail indices, `u32` each; absent is `u32::MAX`, and TTP is wholly present or wholly absent |
| 156 | 4 | reserved, zero |
| 160 | 8 | drawdown, `u64` |
| 168 | 8 | worst loss, `u64` |
| 176 | 8 | losing rate ppm, `u64` |
| 184 | 8 | losing trades, `u64` |
| 192 | 16 | loss ratio ppm: tag byte (`0` absent, `1` present), seven zero bytes, `u64` value (zero when absent) |
| 208 | 8 | pessimistic profit, `i64` |
| 216 | 8 | winning trades, `u64` |
| 224 | 8 | win rate ppm, `u64` |
| 232 | 16 | reward-to-risk ppm, the same tagged shape as the loss ratio |
| 248 | 8 | average win, `u64` |
| 256 | 8 | average loss, `u64` |
| 264 | 8 | assurance ppm, `u64` |
| 272 | 32 | admission reasons, failed, unmeasured and refused bit sets, `u64` each |
| 304 | 8 | first eight BLAKE3 bytes over `0..304` |

A row whose seal, tag, rate or mask fails its check is refused, not skipped.
`cli::population::tests::the_store_format_doc_states_the_population_row_this_build_writes`
binds this section to the constants.

## 29. Live top-N — `results/live/<identity>.bin`, version 2

A live file is not history. It is the in-flight top-N of one run, named by the
64 lowercase hex characters of its run identity, replaced whole by write, sync
and rename, and removed when the run finishes. A killed run leaves its file
behind; `cli::live` says why that cannot be fixed from the reader. Offsets are
from `cli::live::Live::publish` (D-1940). All integers are little-endian.

| Offset | Size | Field |
|---:|---:|---|
| 0 | 8 | `BRUTEXLV` |
| 8 | 4 | version `2`, `u32` |
| 12 | 4 | row count, `u32` |
| 16 | 8 | trials weighed so far, `u64` |
| 24 | 8 | the `\|t\|` bar in thousandths, `i64` |
| 32 | 8 | candidates priced, `u64` |
| 40 | `count * 280` | rows, each a §13 frontier row of the same 280 bytes |

The reader skips, rather than refuses, a file with the wrong magic, another
version, a short header or a length that is not `40 + count*280`: a half-written
file is an ordinary state here. There is no older version to read, because a
live file from an older build belongs to a process that has stopped.
`cli::live::tests::the_store_format_doc_states_the_live_file_this_build_writes`
binds this section to the constants.

## 30. Pre-Admission Data audit ledger — version 2

`pre-admission-data-v2.bin` and `pre-admission-data-v2.lock` are the successor
of §24. Version 1 bytes are not reinterpreted. Version 2 repeats every version-1
source field and adds the Candidate Universe reconciliation, and it may carry
zero Candidate rows only when the Candidate commit proves natural extinction.
Offsets are from `encode_v2_core`, `encode_v2_reconciliation` and `header_v2` in
`cli::pre_admission_data` (D-1940). All integers are little-endian.

The 64-byte header is `BTX-PREADMIT-V2\0` at `0..16`, version `2` at `16..20`,
kind `2` at `20..24`, stride `812` as a `u64` at `24..32`, and a 32-byte BLAKE3
seal over `0..32` under the domain `brutex-pre-admission-header-v2\0` at
`32..64`.

Each 812-byte record is a 780-byte payload and a 32-byte domain-separated
BLAKE3 seal. As in §24, each logical entry is a Data record (kind `1`) synced
first and a Completion record (kind `2`) synced last; only the kind differs.

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 4 | record version `2`, `u32` |
| 4 | 4 | kind: `1=Data`, `2=Completion`, `u32` |
| 8 | 8 | canonical sequence, `u64` |
| 16 | 96 | Pre-Admission identity, Candidate Universe identity, Candidate completion digest |
| 112 | 8 | Candidate row count, `u64` |
| 120 | 1 | instrument family: `1=NIFTY`, `2=BANKNIFTY` |
| 121 | 3 | reserved, zero |
| 124 | 4 | signal rung seconds, `u32` |
| 128 | 4 | one-minute horizon bars, `u32` |
| 132 | 4 | reserved, zero |
| 136 | 20 | canonical requested month span |
| 156 | 128 | feed, source commit, calendar policy and daily-reference policy digests |
| 284 | 40 | exact signal stream: count `u64`, digest |
| 324 | 56 | complete one-minute context: count, first and last timestamp `i64`, digest |
| 380 | 40 | exact execution subslice: count `u64`, digest |
| 420 | 56 | prior-day daily stream: count, first and last timestamp `i64`, digest |
| 476 | 88 | eligibility: count, eligible count, excluded-day count, eligibility digest, excluded-days digest |
| 564 | 56 | signal calendar: rung `u32`, four zero bytes, first and last day `i64`, digest |
| 620 | 56 | one-minute calendar, the same shape |
| 676 | 24 | signal, minute-context and daily stored-load ceilings, `u64` each |
| 700 | 8 | execution start index inside the minute context, `u64` |
| 708 | 64 | reconciliation: sweep trials, frequent, infrequent, closed, redundant and unknown-closure itemsets, long and short exit cells per mask, `u64` each |
| 772 | 4 | extinction depth, `u32` |
| 776 | 1 | extinction complete, `0` or `1` |
| 777 | 1 | closure complete, `0` or `1` |
| 778 | 2 | reserved, zero |
| 780 | 32 | seal over payload `0..780`, domain `brutex-pre-admission-record-v2\0` |

`0..708` is the §24 payload with the version field reading `2`; a `const`
assertion in the module holds the two widths equal.
`cli::pre_admission_data::tests::the_store_format_doc_states_the_pre_admission_v2_this_build_writes`
binds this section to the constants.

## 31. Execution V3 authority — record layout 5

Execution V3 is four append-only fixed-record files and one lock, beside each
other in the root the ledger is opened on:

| File | Magic (16 bytes) | Domain | Stride |
|---|---|---:|---:|
| `execution-parameters-v3.bin` | `BTX-EXV3-PARAM\0\0` | 1 | 1,024 |
| `execution-percentiles-v3.bin` | `BTX-EXV3-PCTL\0\0\0` | 2 | 128 |
| `execution-dispositions-v3.bin` | `BTX-EXV3-DISP\0\0\0` | 3 | 1,024 |
| `execution-completions-v3.bin` | `BTX-EXV3-CMPL\0\0\0` | 4 | 1,024 |
| `execution-v3.lock` | none; must be empty | — | — |

The files have no file header. A length that is not a whole number of strides
is refused. The semantic seam is V3; the record layout written in every record
is `5`, and layout-4 bytes are refused rather than reinterpreted. Parameters,
percentile atoms and dispositions are synced in that order, then one
Completion is appended and synced last. Offsets are counted from the four
`encode` functions in `cli::execution_v3` (D-1940). All integers are
little-endian. Every record starts with the same 24 bytes and ends with a full
32-byte BLAKE3 seal over its payload, under a per-kind domain
`brutex-execution-v3-layout-5-<kind>-seal\0`:

| Offset | Size | Field |
|---:|---:|---|
| 0 | 16 | magic |
| 16 | 4 | layout `5`, `u32` |
| 20 | 4 | domain, `u32` |
| stride − 32 | 32 | seal |

Every byte between the last field below and the seal is zero, and is checked.

**Parameter, 1,024 bytes.**

| Offset | Size | Field |
|---:|---:|---|
| 24 | 512 | sixteen digests: parameter core ID, parameter ID, population ID, population ordered digest, source finalization ID and completion ID, policy, resolution, training, instrument, feed, commit, calendar, percentile digests, cost-model ID, execution-law digest |
| 536 | 163 | evaluation fingerprint |
| 699 | 5 | family (`1=NIFTY`, `2=BANKNIFTY`), direction (`1=long`, `2=short`), range-policy, selector-policy and forced-stop-policy tags |
| 704 | 3 | reserved, zero |
| 707 | 24 | rung, horizon bars, execution resolution seconds, entry delay minutes, forced-exit IST minute, forced-stop index (`u32::MAX` for none), `u32` each |
| 731 | 32 | four run parameters, `u64` each |
| 763 | 16 | ratio minimum and maximum, hundredths, `i64` each |
| 779 | 88 | maximum levels, maximum ratio pairs, policy maximum cells, resolved stop, target, trail, ratio-pair and cell counts, maximum ambiguity bars, maximum gap bars, training bars, `u64` each |
| 867 | 24 | forced-stop ppm, training first and last timestamp, `i64` each |
| 891 | 16 | percentile offset and count, `u64` each |
| 907 | 85 | reserved, zero |

**Percentile atom, 128 bytes.**

| Offset | Size | Field |
|---:|---:|---|
| 24 | 32 | parameter core ID |
| 56 | 1 | axis: `1=stop`, `2=target`, `3=trail` |
| 57 | 3 | reserved, zero |
| 60 | 12 | ordinal, numerator, denominator, `u32` each |
| 72 | 24 | reserved, zero |

**Disposition, 1,024 bytes.**

| Offset | Size | Field |
|---:|---:|---|
| 24 | 512 | sixteen digests: disposition ID, population ID, population row ID, candidate semantic ID, candidate base-row ID, admission decision ID, finalization row ID and completion ID, parameter ID, execution-run ID, evaluated-grid, resolution, column, context, runner-disposition and selected-exit digests |
| 536 | 40 | global sequence, family sequence, cell ordinal, support hits, refusal bits, `u64` each |
| 576 | 4 | family, direction, admission status (`1=admitted` … `4=refused`), terminal (`1=authorized`, `2=policy refused`) |
| 580 | 4 | reserved, zero |
| 584 | 28 | rung, horizon bars, stop, target, TSL, TTP-arm and TTP-trail indices, `u32` each; absent index is `u32::MAX` |
| 612 | 380 | reserved, zero |

**Completion, 1,024 bytes.**

| Offset | Size | Field |
|---:|---:|---|
| 24 | 56 | block sequence, first parameter record, parameter count, first percentile record, percentile count, first disposition record, disposition count, `u64` each |
| 80 | 480 | fifteen digests: completion ID, population ID, population ordered digest, source finalization ID and completion ID, source admission block ID and completion ID, execution-law digest, ordered parameter, percentile and disposition digests, NIFTY and BANKNIFTY disposition digests, NIFTY and BANKNIFTY authority IDs |
| 560 | 128 | four parameter-ID slots |
| 688 | 40 | row, NIFTY, BANKNIFTY, authorized and policy-refused counts, `u64` each |
| 728 | 4 | rung, `u32` |
| 732 | 4 | reserved, zero |
| 736 | 32 | four admission-status counts, `u64` each |
| 768 | 128 | sixteen terminal-matrix cells, `u64` each |
| 896 | 96 | reserved, zero |

A decoded record is re-encoded and compared byte for byte; a record that
decodes but is not canonical is refused.
`cli::execution_v3::tests::the_store_format_doc_states_the_execution_v3_layout_this_build_writes`
binds the magics, layout, domains and strides of this section to the
constants.

## 32. Execution V4 authority — version 4

Execution V4 is an independent successor of §31 after Population V6. No V3 byte
is accepted. It has the same four-files-and-a-lock shape, the same 24-byte
record prefix (magic, version `4`, domain) and the same trailing 32-byte seal,
under the domains `brutex-execution-v4-<kind>-seal\0`. Offsets are from the four
`encode` functions in `cli::execution_v4` (D-1940).

| File | Magic (16 bytes) | Domain | Stride |
|---|---|---:|---:|
| `execution-parameters-v4.bin` | `BTX-EXV4-PARAM\0\0` | 1 | 1,280 |
| `execution-percentiles-v4.bin` | `BTX-EXV4-PCTL\0\0\0` | 2 | 128 |
| `execution-dispositions-v4.bin` | `BTX-EXV4-DISP\0\0\0` | 3 | 1,024 |
| `execution-completions-v4.bin` | `BTX-EXV4-CMPL\0\0\0` | 4 | 1,280 |
| `execution-v4.lock` | none; must be empty | — | — |

A naturally extinct family writes no parameter and no disposition; its
terminal is kept in the Completion.

**Parameter, 1,280 bytes.**

| Offset | Size | Field |
|---:|---:|---|
| 24 | 608 | nineteen digests: the sixteen of §31 with population completion ID after population ID, and source admission block ID and completion ID after the source finalization pair |
| 632 | 163 | evaluation fingerprint |
| 795 | 5 | family, direction, range-policy, selector-policy and forced-stop-policy tags |
| 800 | 3 | reserved, zero |
| 803 | 24 | the six `u32` of §31 |
| 827 | 32 | four run parameters, `u64` each |
| 859 | 16 | ratio minimum and maximum, `i64` each |
| 875 | 88 | the eleven `u64` of §31 |
| 963 | 24 | forced-stop ppm, training first and last timestamp, `i64` each |
| 987 | 16 | percentile offset and count, `u64` each |
| 1003 | 245 | reserved, zero |
| 1248 | 32 | seal |

**Percentile atom, 128 bytes.** The §31 layout, with version `4`.

**Disposition, 1,024 bytes.**

| Offset | Size | Field |
|---:|---:|---|
| 24 | 640 | twenty digests: disposition ID, population ID, population completion ID, population family-row ID, population row ID, candidate semantic ID, candidate base-row ID, base-evidence ID, admission decision ID, finalization family-row ID, finalization row ID, finalization completion ID, parameter ID, execution-run ID, evaluated-grid, resolution, column, context, runner-disposition and selected-exit digests |
| 664 | 40 | global sequence, family sequence, cell ordinal, support hits, refusal bits, `u64` each |
| 704 | 4 | family, direction, admission status, terminal |
| 708 | 4 | reserved, zero |
| 712 | 28 | rung, horizon bars and five exit indices, `u32` each |
| 740 | 252 | reserved, zero |
| 992 | 32 | seal |

**Completion, 1,280 bytes.**

| Offset | Size | Field |
|---:|---:|---|
| 24 | 56 | block sequence and the first/count pairs for parameters, percentiles and dispositions, `u64` each |
| 80 | 640 | twenty digests: completion ID, population ID, population completion ID, population ordered digest, NIFTY and BANKNIFTY population family-row IDs, NIFTY and BANKNIFTY finalization family-row IDs, source finalization ID and completion ID, source admission block ID and completion ID, execution-law digest, ordered parameter, percentile and disposition digests, NIFTY and BANKNIFTY disposition digests, NIFTY and BANKNIFTY authority IDs |
| 720 | 128 | four parameter-ID slots |
| 848 | 88 | row, evaluated and decision counts; the same three for NIFTY and for BANKNIFTY; authorized and policy-refused counts; `u64` each |
| 936 | 8 | rung and horizon bars, `u32` each |
| 944 | 2 | NIFTY and BANKNIFTY family terminal: `1=evaluated`, `2=naturally extinct` |
| 946 | 2 | reserved, zero |
| 948 | 32 | four admission-status counts, `u64` each |
| 980 | 128 | sixteen terminal-matrix cells, `u64` each |
| 1108 | 140 | reserved, zero |
| 1248 | 32 | seal |

`cli::execution_v4::tests::the_store_format_doc_states_the_execution_v4_records_this_build_writes`
binds this section to the constants.

## 33. Global Replay V3 — version 3

Global Replay V3 is five append-only fixed-record files and one lock. It never
opens V1 or V2 replay records. One commit appends witnesses, candidates,
decisions and money rows, each file synced, and then one Completion last.
Offsets are from the five `encode` functions in `cli::global_replay_v3`
(D-1940). All integers are little-endian.

| File | Magic (16 bytes) | Stride |
|---|---|---:|
| `global-replay-witnesses-v3.bin` | `BTX-GRV3-WIT\0\0\0\0` | 512 |
| `global-replay-candidates-v3.bin` | `BTX-GRV3-CAN\0\0\0\0` | 512 |
| `global-replay-decisions-v3.bin` | `BTX-GRV3-DEC\0\0\0\0` | 384 |
| `global-replay-money-v3.bin` | `BTX-GRV3-MNY\0\0\0\0` | 512 |
| `global-replay-completions-v3.bin` | `BTX-GRV3-CMP\0\0\0\0` | 1,024 |
| `global-replay-v3.lock` | none | — |

There is no file header and no domain field. Every record starts with the
16-byte magic and version `3` as a `u32` at `16..20`, and ends with a 32-byte
BLAKE3 seal over the payload under `brutex-global-replay-v3-<kind>-seal\0`. The
bytes between the last field and the seal are zero and checked. Family is
`1=NIFTY`, `2=BANKNIFTY`; direction `1=long`, `2=short`; feed `1=groww`,
`2=dhan`, `3=truedata`, `4=gdfl`, `5=zerodha`; path `1=priceable`,
`2=block-only`, `3=crossing refused`, `4=both`.

A **trade row** is 88 bytes: signal, entry and exit bar as `u64`, then best,
worst, entry and exit time, adverse, adverse paisa, favourable and favourable
paisa as `i64`. A **VIX stamp** is 64 bytes: tag `0` and 63 zero bytes when
absent, or tag `1`, seven zero bytes and the seven `i64` of the India VIX
candle (time, open, high, low, close, volume, open interest).

**Witness, 512 bytes.**

| Offset | Size | Field |
|---:|---:|---|
| 20 | 288 | nine digests: witness, selection, row, selected exit, universe, run, feed, strategy, ordered candidates |
| 308 | 48 | six condition-mask `u64` words |
| 356 | 24 | first candidate, candidate count, pricing-refused count, `u64` each |
| 380 | 4 | rung seconds, `u32` |
| 384 | 2 | rank, `u16` |
| 386 | 3 | family, direction, feed |
| 389 | 91 | reserved, zero |
| 480 | 32 | seal |

**Candidate, 512 bytes.**

| Offset | Size | Field |
|---:|---:|---|
| 20 | 160 | five digests: candidate, witness, selection, row, strategy |
| 180 | 2 | stream ordinal, `u16` |
| 182 | 32 | candidate ordinal, signal bar, entry bar, occupied-through bar, `u64` each |
| 214 | 24 | signal, entry and occupied-through time, `i64` each |
| 238 | 1 | path |
| 239 | 89 | price: tag `0` and 88 zero bytes, or tag `1` and a trade row |
| 328 | 16 | ambiguous bars, gap fills, `u64` each |
| 344 | 4 | rung seconds, `u32` |
| 348 | 2 | rank, `u16` |
| 350 | 3 | family, direction, feed |
| 353 | 127 | reserved, zero |
| 480 | 32 | seal |

**Decision, 384 bytes.**

| Offset | Size | Field |
|---:|---:|---|
| 20 | 128 | four digests: decision, candidate, witness, strategy |
| 148 | 8 | sequence, `u64` |
| 156 | 2 | stream ordinal, `u16` |
| 158 | 8 | candidate ordinal, `u64` |
| 166 | 16 | entry and occupied-through time, `i64` each |
| 182 | 4 | rung seconds, `u32` |
| 186 | 2 | rank, `u16` |
| 188 | 3 | family, direction, path |
| 191 | 1 | disposition: `1=admitted`, `2=blocked occupied`, `3=blocked simultaneous`, `4=unreachable`, `5=refused` |
| 192 | 8 | occupied-through time, `i64`; zero for tags 3 to 5 |
| 200 | 32 | the admitted decision for tag 3; zero otherwise |
| 232 | 120 | reserved, zero |
| 352 | 32 | seal |

**Money, 512 bytes.** Written only for a globally admitted, priceable decision.

| Offset | Size | Field |
|---:|---:|---|
| 20 | 160 | five digests: money, decision, candidate, witness, strategy |
| 180 | 8 | decision sequence, `u64` |
| 188 | 2 | stream ordinal, `u16` |
| 190 | 4 | rung seconds, `u32` |
| 194 | 2 | rank, `u16` |
| 196 | 3 | family, direction, feed |
| 199 | 88 | trade row |
| 287 | 64 | India VIX stamp at entry |
| 351 | 64 | India VIX stamp at exit |
| 415 | 65 | reserved, zero |
| 480 | 32 | seal |

**Completion, 1,024 bytes.**

| Offset | Size | Field |
|---:|---:|---|
| 20 | 96 | completion, replay and publication IDs |
| 116 | 256 | eight selection IDs, one per rung |
| 372 | 128 | ordered witness, candidate, decision and money digests |
| 500 | 64 | first and count for witnesses, candidates, decisions and money, `u64` each |
| 564 | 48 | offered, admitted, blocked-occupied, blocked-simultaneous, unreachable and refused counts, `u64` each |
| 612 | 24 | pricing-refused candidates, admitted pricing-refused, block sequence, `u64` each |
| 636 | 356 | reserved, zero |
| 992 | 32 | seal |

India VIX is stamped after admission and never changes selection, execution,
P&L or the replay identity (`CLAUDE.md` §1). It is hashed into each money id
(`money_id` hashes the entry and exit stamps after the trade row), and so into
the ordered money digest, the publication identity and the completion identity:
a VIX backfill or correction changes those four and no other (numeric-pass19
p19num-1, D-1957). This said VIX "never changes ... money ... identity", which
contradicted the code and the V1 statement in §18.

No production command writes Global Replay V3. Nothing outside
`cli::global_replay_v3` names the module; the Step-3 orchestrator writes Global
Replay V4 instead (`ledger_v6`). This section describes the format the V3 code
and its tests define (crash-edge-pass20 CE-95, D-1956).
`cli::global_replay_v3::tests::the_store_format_doc_states_the_global_replay_v3_this_build_writes`
binds this section to the constants.

## 34. Population Statistics audit ledger — version 3

`population-statistics-v3-audit.bin` and its lock are the mixed-family
successor of §25. Version 2 bytes are unchanged and are not read here. One
block is Data, `Family(NIFTY)`, `Family(BANKNIFTY)`, every candidate statistic
in family order, then Completion last; everything before Completion is synced
first. Offsets are from `encode_manifest`, `encode_family`, `encode_candidate`
and `header` in `cli::population_statistics_v3` (D-1940). All integers are
little-endian.

The 64-byte header is `BTX-POPSTATS-V3\0` at `0..16`, version `3` at `16..20`,
kind `1` at `20..24`, stride `1,024` as a `u64` at `24..32`, and a 32-byte
BLAKE3 seal over `0..32` under `brutex-population-statistics-v3-header\0`.

Every record is a 992-byte payload and a 32-byte seal over it under
`brutex-population-statistics-v3-record\0`. Every payload starts with:

| Offset | Size | Field |
|---:|---:|---|
| 0 | 4 | record version `3`, `u32` |
| 4 | 4 | kind: `1=Data`, `2=Family`, `3=Candidate`, `4=Completion`, `u32` |
| 8 | 8 | physical record sequence, `u64` |
| 16 | 32 | authority identity |

**Data and Completion** carry the same manifest; only the kind differs.

| Offset | Size | Field |
|---:|---:|---|
| 48 | 32 | logical sequence, bootstrap draws, seed, block length, `u64` each |
| 80 | 8 | rung seconds and horizon bars, `u32` each |
| 88 | 16 | requested span: from year, from month, to year, to month, `u32` each |
| 104 | 160 | feed, source commit, calendar policy, daily-reference policy and statistics policy digests |
| 264 | 64 | NIFTY and BANKNIFTY family identities |
| 328 | 32 | ordered candidate digest |
| 360 | 8 | candidate count, `u64` |
| 368 | 624 | reserved, zero |

**Family.**

| Offset | Size | Field |
|---:|---:|---|
| 48 | 4 | family: `1=NIFTY`, `2=BANKNIFTY`, `u32` |
| 52 | 4 | terminal: `1=evaluated`, `2=insufficient for CSCV`, `3=naturally extinct`, `u32` |
| 56 | 16 | Pre-Admission sequence and record index, `u64` each |
| 72 | 320 | ten digests: Pre-Admission authority, Candidate universe, Candidate completion, observation authority, observation identity, observation policy, observation data, observation completion, accepted sessions, CSCV layout |
| 392 | 24 | candidate, period and split counts, `u64` each |
| 416 | 4 | segment count, `u32` |
| 420 | 4 | presence flags: bit 0 White, bit 1 SPA, bit 2 Romano–Wolf, bit 3 PBO; other bits refused |
| 424 | 64 | White: statistic bits, probability bits, exact probability numerator and denominator, `u64` each, family digest; zero when absent |
| 488 | 64 | SPA, the same shape |
| 552 | 32 | Romano–Wolf family digest; zero when absent |
| 584 | 48 | PBO: contributing, bottom-half and unrankable splits, probability bits, numerator, denominator, `u64` each; zero when absent |
| 632 | 128 | ordered candidate, period and split digests, family identity |
| 760 | 232 | reserved, zero |

**Candidate.**

| Offset | Size | Field |
|---:|---:|---|
| 48 | 8 | global sequence, `u64` |
| 56 | 4 | family, `u32` |
| 60 | 4 | reserved, zero |
| 64 | 8 | family sequence, `u64` |
| 72 | 64 | candidate semantic digest, Pre-Admission authority |
| 136 | 80 | trades, wins, Wilson lower bits, Romano–Wolf statistic bits, rank, strict exceedances, initial numerator and denominator, adjusted numerator and denominator, `u64` each |
| 216 | 64 | ordered period and split digests |
| 280 | 712 | reserved, zero |

A naturally extinct family has no candidate rows and no numeric statistic.
`cli::population_statistics_v3::tests::the_store_format_doc_states_the_statistics_v3_this_build_writes`
binds this section to the constants.

## Dated NSE cash-session cache (D-0519)

Runtime `session-masters/NSE_CM_security_ddmmyyyy.csv.gz` retains the exact
compressed exchange response. Its `.receipt` sibling is the publication
marker, UTF-8 lines in this exact order:

```text
brutex-nse-cash-session-v1
date=YYYY-MM-DD
source=<exact dated NSE source URL>
compressed_bytes=<decimal length>
sha256=<lowercase SHA-256 of compressed bytes>
```

The limits are 4 MiB compressed, 32 MiB expanded, and 1 KiB receipt. A dated
advisory lock serializes cooperating installations. Payload, receipt and
directory are synced; an interrupted or conflicting pair refuses rather than
being overwritten. The receipt binds acquisition provenance, not an exchange
signature or an independently embedded trade date. No candle format changes.

## Explicit bar repair V1 (not transparently promoted)

`crates/store/REPAIR.md` specifies the additive revision protocol and exact
144-byte completion receipt. The bar geometry (versions 2 and 3 share it) is
unchanged; a revision file is born at `Layout::CURRENT`, version 3 since D-1571,
whatever its source's version (P14-05, D-1959). Revisions
live below `bar-revisions-v1/<ordinal>/` with their own bar/CRC/lock pair,
create-once `.reserved-v1` marker, and completion-last `.repair-v1` receipt.
Ordinary readers and census paths still select the original data. Missing or
torn revision completion never falls back silently. Publication is the explicit
exception to leaf-only month locks: original shared then revision writer,
nonblocking; there is no reverse acquisition. No live pair is renamed or replaced.

## Evidence-scoped recovery journal V1

Separate files under `audit/recovery-v1/`; no candle or ordinary audit format
changes. Each record is 1,024 bytes, little-endian:

| Byte offset | Length | Field |
|---|---|---|
| 0 | 4 | Magic `BXRJ` |
| 4 | 2 | Version 1 |
| 6 | 1 | Queued 0, InFlight 1, Verified 2, Unverified 3, Exhausted 4, Blocked 5, NotApplicable 6 |
| 7 | 1 | Zero |
| 8 | 32 | Domain-separated work identity |
| 40 / 44 | 4 each | Reserved attempts / unchanged attempts |
| 48 / 56 | 8 each | Actually committed source rows / diagnostics |
| 64 / 66 | 2 each | HTTP status / UTF-8 request-body length |
| 68 | 12 | Zero |
| 80 | 768 | Exact body, 1–768 bytes, then zero padding |
| 848 / 856 | 8 each | Missing / unverified coverage quantities |
| 864 | 156 | Zero |
| 1020 | 4 | CRC-32C over bytes 0..1020 |

Missing/unverified quantities are source days on 1day units and minutes on 1min
units; diagnostics are not mixed into either quantity. An existing work key
cannot bind a different body. A lifetime advisory writer lock protects each
file. Append syncs before publishing the in-memory index; uncertain I/O poisons
that handle. Reopen validates and syncs surviving records, retaining an
interrupted reservation. Corrupt/torn files refuse without truncation.

`<plan-id>.bin` stores month scans, exact-day retry children and evidence
notes. The all-zero key, body `plan-seeded-v1`, seals plan creation. Its
Verified state means reconciliation finished, NOT that all source data exists.
`active.bin` stores activation pointers, with the activation ordinal in
attempts; highest ordinal selects the most recently activated plan, not
first-seen order. Pointer body is the lowercase plan identity and must agree
with its key. File and containing-directory sync precede vendor work.

`attempts.bin` uses the same V1 record layout as an authoritative shared
exact-day attempt index across overlapping plans. Reservations sync there before
the per-plan event mirror. `<plan-id>.stop.bin` records explicit stop/clear
intent separately, so acknowledging a stop never waits for ownership of the
active plan writer. These companions do not reinterpret existing record fields.

The STOP companion has exactly one key, the plan identity, and immutable body
`recovery-stop-v1:<lowercase-plan-id>`. `Blocked` means stopped; `Queued` means
an explicitly requested clear. All numeric quantities and HTTP status are zero.
Repeated same-state writes are idempotent; clear appends history rather than
deleting it. An empty existing companion, foreign key, other state, conflicting
body, corruption or unavailable store refuses; it never grants permission to
resume. The record and newly created directory ancestry are synced before a
STOP is acknowledged. A normal pull with no recovery owner touches no STOP file.

Full open validates the entire journal. The bounded status tail validates only
the requested suffix and physical extent; it is not a prefix audit or a
guarantee that an active writer has completed its sync. A new incompatible
layout requires a new version, never reinterpretation of this V1 stride.

## Sweep attempt and explicit expression evidence (D-0523)

The successor protocols in D-0524 add immutable checkpoint journals and full
canonical cursor states ([AND](20-sweep-resume.md),
[expression](22-expression-search.md)), exact selected-cell candidate/trade
catalogs with distinct AND and expression namespaces
([V1 contract](19-candidate-trades.md)), and separate fixed Selection V6 /
Global Replay V4 records ([contract](21-institutional-sweep.md)). Their widths,
domains, locking and completion ordering are specified in those documents.
Legacy versions and meanings are unchanged; a new reader may not interpret
an older authority as its successor merely because fields look similar.

New evidence lives beside existing historical formats; no existing stride,
condition position or ledger version is reinterpreted. The exact schemas,
magic values, identities, directory layout, seals and publication ordering are
specified in [Sweep evidence V1](16-sweep-evidence.md) and
[Expression evidence V1](17-expression-evidence.md).

Expression V1 uses a 3,457-byte fixed program descriptor, a 3,497-byte header,
56-byte source-row records and a 64-byte counts/file-seal footer. Each row has
24 data bytes plus a 32-byte header/ordinal-bound BLAKE3 seal. The file is
published without replacement before terminal completion is recorded.

Sweep lifecycle records distinguish preparation, orchestration, actual probes,
audits and expressions. A start with no terminal record is not completion or
proof that a process is alive. Terminal records must agree with acknowledged
child contents/cardinalities and identities. Missing, replaced, torn, foreign
or corrupt required evidence refuses; it is never represented as successful
zero rows. Integrity scans are linear in saved rows even though records have a
fixed stride and bounded per-row processing.

## Separate checksum admission V1 (D-0525)

The ordinary bar format and legacy run identities remain unchanged. A cold
audit produces Evidence256 (`BRCAS001`): thirteen little-endian u64 format and
extent fields, four 32-byte digests (raw header, committed records, exact CRC
sidecar, entire data file), and sixteen reserved zero bytes. Signed timestamp
fields retain their two's-complement bits.

The separately keyed Receipt512 (`BRHCRC01`) contains version 1 at byte 8,
source identity at 16, receipt identity at 48, Evidence256 at 80, reserved zero
bytes from 336 to 480, and a separately domain-bound completion seal at 480.
Publication appends an exact existing prefix only: payload sync precedes the
seal, then file and parent directory sync precede acknowledgment. Completed
identical receipts are verified and reused read-only. A typed reader retains
shared receipt and source locks; visible bytes under an exclusive publisher
lock do not establish durability.

Strict stored sweeps also retain a separate fixed 512-byte six-role input
manifest in `audited-inputs-v1/`. Ordered roles are signal, execution, prior
daily, current daily, prior minute and current minute. Exact role receipt IDs
extend the executed data digest under a new domain before the existing nine-term
run identity is formed. See [the full byte and admission contract](24-checksum-admission.md)
for fields, domains, locks, physical caps and failure semantics.

Sweep lifecycle operation 10 is `checksum-audit`. It attests input integrity
only; completion is neither a completed sweep nor a pricing/admission result.

## Additive strict range input links (D-0526)

`audited-spans-v1/` stores 512-byte `BRHSP001` nodes: version at8, preceding
node identity at16, ordinal at48, year at56, month at64, six ordered 40-byte
role/receipt pairs at72, reserved zeros at312..480 and completion seal at480.
Every field uses the existing little-endian/32-byte digest conventions.
The chronological chain's final identity binds all required months and extends
the exact executed digest under a distinct span domain. The same retained
receipt publication/locking protocol applies; existing monthly bytes do not
change. [The admission contract](24-checksum-admission.md) gives the exact
domains, zero-predecessor rule and actual strict audit/pricing handoff.

## Boolean catalog successor records (D-0536–D-0537)

The separate `boolean-candidates-v1`, `boolean-statistics-v1` and
`boolean-admission-v1` namespaces preserve full expression and cash-family
identity without reinterpreting legacy records. They share the112-byte
`BRBLCM01` body-completion protocol. Statistics use512-byte records and research
admission uses a512-byte header plus1024-byte candidate rows; its448-byte
`BRAPRO01` comparison grants no legacy authority. The
[catalog storage contract](27-boolean-catalog-research.md#storage-domains)
records the layout, actual grid descriptors, publication order and strict
observation boundary. Lifecycle operations11,12 and13 name these three stages.

## Incremental Boolean campaign and later evidence — D-0539 through D-0541

`boolean-campaign-v1/<identity>/<sequence>/payload` retains the existing
immutable Journal envelope, seal and separately synced completion marker.
The `BRBCAM01` payload has a256-byte header: magic8, campaign identity32,
preparation descriptor32, ordered-program digest32, then seven u64 fields
(program count, family count, first YYYYMM, last YYYYMM, one-minute horizon,
aggregate state, predecessor sequence), predecessor seal32 and64 zero bytes.
The initial predecessor sequence is `u64::MAX` with a zero seal.

Exactly eight canonical timeframe rows follow. Each uses1168+192F bytes for
F families: state u64, statistics identity/pin64, admission identity/pin64,
reason byte length u64, fixed1024-byte zero-padded UTF-8 reason area, then F
records of ResearchFamily128 + expected candidate identity32 + completion pin32.
Paired zero stage identity/pins mean absent; zero candidate pin means its
completion was not acknowledged. State0 is waiting,1 running,2 paused,3
refused,4 completed. Total Journal payload envelope stays within the smaller
of the configured byte ceiling and2MiB; reserved sequence must remain below1024.
No existing records are deleted, rotated or reinterpreted.

`boolean-oos-v1` shares the112-byte `BRBLCM01` completion receipt but uses a
new232-byte `BRBOOS01` body header. It stores the later identity, original
candidate identity/pin, later source identity, later execution digest, first
and last actual timestamps, execution record count, four civil-month bounds
and nested-body length. The nested candidate-shaped body carries the same
original family, programs, resolved training grids and full coordinate order,
with later observations. Lifecycle operation14 is `boolean-oos`. Its cold
reader authenticates both complete bodies and their exact original/later
links, and grants no source or institutional selection capability.

`boolean-grammar-v1` uses the same Journal container with a48-byte caller
envelope: `BRBGPN01` plan magic, predecessor sequence u64 (zero initially),
predecessor seal32. The enclosed `BRBGBP01` batch is64 bytes plus two
`CURSOR_BYTES` descriptors and N fixed `ENCODED_LEN` programs. Its six u64
fields are work before, work after, programs before, program allowance, node
allowance and program count; one exhaustion byte and seven zero bytes follow.
`BRBGDN01` completion records use112 bytes: magic8, exact plan sequence8,
plan seal32, campaign identity32 and campaign snapshot pin32. Both campaign
fields are zero only for a batch with no programs. No completed child is
inferred from a work reservation or elapsed time.

### Fixed-training qualification successors (D-0542–D-0544)

`BRBQPL01` is a new1,024-byte plan header plus16 bytes per declared later civil
window. It binds the complete original/later campaign descriptors, catalog,
scope, policy, bootstrap procedure, physical limits and finite allocation.
Eight ordered original-catalog digests and eight ordered later-source digests
bind each rung independently, so a valid child cannot be relabelled as a
different timeframe in either production or saved-result observation.
Each window is two signed64-bit IST civil-day numbers. Decoding validates the
deterministically derived complete partition and returns observation metadata;
it does not construct a live plan from saved bytes.

`BRXFVL01` is a separate320-byte fixed-training projection header plus64 bytes
per window. It carries seven full32-byte source/selection identities, ordinal,
fold counts, aggregate return, execution-refusal bits, requested date interval
and original last training day. Each fold retains exact interval, actual session
count, trades, wins, pessimistic integer return and reserved zero padding.
The optional projection never changes `boolean-oos-v1` identity or body bytes.

`boolean-qualification-v1` uses the existing112-byte receipt-last completion
envelope. `BRBQLF01` has a1,024-byte header, complete canonical plan bytes,
128-byte original-linked later-family records, and1,024-byte coordinate rows.
Each coordinate also reserves the complete320+64F fold section for F declared
windows. An explicit absence tag requires that entire section to be zero.
All coordinates are retained. The row records original/later identities,
family/coordinate offsets, the448-byte common policy projection and exact
zero-conservative/shared statistical facts. Unrounded statistics and exact
fractions remain stored; conservative ppm comparisons are derived observations.
Operation15 is `boolean-qualification`; prior operation numbers are unchanged.

`boolean-qualified-campaign-v1` uses the acknowledged Journal container.
Its `BRQCAM01` payload is9,200 bytes:112-byte header and eight1,136-byte slots.
The header binds campaign identity, descriptor and predecessor sequence/pin.
Each slot retains its unit identity, started flag, qualification identity/pin,
reason length and a1,024-byte zero-padded UTF-8 reason. A retry cannot change
completed slots or skip the durable start transition. Restore reconciles all
acknowledged records while charging reservation-only holes to sequence capacity.
No old format, discriminator, condition bit or receipt is rewritten in place.

### Search-wide qualification journal (D-0549)

`boolean-qualified-search-v1` uses the same acknowledged, sealed Journal
container in its own namespace. `BRBQSS01` declares `144 + CURSOR_BYTES` bytes:
magic8, two source digests32 each, policy32, the exact initial cursor, then five
u64 values (program allowance, node allowance, source-byte admission,
record/replay admission and alpha ppm). The search identity hashes this entire
declaration.

`BRBQSR01` has a2,520-byte header, the complete declaration, an exact `BRBGBP01`
batch and optional original `BRBQPL01` qualification plan. Header positions are
magic0, predecessor sequence8, predecessor seal16, ordinal48, phase56, batch
length64, plan length72 and reason length80. Campaign identity/pin occupy88/120.
Eight168-byte rung summaries start at152; each holds child identity32, child
pin32, coordinate count8, four status counts8 each, projection hash32 and
allocation hash32. Reason UTF-8 starts at1496 and has a1,024-byte zero-padded
region. Phase0 means reserved,1 complete and2 refused. Unused child fields and
summaries must be exactly zero. A node-only batch has no qualification plan or
invented result links.

The predecessor chain must include every acknowledged record. A reservation
binds the exact declaration, ordinal, before/after cursor, work counters and
qualification plan. A retry cannot alter that binding. A Complete permits only
the exact next reserved batch; exhaustion is taken from the grammar cursor,
never a work limit. History admission charges payloads, journal envelopes and
retained indexes, and each decoded batch's declared node allowance. Detail reads
reserve one additional batch replay. Stored program-capacity claims are checked
against the external byte bound before allocation. Original qualification rows
are not mutated; their separately derived search projection is448 canonical
bytes per coordinate and its complete order is hashed into the rung summary.

### Shared probability ceilings, additive search V2 (D-0553)

`BRBQSS02` and `BRBQSR02` retain the exact V1 declaration and record strides,
offsets and envelope namespace. Their magic selects `SharedCeilingsV2`:
the original policy's four family-probability ceilings are each capped at the
declared alpha before the complete projection is evaluated. The rule changes
the declaration's bytes and therefore its search identity. V2 full-rung
projection hashing uses `brutex-search-wide-projection-v2\0`; V1 keeps its
original `brutex-search-wide-projection-v1\0` domain and original arithmetic.

Both record and enclosed declaration must name the same known version. A
history transition cannot change the rule. No extra mutable field or optional
fallback controls interpretation. Older V1 records are decoded, recomputed and
rendered with the original policy and guard, including any original rejection
partitions. They are explicitly historical observations, not V2 decisions.
New writer reservations use V2. Original qualification/catalog bodies remain
unchanged and the new format never rewrites old acknowledged history.

## Outer invocation journal V1 — 2026-09-08

This append-only format is separate from strategy identities and result files.
The configured store contains `audit/invocations-v1/index.bin`, one immutable
start per invocation, and one `<exact-id>.bin` journal per ID. IDs are
`(1 << 63) + ordinal`, where ordinals start at one. Every record is 256 bytes:

| Bytes | Field |
|---|---|
| 0..8 | Magic `BXOPAU01` |
| 8..16 | Exact u64 invocation ID |
| 16..24 | Wall-clock milliseconds; zero means unavailable |
| 24..32 | Elapsed microseconds in the owning process |
| 32..40 | Completed structural boundaries, with no invented total |
| 40, 41 | Phase and origin codes |
| 42..44 | HTTP status, zero when unavailable/not applicable |
| 44..46 | Public ASCII label length |
| 46..48 | Reserved zero |
| 48..144 | Bounded public label and zero padding |
| 144..252 | Reserved zero |
| 252..256 | CRC32C of the preceding bytes |

Integer fields are little-endian. Phase codes are start=0, progress=1,
completed=2, refused=3, failed=4 and cancelled=5. Origin codes are CLI=1,
browser task=2 and HTTP request=3. Start/progress without a terminal are
unconfirmed. The index and journal starts must agree. A bounded exact read
checks the indexed start plus the journal's first/last records; it does not
claim to rescan every intermediate CRC. Unknown formats, padding, corruption,
torn tails and inconsistent ancestry refuse. No repair truncates history.

## Native index consistency and single-stop evidence — D-0579/D-0582

All namespaces below retain a zero-byte `owner.lock` and immutable body bytes.
The existing112-byte `BRBLCM01` completion envelope is published last: magic8,
artifact identity32, payload digest32, byte length8, envelope digest32.
Integer fields are little-endian. Existing records are never reinterpreted.

| Namespace / body magic | Fixed layout | Variable sections |
|---|---|---|
| `index-consistency-v1` / `BRICST01` | Header152: magic8, parent32, parent pin32, policy72, count8. Coordinate1824: binding176, full/training/later evaluations536 each, five counts8 each. | Full, training and later week arrays at144 bytes/week, then observed sessions at32 bytes/session. The final count retains the training-session prefix. |
| `index-stop-candidates-v1` / `BRISCT01` | Header128: magic8, identity32, native policy80, count8. Candidate metadata3929: three digests96, family128, direction/rung/day bounds32, full expression3457, truth32, metrics160, three counts24. | Per candidate: events112, trades168, then periods96. Complete program-long/short pairs are mandatory. |
| `index-stop-qualification-v1` / `BRISQF01` | Header990 binds both candidate ancestors, native/daily/institutional policies, numerical procedure, allocation and bounds. | CSCV splits64 each; each candidate row688 is followed by its fixed later folds64 each. Row688 retains four digests128, policy projection448, Wilson source bits8, twelve Romano values96 and fold count8. |

Qualification cold-open replays numerical comparisons against its exact native
parents, verifies all mandatory daily assessments, and admits the aggregate
ancestor bytes under both the recorded and current read bounds. Read-only
decoders cannot mint a native execution capability. Once admitted, direct
indexed pages retain publication leases over the four distinct artifacts;
they do not nest another same-file lock/unlock inside a held lease.

Operation16 is `index-consistency`,17 is `index-stop`, and18 is
`index-stop-qualification`. All prior discriminators retain their old meaning.

`index-stop-search-v1` uses the shared96-byte-overhead acknowledged Journal
container. Its declaration magic is `BRISSD01`: length-framed exact index/feed,
physical-rung mask, four civil-day bounds, initial canonical grammar cursor,
three policy digests, fifteen fixed numeric admissions/procedure values, and
each selected rung's original/later native source digests. Its full hash is
the search identity.

Its `BRISCP01` payload starts with magic8, eight flag bytes, previous sequence8,
previous pin32, five u64 counters/lengths, a complete grammar cursor, eight
identity/pin pairs64 each, then the declaration and optional canonical batch.
Flags retain pending, exhausted, scope and predecessor presence; padding is
zero. The fixed extent is608 + CURSOR_BYTES. A complete nonempty batch carries
exactly the selected qualification links. A node-only batch carries none.
Ancestry covers every acknowledged checkpoint and retains exact program/node
limits. Missing or changed child evidence refuses recovery.

Serialized history and replay budgets are checked before acknowledgment. The
common immutable body writer refuses differing or incomplete prior bytes;
this is not an automatic repair for a torn artifact. Already successful child
artifacts can be reused, but an incomplete conflicting artifact is preserved
and reported. No reader turns a partial write into a completion receipt.

## Original single-stop source companions — D-0585

The immutable body/completion container remains unchanged. Two additive
namespaces retain new source provenance:

| Namespace / magic | Bound contents |
|---|---|
| `index-stop-source-context-v1` / `BRISSC01` | Original native source and loader/calendar versions, build, vocabulary names, input limits, source-day bounds, excluded sessions, eligibility and the exact separately stored signal/execution/exact-minute/daily OHLCV roles. Candles use seven little-endian i64 fields (56 bytes); an explicit alias records a shared signal/execution role instead of a duplicate array. |
| `index-stop-catalog-context-v1` / `BRISCL01` | Fixed 136-byte body: magic8 followed by catalog identity32, catalog completion32, source-context identity32 and source-context completion32. |

The source decoder walks and admits all length/count extents before allocating
arrays. Unknown versions, tails, malformed ordering and absent required role
evidence refuse. The original names are part of the immutable source identity.
The relation cannot attach a foreign source to an otherwise correctly sealed
catalog: inspection must reconstruct every native source/run identity and the
new catalog identity before exposing metadata or candles.

New catalog identity domain `brutex-index-stop-catalog-v2\0` binds the legacy
catalog identity to the source-context identity and completion. This changes
the identity domain, not any `BRISCT01` or `BRISQF01` record stride. Legacy
catalogs lacking a companion retain their old table semantics and explicitly
refuse original-source inspection.

The additive search declaration `BRISSD02` is magic8, original V1 body length8,
that exact `BRISSD01` body, the fixed canonical institutional policy, and one
training/later source-context pair per selected physical rung. Each context
is identity32 plus completion32. The hash of the complete outer declaration
is the new search identity. V1 decoding is retained without rewriting old bytes.

Cumulative ranking stores no replacement result file. It verifies the exact
acknowledged checkpoint prefix and retains each family's original completion
pins. Paging does not mutate a checkpoint or add a second ordering authority
to persistent candidate/qualification records.

## Original single-stop VIX reference companion — D-0587

`index-stop-vix-reference-v1` uses the existing immutable body, zero-byte owner
and 112-byte completion container. All integer fields are little-endian. The
body is additive; no original candidate, source or qualification stride changes.

| Part | Layout |
|---|---|
| Header, 200 bytes | `BRISVX01` magic8, lookup identity32, publication identity32, catalog identity32, catalog completion32, reference-policy digest32, then feed discriminator, setting count, trade count and reference-month count as four u64 words. |
| Setting, 80 bytes each | Original run32, original source32, first trade offset8, trade count8. Extents are contiguous in the original setting order, including zero-trade settings. |
| Month, 72 bytes plus reason | Year8, month8, validated-snapshot flag8, original row count8, snapshot digest32, reason byte length8, then bounded UTF-8 diagnostic bytes. A validated month has no reason; an unavailable month has no row count or snapshot digest. |
| Trade annotation, 240 bytes each | Original local ordinal8, run32, original native trade digest32, entry/exit-bar/exit-from/exit-until timestamps32, month index8, then two 64-byte stamps. Each stamp is state8 and seven i64 candle fields56. Absent and unavailable states require zero payload padding. |

**Units of the stamped candle.** The seven `i64` candle fields are the stored
`NSE-INDIAVIX` bar: time in microseconds, then open, high, low and close in
**hundredths of an India VIX index point** (the price write path's ×100
scale; VIX is a volatility index in points, not a price in rupees), then volume
and open interest (`i64::MIN` when absent). `/index-stop-vix.json` serves the
four price-path fields as `open_paisa` .. `close_paisa`; those names are the
shared path's and are frozen, and the response's `policy` text states the unit
(`api::indexstopvixjson::CANDLE_UNIT`). A `close_paisa` of `1345` is VIX 13.45,
not ₹13.45. numeric-pass19 p19num-2, D-1968.

The body order is header, settings, months and their reasons, then annotations.
`brutex-index-stop-vix-reference-lookup-v1\0` binds catalog identity and pin.
`brutex-index-stop-vix-publication-v1\0` hashes the body except its own
32-byte publication field. The ordinary completion envelope authenticates the
complete resulting body under the lookup identity.

The reader checks every saved setting and original trade digest/interval
against the exact catalog before serving pages. Unknown states, contradictory
month availability, disagreement between repeated references to the same minute,
foreign extents, tails and insufficient whole-publication admission refuse.
The independent reference pins never become a source/run,
search, qualification or ranking identity.

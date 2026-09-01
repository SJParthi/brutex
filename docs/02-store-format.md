# 02 — Store format, version 2

Bytes on disk. This document is the authority; the code follows it.

The version number is in the header. A format change mints a **new version**
and old files stay readable at their own stride. Nothing is ever mutated in
place — `CLAUDE.md` §3 rule 8.

**Version 1 is retired.** §10 says what it was and why it is refused rather
than decoded. Its number is never reused.

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
| 0 | 8 | `magic` | `b"BRUTEXB2"` |
| 8 | 2 | `format_version` | `2` |
| 10 | 2 | `record_stride` | `56`. Read it; never assume it. |
| 12 | 4 | `flags` | bit 0: block checksums present |
| 16 | 8 | `generation` | which commit this slot holds. Higher wins. |
| 24 | 8 | `n_valid` | **the commit counter.** See §5. |
| 32 | 8 | `first_ts_micros` | of record 0. Meaningful only when `n_valid > 0`. |
| 40 | 8 | `last_ts_micros` | of record `n_valid-1` |
| 48 | 4 | `symbol_id` | resolved from the path; a cross-check, not the index |
| 52 | 4 | `timeframe_secs` | 60 for 1-minute |
| 56 | 4 | `slot_crc` | CRC-32C over bytes 0..56 **and** 60..64 |
| 60 | 4 | reserved | zero |

The checksum covers every byte of the slot except the four it occupies, so a
flipped bit anywhere in the 64 is detected — there is no window a corruption
can land in and be called clean.

Reserved bytes are zero and stay zero. A future field takes reserved space in
a **new version**, never by reinterpreting version 2.

The 64-byte slot size and the 16384-byte slot spacing are **family** constants:
every version uses them, which is what lets a reader locate the slots before it
knows which version the file is.

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

**Prices are paisa integers.** Never a float, at any layer, for any reason.
A float price is how a rounding difference becomes a divergent result set six
months later.

**`i64::MIN` is the open-interest null sentinel.** Zero means zero. Spot
indices carry no open interest and store the sentinel; conflating that with a
genuine zero would make a derivative series and an index series indistinguish-
able on a field that matters.

**A record has no structure that a lost write violates.** An all-zero record is
a legal flat bar whose open interest is a real zero. Nothing about the record
can tell it apart from an extent that was never written; that is §6's job, and
it is why the checksum flag is not decoration.

---

## 4. Reading — read-only mapping

The file is mapped **read-only**. Reads are pointer arithmetic against
resident pages.

**Writes never go through the mapping.** They use positional writes
(`pwrite`), which return an error on a full disk.

This is not a preference. A writable mapping that runs out of space raises
**SIGBUS** — a signal, delivered asynchronously, catchable by no language
construct in any language. Every clean disk-full halt the system has is
disabled the moment a writable mapping is introduced. The predecessor system
halted correctly on a full disk; a naive port to a writable mapping would have
been strictly worse, and that is the single most valuable thing the failure
audit produced.

A header decoder over a shared mapping copies its 64 bytes **once** before it
checks anything, so the checksum covers exactly the bytes the decoded header
carries. A copy that caught a concurrent `pwrite` mid-flight fails that
checksum rather than returning half of each image.

---

## 5. Appending — the commit counter

```
1. pwrite the new records at offset  32768 + n_valid * 56
2. pwrite the affected block checksums into the .crc sidecar
3. fsync the data, through Commit::durable_through
4. pwrite the 64-byte header slot for generation g   <- one write
5. fsync the header
```

A reader treats records `0 .. n_valid` as the whole file. Bytes past `n_valid`
are, by definition, not there yet.

Consequences:

* **A torn record is unobservable.** A crash between steps 1 and 4 leaves bytes
  on disk that no reader will look at; the next append overwrites them.
* **A torn header is unobservable.** Step 4 is one write of one self-checked
  64-byte unit, into the slot that does *not* hold the previous commit. A crash
  during it leaves a slot that fails its own checksum, and the reader takes the
  previous generation.
* **A header that outran its data loses the tail, not the file.** If the slot
  becomes durable and the records do not, the counter names records the length
  cannot support; the reader falls back to the previous generation rather than
  refusing the whole file.

`Commit::durable_through` is the byte offset step 3 must cover. **It states the
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

Each block's CRC-32C lives in a sidecar at the same **block index**:

```
bars/groww/NSE/INDEX/NIFTY/1min/2024-03.bin
bars/groww/NSE/INDEX/NIFTY/1min/2024-03.crc
```

The sidecar is indexed by `i / 73`, not by `(32768 + i*56) / 4096`.

**Why this is not optional.** A flipped bit in a raw `i64` price produces a
different price — a *plausible* one. There is no structure to violate, no
parse to fail. Without a checksum the corruption is silent and permanent, and
every downstream result derived from it is wrong in a way nobody can detect.
It is also the only thing that can tell a lost write from 73 flat bars.

A file whose `flags` bit 0 is clear carries no sidecar, and a verification
request against it is **refused** rather than answered — "verified" and "there
was nothing to verify against" are different answers.

Verification is O(1) per read: one block CRC, not a file scan. Enforced by
`docs/04-invariants.md` C-07 — sealing one block costs the same at 1×, 10× and
100× the file's record count.

**Read "O(1)" precisely.** The *operation* is constant because a block is a
fixed 4,088 bytes; the CRC inside it still reads every one of them, and always
will. Measured on an Apple M4 Pro after D-0032: 1,485.0 ns for `block::seal` end
to end, 20.3 ns amortised over the 73 records a block covers. Before D-0032 the
same seal cost 15,291.0 ns. `docs/06-limits.md` §14 carries the full numbers and
states what is not claimed.

---

## 7. Opening — boundary check

```
if (len - 32768) % 56 != 0 {
    // truncate to the last whole record, log loudly, continue
}
```

A file whose length does not divide by the stride was interrupted. The tail is
truncated to the last whole record and the event is logged with the byte count
discarded. Never silently, never by ignoring the remainder.

A file shorter than the header region is all tail, and is reported as such.

---

## 8. Sibling files of a month

Overlay fields — implied volatility, greeks, anything computed — do **not**
widen this record. They live in a sibling file with their own version, their
own stride, and their own commit counter, addressed by the same index *i*.

| Extension | Holds |
|---|---|
| `.bin` | the bar records |
| `.crc` | one block checksum per block index (§6) |
| `.ovl` | computed overlay fields, at their own stride |
| `.lock` | the advisory lock one writer holds for the month (§5, §9) |

```
bars/<vendor>/NSE/FNO/<contract>/1min/2024-03.bin      56-byte stride, version 2
bars/<vendor>/NSE/FNO/<contract>/1min/2024-03.ovl      its own stride, version 1
```

This keeps the base stride constant forever. A base file written in year one
is readable in year ten by arithmetic that has not changed.

Every one of those names is rendered by `store::path::StorePath` and nothing
else. The vendor is the first segment (D-0019); every segment is
case-canonical, because two segments differing only in case are two files on
ext4 and one file on APFS.

---

## 9. What this format does not solve

| Hazard | Position |
|---|---|
| Page fault on a network mount that has gone away | Uninterruptible. Keep the store on local disk. This is an operational rule, not an architectural fix. |
| Two writers on one file | Not supported. One writer per file, enforced by an advisory lock on the `.lock` sibling, and the lock is a leaf — never held while acquiring another. Nothing in `crates/store` can check it. |
| A failure coarser than 16384 bytes | Takes both header slots. No arrangement inside one file survives a dead device. |
| A symlink at a path component | Defeats vendor-prefix isolation, which is a **lexical** property of `StorePath`, not a filesystem one. The writer must open with `openat` + `O_NOFOLLOW` per component and halt naming the linked component. |
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

## 11. The census file — `BRUTEXM`, versions 1 and 2

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

### 11.3 One header slot — 64 bytes, both versions

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | 8 | `magic` | `b"BRUTEXM1"` or `b"BRUTEXM2"` — the last byte is the version |
| 8 | 2 | `format_version` | `1` or `2`. **Selects the geometry.** |
| 10 | 2 | `entry_stride` | `64` at version 1, `128` at version 2. Read it; never assume it, and check it against what the version declares. |
| 16 | 8 | `generation` | which commit this slot holds. Higher wins. |
| 24 | 8 | `n_valid` | the commit counter: entries readable |
| 32 | 8 | `n_keys` | distinct `(instrument, timeframe, month)` keys among them |
| 40 | 8 | `total_rows` | bars held across every distinct key |
| 48 | 8 | `vendor` | NUL-padded text; a cross-check against the file name |
| 54 | 6 | reserved | zero |
| 60 | 4 | `crc` | CRC-32C over bytes `0..60`, one contiguous run |

The slot layout is **identical across both versions**: only the magic, the
version and the stride differ, which is why one decoder reads both and dispatch
is a table lookup rather than a second parser.

A slot whose magic and version field disagree is refused **by version**. There
is no way to tell which of the two is the lie, and the version field is the
thing that selects the geometry.

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

Reserved bytes are zero and stay zero. A future field takes reserved space in a
**new version**, never by reinterpreting version 2 — §2's rule, unchanged.

### 11.6 Old files stay readable, and the upgrade is a rewrite

`pull::manifest::Layout` is the dispatch: `KNOWN` holds one row per version,
`for_version` refuses an unknown version **by number**, and every geometric
question is a method on the resolved row. Reading a version-1 region in 128-byte
steps would decode every second entry as the tail of the one before it and
return plausible integers, which is §9's "a wrong value that is well-formed"
arriving through the front door.

This build **writes** version 2 only. A census loaded from a version-1 file is
published at version 2, and because the two strides disagree from the first
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

## 13. Ranked frontier — `results/frontier.bin`, version 4

The ranked frontier is append-only and fixed-stride. Its 16-byte header is
`BRUTEXFR` at bytes `0..8`, little-endian version `4` at `8..12`, and four
reserved zero bytes at `12..16`. A non-zero reserved header byte is an unknown
schema and is refused; the reader does not wait for it to affect a later field
before noticing it.

Each row is 272 bytes. All integer fields are little-endian:

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
| 195 | 5 | reserved, all zero; any non-zero byte is refused |
| 200 | 8 | `max_mae_ppm`, `i64` |
| 208 | 8 | `min_rr_bp`, `i64` |
| 216 | 8 | `min_win_rate_bp`, `i64` |
| 224 | 8 | `min_assurance_bp`, `i64` |
| 232 | 8 | `min_weakest_bp`, `i64` |
| 240 | 8 | `min_ret_over_dd_bp`, `i64` |
| 248 | 8 | `min_trades`, `u64` |
| 256 | 8 | `top`, `u64` on disk and saturated to the target `usize` on read |
| 264 | 8 | first eight BLAKE3 bytes over `0..264` |

The seal detects accidental byte damage; it does not assign meaning. A sealed
row with an unknown direction or non-zero reserve is therefore still refused.
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
| `results/execution-parameters-v1.bin` | `BRUTXEP1` | 608 | 640 |
| `results/execution-percentiles-v1.bin` | `BRUTXEG1` | 96 | 128 |
| `results/execution-capabilities-v1.bin` | `BRUTXEC1` | 288 | 320 |
| `results/execution-capability-completions-v1.bin` | `BRUTXEF1` | 384 | 416 |

The 608-byte parameter payload is:

| Payload offset | Size | Field |
|---:|---:|---|
| 0 | 192 | parameter, population, Population V4, policy, expected-resolution and training digests; six 32-byte values |
| 192 | 155 | exact evaluation-spec fingerprint |
| 347 | 9 | direction, family, execution/range/selector/forced-stop, next-minute-entry and forced-exit tags; one reserved zero byte |
| 356 | 24 | signal rung, horizon, execution seconds and stop/target/trail atom counts; six `u32`s |
| 380 | 2 | entry delay, exactly one minute |
| 382 | 2 | forced-exit IST minute, exactly `910` (15:10) |
| 384 | 40 | four runner parameters and maximum levels per axis; five `u64`s |
| 424 | 16 | minimum and maximum reward/risk hundredths; two `i64`s |
| 440 | 64 | ratio-pair/cell bounds, forced-stop ppm, ambiguity/gap bounds, training count and training endpoints |
| 504 | 96 | ordered-percentile, cost-model and exact-execution-law digests |
| 600 | 8 | reserved, zero |
| 608 | 32 | BLAKE3 seal over payload `0..608` |

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
| `results/execution-disposition-parameters-v2.bin` | `BRUX2PR1` | 608 | 640 |
| `results/execution-disposition-percentiles-v2.bin` | `BRUX2PC1` | 96 | 128 |
| `results/execution-row-dispositions-v2.bin` | `BRUTXED2` | 480 | 512 |
| `results/execution-capability-completions-v2.bin` | `BRUTXEF2` | 512 | 544 |

The first two files reuse the exact sealed V1 scalar-parameter and percentile
record codecs. One 480-byte row payload stores thirteen 32-byte digests at
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
partition, admission marginals, 4×2 matrix and ordered digests before indexing.
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
raw direct-cell fields; production derivation of those terms is not yet proved.
It contains no assurance, admission verdict, ranking score, selected rank or
final Population identity. The current public surface is deliberately
**audit only**: it can reopen, validate and page already existing V1 bytes, but
there is no public production constructor or writer. Until the typed
closed-frontier/full-grid producer exists, this format cannot authorize a live
run merely because controlled fixture bytes pass its codec.

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
filesystem latency remain input/system dependent. D-0471, PA-01/PA-02 and
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

# Handover — the bar store seals a checksum it never verifies

**For the session that owns `crates/store`.** Measured 23 Aug 2026 on branch `feat/pull`.
Nothing in `crates/store` was edited to produce this; it is a report, not a change.

---

## The finding in one line

`block::seal` writes a CRC32C sidecar on every commit. **No read path calls
`block::verify`.** A flipped bit in a real bar file enters a sweep undetected.

---

## Reproduced, on the operator's own data

```
cp ~/.brutex/store/bars/zerodha/NSE/INDEX/BANKNIFTY/60min/2023-12.{bin,crc} <scratch>/…/60min/
# flip one bit at byte 100 of the .bin
cli sweep-stored zerodha BANKNIFTY 60min 2023 12 20
```

| Check | Result |
|---|---|
| `FLAG_CHECKSUMS` in the real header, offset 12 | `0100 0000` — **set** |
| `.crc` sidecar present | yes, 8 bytes |
| Bit actually changed | `cmp -l` → `101 1 0` |
| Sweep verdict | **140 offered, 0 refused, exit 0** |

The corrupted bar was swept. Every figure downstream — MAE, the exit grid, the
p-values — was computed over it.

**A first attempt at this test was invalid** and is recorded so it is not repeated:
only the `.bin` was copied, so the reader would legitimately have reported *"no
checksums to verify against"*. The run above copies both files.

---

## Where the gap is, precisely

| Side | State |
|---|---|
| **Write** | `block::seal` is called and the sidecar is produced. Correct. |
| **Read** | `block::verify` has **no caller in `file.rs`** — only comments referring to it. |

Its only production-shaped callers are in `emits.rs`, which drives each emit site
to prove the telemetry line reaches a file. That proves the *log* fires, not that
any bar was checked.

```
grep -n "verify(" crates/store/src/file.rs     # comments only
```

---

## Why this is not a new observation, quite

`file.rs` already carries the other half of the story, at the comment beginning
**"BORN WITH CHECKSUMS, AND ONLY EVER BORN WITH THEM."** It records that
`FLAG_CHECKSUMS` was once declared, documented, implemented and **set by no
writer**, so `block::verify` had no production caller *because nothing produced a
sidecar for it to check*.

The writer was fixed. The reader was not — so the sentence is still true today
for a different reason, and the comment reads as though the matter is closed.

That same comment states the failure shape better than this document can:

> the header names records whose bytes are zeros from a newly allocated extent —
> and nothing in the record can detect it. An all-zero `Bar` satisfies
> `ohlc_is_sane`, and its `open_interest` of zero is a REAL zero rather than
> `OI_NULL`, so a spot-index file quietly acquires derivative-shaped bars that go
> on to enter a sweep.

---

## What a fix has to decide

These are the questions, not a prescription — the crate is yours.

1. **Where.** Per block on read, or once at `open_existing`? Per block refuses only
   the damaged block and keeps the rest readable; at open it is one pass but
   refuses a whole month over one bad block.
2. **Cost.** Verification is O(bytes read) and the sweep reads every bar of every
   month. `CLAUDE.md` §3 rule 4 governs per-operation cost, and a CRC over a block
   is O(1) per block — but it is not free, and gate 8's ratio benches should see it.
3. **What a mismatch does.** Refuse the month, or drop the block and count it? The
   ledger's own precedent is to refuse and name both halves — how many bytes are
   bad and how many records survived — because §4 bans a fallback that hides a
   failure.
4. **The absent-sidecar case.** `emits.rs` already has a `Warn` for *"no checksums
   to verify against"*. A file written before the writer was fixed will hit it, and
   it must stay a warning rather than becoming a refusal, or existing months stop
   loading.

---

## What the `cli` side did for the same class of problem

Offered only as precedent, since the ledger hit this exact shape this week:

| Failure | What was done |
|---|---|
| Part-record from an interrupted write | Refused at open, naming orphan bytes **and** surviving records |
| Whole-stride record damaged after the write | 8-byte blake3 seal, verified **on read**, refusing that record only |
| Two writers racing | Exclusive `flock` + a re-scan of anything appended since |
| A path that accepts bytes and keeps none | Header written, **read back**, refused when it does not return |

Commits: `f628350`, `2d229b1`, `be3a678`, `9d4c545`, `9cd5b97`, `ca2345f`.

The one deliberate asymmetry there is worth carrying over: **at open, the cause of
bad bytes is unknown, so it refuses; at a failed append the cause is known — our
own write just failed — so it rolls back.** An automatic repair is only honest
when the code knows it caused the damage.

---

## Not touched

`crates/store` is unmodified by the session that wrote this. No test was added
there, no signature changed. If it would help, say so and the `cli`-side session
can prepare a patch for review rather than landing one.

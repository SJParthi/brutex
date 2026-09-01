# 10 — The shared core: what other projects may depend on

brutex is where the vocabulary is defined and where new conditions are added.
Everything downstream — the live-trading system, any future project — consumes it
rather than reimplementing it. This file records **which crates are shareable, why,
and what makes adding condition N+1 safe for consumers that are already running.**

Adding this file is a locked choice and needs an entry in `docs/05-decisions.md`.
It is written here first because the boundary it describes is now enforced by
`Cargo.toml`, and a boundary in the build with no document is the thing this
repository keeps finding in audits.

---

## 1. The six crates that are shareable, and the one change that made them so

| Crate | Depends on | Holds |
|---|---|---|
`core` | **nothing** | `Instrument`, `Isin`, `Symbol`, `Price`, `Vendor`, the universe. The nouns. |
`vocab` | **nothing** | The bit table, `ConditionMask`, `Tolerance`. The alphabet. |
`indicators` | **`vocab` only** | Candle in, condition bits out. Twelve position sources, 328 positions. |
`engine` | **`vocab` only** | The Apriori ladder. Bit vectors in, frequent combinations out. |
`greeks` | **nothing** | Black-Scholes, integer-safe. Already shared. |
`costs` | **`core` only** | Brokerage, STT, stamp duty, GST, slippage. Paisa integers. |

All six declare **zero external packages** — measured with `cargo tree --edges
normal`, not assumed — so a consumer takes them without inheriting a dependency tree.

Not shareable, and deliberately: `store` (this repository's file format), `pull`
(vendor ingest and credentials), `api` (HTTP), `lake`, `telemetry`. Those encode
decisions that belong to brutex and to no consumer.

**`costs` was on that list until it was challenged and the challenge was right.** A
brokerage-and-tax calculator is not a backtesting decision: the charge a real trade
incurs is the same charge whether the trade is simulated here or placed live, and a
live consumer computing it a second time from a second source is how the two
disagree. It is a shared crate because the *number must be identical on both sides*,
which is a stronger reason than convenience.

### What was blocking it

`indicators` used to depend on `store` — for one thing: `store::format::Bar`. A
seven-field struct. That single arrow coupled the entire condition layer to
**brutex's on-disk file format**, and it is the reason no other project could use it:
a live-trading consumer holds ticks off a socket, not records out of a fixed-stride
file, and it should not have to link a storage engine to ask whether a candle is a
hammer.

So `indicators` now declares the shape it needs — `indicators::Candle` — and depends
on `vocab` alone. The fields are deliberately identical to a stored bar, so the
conversion at a consumer's boundary is a struct literal and not a decision:

```rust
let candle = indicators::Candle {
    ts_micros: bar.ts_micros,   // microseconds since epoch, UTC, the bar's OPEN
    open: bar.open,             // paisa, i64, never a float (§7)
    high: bar.high,
    low: bar.low,
    close: bar.close,
    volume: bar.volume,         // 0 is a real zero, not an absence
    open_interest: bar.open_interest, // i64::MIN when absent (§7)
};
```

The conversion lives at the consumer's edge on purpose. That is where the knowledge
of the consumer's own format already lives, and putting it there is what keeps the
arrow from pointing back into brutex.

This also closes a `CLAUDE.md` §5 finding: the graph never drew `indicators → store`,
and now nothing needs it drawn.

---

## 2. How a consumer depends on it

The repository is public, so a git dependency pinned to a tag is the whole
mechanism. No registry publish, no vendoring, no copy.

```toml
[dependencies]
vocab      = { git = "https://github.com/SJParthi/brutex", tag = "vocab-v0.1.0" }
indicators = { git = "https://github.com/SJParthi/brutex", tag = "vocab-v0.1.0" }
engine     = { git = "https://github.com/SJParthi/brutex", tag = "vocab-v0.1.0" }
```

**Pin to a tag, not to a branch.** A branch dependency changes the meaning of a
consumer's stored results without a diff anywhere in the consumer, and §3 rule 3
puts `vocab_version` inside the run identity precisely so that cannot happen
silently. A tag bump is a visible line in the consumer's `Cargo.toml` and its
lockfile.

---

## 3. Why adding condition N+1 is safe for a consumer already running

This is the part that makes the approach incremental rather than merely shared, and
it rests on three mechanisms that already exist.

### Bit numbers are append-only (§3 rule 8)

A condition is identified by its **index**, and indices are never renumbered or
reused. A retired condition keeps its number and evaluates false forever; a voided
one keeps its number and refuses to be set. So a mask recorded by a consumer last
month means exactly the same thing after brutex adds fifty conditions. **Old results
stay readable and old masks stay comparable** — that is the whole reason the ledger
is append-only rather than tidy.

`vocab::table::NEXT_FREE` is where the next condition goes. Nothing else.

### The width is checked by the compiler, not by review

`ConditionMask` is `[u64; WORDS]`, currently 6 words = 384 bits, against 370
allocated positions — 14 free. The five weekday rows (365–369) took five of the
nineteen. Adding conditions consumes headroom, and when it runs out:

```rust
const _: () = assert!(COUNT <= ConditionMask::BITS as usize, ...);
```

That is a **const assertion**: it fails the build, not a test. This matters more than
it looks. In the predecessor system the frontier mask was 64 bits wide against a
74-condition vocabulary, every real run silently tripped a width guard and fell back
to a hardcoded depth of 2, and nobody could see it. The guard here cannot be missed
because nothing compiles until `WORDS` is widened in the same change.

`crates/engine` carries the same assertion independently, so a consumer that takes
`engine` without `indicators` is still protected.

### The set of positions is derived, never listed

`indicators::evaluator::Evaluator::positions()` is the union of the ELEVEN position
sources' own `positions()` — eight modules, the current-day Fibonacci rung range, the
four the evaluator computes from its own session bookkeeping, and the crossing family it
derives from `vocab::table::CROSSINGS` — **272 positions** in total. Adding a position to a module adds it to the evaluator with no
second edit, so the two cannot drift. A hand-maintained list is exactly the kind of
thing that goes stale silently.

### What a consumer must do when brutex adds conditions

| Change in brutex | What the consumer does |
|---|---|
A new condition at a new index | Bump the tag. Existing masks keep their meaning; the new bit is simply available |
A condition retired | Bump the tag. It reads false; nothing breaks |
`WORDS` widened | Bump the tag. `ConditionMask` is `Copy` and fixed-size; recorded masks widen with zeros in the new words |
A threshold changed | Bump the tag **and expect different bits.** This is the one change that alters the meaning of a stored result, which is why `vocab_version` is in the run identity |

---

## 4. What the shared core deliberately does not decide

Three things are the consumer's, and pushing them into the shared crates would be
the mistake:

**The bar source.** `indicators` takes a `Candle` and never opens a file, a socket or
a database. It cannot, because it depends on `vocab` alone.

**Volume availability.** `Vwap::for_slice` is *handed* the verdict. An index-spot
consumer passes `Availability::Absent` and all twenty VWAP positions stay false for
the run; a consumer with real volume passes `Present` and the same twenty light up
with no code change. Deciding per bar instead would let bit 52 mean "below VWAP" in
the morning and "no opinion" at the open, silently, within one run.

**The ranking metric.** `engine` returns frequent combinations with exact hit counts
and **refuses to order them by anything else**. `docs/00-charter.md` §6 says "rank the
survivors" and names no statistic; §3 rule 1 forbids inventing one. Frequency is not
edge — rank by support alone and the winner is the most trivial condition in the
vocabulary. The consumer supplies the outcome measure, because only the consumer
knows its exit rule, its horizon and its costs.

---

## 5. Cost, which is what makes it usable live

The streaming evaluator does a fixed amount of work per candle with no
allocation. The same table separates that path from the candidate-level sweep
primitives, whose total counts are not constant:

| Operation | Cost | Bounded by |
|---|---|---|
Condition lookup | O(1) | direct index into a fixed array of 370 |
Mask evaluation | O(1) | 6 ANDs, 6 XORs, 5 ORs, 1 compare — branchless, no early exit, identical for a true and a false answer |
One live candidate/bar support step | O(1) | `engine::column::Column` owns one `[u64; 6]` row mask per bar and calls the fixed-width `hits` exactly once; the complete support count remains O(candles) |
One candle through every module | O(1) | a fixed set of fixed-size states, no allocation; `size_of::<Evaluator>()` is asserted at compile time |
Duplicate rejection | O(1) **expected**, not adversarial worst-case | k=1 uses one `HashSet<u32>` insertion; the injective prefix join emits each k≥2 candidate once and therefore needs no dedup set. `HashSet` collisions are not worst-case bounded by this repository |

Nothing in the closure grows with the number of candles fed. That is the property a
live consumer needs and it is asserted in the build rather than described here:
`const _: () = assert!(size_of::<Evaluator>() <= 1824)` fails if any module starts
accumulating. It measures 1776 bytes today, so the assertion has 48 bytes of slack and is
a live guard rather than a rounded-up number that could never fire — it was 1664 until the
non-regular-session `Calendar` was added, 1728 before the growth after that, 1744 before
the crossing family added one `Option<ConditionMask>`, whose
cause is not recorded here because it was not measured here. The test that reads this
number is what refused each stale figure rather than a reader noticing, and it has now
done so twice; 48 bytes is two more `Calendar`-sized additions, not many.

**Stated rather than implied (§3 rule 6):** support counting is O(candles) because it
*is* the measurement, and the Apriori level join is O(|frontier|²), which is that
algorithm's documented shape. Neither is a hidden scan, and neither is on the
per-candle path. The live support constant is measured flat from k=1 to k=8
(0.996× at the far endpoint) and across the complete 384-bit representation
(0.942× at k=384), while the source guard
`engine::column::tests::the_live_support_body_is_one_fixed_width_hit_test` pins the
one-hit-test shape. Those ratios are regression evidence on one machine, not a
worst-case latency guarantee.

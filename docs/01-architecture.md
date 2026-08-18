# 01 — Architecture

Ten crates. Every arrow points one way. The graph is acyclic and the linker
enforces it.

---

## 1. The graph

**Measured, not drawn from memory.** Every arrow below is what
`cargo tree -p <crate> --edges normal --depth 1` prints today. The previous version
of this section had `vocab`, `indicators` and `engine` marked as not existing, gave
all three the wrong dependencies, and omitted `lake` and `telemetry` altogether —
see the note at the end of this section for what that cost.

```
   core        vocab       greeks      telemetry        four roots: no dependencies
    │            │                         │
    ├─ costs     ├─ indicators             │
    │            └─ engine                 │
    │                                      │
    ├──────────── store ───────────────────┤
    ├──────────── lake ────────────────────┤
    │               │                      │
    ├─────────────  pull  ─────────────────┤
    │               │                      │
    └───────────── api ────────────────────┘
                (api also takes pull and store)
```

| Crate | Owns | Depends on, measured | Exists |
|---|---|---|---|
| `core` | instrument, ISIN, symbol, price, vendor, the universes | **nothing** | ✓ |
| `vocab` | the condition bit table, `ConditionMask`, `Tolerance` | **nothing** | ✓ |
| `greeks` | Black-Scholes-Merton, integer-safe: five greeks, implied volatility, the strike ladder | **nothing** | ✓ |
| `telemetry` | the event sink, the encoder, the tail reader | **nothing** | ✓ |
| `costs` | Indian F&O transaction costs: dated statutory regimes, option arithmetic, the round-trip charge stack | `core` | ✓ |
| `indicators` | candles in, condition bits out | **`vocab`** | ✓ |
| `engine` | the Apriori ladder: generation, subset-prune, support, extinction | **`vocab`** | ✓ |
| `store` | the fixed-stride bar file: open, read, append, verify | `core`, `telemetry` | ✓ |
| `lake` | the parquet-shaped reader for vendor archives | `core`, `telemetry` | ✓ |
| `pull` | vendor ingest, rate governor, credential read | `core`, `store`, `telemetry` | ✓ |
| `api` | the HTTP surface | `core`, `pull`, `store`, `telemetry` | ✓ |
| `runner` | the sweep driven end to end: trades, the exit grid, walk-forward, PBO, the bootstrap, the audit | `core`, `vocab`, `indicators`, `engine`, `costs` | ✓ |
| `cli` | the operator entry point for the sweep: generated bars in, ladder walked, report out | `runner`, `engine`, `indicators`, `costs` | ✓ |

**Thirteen crates, all of them real.** (The sentence said eleven while the table
held twelve; `cli` makes it thirteen — D-0169.) `web/` is a directory at the repository root
rather than a workspace member — the front end is unrestricted inside it by D-0052
and D-0053 — and there is **no `cli` crate**, though `CLAUDE.md` §5 still names one.

### The three arrows that are not what §5 draws

`CLAUDE.md` §5 is law and this document may not correct it, so the difference is
recorded rather than resolved (D-0095):

* §5 draws `indicators` as a child of `core`. It is a child of **`vocab`**, and of
  nothing else. `store` used to be its second parent, taken for
  `store::format::Bar`; `indicators::Candle` replaced that record and the arrow is
  gone, which is the single change that made the condition layer reusable outside
  this repository.
* §5 names a `cli` crate that does not exist, and omits `costs`, `greeks`, `lake`
  and `telemetry` — four of eleven members.
* Eight real edges are undrawn there: `store -> telemetry`, `lake -> telemetry`,
  `pull -> telemetry`, `api -> telemetry`, `api -> pull`, `api -> store`,
  `costs -> core`, `engine -> vocab`.

The graph is **acyclic**, which is §5's actual requirement, and that much holds.

### The six that a second repository can take

`core`, `vocab`, `greeks`, `costs`, `indicators`, `engine` — every one declares
**zero external packages**, measured with `cargo tree --edges normal`. A consumer
takes them without inheriting a dependency tree and without a build script.
`docs/10-shared-core.md` is the boundary document; D-0095 is the decision.

`store`, `lake`, `pull`, `api` and `telemetry` are deliberately not shareable: a
file format, a vendor's credentials, an HTTP surface and a log sink are this
repository's decisions and no consumer's.

### Why this table was wrong, and what fixed the class of error

It said six of eleven crates existed while eleven did. It gave `engine` four
dependencies where it has one. Every one of those statements was true when written.

That is the same failure `docs/03-vocabulary.md` §8 and `docs/10-shared-core.md`
had, and D-0102 recorded the rule that closed it: **where a document states
something the code also knows, a test reads the document and compares.** The
numeric claims in those two files now fail the build when they drift. The
dependency table above is the same kind of claim and deserves the same guard.

**This table was also merged from two branches that each rewrote it.** `feat/pull`
added the `Exists` column and the `costs` row; `feat/greeks` added the `greeks` row
against the older four-column shape. Neither was wrong and the union was the
answer — the same resolution the `D-0037` collision and the `06-limits.md` §18
collision needed in the same merge, and the same cause as the three duplicated
decision numbers D-0104 records: **a branch that reads only its own copy of a
shared append-only document.**

---

## 1a. `costs` — added by D-0041, drawn here by D-0045

`crates/costs` shipped across D-0041, D-0043 and D-0044, and **this diagram did
not have it for any of them.** Each of those three entries recorded the omission
as outstanding rather than fixing it, because the crate that wrote them did not
own this file. It is drawn now.

It sits beside `store` and `vocab` as a direct child of `core`, and **its only
arrow points at `core`** — one dependency, `brutex_core`, for three things it
refuses to define twice: `Exchange` (the circulars are exchange-scoped),
`Paisa` (`CLAUDE.md` §7 fixes money at integer paisa) and the calendar
validator (`day::TradeDay` validates through `core`'s, rather than writing a
second leap-year rule). The graph stays acyclic; nothing depends on `costs`.

**Nothing consumes it yet.** `grep 'costs' crates/*/Cargo.toml` returns only its
own manifest. The intended consumer is the engine's trade-costing path, and
`trip::price` is the single call it needs — so `engine` gains `costs` as a
dependency when `engine` exists.

**`CLAUDE.md` §5 draws this same graph and still does not list `costs`.** That
file is session law and is not edited from here. The correction it needs is
recorded in D-0045 and reported to the operator.

---

**`api → pull` was added by D-0038**, and the diagram above still draws them as
siblings. `/pull` and `/store` are the operator's window onto ingest, and every
rule they render already has exactly one definition in `pull`: the validated
calendar (`session::Day`), the inclusive window and the vendor's non-inclusive
`toDate` (`session::Window::wire_to`), the drop reasons and their tally
(`session::{DropReason, DropCensus}`), and the per-vendor counter file
(`manifest::Manifest`). Re-deriving any of them inside `api` would be a second
Gregorian rule and a second answer to what goes on the wire. The graph is still
acyclic — `pull` does not depend on `api` — and the build order is unchanged.

`greeks` is a **leaf with no arrow into it and none out of it**, which is why
it is not drawn in the diagram above. It is not part of the sweep. It is shared
with the `tickvault` repository, which takes it by git URL, so it declares zero
dependencies for the same reason `core` does — and unlike `core`, its public
surface mentions no type from this workspace at all, only `f64` and plain enums
it owns. **Both halves of that are enforced by CI gate 9b**, in the same shape
as gate 9 for `core` and gate 7 for `web`; before D-0046 they were advertised in
six places and enforced in none. It is also the one place in this repository
where a float is the correct type: `CLAUDE.md` §7 keeps statistical values at
full precision and reserves `i64` paisa for prices, and this crate never sees a
paisa. Its `rust-version` is written literally rather than inherited, because
the MSRV of a shared leaf crate is a property of the crate and not of the
workspace hosting it. See `docs/05-decisions.md` D-0046.

**`CLAUDE.md` §5 does not list it.** That is a real discrepancy and §10 makes
`CLAUDE.md` the winner, so this row is the document running ahead of session
law rather than the other way round. `greeks` adds no arrow to §5's graph, so
it violates nothing in it; bringing §5 into line is an operator decision, and
it is **still open** — see D-0046, "One discrepancy an operator has to settle",
which carries the one-line repair.

---

## 2. Why `web` depends on `core` alone

It compiles to WebAssembly. There is no filesystem there. If `web` could
declare `store` as a dependency, someone would eventually call it, and the
failure would surface as a runtime panic in a browser rather than as a
compile error on a laptop.

The constraint is not a convention. `crates/web/Cargo.toml` lists one
dependency and CI gate 7 fails on any other.

The payoff: every display rule — how a price renders, how a percentage is
computed, what counts as a valid mask — lives in `core` and is compiled twice,
once native and once to WASM. One implementation, two targets, no drift
between what the server believes and what the browser shows.

---

## 3. Data flow, end to end

```
vendor
  │  pull: one request per window, rate-governed, resumable
  ▼
raw candles ──► validate ──► paisa integers ──► store::append
                                                    │  pwrite, then
                                                    │  publish n_valid
                                                    ▼
                                            bars/<exch>/<seg>/<sym>/<tf>/<yyyy-mm>.bin
                                                    │
                                            store::open (read-only mmap)
                                                    │
                                            store::Bar ──► indicators::Candle
                                                    │      (a plain seven-field record;
                                                    │       the arrow that used to be a
                                                    │       DEPENDENCY -- D-0095)
                                                    ▼
                                            Evaluator::step  ── one candle at a time
                                                    ▼
                                            bar_bits: [ConditionMask]
                                              [u64; 6] each = 384 bits, 276 allocated
                                                    │
                                            Ladder::walk
                                              k=1 frontier
                                              join + subset-prune
                                              support, dedup
                                              stop at extinction -- no depth parameter
                                                    ▼
                                            frequent survivors ──► ranking (NOT BUILT)
                                                    │              needs a metric; see below
                                                    ▼
                                                results file
                                                    │
                                        api ────────┴──────── web (wasm)
```

Three properties matter more than the boxes:

1. **The disk is touched once per slice per launch.** After the initial open,
   every bar read is pointer arithmetic against a mapping that is already
   resident.
2. **Condition bits are computed once and shared read-only** across every
   thread and every candidate. In the predecessor system this recomputation
   was the dominant cost — roughly eleven thousand times the per-mask cost —
   and it was re-paid per worker per tuple.
3. **There is no boundary to cross.** Nothing is marshalled between runtimes,
   so no per-trade materialisation cost exists to be optimised later.

### The one box that is not built: ranking

`Ladder::walk` returns **frequent** combinations with their hit counts. That is a
complete answer to "which condition sets occur often enough to be worth looking at",
and it is not a ranked strategy list. Ranking needs a **score**, and a score is a
modelling choice `CLAUDE.md` §3 rule 1 will not let this document invent.

What is unblocked and what is not:

* **Support ranking works today.** `Itemset::hits` is a real number and ordering by
  it is honest — it answers "how often". It says nothing about whether the
  combination was *profitable*.
* **Return-based ranking cannot be built yet, for a reason that is not a design
  gap.** It needs, per hit, what happened after the bar — a forward return over some
  horizon, an exit rule, and the cost of the round trip. `crates/costs` supplies the
  last of those. The first two need **bar data, and there is none on this machine**:
  every pulled bar was deleted deliberately and permanently. A metric written now
  could not be run, let alone validated.
* **The horizon and the exit rule are choices, not derivations.** Fixed N bars, next
  session's open, a target-or-stop bracket, and a trailing exit are four different
  strategies that would rank the same vocabulary differently. Picking one silently
  would be the kind of invention §3 rule 1 exists to prevent, and it would be
  invisible in the output — the ranked list would look equally authoritative either
  way.

So the ranking metric is **an operator decision, recorded as open**, and it is the
last thing standing between the vocabulary and a ranked strategy list. Everything
upstream of it — 234 computable conditions, the mask, the ladder, extinction, the
cost stack — is built and measured.

---

## 4. Path is the index

```
bars/groww/NSE/INDEX/NIFTY/1min/2024-03.bin
     │     │   │     │     │    └── the month file
     │     │   │     │     └─────── timeframe
     │     │   │     └───────────── symbol
     │     │   └─────────────────── segment
     │     └─────────────────────── exchange
     └───────────────────────────── vendor (D-0019)
```

Locating a slice is a string join and an open. There is no catalogue to
consult, no index to rebuild, no registry that can disagree with the
filesystem. Adding a symbol requires no registration: the first write creates
the directory.

Directory listings are never globbed on a read path. A read computes the exact
path it wants; if the file is absent, that is a specific, named absence rather
than a scan that returned nothing.

---

## 5. Concurrency

| Layer | Model | Shared mutable state |
|---|---|---|
| `pull` | async, one task per window, a governor between tasks and the vendor | none — each task owns its window |
| `indicators` | single pass, sequential by construction (state carries forward) | none |
| `engine` | data parallel over candidates via `rayon` | none — bar bits are read-only, each shard owns its own output |
| `store` append | one writer, positional writes, commit counter published last | the counter, published with a release store |
| `api` | async, request-scoped | none |

The rule that makes this hold: **a sweep never mutates anything a reader can
observe mid-flight.** Results accumulate per shard and merge once at the end.

---

## 6. Where the constant-time claims live

Every row now names the bench that measures it. Until 2026-08-11 this table was six
assertions with nothing behind three of them — `vocab`, `indicators` and `engine`
made seventeen cost claims between them and measured none. D-0103 is the entry;
`docs/04-invariants.md` holds the rows and the numbers.

| Operation | Cost | Mechanism | Measured by |
|---|---|---|---|
| Locate a slice | O(1) | path join | — |
| Read bar *i* | O(1) | `base + 32768 + i·56` — see `docs/02-store-format.md` §1 | `C-01` |
| Read condition bits for bar *i* | O(1) | index into a slice of `ConditionMask`, each `[u64; 6]` = 384 bits | `C-E-01` |
| Test one candidate against one bar | O(1) | `(bits & mask) == mask`: six ANDs, six XORs, five ORs, one compare — branchless, no early exit | `C-V-01`, `C-V-02`, `C-V-03` |
| Reject a duplicate candidate | O(1) | one `HashSet` probe on a `Hash + Eq` mask | `C-E-04` |
| Append one result | O(1) amortised | `Vec::push` | **UNMEASURED**, `C-04` |
| Fold one candle into every module | O(1) | a fixed set of fixed-size states, no allocation; `size_of::<Evaluator>()` asserted at compile time | `C-I-01`, `C-I-02`, `C-I-04` |

**Two rows corrected here rather than left standing.** "Index into a `Vec<u128>`" was
true when the mask was two words wide and has been wrong since it reached six —
384 bits, not 128, and a `u128` would have run out of room at position 128 against a
276-position table. "A filter probe, then a sharded exact confirm" describes a design
that was never built; the engine does one `HashSet` probe, and claiming a sharded
confirm made the mechanism sound more careful than it is.

**The three measurements worth reading.** A miss in word 0 against a miss in word 5
is **0.996×** — six words are read on every call whatever the answer, so there is no
early exit. A 1-bit candidate against a 234-bit one is **1.037×**, and per bar from
k=1 to k=8 is **1.117×** — which is what makes §6's absent depth parameter a design
decision rather than a performance defect. One candle at 200,000 candles folded
against 1,000 is **1.009×**.

**A ratio near 1.0 is evidence, not proof.** A compiler may introduce a branch. The
source-level guarantee is `vocab::mask::hits_does_the_same_work_for_every_input`,
which reads the function body, refuses `for`/`while`/`loop`/`return`/`if`, counts the
operators against `WORDS`, and asserts every word index is read. The claim is the
conjunction of the two; neither half is sufficient alone.

Total sweep work is **not** constant — it scales with bars × candidates. Support
counting is O(bars) because it *is* the measurement, and the Apriori level join is
O(|frontier|²), which is that algorithm's shape. Only the per-operation cost is
constant, and that is the only thing a design can promise. `docs/06-limits.md` says
the same at more length, and §§51–53 there record the three places where a per-
operation cost is **not** flat and the honest numbers for each.

# 01 — Architecture

Thirteen crates. Every arrow points one way. The graph is acyclic and the linker
enforces it.

---

## 1. The graph

**Measured, not drawn from memory.** Every arrow below is what
`cargo tree -p <crate> --edges normal --depth 1` prints today. The previous version
of this section had `vocab`, `indicators` and `engine` marked as not existing, gave
all three the wrong dependencies, and omitted `lake` and `telemetry` altogether —
see the note at the end of this section for what that cost.

```
depends on NOTHING          core · vocab · greeks · telemetry

core          <-- costs
core telemetry <-- store · lake
core costs greeks store telemetry <-- pull
core pull store telemetry cli vocab <-- api
vocab         <-- indicators · engine
core costs engine indicators vocab <-- runner
core costs engine indicators pull runner store telemetry <-- cli
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
| `pull` | vendor ingest, rate governor, credential read, option pricing | `core`, `store`, `telemetry`, `costs`, `greeks` | ✓ |
| `api` | the HTTP surface | `core`, `pull`, `store`, `telemetry`, **`cli`**, **`vocab`** | ✓ |
| `runner` | the sweep driven end to end: trades, the exit grid, walk-forward, PBO, the bootstrap, the audit | `core`, `vocab`, `indicators`, `engine`, `costs` | ✓ |
| `cli` | the operator entry point for the sweep: generated **or stored** bars in, ladder walked, report out — `sweep` takes `runner::synthetic`, `sweep-stored` takes one real instrument-month off disk, and a different provenance banner leads each | `runner`, `engine`, `indicators`, `costs`, `core`, `pull`, `store`, `telemetry`, `vocab` | ✓ |

**Thirteen crates, all of them real.** (The sentence said eleven while the table
held twelve; `cli` makes it thirteen — D-0169.) `web/` is a directory at the
repository root rather than a workspace member — the front end is unrestricted
inside it by D-0052 and D-0053. `cli` is a real workspace crate; `web` is not.

### The measured graph and its gate

The dependency table is parsed by `core/tests/graph.rs`. The parser reads only
the `## 1. The graph` section, recognises a crate row only when it begins
`| \`crate\``, and treats backticked workspace names in the third cell as the
claimed dependency set. It checks all thirteen members, every normal/dev/build
dependency spelling, both directions of table/manifest equality, and the
acyclic property. The ASCII diagram is deliberately not the parser's authority.

That gate found the direct `cli -> pull` edge omitted here while the manifest
and `cargo tree -p cli --edges normal --depth 1` both named it. The edge is
production: stored-run calendar attestation calls `pull::calendar::kind_of`,
Population V4 civil bounds use `pull::session::Day`, and global replay/VIX
month identity uses `pull::session::IstMoment`. It does **not** authorize a
vendor request or network fallback. Stored readers still fail closed on absent
or invalid calendar/session evidence rather than calling ingest. D-0453 records
the correction.

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

**The constraint is now a convention, and saying otherwise was the drift.** This
read "`crates/web/Cargo.toml` lists one dependency and CI gate 7 fails on any
other". There is no `crates/web` — D-0052 moved the browser to `web/` as an
unrestricted directory — so gate 7 tests for that manifest, finds nothing and
**exits 0 every run**. It has never failed and cannot. What governs the front end
instead is `CLAUDE.md` §2: no crate may depend on its toolchain to build, test or
run, which is the arrow that actually matters and is enforced by `cargo build`
succeeding on a machine with no Node at all.

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
                                              [u64; 6] each = 384 bits, 370 allocated
                                                    │
                                            Ladder::walk_into
                                              k=1 frontier
                                              join + subset-prune
                                              fixed-width support; k=1 input dedup
                                              stop at extinction -- no depth parameter
                                                    │
                              ┌── plain sink: retain each Frontier ──► Sweep
                              │
                              └── ranked sink, at each retirement
                                    lower + next ──► exact closure / trial counts
                                    score on the execution Column + Forward
                                    merge into one bounded global top-N heap
                                    drop lower ──► one fixed-size Tally per depth
                                                    │
                                                    ▼
                                      RankedOutcome + Ranked
                                      (top, closed_top, lens, counts)
                                                    │
                                      exit grid, costs, validation
                                                    ▼
                                      result-set persistence
                       frontier.bin -> chosen-trades.bin -> detail-sets.bin
                                  -> directory sync -> runs.bin (commit marker)
                                                    │
                                            api ────────┴──────── web/
```

The newer Step-3 authority path is append-only beside that legacy run path; it
does not reinterpret an old receipt as provisional:

```
clean build stamp + existing stored root
                  │
                  ▼
bounded signal + prior-month 1day + prior-month 1min loads
                  │  complete IST calendars; exact requested 1min subspan
                  ▼
attested long/short dynamic grids + naturally-extinct Candidate Universe V1
                  │  rows sync, Candidate completion last, fresh reopen
                  ▼
Pre-Admission Data V1
                  │  Data sync, Completion last, fresh reopen, exact ID join
                  ▼
        [implemented, focused-green D-0475 boundary]
                  │
                  ├ - - > aligned trade-period observation authority  (open)
                  ├ - - > Statistics V2 + finalization/admission       (open)
                  ├ - - > Execution V2 + Selection V4 reconstruction  (open)
                  └ - - > eight lists / 200-witness Global Replay V2   (open)
```

The solid boundary is callable from non-test Rust and accepts no raw bars,
digest, calendar receipt, commit string, pre-resolved grid or depth. Its daily
and minute typed loaders reuse the existing canonical converters after the
month headers enforce explicit cumulative record ceilings. The dashed arrows
are deliberately not called implemented by the green Candidate/Pre-Admission
tests: Candidate now derives an in-memory aligned trade/session observation
capability, but its bounded fixed-stride receipt-last authority and fresh-reopen
refusal suite are still being implemented, so it cannot yet construct
production Statistics V2. The first adversarial warm-up findings are closed:
canonical NSE session identity classifies every daily/signal day, exact
terminal-minute geometry binds the prior accepted session, and transformed
allocations are fallible. D-0475 and `docs/06-limits.md` §156 record the exact
proof, remaining authority gap and cost boundary.

Three properties matter more than the boxes:

1. **The disk is touched once per slice per launch.** After the initial open,
   every bar read is pointer arithmetic against a mapping that is already
   resident.
2. **Condition bits are computed once and shared read-only** across every
   thread and every candidate. In the predecessor system this recomputation
   was the dominant cost — roughly eleven thousand times the per-mask cost —
   and it was re-paid per worker per tuple.
3. **There is no language-runtime boundary to cross.** The finally selected
   grid cell is deliberately materialised once into its durable Rust trade
   rows; that O(selected trades) output cost is not serialization between two
   engine implementations and is named in `docs/06-limits.md` §111.
4. **Ranked retention is not result-set retention.** The engine needs the
   current frontier and the successor it is building; after the successor has
   decided the lower frontier's exact closure, every lower survivor has already
   been scored and the frontier is dropped. The result keeps one `Tally` per
   level and at most `keep` ranked rows. Neither bound is read by the join, so
   neither can choose the ladder's depth.

5. **A recorded run becomes public at one last-written marker.** The frontier
   and exact chosen-grid trade blocks are variable output, so
   `detail-sets.bin` records their exact row counts plus selected direction and
   trade policy, including zero-row results. Those three children are synced,
   their directory entries are confirmed, and only then is the fixed-stride
   parent appended to `runs.bin`. Read-only detail paths require the ledger
   identity plus the matching receipt/count; a pre-marker child stays private,
   while receipt-less legacy results are refused as unverifiable. The four-file
   byte layout and recovery states are authoritative in
   `docs/02-store-format.md` §12 and the non-atomic filesystem limits in
   `docs/06-limits.md` §98.

### Ranking is built; its modelling choices stay explicit

`Ladder::walk_into` still answers the frequency question and does not know what
a return is. `runner` joins that walk to the bar series and makes the modelling
choice at the caller-owned boundary. There are two public lenses:

* `Lens::Detectability` orders by finite `|t|`, the historical default. A setup
  preceding a fall therefore competes on the magnitude of its evidence rather
  than being discarded for having a negative sign.
* `Lens::Payoff` orders by forward winner/loss payoff, with `|t|` as its evidence
  tie-break. It exists because a hard top-N cut made under one question cannot
  be reinterpreted as the other after everything below the cut has gone.

The score is computed **before a frontier is discarded**. Same-series runs use
the signal column itself. Projected runs pass the one-minute execution column
and its `Forward` to `run_prepared_ranked_by_reporting`; every frequent mask is
scored there, so a true execution-series winner cannot be lost merely because
it fell below a signal-series top-N. All levels feed the same bounded global
rank heap. The parallel scorer uses bounded worker heaps and merges each into
that one global cut; it never makes a separate top-N decision per level.

Retirement is also the last instant adjacent frontiers coexist. Equal-support
immediate supersets decide closure exactly there. The raw trial count is every
candidate whose support was actually measured — `survivors + infrequent` — and
the effective count subtracts those exact redundant tests. Historical selection
semantics remain rank all, cut to top-N, then filter that retained top to
`closed_top`; closure does not backfill a row from below the cut.

What this first rank does **not** claim is that a forward-edge lens is a final
exit-rule or net-cost optimum. The exit grid, cost stack and validation still
run downstream over the bounded ranked set, because horizon, stop, target,
trail and costs answer a different modelling question. A halted ladder is not a
smaller valid search: its last non-empty frontier has no successor to certify
closure, `RankedOutcome::is_complete` is false, and the operator path refuses it
rather than trading or recording a partial answer.

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
| `engine` | ~~data parallel over candidates via `rayon`~~ — **THIS WAS NEVER TRUE, AND IT CANNOT BE.** No crate took the `rayon` arrow at all: an audit grepped every manifest and every source file and found this row was the only mention in the tree, so the ladder was entirely single-threaded. It cannot be made true here either — CI gate 22 pins `vocab indicators engine` to `vocab` alone, so `engine` may not declare `rayon`. Parallelism over candidates needs a law change, not a patch. D-0232 | none — bar bits are read-only, each shard owns its own output |
| `cli` | data parallel over **instrument-months** in `batch::sweep_under`, via `rayon` | none — each month is its own file, evaluator, ladder and identity. Determinism holds by shape: indexed `collect` preserves order and `Tally` is folded sequentially afterwards, so no output depends on thread scheduling (§3 rule 5). D-0232 |
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
| Reject a duplicate candidate | O(1) expected for the remaining k=1 probe; no probe at k≥2 | one `HashSet` insertion at k=1; the injective prefix join cannot emit duplicates at k≥2. Rust's hash table supplies no adversarial worst-case O(1) guarantee | `C-E-10` measures the historical isolated hit/miss probe; the injectivity tests prove its removal at k≥2 |
| Append one result | O(1) amortised | `Vec::push`; an individual growth can move the existing allocation, so this is not worst-case O(1) | `C-E-11`, retained as `C-04` |
| Fold one candle into every module | O(1) | a fixed set of fixed-size states, no allocation; `size_of::<Evaluator>()` asserted at compile time | `C-I-01`, `C-I-02`, `C-I-04` |

**Two rows corrected here rather than left standing.** "Index into a `Vec<u128>`" was
true when the mask was two words wide and has been wrong since it reached six —
384 bits, not 128, and a `u128` would have run out of room at position 128 against a
370-position table. "A filter probe, then a sharded exact confirm" describes a design
that was never built. At the time of that correction the engine did one `HashSet`
probe; the later injective prefix join proved the level set redundant and removed
it at k≥2. Claiming a sharded confirm made the mechanism sound more careful than it
ever was.

**The live measurement worth reading.** `Column` now owns one fixed-six-word row mask
per bar, and `Column::support` calls `ConditionMask::hits` exactly once per row. On
the 2026-08-30 engine gate, k=1 → k=4 measured **0.971×**, k=1 → k=8 measured
**0.996×**, and the full representational extreme k=1 → k=384 measured **0.942×**.
`column::tests::the_live_support_body_is_one_fixed_width_hit_test` is the structural
half: it refuses the former position/bitmap loop from returning. The old vertical
layout was Θ(k), measured 7.307× at k=8; that figure is retained as the defect that
caused the replacement, not as a description of the shipping path.

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

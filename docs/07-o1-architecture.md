# 07 — The O(1) architecture

Thirteen layers and five laws. Each layer is a rule about what the code **may
do**, not a hope about how fast it will run. Break any one and every layer above
it stops being constant time.

Status: `✓` built · `◐` partly built · `○` not built.

---

## The five laws

Every layer below is one of these applied somewhere specific.

**1 · Fixed width everywhere.** Variable-length input means variable-time
hashing. A 2-letter ticker and a 24-letter one must cost the same.

**2 · Pre-size every map.** Growth is the whole-table O(n) resize this rule can
exclude. Reserving a bound does **not** turn a hash table into a worst-case O(1)
lookup: collisions remain, and Rust's `HashMap`/`HashSet` promises no adversarial
probe bound. Where one remains, this document says **expected O(1)**.

**3 · Never scan to answer a question.** Maintain a counter instead. "How many
do I have?" must be a read, not a walk.

**4 · Arithmetic beats lookup.** If the address can be computed, never search
for it.

**5 · Bound every input at the boundary.** Unbounded input is unbounded time.
Every O(1) claim dies at the first unbounded input, and unbounded input always
arrives from outside.

---

## The thirteen layers

| # | Layer | The rule | How | |
|---|---|---|---|---|
| 1 | Identity | Fixed width, never variable | `Symbol` 24 B, `Isin` 12 B, `InstrumentKey` is `Copy` with a structural hash | ✓ |
| 2 | Hashing | No cryptographic hash on a trusted path | FNV-1a, not SipHash. `core` may declare no dependency (gate 9), so the hash is four `const` lines rather than a crate | ◐ |
| 3 | Maps | Pre-sized with headroom | `HashMap::with_capacity(reservation_for(n))`, factor 2. **Lookup** is expected O(1), not adversarial worst-case. **Append** avoids a resize for the first `n_valid` calls after a load and is amortised O(1) after that — see below | ◐ |
| 4 | Membership | No search of any kind | Open-addressed table built at compile time. **Never `binary_search`** | ✓ |
| 5 | Address | Arithmetic, never lookup | `base + header + i·stride`. The path is the index | ✓ |
| 6 | Hot data | One fixed-width mask per bar | **48 bytes.** `ConditionMask` is `[u64; WORDS]` with `WORDS = 6`, pinned by a `const` assertion in `vocab::mask`; `engine::column::Column` owns exactly one such row per bar and no position bit plane | ✓ |
| 7 | Residency | Build once, reuse without re-reading the source slice | `engine::column::Column::from_rows` copies the fixed rows once and every threshold/candidate probe reuses them. This is logical reuse, not a claim that the OS physically pins all pages in RAM | ◐ |
| 8 | Evaluation | One fixed-width hit test per candidate/bar pair | Six ANDs, six XORs, five ORs and one compare over `[u64; 6]`; `Column::support` invokes it exactly once per row, with no candidate-position loop or early exit | ✓ |
| 9 | Allocation | **Zero in the hot loop** | Preallocated frontier. No `format!`, no `String`, no `push` | ○ |
| 10 | Parallelism | Work-stealing, never static | 10 P-cores. A static split stalls waiting on the 4 efficiency cores | ○ |
| 11 | Blocks | Whole records only | `BLOCK_LEN` is a whole multiple of the stride, so straddling is **unrepresentable** rather than handled | ✓ |
| 12 | Rendering | Bounded page **answered from a precomputed order** | A fixed row cap is not enough — see below. `crates/api/benches/ratio.rs`, C-14 … C-17. **Substring search is the named exception and stays O(universe)** | ◐ |
| 13 | Counting | Counters, never scans | A manifest per vendor. One read, not a walk of every file | ◐ |

---

## Always · never

| ✓ Always | ✗ Never |
|---|---|
| Fixed-size types | Growing a map in a hot path |
| `with_capacity` from a known bound | `binary_search` behind an O(1) label |
| Compute the offset | Cryptographic hashing on a trusted path |
| Counters maintained on write | Allocating inside a loop |
| Refuse over-long input, loudly | Scanning to count |
| Work-stealing across cores | Accepting unbounded input |

---

## What is measured, and what is not

Measured on an Apple M4 Pro, 48 GB, macOS 26.5.2, rustc 1.97.1.

| Operation | Measured | Note |
|---|---|---|
| One mask evaluation | fixed **384-bit representation**, 370 allocated positions (`C-V-01…03`) | Six words are evaluated every time, independent of candidate popcount and answer. This row was first labelled `1→74`, then `1→234`; both were stale vocabulary counts. The live extreme is now tested separately all the way to the representation's padding at k=384, so a width correction cannot again stop at a condition-family boundary |
| Live `Column::support`, candidate k=1 → k=4 / k=8 / k=384 | **0.971× / 0.996× / 0.942×** (`C-E-02`, `C-E-09`) | One fixed-six-word `hits` per row. `the_live_support_body_is_one_fixed_width_hit_test` structurally refuses the former Θ(k) position-bitmap loop; the ratios are regression evidence, not an adversarial latency guarantee |
| Universe membership | worst probe **6** (750 members) / **7** (213 members) | Replaced ~10 comparisons that grew with the list |
| NSE series membership | worst probe **2** (6 members) / **1** (2) / **6** (120) | Layer 4's last holdout. `core::vendor::board_of` binary-searched these three until D-0065. The 120-code table measured **10** at 256 slots and was refused by its own test until it was 512 — the second time this section's own warning has caught a table that was accepted by `build` and too slow to ship |
| Page render, 2,787 → 50,000 instruments | **0.974× – 1.084×**, every sort column × pill, plus the hatch and a clamped deep page | Layer 12. `cargo bench -p api`, exit 0, 2026-08-12 — re-measured for D-0130. Absolute ~157 µs at *both* sizes, release profile. Marginal cost of one more instrument: **0 – 280 ps** per request (C-15), against 85,400 ps before D-0042. **It regressed to 1,433 – 2,100 ps on all thirty C-15 lines and was caught by this bench**, not by the rows or the counts — the page drew its notes, and a note's length is the universe. D-0130 |
| Dashboard, 2 → 50,000 instruments | **0.974×** | Layer 12. Was 80.640× under a docstring that already said "nothing here scans" (C-16). Then **1.954× and still green**, at 87.1 → 170.2 µs, through the whole D-0130 regression: a 3.0× ratio ceiling is wide enough to hide a doubling, which is why C-15's slope exists beside it. Now 10.7 → 10.4 µs |
| Instruments **search**, 2,787 → 50,000 | **6.53 ms** at n = 50,000 for a 2-byte needle | **NOT flat and not asserted.** Printed by the same bench, never gated. `docs/06-limits.md` §24 |
| Checksum per block | **~7,050 ns** table, from 15,622 ns bitwise | The hardware instruction reaches 381.9 ns; see `docs/06-limits.md` |
| GPU vs all 14 cores | **66 ms vs 142 ms — 2.1×** | Compute-bound: 1,219 GB/s effective, 5.3× measured DRAM bandwidth |
| Bar read past RAM | ~100 ns → **61,566 ns** | 616×. Physics. Layer 7 exists because of it |
| Manifest census vs deriving it | **193,449×** and **402,568×** on two runs | Layer 13. The counter against decoding 10,000 entries, same process. The counter side has been seen at 378–789 ps across runs while the scan holds at ~152 µs; **the cause of that spread is not established** — see below. D-0035, D-0036 |
| Manifest entry lookup, 1→100× census | **0.994–1.049×** | Layer 3 in the layer-13 file: reserved from a known bound, so no rehash — measured on the map a **loaded** manifest holds, which is the only one that reservation applies to (D-0036) |

**Not O(1), and never claimed to be:** the sweep. Apriori over the vocabulary is
combinatorial — each *step* is 0.2 ns, the number of steps is not constant.
`AGENTS.md` §3 rule 4 says "constant **per-operation** cost" for that reason.

**The C-11 spread was explained here, and the explanation was wrong.** This
table said "the spread is the counter side at the clock's floor". The bench now
measures the clock and prints it: the smallest non-zero interval `Instant`
reports on this host is **41 ns**, and the counter side is averaged over 100,000
repetitions, so one tick is **410 fs** of reported cost — the 789 ps observation
is 1,924 ticks. The counter side is three orders of magnitude above the clock
floor, so the floor is not what moves it. The observation stays; the cause is now recorded as unestablished.
`CLAUDE.md` §3 rule 6 — never claim a measurement you did not take, and a causal
claim asserted as measured fact is that. D-0036.

**Layer 12 was `✓` with the words "a fixed row cap with paging. Never
O(universe)", and the words were false while the row cap was real.** Only the
*rendered rows* were capped. Every request still re-folded the whole instrument
map for the pill counts, re-filtered it into a fresh `Vec`, re-sorted it and
reversed it — to draw a fixed 200 rows. An audit measured **3.569 ms at 2,787
instruments and 124.916 ms at 50,000, rendering exactly 200 rows both times**:
62.5 ns per instrument per request, on a page whose output never changed shape.
**The row cap is what hid it**, and this table's tick is what carried it. D-0042
moved every ordering and every filter into one build at load time; the layer is
now backed by a bench that asserts a ratio rather than by a sentence.

It is `◐` and not `✓` for two reasons, both named rather than rounded off:

- **Substring search is O(universe) and no index fixes it.** A needle of one or
  two bytes has no trigram to narrow with, and a needle whose rarest trigram
  most of the universe carries narrows to nearly the universe. Measured and
  printed by the same bench, asserted by none of it: **6.53 ms at n = 50,000**.
  `docs/06-limits.md` §24 states why a 1-gram/2-gram index would cost memory and
  buy nothing.
- **The catalog build itself has no ceiling.** The bench prints it (41 ms for
  all three universes together) and asserts nothing about it, so a build that
  turned quadratic would fail no gate. That is a real hole in this layer.

**Layer 3 was `✓` with "zero rehash, so O(1) worst case rather than average",
and D-0040 found the append could still rehash.** The reservation was
`with_capacity(n_valid)` — exactly the census, no free slot — so the *first*
append after a load rebuilt the table at every census of the form `7·2^k`. It
measured **5,100,585 ps per append at a 57,344-entry census**, and it passed
every round-number bench because `with_capacity` happened to round 1,000 /
10,000 / 50,000 up and leave spare slots. The reservation is now the census
doubled, capped at `MAX_ENTRIES`. What that buys is stated exactly: **no
whole-table resize** for the first `n_valid` appends after a load, then
amortised O(1) growth. It does not bound hash collisions, so the complete
lookup/insert remains expected O(1), not adversarial worst-case.
`pull::unit::a_loaded_index_carries_headroom_for_the_appends_after_it`
(M-19) asserts the growth past the headroom, deliberately, so the row cannot be
read as an unconditional claim. `docs/06-limits.md` §23.

**Layer 13 is `◐`, not `✓`.** The manifest exists, its three totals are
maintained on write and checked against the entries on load, and both bounds
above are measured. What is *not* built is a **filtered** census — "how many
expired option series" still walks the manifest's own entries, which is one
sequential file read instead of ~248,000 directory operations but is not a
counter read. The counters are also never checked against `bars/` itself, so
they can drift from the files they describe in both directions and nothing here
detects it. And the directory walk the file replaces has never been measured
here at all; every statement about that saving is an **EXTRAPOLATION**. See
`docs/06-limits.md` §17.

---

## How a layer is proven

A layer is not built because the code looks right. It is built when a test
asserts the bound as a **number**:

- Layer 4's probe length is asserted at `<= 8` and printed. The first attempt
  measured **14** — worse than the `binary_search` it replaced, still O(1) by
  definition — and the test refused it until the table was widened.
- Layer 8's flatness is asserted against the 1.4× ceiling in
  `docs/04-invariants.md`: C-E-02 checks ordinary k=1/4/8 candidates and C-E-09
  drives the complete k=384 representation. The source guard separately pins one
  fixed-width `hits` call per row, because a ratio alone can hide a uniformly
  slower implementation.
- Layer 3's guarantee is the *absence* of a rehash **over a stated number of
  appends**, so the bound is the reservation itself, taken from a count known
  before the loop starts — and the number of appends it covers is part of the
  claim, not a detail. Stated without that clause (as it was until D-0045) the
  sentence is broader than the code.
- Layer 12's flatness needs a **second** measurement to mean anything: cost per
  *rendered row* (C-17). A page that got flat by rendering less would pass every
  ratio and be worse than what it replaced. That is the same class of mistake as
  capping the rows and calling the page constant, which is exactly what this
  layer shipped with.

Two traps this has already hit, recorded so they are not hit again:

**`const fn` is invisible to coverage.** A table built at compile time is
executed by the compiler, so runtime instrumentation sees none of it — the same
hole as an unreachable branch, arriving by a different route. It needs a test
that calls the builder in a non-`const` context.

**A collision branch needs a real collision.** Filling a small table does not
necessarily collide; the probe branch then stays unentered while the test
passes. The colliding pair is computed against the actual hash, not hoped for.

---

## Layer 12's other half — the browser

Everything above is the engine and `crates/api`'s server-rendered page. Layer
12's bench is `cargo bench -p api`. **The SvelteKit console is a second
rendering surface, and until now this document said nothing about it** — so its
bounds were neither claimed nor refused, which is worse than either.

Measured 2026-08-29 against the running server: `/db`, Zerodha NIFTY 1min,
618,296 matched bars, at load average ~194 with three `cli` test binaries
holding ~430 % CPU each. Every figure is a browser measurement, taken with
`performance.now()` around the state change and read back off the DOM.

| Operation | Measured | Bound |
|---|---|---|
| Sequential page (next) | **17, 18, 17, 17 ms** | O(1) |
| Jump to page 12,345 · 33,333 · 41,000 | **67 · 90 · 69 ms, ONE request each** | O(1) in distance |
| Jump to an already-read month | **105 ms, zero requests** | O(1) |

**Why the paging figure is O(1) and not merely fast.** The page number maps to a
month file by arithmetic — layer 5's rule applied in the browser — so a jump
opens exactly one file whatever its distance. 12,345 and 41,000 cost the same
because neither is *searched* for. The first measurement of this was **1.16 s,
2.3 s and 9.1 s** across three jumps in quick succession, which looked like O(n)
and was three cold reads queueing on a saturated host. **Both are recorded**:
the slow one is what a reader will sometimes see, and a table showing only the
fast one would be the same class of dishonesty as a row cap hiding a fold.

**Both price marks this section used to measure are gone, and the rows are
struck rather than quietly dropped.** The table above once carried *"Price line
over the window — 240 samples at 7,500 rows and at 618,296"* and *"Price
position mark, per cell — walks `barPage`, 12 rows at `Fit`"*. Both bounds were
real and both features were removed for reasons that had nothing to do with
cost: the per-cell mark resolved to two distinct positions across twenty drawn
dots — one bit of information per cell — and the price line drew a monotone ramp
whenever the grid was sorted by close, which is D-0377. A correct measurement of
a thing that no longer exists is still a false row in a table a reader trusts.

### Not O(1) in the browser, and not claimed to be

Three derived values walk every row **currently loaded** — not the store, but
not a constant either. At today's read budget that is ~7,500 rows.

| What | Per operation | Total | Verdict |
|---|---|---|---|
| `barWindowed` | O(1) per row tested | O(loaded) per window change | Compliant |
| `timeOptions` | O(1) per row folded | O(loaded), once per fetch | Compliant |
| `csvRows` / export | O(1) per row written | O(rows written) | Compliant, and irreducible |

**THIS TABLE FIRST SAID ALL THREE WERE LAW-3 VIOLATIONS, AND THAT WAS WRONG.**
It read "each is a maintain-incrementally problem — law 3, never scan to answer
a question — and none of them does". Checked against the definition this
repository actually uses, in `docs/06-limits.md` §1: "**Per-operation cost is
constant.** … **Total work is not constant.** It scales with symbols × bars ×
candidates. It has to; that is the shape of the problem." By that standard all
three are exactly what §1 describes as correct, and filing them as defects put
a false entry in the ledger — which is worse than the omission it replaced,
because a reader would go and "fix" code that is already right.

**Law 3 is about COUNTS, not transformations.** Its words are "How many do I
have? must be a read, not a walk" — a question a counter can answer without
touching the elements. None of these three asks that. `barWindowed` produces a
subset, `timeOptions` produces a distinct set, and the export produces bytes.
You cannot emit 7,500 CSV rows in fewer than 7,500 writes, and a function that
claimed to would be writing something other than the rows.

What remains true, and is waste rather than complexity: **`barWindowed`
recomputes wholesale on every window change.** Ten adjustments of the time
bound cost ten passes over the loaded rows to produce ten answers, nine of
which are thrown away. That is a constant factor on an already-proportional
operation, not a class — and it is named here so the distinction between the
two is on the record rather than being rediscovered as a "violation".

### O(1) space is not achievable and is not offered

**Per-operation space is constant. Total space is not, and cannot be.** 618,296
bars cannot be held in constant memory; the store is on disk precisely because
of that, and layer 7 exists because a bar read past RAM costs 616× more.

What IS constant in the browser, which is the honest form of the claim:

- **DOM nodes per screen** — bounded by the page size, not the match count. A
  618,296-row query and a 12-row one draw the same twelve rows.
- **Memory per page** — the window route returns a slice; the exact-plan path
  holds whole month files, bounded by months read rather than by the query.

The distinction `docs/06-limits.md` §1 draws for the engine — per-operation
constant, total not — holds here word for word, and a browser claim that omits
the second sentence is false in the same way.

### What has no bound at all yet

Stated because an absent bound is worse than a slow one.

- **No browser bench exists.** Every figure above was taken by hand in a
  console. Nothing re-measures them, so a regression of the kind D-0042 found on
  layer 12 — a fold hidden behind a row cap — would be caught here by nobody.
  Layer 12 earned its `◐` when a sentence was replaced by a bench; this half has
  only the sentence.
- **Substring filtering in the browser** is the same O(universe) hole
  `docs/06-limits.md` §24 records for the server, arriving by the other route.

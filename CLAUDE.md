# brutex — session law

Read this file completely before any action. It outranks convenience,
precedent, and your own judgement about what would be easier.

---

## 1. What this is

A brute-force backtesting engine for Indian spot indices. It sweeps
combinations of boolean market conditions over historical 1-minute bars and
ranks what survives.

**Engine surface — two shapes, NSE only:**

1. The two spot indices `NSE-NIFTY` and `NSE-BANKNIFTY`.
2. The **cash equities of the 213 F&O underlyings** — the stock's own price
   series, the same thing the spot level is for an index. Widened by D-0506.
   The list is `core::universe::FNO_UNDERLYINGS` and nothing else names it.

BSE and MCX are not swept and not pulled. Narrowed from three
instruments by D-0017. Existing BSE data already on disk is not deleted --
append-only history applies to the store as well.

`NSE-INDIAVIX` is **reference only**: it is stored and it is stamped onto
observable trades, but it never enters the condition vocabulary, never enters
ranking, and never enters run identity.

Futures and options **contracts** may be **stored**. They are never swept: they
expire, and NSE reuses instrument tokens across an expiry boundary, so a
contract is a moving target. Single stocks outside the F&O universe may be
stored and are never swept.

**Why the equities were added, and what it changes.** The operator's stated
objective is the rare, massive winner — a setup that fires seldom, loses tiny
when it loses, and pays enormously when it pays. Those moves exist in single
stocks and are averaged away in an index. Two consequences follow and neither
is optional: a stock CAN be bought, so its costs are real and must be charged
before ranking, not after; and 213 instruments multiply the search by 213, so
an in-sample result across the pool is the largest of billions and means
nothing until it is validated out of sample.

---

## 2. The one hard rule

**Rust is the only language in this repository.**

Allowed tracked extensions: `.rs` `.toml` `.md` `.lock` `.html` `.css` `.yml`
(the last only under `.github/`), and `.json` only under `.claude/`.

Four files are allowed **by name** rather than by extension, because a repository
cannot exist without them and none is source: `LICENSE`, `CODEOWNERS`,
`.gitignore`, `.gitattributes`.

**Both parentheses are the rule, not a note.** A `.yml` outside `.github/` and a
`.json` outside `.claude/` are the same build failure any other extension is.

The `.claude/` clause covers **operator tooling, which is neither source nor
shipped**: no crate reads it, no build step opens it, and gate 1e's premise —
that the workspace builds with the front end moved aside — is untouched by it.
Exactly one tracked file uses it, `.claude/launch.json`, which carries the two
run configurations `docs/07-plan.md` §0 names. It was written into gate 1 and
gate 1b first and into this list second; gate 1's own comment refused to close
the gap on its own, in these words: *"Resolving that is a `CLAUDE.md` edit and a
`docs/05-decisions.md` entry, which a CI gate must not make on its own — a gate
that widens the law to match the tree is the shape this whole file exists to
refuse."* D-0210 is that entry, and this sentence is that edit.

**One exception, and it is a path, not a language.** Under `web/` — and nowhere
else — the front end is unrestricted. Any language, any framework, any
toolchain, any file extension. Narrowed to that directory by D-0052 and widened
to "unrestricted within it" by D-0053.

Everything the exception does not name is unchanged. `crates/**` is Rust. The
engine, the store, the vocabulary, the sweep, the ingest and the HTTP surface do
not gain a second language, and a file under `crates/` with one of those
extensions is the same build failure it always was. **The one rule that remains, and it is about the ENGINE, not the browser:** no
crate may depend on the front end's toolchain to build, test or run. `cargo
build`, `cargo test` and `cargo clippy` must pass on a machine with no Node, no
package manager and no `web/` build ever run. A `build.rs` that invokes one is
the build failure §2 already makes it.

That is the whole boundary. Inside `web/` do whatever serves the page best.
Outside it, everything above stands unchanged: `crates/**` is Rust, and a file
with a front-end extension under `crates/` is the same build failure it always
was.

Forbidden without exception:
- any interpreted runtime, as a dependency, a dev-dependency, or a tool
- any `build.rs` that invokes an external process
- any vendored binding to another language
- any generated source in another language, checked in

If a task appears to require one of these, **stop and say so**. Do not add it
and explain afterwards.

CI gate 1 enforces this by walking every tracked file. It is not advisory.

---

## 3. Golden rules

1. **No invention.** Every claim about a vendor, an exchange, an instrument or
   a cost is traceable to a source recorded in `docs/00-charter.md`. If you are
   unsure, write `UNVERIFIED` and stop.
2. **No silent scope change.** The engine surface in §1 does not widen OR
   narrow without a new entry in `docs/05-decisions.md`.
3. **Reproducibility.** Every run is identified by
   `blake3(mask ‖ direction ‖ instrument ‖ timeframe ‖ params ‖ data_digest ‖
   vocab_version ‖ commit ‖ feed)`. No computation without that identity
   recorded.

   **`feed` is the ninth, and it was added because the eight identify what was
   computed and none of them identifies whose data it was computed on.** The
   store is keyed by vendor, so one instrument-month exists once per feed and
   sweeping two of them is two runs. Two feeds were separated only
   *incidentally* — by their bytes happening to differ — and two vendors
   redistributing one NSE feed publish the same OHLCV at the same timestamps for
   a clean month, at which point `data_digest` is equal and so is every other
   term. Meanwhile `cli`'s own banner told the reader the identity *"names the
   exact column they came from"*. D-0225 is that entry, and this sentence is
   that edit.
4. **Constant per-operation cost.** Bar lookup, condition lookup, mask
   evaluation, duplicate rejection and result append are each O(1). A change
   that makes one of them scan fails the bench gate.

   **An O(1) audit found two drifted implementations, and both corrections are
   now on the live path.** The history remains stated because §3 rule 6 asks for
   honest limits; describing either historical defect as current would now be a
   second documentation defect.

   *Mask evaluation* is O(1) per bar in the production
   `engine::column::Column::support` path. The owned column is row-major and
   folds exactly one fixed-six-word `vocab::ConditionMask::hits` call per bar,
   independent of candidate width and of whether the row matches. The former
   vertical implementation did one bitmap intersection per named condition and
   was Θ(k); `C-E-02b` retains that failed measurement, while `C-E-02`,
   `C-E-09` and the source-shape tests bind the corrected live method.

   *Duplicate rejection* is one pre-sized `HashSet<u32>::insert` for each k=1
   offered position. That probe is expected/amortised O(1), not an adversarial
   worst-case hash-table guarantee. At k≥2 the prefix join is injective, so its
   former candidate `seen` set was removed: there is no dedup operation on that
   path to call O(1).

   *Result append* is `Vec::push` **at k=1 and `Vec::extend` at k≥2, and the
   difference is the live path rather than a detail.** `engine::drain` hands each
   support lane its own pre-sized `kept`, pushes into that, and then folds the
   lanes into `out` with one `extend` per chunk — so the per-candidate `push`
   happens into a lane-local vector and the result vector is appended in batches.
   Same amortised class, different operation from the one this rule named for as
   long as it has existed. k=1 reserves the offered width and later levels
   reserve a capped previous-frontier heuristic. Appends within that reservation
   allocate nothing; an expanding level can outgrow it, so the unconditional
   bound is amortised O(1), not worst-case O(1) for every append.

   These qualifications do not widen the rule: they identify where its named
   primitive exists, where injectivity removes the need for one, and where
   Rust's collection guarantee is amortised rather than worst-case.
5. **Idempotence.** Same inputs, same outputs, byte for byte. Reruns are safe.
6. **Honest limits.** If a bound cannot be met, say so. Never claim a
   measurement you did not take. Label extrapolations as extrapolations.
7. **No look-ahead.** At bar N the engine may read bars 0..N.

   **Enforced by the SHAPE of the fold, not by an accessor.** This rule used to
   end *"enforced by an index-guarded accessor, not by review"*, naming
   `indicators::PastPrefix`. That type exists and has **zero production call
   sites** — measured: one doc comment and two declaration lines, every other
   reference inside `#[cfg(test)]`. The property does hold, but not for the
   stated reason.

   What actually holds it: `Column::build` walks
   `for (index, bar) in bars.iter().enumerate()` and hands the evaluator **one
   bar at a time**, so a later bar is not in scope when an earlier one folds —
   look-ahead is unreachable rather than rejected. `Evaluator` keeps its own
   running state and is never given the slice.

   `PastPrefix` remains for a caller that indexes rather than streams, and is
   the right tool there. Two rules follow from the distinction, and the second
   is the one that bites: **a new consumer that takes `&[Candle]` and indexes
   into it inherits no protection at all** — `vwap::availability_of` reads the
   whole slice, which is exactly why `cli` and `runner` pass
   `Availability::Absent` rather than deriving it. Wiring `PastPrefix` into the
   fold, or writing a gate that refuses slice indexing on that path, would make
   the original sentence true; until one of those lands this is the honest
   statement. D-0212.
8. **Append-only history.** Condition bits are never renumbered or reused.
   Store format versions are never mutated in place.

---

## 4. What may never appear

| Banned | Because |
|---|---|
| A depth parameter on the sweep | Depth is decided by extinction, not by a caller. See §6. |
| A query planner or ORM | The path is the index. There is nothing to plan. |
| A dynamic schema | A new field is a new file version at its own stride. |
| A writable memory mapping | It raises a signal on a full disk, and a signal cannot be caught. |
| A fallback that hides a failure | Degrade loudly and name the reason, or refuse. Never both silently. |
| A test that asserts nothing | A surviving mutant is a missing test and blocks the build. |

---

## 5. Crate graph — acyclic

**This is the MEASURED graph**, derived from `cargo metadata --no-deps` rather
than drawn. The drawing it replaces was wrong in three ways at once and each was
load-bearing: it named `web` and `cli`, neither of which exists as a crate; it
omitted `greeks`, `costs`, `lake`, `telemetry` and `runner`, all of which do; and
it drew every crate as a child of `core` when four of them depend on nothing at
all and two depend only on `vocab`. A graph that cannot be checked against the
manifests is a picture, not a law.

```
depends on NOTHING          core · vocab · greeks · telemetry

core          <-- costs
core telemetry <-- store · lake
vocab         <-- indicators · engine
core costs greeks store telemetry <-- pull
core pull store telemetry cli vocab <-- api
core costs engine indicators vocab         <-- runner
core costs engine indicators pull runner store telemetry vocab <-- cli
```

Thirteen members, and the root `Cargo.toml` `members` list is exactly the
thirteen directories under `crates/`. The graph is acyclic and CI proves the two
arrows that carry a rule: gate 9 that `core` depends on nothing, gate 9b that
`greeks` does.

**This block is hand-maintained and has drifted at least four times.** D-0208
corrected five statements in this section against `cargo metadata --no-deps`:
the member count, a missing `cli` row, `pull`'s `costs` arrow (decided by D-0206
and never drawn here), `cli`'s dependency set, and the banner sentence below.
`pull`'s `greeks` arrow (D-0217) and `api`'s `cli` arrow were each drawn here
only after their manifests took them. `cli`'s `pull` and `vocab` arrows, both
declared on 2026-09-01, were missing here until D-0683: D-0453 drew `pull` in
`AGENTS.md` and `docs/01-architecture.md` but not in this file, and its own list
left out `vocab`. Gates 9 and 9b pin one arrow each, and `core/tests/graph.rs`
checks the table in `docs/01-architecture.md` against all thirteen manifests;
**nothing parses this block as a whole**, so check it against that gate and the
manifests rather than trusting it.

**`indicators` and `engine` may not name each other.** Gate 22 clause A pins both
of their dependency sets to `vocab` alone and ships no allowlist, so a bar cannot
reach the sweep. The only crate where the two halves may legally meet is one that
is not on gate 22's list, and that is `runner` — which is why it exists and why
keeping the join there preserves the gate rather than avoiding it.

**The browser is not a crate.** It was drawn here as `web`, a wasm32 crate
depending on `core` alone, and that has not been true since D-0052 and D-0053
moved the front end to `web/` as an unrestricted directory. There is no
`crates/web`, so the "depends on core ONLY" rule has nothing to bind and **CI
gate 7 skips permanently**. §2's boundary is what governs the front end now: no
crate may depend on its toolchain to build, test or run.

**`api` names `vocab`, and it is the sixth arrow on that row.** `/backtest.json`
serves a run's `mask_words` as six raw `u64`s and cannot serve NAMES: turning a
bit into a name needs the table. Decoding in the ledger response would repeat 370
rows of vocabulary on every run in every page, so `/vocab.json` serves the table
once and the browser decodes every mask it is shown.

The alternative was a hand-kept copy of the table in JavaScript, and that is the
one this graph exists to refuse: two vocabularies for one fact, correct the day
it is written and silently wrong the first time a bit is appended — a mask
decoded against a stale table names the WRONG conditions and looks exactly like
an answer. `api` is not on gate 22's list and `vocab`'s own dependency set is
untouched, so clause A is unaffected. D-0288.

**`cli` now exists, and closing that gap is what it is for.** Until D-0169 the
only binary was `api`, whose dependency set is `core`, `pull`, `store` and
`telemetry` — so nothing that could be RUN reached `runner`, `engine`,
`indicators`, `vocab` or `costs`. The sweep compiled and was tested and was
unreachable from any entry point, and the three render surfaces that display it
had no caller at all.

`cli` depends on `runner`, `engine`, `indicators`, `costs`, `store`, `core`,
`pull`, `telemetry` and `vocab` — **nine arrows, and `store`, `pull` and `vocab`
are among them.**

**`pull` is the calendar-attestation edge, not a vendor-network fallback.** The
stored-run boundary calls `pull::calendar::kind_of` as the canonical IST session
authority, Population V4 civil bounds use `pull::session::Day`, and global
replay/VIX month identity uses `pull::session::IstMoment`, and `cli::fold_audit`
folds minutes into rung bars with `pull::fold`, the one fold authority. These are
the principal uses, not an exhaustive list: `pull::session::Day` also appears in
the boolean and candidate modules. Repeating those types or rules in `cli` would
create a second calendar or fold authority. No stored reader uses this edge to
fetch vendor data or silently replace missing evidence; those paths still
refuse. The edge is acyclic because `pull` does not depend on `cli`. D-0453,
D-0683.

**`vocab` is named directly because `cli` stores and checks the vocabulary's own
types.** The resumable expression and Boolean-grammar searches checkpoint a
`vocab::expression_search::Cursor` and size their records by its
`CURSOR_BYTES`; candidate trades, the pool and `cli::verify`'s
suffix-independence check carry `vocab::ConditionMask`; the search-sizing model
checks its lower bound against `vocab::expression::MAX_INSTRUCTIONS`; and the
candidate-universe vocabulary digest hashes `vocab::VOCAB_VERSION` and every row
of `vocab::table::TABLE`. `vocab` depends on nothing, so the arrow cannot cycle,
and gate 22 clause A pins `vocab`'s own dependency set, not who may name it.
D-0683.

**`telemetry` remains direct, and gate 17 is why it is here rather than one crate
deeper.** Until D-0226 a `sweep-stored` or `sweep-all` run produced no event of
any kind — not the file it opened, not the bars it read, not a refusal — so the
`/logs` page covered the pull half of the data path and nothing of the read half.
Gate 17 silences `vocab engine indicators runner`, because those hold the loops
and its rule is not "each call is cheap" but "the innermost loop calls nothing at
all". `cli` holds no loop over bars and none over candidates: it is the
structural boundary, one event per run and one per instrument-month, which is the
granularity gate 17's own comment prescribes as the affordable one. D-0226.

`cli` once deliberately had no `store` arrow, on the reasoning that the
operator's standing rule forbade both a vendor pull and the bars already on
disk, leaving `runner::synthetic` as the only honest input. **That is no longer
what the crate does, and the sentence is not merely stale — it argued for an
absence that has been filled.** `cli sweep-stored` loads one real
instrument-month through `store::file::BarFile` and sweeps it. The arrow is
what makes the run identity §3 rule 3 demands recordable at all: a synthetic
bar has no instrument to name, and naming one would be the invention §3 rule 1
forbids. D-0208.

What survives from that reasoning is the half about gate 22, and it is the half
that carries the rule: `cli` declines to be a *swept* crate, not to be a caller.

It is **not** on gate 22's list and must never be added to one: clause A pins
`vocab`, `indicators` and `engine` to `vocab` alone. `cli` is a caller, exactly
as `runner` is.

Every report it renders is led by a **provenance banner, and there are two of
them making opposite claims** — `PROVENANCE` says the bars were GENERATED,
`STORED_PROVENANCE` says REAL MARKET DATA and must not carry the generated one's
disclaimer. A sweep over invented data is byte-identical in shape to one over
real data, so the banner is the only thing separating them, and
`the_generated_and_stored_banners_make_opposite_claims` fails the build if they
ever converge. Without that line either would be the failure wearing a success's
clothes that §4 bans.

---

## 6. Sweep depth

**There is no `k` parameter.** Not a default, not a token, not an environment
override. The type does not carry the field.

The sweep walks the combination ladder upward from k=1 and stops where the
frequent frontier empties — classic Apriori level-wise join and subset-prune,
justified by anti-monotonicity: a mask hits a bar iff `(bits & mask) == mask`,
so adding a required bit can only remove hits. If any (k−1)-subset is
infrequent, the k-combination cannot be frequent, and is never enumerated.

*Why the parameter is absent rather than defaulted:* in the predecessor
repository the flag defaulted to a dynamic token, but the frequent-frontier
mask was 64 bits wide against a 74-condition vocabulary. Every real run tripped
the width guard and silently fell back to a hardcoded `k = [1, 2]`. Dynamic
depth was unreachable on the only vocabulary that existed, and nobody could see
it from the flag. A parameter that can be set can be set wrongly and silently.

---

## 7. Money and prices

- Prices are **paisa integers** (`i64`). Never a float.
- The tick grid is 2 decimal places. Snapping happens once, at the write
  boundary, half-up.
- `i64::MIN` is the open-interest null sentinel. Zero means zero.
- Statistical values (Sharpe, p-values, ratios) keep full precision and are
  never rounded for storage.

---

## 8. Credentials

Read-only, from AWS Parameter Store SecureStrings, region `ap-south-1`.

**No literal parameter path appears in any tracked file.** This repository is
public. Paths are written here and in the documents only as their generic
shape:

```
/<org>/<env>/<vendor>/<field>
```

The real path segments are supplied at runtime from a local, untracked
configuration file that is never committed. `crates/pull` holds the shape and
the field names; it holds no `org`, `env`, or `vendor` literal. A missing or
malformed configuration halts the pull loudly — there is no default and no
fallback. See `docs/05-decisions.md` D-0013 and CI gate 1c.

The **credential value** is never an environment variable, **never** a file,
**never** a prompt. Only the *path* comes from the local configuration; the
secret itself is read from Parameter Store and nowhere else.
**This repository never mints a token.** A stale token is re-read; if the
re-read returns the same dead value, the pull halts loudly. A local mint would
invalidate the token another system shares.

---

## 9. Definition of done

A change is done when all of these are true, verified not assumed:

- `cargo fmt --check` clean
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `cargo test --workspace --locked` green
- `cargo deny check` green
- line and branch coverage 100% on every touched crate
- no surviving mutant on touched modules
- every new invariant appears in `docs/04-invariants.md` beside the test that
  proves it
- a `docs/05-decisions.md` entry exists for every locked choice

Report failures plainly. Do not paper over a red gate.

---

## 10. Documents

| File | Authority over |
|---|---|
| `docs/00-charter.md` | scope, verified external facts, prohibitions |
| `docs/01-architecture.md` | crates, arrows, data flow |
| `docs/02-store-format.md` | bytes on disk |
| `docs/03-vocabulary.md` | condition bit table |
| `docs/04-invariants.md` | what must hold, and its proof |
| `docs/05-decisions.md` | append-only ledger |
| `docs/06-limits.md` | what is not constant-time, and what is unmeasured |
| `docs/07-plan.md` | the requirements, what is done, what is next, and what blocks it |
| `docs/07-o1-architecture.md` | the layered O(1) design the bench gate measures against |
| `docs/08-vendor-samples.md` | what a real vendor payload actually looked like |
| `docs/09-design-sources.md` | where the front end's design came from |
| `docs/09-verify.md` | the verification surface and what it proves |
| `docs/10-shared-core.md` | the crates shared with `tickvault`, and their contracts |
| `docs/11-findings.md` | the adversarial findings ledger — append only, no row deleted |

**The table was eight rows while fourteen documents existed**, so six carried no
stated authority at all and a reader had no way to know whether they bound
anything. All fourteen are listed now. Two numbers are used twice — `07-` and
`09-` — which is a naming defect, not two documents pretending to be one; both
of each pair are named above and neither is authoritative over the other.

If this file and a document disagree, **this file wins** and the document is
the stale copy to fix — **with one caveat that has already bitten.** That rule
resolves a *contradiction*; it is not evidence about which copy is *correct*.
D-0208 found the opposite case: `docs/01-architecture.md` was right and §5 was
stale, because a test checks that document and nothing checks this file. Where a
document is gate-checked and this file is not, believe the gate.

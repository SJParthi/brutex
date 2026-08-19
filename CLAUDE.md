# brutex — session law

Read this file completely before any action. It outranks convenience,
precedent, and your own judgement about what would be easier.

---

## 1. What this is

A brute-force backtesting engine for Indian spot indices. It sweeps
combinations of boolean market conditions over historical 1-minute bars and
ranks what survives.

**Engine surface — exactly two instruments, NSE only:**
`NSE-NIFTY`, `NSE-BANKNIFTY`.

BSE and MCX are not swept and not pulled. Narrowed from three
instruments by D-0017. Existing BSE data already on disk is not deleted --
append-only history applies to the store as well.

`NSE-INDIAVIX` is **reference only**: it is stored and it is stamped onto
observable trades, but it never enters the condition vocabulary, never enters
ranking, and never enters run identity.

Futures, options and single stocks may be **stored**. They are never swept.

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
   vocab_version ‖ commit)`. No computation without that identity recorded.
4. **Constant per-operation cost.** Bar lookup, condition lookup, mask
   evaluation, duplicate rejection and result append are each O(1). A change
   that makes one of them scan fails the bench gate.
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
core costs store telemetry      <-- pull
core pull store telemetry       <-- api
core costs engine indicators vocab         <-- runner
core costs engine indicators runner store  <-- cli
```

Thirteen members, and the root `Cargo.toml` `members` list is exactly the
thirteen directories under `crates/`. The graph is acyclic and CI proves the two
arrows that carry a rule: gate 9 that `core` depends on nothing, gate 9b that
`greeks` does.

**This block is hand-maintained and has drifted once.** D-0208 corrected five
statements in this section against `cargo metadata --no-deps`: the member count,
a missing `cli` row, `pull`'s `costs` arrow (decided by D-0206 and never drawn
here), `cli`'s dependency set, and the banner sentence below. Gates 9 and 9b pin
one arrow each; **nothing pins this block as a whole**, so check it against the
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

**`cli` now exists, and closing that gap is what it is for.** Until D-0169 the
only binary was `api`, whose dependency set is `core`, `pull`, `store` and
`telemetry` — so nothing that could be RUN reached `runner`, `engine`,
`indicators`, `vocab` or `costs`. The sweep compiled and was tested and was
unreachable from any entry point, and the three render surfaces that display it
had no caller at all.

`cli` depends on `runner`, `engine`, `indicators`, `costs`, `store` and `core` —
**six arrows, and `store` is one of them.**

It was once deliberately *not*, on the reasoning that the operator's standing
rule forbade both a vendor pull and the bars already on disk, leaving
`runner::synthetic` as the only honest input. **That is no longer what the crate
does, and the sentence is not merely stale — it argued for an absence that has
been filled.** `cli sweep-stored` loads one real instrument-month through
`store::file::BarFile` and sweeps it. The arrow is what makes the run identity
§3 rule 3 demands recordable at all: a synthetic bar has no instrument to name,
and naming one would be the invention §3 rule 1 forbids. D-0208.

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

# Handover: wire the backtest into `api` and `web`

## READ THIS FIRST — what you are being asked to do

Build the backtest page. The engine, the sweep, the 1-minute execution layer,
both fill models and the results ledger are **already built, tested and on
`feat/pull`**. You are wiring them to a route and a page. You are not writing an
engine, and you should not need to touch `crates/runner`, `crates/cli`,
`crates/indicators`, `crates/costs`, `crates/vocab` or `crates/engine` at all —
another session owns those and is actively editing them.

**Your files:** `crates/api/**`, `crates/store/**`, `crates/pull/**`, `web/**`.

Commit with **explicit pathspecs** — the git index is shared between two live
sessions. Never `git add -A`.

---

## The standing requirements this repository is held to

These are the operator's, and they apply to everything below:

- **Rust only** outside `web/`. Inside `web/`, unrestricted.
- **O(1) always** — uniqueness, deduplication, mapping, latency, time and space.
  Frontend, backend, database, anything. Every read of the ledger below is
  `O(1)` at a computed offset; keep it that way. A scan added to a request path
  is a regression.
- **Everything auditable**: saved, logged, tracked, captured, searchable,
  monitorable, visualised, and reachable from the nav. A page that ships
  unreachable has not shipped — `/logs` did exactly that once.
- **Cover every worst case**: errors, exceptions, empty states, partial data,
  concurrent writes, malformed input. Each is a FACT that gets a sentence, never
  a blank and never a silent default.
- **Common, runtime-dynamic, incremental, scalable.** No hardcoded instrument,
  no hardcoded span, no hardcoded rung list that the store contradicts.

---


**For the session that owns `crates/api`, `crates/pull`, `crates/store` and `web/`.**
Written by the session that owns `crates/cli`, `crates/runner`, `crates/indicators`,
`crates/costs`, `crates/vocab`, `crates/engine`. Nothing in this document has been
edited by that session — every file named below is yours.

---

## 1. The one-sentence problem

`api` can serve pages about DATA and cannot serve a page about a BACKTEST,
because its dependency set is `core, pull, store, telemetry` and the sweep lives
behind `runner`. `/audit` is the INGEST console; there is no backtest page at
all.

---

## 2. What already exists, so you build none of it

Everything below is on `feat/pull` and passing. You are wiring, not writing an
engine.

| Thing | Where | State |
|---|---|---|
| Sweep a contiguous span as one series | `cli::stored::load_span` | done |
| Full audit over a span | `cli::audit_range` | done |
| All nine rungs, one table | `cli::range_all` | done |
| **Results ledger, append-only** | `cli::results` | done |
| **List / rank recorded runs** | `cli::results_list` | done |
| 1-minute execution layer | `runner::align`, `Column::reproject` | done |
| Both fill models | `costs::fill::Anchor::{Open, AdverseExtreme}` | done |

---

## 3. The results ledger — read this, do not re-derive it

**Path:** `<store_root>/results/runs.bin` where `store_root` is `$BRUTEX_STORE`,
else `$HOME/.brutex/store`. The same two-step every other root uses.

**Layout** (`crates/cli/src/results.rs`):

```
header 16 bytes : magic b"BRUTEXRS" (8) | version u32 le (4) | reserved (4)
record 205 bytes, little-endian throughout
```

Address of record *i* is `16 + i * 205`. Count is `(file_len - 16) / 205`. Both
O(1).

Field order is exactly `Record::to_bytes` in that file — **read it, do not
re-type it from this table**, because the stride is checked against the field sum
by `the_stride_is_exactly_what_the_writer_writes` and only that test is
authoritative. A wrong stride still PARSES and produces a silently wrong file.

Fields worth surfacing: `identity[32]`, `finished_micros`, `feed[16]`,
`underlying[16]`, `timeframe[16]`, span `from/to year+month`, `months_asked`,
`months_found`, `bars`, `min_hits`, `combinations`, `depth`, **`halted`**,
`trades`, `pessimistic`, `optimistic`, `worst_trade`, `max_drawdown`,
`winner_mae`, `winner_mfe`, `all_mae`, `exit_rungs[5]` (`-1` = no rung).

### `halted` is the field that decides whether the others mean anything

`halted == 1` means a budget stopped that ladder short: `depth` is PARTIAL and
`combinations` covers LESS of the search while reading LARGER. **A page that
ranks halted rows against complete ones is wrong.** `cli::best_complete_line`
filters them out and says `NO COMPLETE RUN` when none survive; do the same.

---

## 4. Two ways to wire it — pick B unless you have a reason

### A. `api` reads the ledger only *(no new dependency)*

`api` opens `runs.bin` itself and serves rows. Needs **no** `runner` arrow, so
the crate graph in `CLAUDE.md` §5 does not change. But it cannot START a run —
the page is a viewer, and the operator still runs sweeps from the CLI.

### B. `api` depends on `runner` *(recommended)*

Add to `crates/api/Cargo.toml`:

```toml
runner = { path = "../runner", version = "0.1.0" }
```

**CI check, already done for you:** gate 22's `PINNED` list is
`'vocab indicators engine'` (`.github/workflows/ci.yml:3134`). `api` is not on
it, so this arrow breaks no gate. It DOES change the measured graph, so
`CLAUDE.md` §5 must be amended and a `docs/05-decisions.md` entry appended —
§5's own banner says that block is hand-maintained and has drifted once.

**Do not move `cli::results` into `api`.** Either re-declare the reader in `api`
against the byte layout above, or lift `results.rs` into a small shared crate.
Two writers to one format is how the format diverges.

---

## 4b. The log directory is now SPLIT, and `/logs` must read both

`telemetry::sink::BASENAME` is a constant, so every writer pointed at one
directory appends to one `events.ndjson`. `api` and `cli` were both resolving
`<store>/logs`, which meant a long `range-all` and a live server appending to the
same file.

`cli` now writes to **`<store>/logs/cli/`** and `api` keeps whatever it resolved.
A directory each removes the race with no lock and no coordination.

**Consequence for you:** `/logs` shows only the server's half until it walks both
`logs/` and `logs/cli/`. The CLI events are the richer ones for a backtest -- each
carries feed, symbol, rung, span, months asked/found/**missing**, bar count and
the derived `min_hits`:

```
"stored span loaded"  feed=zerodha underlying=NIFTY rung=10min
                      from=2019-12 to=2026-08
                      months_asked=81 months_found=81 months_missing=0
                      bars=63192 min_hits=12638
```

Merge them newest-first by the `ts` field; both files are NDJSON with the same
shape, and both rotate independently.

---

## 5. Routes to add

Follow the shape of `/logs` + `/logs.json` in `crates/api/src/server.rs:11424`:
one HTML page, one JSON endpoint the page fetches. `crates/api/src/audit_json.rs`
explains why the split exists — its header is worth reading before you copy it.

```
GET  /backtest        HTML page, works with no script
GET  /backtest.json   the ledger as JSON, newest first
```

If you take option B, add:

```
POST /backtest/run    body: {feed, underlying, rung|"all", from, to, min_hits}
```

**Run it in a background task and return immediately with the run identity.** A
sweep takes seconds to hours: 1-day span 43 ms, 15-minute span at 4.7% support
390 s per month. An HTTP handler that blocks on a sweep is a timeout.

---

## 6. The page — what it must show, and what it must never do

### Must show

1. **The ledger**, newest first: feed · symbol · rung · span · months found/asked
   · depth · **complete yes/NO** · trades · worst · best · identity.
2. **The best COMPLETE run**, called out — excluding halted rows.
3. **The nine rungs side by side** for one span, the way `cli range-all` prints
   them. This is the question an operator actually has: *which timeframe carries
   the edge?*
4. **`months_found` vs `months_asked`** wherever a run is shown. A span with a
   hole is a SHORTER sample, not a corrected one.

### Must never do

- Rank or crown a row with `halted == 1`.
- Show `best` (open fills) more prominently than `worst` (adverse extreme).
  **Selection ranks on `worst` everywhere in this workspace.** A page that leads
  with the flattering number proposes a different winner from every other
  surface.
- Render a missing month, an empty ledger, or a failed read as a blank. Each is a
  FACT and gets a sentence. `CLAUDE.md` §4 bans a fallback that hides a failure.
- Invent a figure the ledger does not carry.

---

## 7. Design — animated, attractive, visualised

`web/` is unrestricted by `CLAUDE.md` §2 — any framework, any toolchain. Match
`web/src/routes/db` and `web/src/routes/logs`; the design source is `web/design`.

**Motion carries meaning, and only meaning.** The console already has thirteen
keyframes; extend them only where a FACT moved. Specifically:

| Element | Motion | The fact it carries |
|---|---|---|
| A new row arriving | slide in, settle | a run just finished |
| `complete: NO` | a persistent amber rail, **not** a flash | this row is not comparable |
| The best-complete row | a steady highlight | this is the answer |
| Span coverage | a bar, months found vs asked | how much of the window is real |
| Equity / drawdown | a sparkline per row, endpoint emphasised | shape, not just total |
| A running sweep | indeterminate progress + elapsed | it is alive, duration unknown |

**Do not animate:** numbers counting up (they read as changing data), decorative
parallax, anything that moves when nothing happened. Respect
`prefers-reduced-motion`.

**Nine-rung comparison:** a small-multiples row of nine sparklines sharing one
y-axis, plus the table. One glance answers "which rung", the table answers "by
how much".

---

## 8. Rules that bind you here

- `crates/api/**` is Rust. `web/**` is unrestricted.
- **No crate may depend on the front end's toolchain.** `cargo build/test/clippy`
  must pass with no Node and no `web/` build ever run.
- Every new invariant goes in `docs/04-invariants.md` beside the test proving it.
- Every locked choice gets a `docs/05-decisions.md` entry. **Derive the D-number
  at append time** — both sessions are appending.
- 100% line and branch coverage on touched crates; no surviving mutant.
- `cargo fmt --check`, `clippy --workspace --all-targets -D warnings`,
  `cargo test --workspace --locked`, `cargo deny check` all green.

---

## 9. Verify it end to end

```bash
BRUTEX_COMMIT=$(git rev-parse HEAD) cargo build --release -p cli
./target/release/cli range-all zerodha NIFTY 2019 12 2026 8 500
./target/release/cli results zerodha NIFTY
```

Then open `/backtest` and confirm the page shows the same rows, with the same
`complete` flags, and names the same best-complete run. **If the page and the CLI
disagree, the page is wrong** — the CLI reads the ledger directly.

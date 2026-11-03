# Part FNO-CLOCK: built on the cloud, the Mac's check owed

Status: built on the cloud branch and pushed; not yet checked. Session FNO-CLOCK of the 3 Nov 2026 wave, token `FNO-CLOCK-20261103-1`.

- **Branch:** `cloud/fib-fno-clock-20261103`, cut from `origin/cloud/fib-kit-20261103b` (`fa9022bac962`): FNO-SEAM's passed tip `1ad6e24ef8e0` plus the kit folder.
- **Base for the D2 set and coverage:** `1ad6e24ef8e0`.
- **Not speculative.** FNO-CLOCK depends on FNO-SEAM alone (PARTS FNO-CLOCK, "Depends on"), so the final base is this base and no catch-up is owed (PARTS, L12 decision).
- **Decision block:** D-8220 to D-8226 used; D-8227 to D-8229 unused.
- **Ids:** invariants FNO-CLOCK-I01 to FNO-CLOCK-I14; bench C-FNO-CLOCK-01.

## What FNO-RUN and FNO-SEC-WIRE call

All in `crates/cli/src/fno_clock.rs` (`cli::fno_clock`), a library module: no verb, no dispatch arm, no `COMMANDS` entry, no telemetry (D-8226).

| Item | What it is |
|---|---|
| `Key { stock: Symbol, day: Day }` | One stock-day: the store's spelling of the stock on that IST day (`JoinMap::keys_for` gives it for a joined stock). |
| `trait BarSource: Sync { fn bars(&self, key: Key) -> Result<DayBars, String> }` | The bar seam (D-8220). `DayBars { seconds: Vec<Bar>, minutes: Vec<Bar> }`: GDFL 1s bars (raw paisa) and Zerodha 1m bars (adjusted paisa), `store::format::Bar`, each stamped at its interval's open in UTC microseconds, in time order; empty lists when the store holds none. **FNO-RUN implements it** over the store with FNO-SEC-WIRE's seconds reader and the Zerodha minute reader. An `Err` stops the build before any write. |
| `build(keys, bars, ticks, out) -> Result<Built, BuildRefusal>` | `keys` the one input (PARTS READINESS ADDENDUM item 8), deduplicated. `ticks` any `pull::gdfl_cm::CmSource`: in a run `pull::gdfl_tickstore::TickStore::new(root)`, given `with_scope` over a `CmScope::stored` naming the 13 names off the swept surface (D-8222). `out` is the folder the table goes under: pass `<shadow>/reference`. Writes `<out>/fno-clock-v1/<STOCK>.fcl`. |
| `Built` | `keys`, `written`, `unchanged`, `calibrated`, `offset_only`, `uncalibrated`, `flagged` (per flag, in `Flags::ALL` order), `tick_refusals`, `named` (first 16 `(Key, reason)`), `leftovers`, `rows`, `digest`. The verb prints it and logs it at its boundary. |
| `Table::open(out) -> Result<Table, TableRefusal>`; `Table::get(key)`; `Table::digest()` | The reader a run uses: every file read and checked whole, one hash probe a lookup. `digest()` is `clock_digest` for `FnoRunV1` (PARTS READINESS ADDENDUM item 2). A missing folder is refused, never read as "uncalibrated everywhere". |
| `ClockRow::floor() -> Floor` | `Calibrated(max(0, offset) + back_step)`, `OffsetOnly(max(0, offset))` (label "back step unmeasured"), `Uncalibrated` (every latency "uncalibrated clock"); seconds (D-8223). The run's headline uses only latencies at or above the floor. |
| `calibrate(key, &DayBars, Ticks) -> ClockRow`; `read_back_step(source, listing, stock)`; `back_step(stamps)` | The pure calibration and the back-step reader, for a caller that does not want `build`. |
| `Flags` | `NO_SESSION`, `NO_MINUTES`, `NO_SECONDS`, `NO_AGREEMENT`, `MINUTES_MALFORMED`, `SECONDS_MALFORMED`, `TIED`, `TICKS_ABSENT`, `TICKS_REFUSED`; `UNCALIBRATED` and `UNMEASURED` group them. |

**A resume** needs nothing more of this part: a list rebuild of the keys not yet written writes the same bytes a full build would (FNO-CLOCK-I08), and a rebuild of a key already written replaces its row only (FNO-CLOCK-I09).

## The design choices (D-8220 to D-8226)

- **D-8220** the table: one fixed-stride file per stock (72-byte header with the stock and a BLAKE3 digest, 48-byte rows in day order), the key list as the one input, a list rebuild byte-identical and touching no other stock's file; the bars behind `BarSource`, the ticks through any `CmSource`.
- **D-8221** the offset: compared minutes are traded continuous-trading minutes by `pull::calendar::phase_at(NseCash, .., CasEligibility::Unknown)` (so the auction band from 2026-08-03 and recorded halts are never compared); the shifted fold through a per-day sparse table; agreement counted under the session's best-supported exact ratio, a ratio counting only when prices of two minutes give it; an empty window matches nothing; ties to the larger |delta| then the positive one; ±60 s doubled until the best is inside or the reach is covered; no agreement leaves the day uncalibrated.
- **D-8222** the back step: PARTS' UNVERIFIED is answered, `pull::gdfl_tickstore` exposes raw order through `gdfl_cm::read_listed`; measured over every kept row in file order; absent or refused leaves it unmeasured, flagged, refusals named.
- **D-8223** the floor and its three readings.
- **D-8224** every outcome a flagged row; a bar-source refusal or a damaged existing file refuses the build before any write; writes through `store::staging::create`; the reader refuses strays and every broken file check.
- **D-8225** C-FNO-CLOCK-01's method.
- **D-8226** what stays FNO-RUN's.

## D2 touched set (against `1ad6e24ef8e0`)

| File | Change |
|---|---|
| `crates/cli/src/fno_clock.rs` | new |
| `crates/cli/src/lib.rs` | `pub mod fno_clock;` and its doc, under `// slot FNO-CLOCK (fno_clock)` |
| `crates/cli/benches/ratio/fno_clock.rs` | new, the thin bench wrapper |
| `crates/cli/benches/ratio.rs` | the `#[path]` module under `// slot bench-mod FNO-CLOCK`, the row under `// slot bench-row FNO-CLOCK` |
| `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `docs/07-plan.md` | rows under `<!-- slot FNO-CLOCK -->` only |
| `reports/fib-cloud-20261103/OWNER-FNO-CLOCK.md`, `report-FNO-CLOCK.md`, `part-FNO-CLOCK.md` | this session's own records |

No other line of any file changed: `git diff --stat 1ad6e24ef8e0` shows these files and the kit folder (`fa9022bac962`), which is not this part's.

## Coverage (Route B on the cloud host, advisory until the Mac's)

`nightly-2026-08-08` (`rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`), `-C instrument-coverage -Z coverage-options=branch`, opt-level 0; the module's 41 tests and one instrumented run of the bench target; `llvm-cov` over the touched set. Not speculative (no catch-up owed).

- `crates/cli/src/fno_clock.rs`: lines 2,082 of 2,082; branches 144 of 144; functions 206 of 206 (regions 3,881 of 3,889: the 8 missed are `matches!`'s non-matching arms inside passing assertions in the test module).
- `crates/cli/benches/ratio/fno_clock.rs`: lines 37 of 37; no branch; functions 3 of 3.
- `crates/cli/benches/ratio.rs` and `crates/cli/src/lib.rs`: the added `row` call runs; the rest of what this part added there is a module declaration and comments.

No `(F)` or `(U)` line is claimed. The D3 recipe is not in the cloud kit, so its own commands and its touched-set rule are the Mac's to apply.

## Benches (C-FNO-CLOCK-01, advisory)

Four runs of the final method on the cloud host (4 vCPU Xeon at 2.1 GHz, shared): p99 341 to 828 ns a step (553 to 1,338 instruction floors against a 1,600-floor budget, PARTS 11's 1 µs at a 617 ps floor); p99 ratios 1x to 10x 0.986 to 1.516 and 1x to 100x 1.053 to 1.758 against the crate's 2.5x ceiling. A first method (200 samples of 1,000 steps) breached 1x to 10x at 3.778x on scheduler noise; D-8225 and L-FNO-CLOCK-07 record both. Every run's p50, p99, max and n is in `report-FNO-CLOCK.md`.

## Gates on the cloud tip

fmt and clippy `-D warnings` on every cli target clean; the module's 41 tests green; `cargo test -p cli` green but for three lib tests that assert a file mode stops a read or a write, which root (this container's user) ignores, and which fail identically on the base; core's doc tests and every other test file of the workspace that reads the ledger documents green but store's `cited_commits`, which fails identically on the base over BASE-STACK-1028's `1a98b1e13`; gates 10b, 14, 27, 28 green; gates 10, 11, 12, 27b red with exactly the base's refusals, none in the touched set. Gate 10 re-runs after BASE-REDS-1023 lands (PARTS R2).

## Problems

- None in the part's own code. The reds above are the base's.
- `docs/02-store-format.md` has no F&O slot, so the `fno-clock-v1` row there is owed (L-FNO-CLOCK-06).
- C-FNO-CLOCK-01's margin is 1.2x on a shared host (L-FNO-CLOCK-07).
- This session once named the forbidden language in a shell command (a version probe; nothing ran, nothing tracked carries the word).

## Catch-up

None owed: the part depends on FNO-SEAM alone and was built on its passed tip.

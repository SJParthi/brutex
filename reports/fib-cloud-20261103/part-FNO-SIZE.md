# Part FNO-SIZE: built in the cloud, ready for the Mac's check 1

Status: built. Every test is green, and Route B measures 100% of lines and branches of `crates/runner/src/fno_size/`. Nothing
is merged or landed. The benches are advisory, and on the cloud VM they breach (below).

- Session FNO-SIZE of the 3 Nov 2026 wave, token `FNO-SIZE-20261103-1`, 5 Nov 2026.
- Branch `cloud/fib-fno-size-20261103`, cut from `origin/cloud/fib-kit-size-20261103` at `933113ee`.
- Base `1ad6e24ef8e0` (FNO-SEAM's passed tip). It depends on FNO-SEAM only.
- Design: `reports/fib-cloud-20261103/PARTS-FNO-SIZE.md`, whose sha256 was checked against `MANIFEST-SIZE.md`.
- Full detail is in `report-FNO-SIZE.md`.

## What it is

`runner::fno_size` holds the sizing and admission policies, setting S82 (D-8630):

| Policy | What it does |
|---|---|
| SIZE-FIXED-20-SEEDED | today's rule |
| SIZE-FIXED-20-RANKED | the learned edge, then the stop in R, liquidity, the seeded key of tie-break (e), the stock and the recipe, applied through a fixed bitonic network |
| SIZE-SPLIT-ALL | equal floor shares of the free margin and of the exposure room |
| SIZE-RISK | equal risk under an open-risk cap, both budgets learned |

`plan` takes one batch of entry intents that start in one second (D-8631). It returns, per candidate, a sized row or a refusal
by name: the rows in plan order under RANKED, in batch order otherwise. FNO-PORTFOLIO stays the owner of the caps and of S81.

The module also holds:

- the edge table, with three levels, generated buckets and leave one month out;
- the risk budget and the risk book, which takes releases in any order;
- the capital actually needed;
- the labels;
- the measuring functions of C-FNO-SIZE-01..07.

It takes every other part's figures as plain integers and calls none of their code.

## Files (one owner: FNO-SIZE)

- **New:** `crates/runner/src/fno_size/{mod,share,order,edge,risk,capital,measure,tests}.rs` and
  `crates/runner/benches/ratio/fno_size.rs`.
- **Edited at this part's seven anchors (`PARTS-FNO-SIZE.md` section 2), insertions only:**
  - `crates/runner/src/lib.rs`, after :62;
  - `crates/runner/benches/ratio.rs`, after :59 and after :589;
  - `docs/04-invariants.md`, after :9111;
  - `docs/05-decisions.md`, after :87601;
  - `docs/06-limits.md`, after :22147;
  - `docs/07-plan.md`, after :1125.
- **Ledger ids:** FNO-SIZE-I01..I12, C-FNO-SIZE-01..07, D-8630..D-8638 (D-8639 spare) and L-FNO-SIZE-01..07.

## Proof

- **Tests:** 79 of 79 pass, the 75 section 7 names plus 4 this part adds. `cargo test -p runner --locked` passes 876 of 876,
  and `cargo test -p core --locked` passes 261 of 261.
- **Coverage, Route B:** run on `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)` with branch coverage at opt-level 0.
  `crates/runner/src/fno_size/` reads 100% regions (5,215), functions (282), lines (2,934) and branches (152), `tests.rs`
  included. The bench wrapper was measured by CC-20's precedent. Everything in it ran except its own failing-check arm, at
  `crates/runner/benches/ratio/fno_size.rs:250-252` (the refusal of a row call without seven ids), which the precedent excuses.
  The `lib.rs` line holds no region.
- **fmt and clippy:** `cargo fmt --all --check` and `cargo clippy -p runner --all-targets -D warnings` are clean.
- **Gates 10, 11, 12, 14, 17, invariant-ids and decision-numbers:** identical on this branch and on the base. Every red is the
  base's own: R427-11 and STACK1028-03, HA-02 and AG05-13, gate 11's 29 refusals, gate 12's 85, and the duplicate D-6247.
  This part adds no refusal.
- **Benches, advisory (one 2.1 GHz Xeon KVM guest, three final runs):**
  - Every row is inside the 2.5x ratio ceiling. The plan costs 7-22 ns per candidate at p50, against a 300 ns budget.
  - The four fixed-work rows miss p99/p50 <= 1.2 at one or more sizes in every run.
  - C-FNO-SIZE-05 (a) misses its 20 ns estimate: p50 19.5-27.5 ns, p99 28-82 ns.
  - So the runner bench exits 1 there. No budget or bound was changed.

## Catch-up

None needed. The part depends on FNO-SEAM only, and the final base is that passed tip.

## NEEDS THE MAC

1. Check 1 by Route B on its pin, with the macOS rows, and the benches on the Mac. If the Mac's run also breaches C-FNO-SIZE's
   steadiness or the insertion budget, the main session settles section 8's budgets and form before F2 lands, because gate 8
   fails on a breached row.
2. Gate 14's cover row at F2's assembly (main session): the runner row gains C-FNO-SIZE-01..07, and its point count stays 3.
   This was verified with the gate, rc 0.
3. FNO-PORTFOLIO's slot cap, margin limit and lock reader (AMENDMENTS P1), before SIZE-SPLIT-ALL and SIZE-RISK can be walked.
4. FNO-RUN's integration tests against the real account, FNO-NET and FNO-WFT (AMENDMENTS R7), including `order::seeded`
   against the real tie-break (e).
5. The anchor cross-check against PSRB-STRAT and PSRB-RUN once they have anchors.

## The operator's question

Does a mark past the margin or exposure limit force a cut ("at every instant")? It changes no code of this part, only
`LIMITS_LABEL`'s words. Built as SPEC 11.3 reads; see `PARTS-FNO-SIZE.md` section 12.

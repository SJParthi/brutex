# Part FNO-PORTFOLIO: built, ready for its check

Status: built in the cloud build session of Tue 3 Nov 2026; not held. The full
account is in `report-FNO-PORTFOLIO.md` beside this file.

- **Branch:** `cloud/fib-fno-portfolio-20261103` (pushed). It was cut from
  `cloud/fib-kit-20261103b` at `fa9022ba`, which is `fix/fib-fno-seam` at
  `1ad6e24ef8e0` plus this folder (Markdown only).
- **Base for the D2 set and coverage:** `1ad6e24ef8e0`, FNO-SEAM's passed
  tip, the base PARTS section 4.3 gives a part that needs only slots. Not
  speculative: FNO-PORTFOLIO depends on FNO-SEAM alone.
- **Last code commit:** `985068e4`. The commits after it add only
  `report-FNO-PORTFOLIO.md` and this file.
- **Commits on the part's own line,** each with the Co-Authored-By line:
  `5ae925a5` (step 1, the module), `fe459e5d` (2, tests), `49412ba6` (3,
  coverage), `3f5e78a5` (4, the bench), `0c211722` (5, ledgers), `51c99996` (6,
  rows only under slots), `985068e4` (7, the bench figures), then the report.
  `git rev-list --no-merges 1ad6e24ef8e0..HEAD` lists these and the kit commit
  `fa9022ba`, which is the cloud branch's own base and adds no code.

## The D2 set (`git diff 1ad6e24ef8e0`)

- N `crates/runner/src/fno_portfolio.rs`
- E `crates/runner/src/lib.rs`: one line, `pub mod fno_portfolio;`, under
  `// slot FNO-PORTFOLIO (fno_portfolio)`
- N `crates/runner/benches/ratio/fno_portfolio.rs`
- E `crates/runner/benches/ratio.rs`: `#[path = "ratio/fno_portfolio.rs"]` and
  `mod fno_portfolio;` under `// slot bench-mod FNO-PORTFOLIO`;
  `fno_portfolio::row(&["C-FNO-PORT-01"]),` under
  `// slot bench-row FNO-PORTFOLIO`
- E `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md`,
  `docs/07-plan.md`: rows directly under `<!-- slot FNO-PORTFOLIO -->` only

No other line of any file is edited. No manifest and no `Cargo.lock` change.

## Coverage (Route B, measured here; the Mac re-measures)

`rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`,
`RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch'`,
`CARGO_PROFILE_DEV_OPT_LEVEL=0`, `cargo test -p runner --lib --locked
fno_portfolio`, plus the bench target run once under the same instrumentation,
the profiles merged, `llvm-cov report` and `show`:

- `crates/runner/src/fno_portfolio.rs`: lines 1,754 of 1,754 (100%),
  branches 90 of 90 (100%), functions 165 of 165. The 8 regions left are `?`
  early returns on executed lines (`exit_key`, and the bench fixture's seven).
- `crates/runner/benches/ratio/fno_portfolio.rs`: lines 88 of 91, branches 5
  of 6. The remainder is the self-checking bench's own failing-check arm, the
  one R2 excuses, named here: `row`'s
  `other => { println!(... REFUSED ...); return false; }` and the false side of
  `Ok(samples) if samples.admitted == expected`.
- `crates/runner/benches/ratio.rs`: the row call ran; the two `mod` lines hold
  no region. `crates/runner/src/lib.rs`: the `pub mod` line holds no region.

## Gates

Green here: `cargo fmt --all -- --check`;
`cargo clippy -p runner --all-targets --locked -- -D warnings`;
`cargo test -p runner --locked` (789 lib tests, 12 integration binaries, 6
doc-tests); `cargo test -p core --locked`; and cli's ten integration tests that
read `docs/05` and `docs/06`.

Gates 10, 12 and 14 (`.github/gates_bounds.rs`, `invariant_paths.rs`,
`gates_ledger.rs`, run as `ci.yml` runs them, on base and tip):

- gate 10: red on both, with the same four base rows (R427-11, STACK1028-03,
  HA-02, AG05-13);
- gate 12: red on both, with the same 85 refusals;
- gate 14: green on both.

None of their refusals is this part's. `cargo deny` is not installed here; no
dependency changed.

Gate 11, rule 4 (no sort on a production path without an allowance), refuses
this part's one sort, in `Account::schedule_second`. SPEC 11.5 orders each
second's batch, so the sort is the rule itself. Its bound is
`docs/06-limits.md` L-FNO-PORT-01: the second's own orders, never the history.
It needs `("crates/runner/src/fno_portfolio.rs", 1)` in `ALLOW_SORT` of
`.github/gates_ledger.rs`, a file no part may edit (no slot and no anchor).
**Problem for the main session.**

Gate 14's cover table: add `C-FNO-PORT-01` to the runner row at F1's family
assembly. Its ratio-point pin stays 3.

## Benches (advisory here)

C-FNO-PORT-01, four runs of the final fixture, every run exit 0:

- cycle p99 96 to 203 ns, admission 67 to 151 ns, release 28 to 61 ns;
- cycle ratios 0.92x to 1.29x against the 2.5x ceiling;
- budget 300 ns (an ESTIMATE).

Measured on a 4-vCPU cloud machine only: NOT MEASURED on the Mac.

## Decisions drafted (D-8180 to D-8189; the part's block)

- D-8180: the account and its check order. A released position stays in force
  through its last fill second. Each window starts at Rs 20 lakh. The bench
  judges its ratio on the cycle.
- D-8181: sliding rings; the 7,000 is a session's count; only a replacement is
  an order.
- D-8182: one total order per second.
- D-8183: exact margin with a 20% floor; the MTM base; the capital bound.
- D-8184 to D-8189 are unused.

## Problems and findings (detail in the report, section 8)

1. Gate 11 rule 4 needs one `ALLOW_SORT` entry (above).
2. Gate 14's cover table needs `C-FNO-PORT-01` (main session, F1 assembly).
3. The Dhan order-rate figures (T09-07) are UNVERIFIED until FNO-LAW records
   them in `docs/00-charter.md`.
4. Open: whether a modification counts in Dhan's per-second budget. SPEC 9.7
   is silent; this part counts the replacement only.
5. READINESS item 5's row-level halves are FNO-RUN's; this part gives
   `CapitalBound`.
6. The callers' contracts: stock ids in universe-file order; re-offer a queued
   order at the second named; pass the largest `M`; the broker's square-off
   orders stay outside the account; FNO-SEC-EXEC reads `Account::margin(slot)`.

7. The checker's first proof (PARTS L12: `git rev-list --no-merges <final
   base>..<tip>` lists only this part's own commits) will also list `fa9022ba`,
   the cloud kit, which this branch was cut from as CLOUD.md says. It adds only
   Markdown under `reports/fib-cloud-20261103/`. The main session decides
   whether to land from a branch without it: the part's own commits touch
   nothing under `reports/` except its two Markdown files.

`refused_files`: none. No harness write was refused.

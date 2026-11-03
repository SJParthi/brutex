# Part FNO-LEVERAGE: built (cloud session, 3 Nov 2026), awaiting the Mac's independent check

Status: built. Every gate this part owns is green; the part's own code is at 100% line and branch coverage by Route B. Two gates and one docs test are red on the base itself, unchanged by this part (gate 1d, gate 27b, `cited_commits`; details below). Not landed.

- **Branch:** `cloud/fib-fno-leverage-20261103` (pushed). It is cut from `origin/cloud/fib-kit-20261103b`, which is `fix/fib-fno-seam` at `1ad6e24ef8e0` plus the kit folder.
- **Base for the D2 touched set and coverage:** `1ad6e24ef8e0` (R2 "BRANCH BASES": the part's own base).
- **Commits:** steps 1 to 6 (`ca51c6c5` … `f6ae8972`), then the reports. Each commit message ends with the `Co-Authored-By` line.
- **No:** merge, pull request, other branch, store, `:8080`, network I/O in the code, credential, or Py-language.

## D2 touched set (against `1ad6e24ef8e0`)

```
.github/gates_tree.rs                   |   14 +-
crates/pull/benches/ratio.rs            |   10 +
crates/pull/benches/ratio/nse_margin.rs |  161 ++
crates/pull/src/lib.rs                  |    5 +
crates/pull/src/nse_margin.rs           | 2629 +++
docs/04-invariants.md                   |   15 +
docs/05-decisions.md                    |  110 +
docs/06-limits.md                       |    8 +
docs/07-plan.md                         |    3 +
```

Anchors used:
- **`lib.rs`:** right after `pub mod nse;` (PARTS ADDENDUM 1 Nov, :1323).
- **`benches/ratio.rs`:** the `mod` right after the blank line after `use std::time::Instant;`; the `println!` and `ok &=` lines right after `ok &= day_table::day_lookup_is_flat();` (:1324-1325).
- **`gates_tree.rs`:** READINESS ADDENDUM item 12. The group goes after the blank line after `LATE_LABEL`, `FNO_LEVERAGE_LITERAL,` after `NAV_WORD,`, and the length line becomes exactly `const DECLARED: &[&str] = &[`.
- **docs:** directly under `<!-- slot FNO-LEVERAGE -->` in 04, 05, 06 and 07, with no other line edited.

## Coverage (Route B): measured

- **Toolchain:** `nightly-2026-08-08`, which reports `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`: the version named at `docs/06-limits.md`:22131.
- **Flags:** `RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' CARGO_PROFILE_DEV_OPT_LEVEL=0`.
- **Run:** the lib test binary was run with filter `nse_margin` (32 tests, 2,226 filtered out). No other module calls the file.
- **Report:** `llvm-profdata merge -sparse`, then `llvm-cov report --show-branch-summary crates/pull/src/nse_margin.rs` (LLVM 23.1.0-rust-1.99.0-nightly).

`crates/pull/src/nse_margin.rs`, the whole file:

| Measure | Covered | Missed | Cover |
|---|---|---|---|
| Lines | 1,669 | 0 | **100.00%** |
| Branches | 78 | 0 | **100.00%** |
| Regions | 2,852 | 0 | 100.00% |
| Functions | 156 | 0 | 100.00% |

The other touched lines:
- The `lib.rs` line has no instrumented region.
- The bench wrapper `crates/pull/benches/ratio/nse_margin.rs` was measured by running the bench target once under the same instrumentation, at the dev profile's opt-level 3 (CC-20's precedent; the run exited 0 with all ratios within the ceiling):
  - lines: 93 of 94 per source line (98 of 106 in `llvm-cov report`, which counts the unexecuted `refuse` closures apart);
  - branches: 4 of 8.
- Everything missed is the self-checking failing-check arm, which R2 excuses, named for this file:
  - line 118 `refuse(…)`;
  - its three `||` guards at 114-116;
  - the `OVER BUDGET` outcome at 100.
- The harness's `println!` (1864) and `ok &=` (1867) lines each have count 1.

## Gates (commands run here, with output)

- `cargo fmt --check -p pull`: clean.
- `cargo clippy -p pull --all-targets --locked -- -D warnings`: clean (lib, tests, benches).
- `cargo test -p pull --locked --no-fail-fast`: 2,734 passed, 9 failed, 54 ignored, across 32 binaries; exit 101.
  - The 9 are the lib's root-only permission tests (this container runs as uid 0).
  - All 9 pass as `nobody`: 9 passed, 0 failed. None is this part's.
  - All 32 of this part's tests pass.
- **Gate 1d:**
  - The base is red on its own 481 words (measured on a worktree of `1ad6e24ef8e0`).
  - This tree's refused list is identical, so no word of `nse_margin.rs` or its bench stays on it.
  - Over the two new files alone it prints "OK — every segment-shaped literal under crates/pull is declared".
  - `gates_tree.rs --test`: 50 passed.
- **Gate 28:** `ok crates/pull/src/nse_margin.rs: 1 temporary name(s), 1 call(s) of store::staging::create`; overall OK.
- **Gate 27** (invariant ids): OK, 3,492 rows unique.
- **Gate 27b:** red on D-6247 only. D-6247 has 2 headings on the base; D-8150..D-8155 head one entry each.
- **`core` docs tests:** `cost_invariants` 5 passed, `decision_citations` 4 passed.
- **`store` docs test** `cited_commits`: 5 passed, 1 failed.
  - The failure is `1a98b1e13`, at the base's `docs/06-limits.md`:22132 (the rustc nightly's own commit).
  - This part adds no citation.
- **Gate 10's crate-first half:** not run, because the base row R427-11 is refused (R2 note (c)). Re-run it after BASE-REDS-1023.

## Decisions, invariants, limits

- **Decisions:** D-8150 to D-8155 (block D-8150..D-8159; D-8156..D-8159 unused).
- **Invariants:** FNO-LEV-I01..I12 and C-FNO-LEV-01.
- **Limits:** L-FNO-LEV-1..6.
- **Plan:** one row in `docs/07-plan.md`.

## Open items for the Mac (operations, not code)

1. The `C_VAR1_DDMMYYYY_1.DAT` download, every session from 3 Sep 2018 plus today's, into a shadow reference folder. The operator's yes is recorded at PARTS decision 9.3.
2. Before `cli fno margin build`, read one real file from each side of 1 Jun 2020 against D-8153's positions:
   - symbol column 2, series column 3, margins in columns 7, 8 and 9;
   - a header line's third column is not `EQ`.
3. FNO-RUN wires `cli fno margin build --src DIR --out DIR` and names the store directory.
4. The main session may copy D-8151's format into `docs/02-store-format.md`, which has no FNO slot.

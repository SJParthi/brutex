# PSRB search kit (6 Nov 2026)

This branch (`cloud/psrb-search-kit-20261106`) is a code-only kit for a cloud
session whose job is to speed up the PSRB preview search engine.

It contains:

- `crates/psrb_preview`: the PSRB preview binary (one fixed backtest over the
  F&O stocks).
- `crates/psrb_search`: the PSRB preview search binary (the search over the
  F&O stocks: phase A, accounts, filters, NIFTY context, merge, spool, report).
- The two member lines in the root `Cargo.toml` and the matching `Cargo.lock`.
- `crates/indicators/src/trend.rs`: one new read-only accessor `Ema::scaled()`
  that the search uses for exact above/below tests against the average.

The parent commit is the scratch tree's detached HEAD a6ba0d5d400c81373dbff055cc7173ea50b172c8:
`origin/feat/db-fix` (f01d7a0e430e8f9b01e424a299b0bed20738a0ef) with the FNO-COSTS
repair and the PSRB indicator parts (RSI, BOLL, STV, WILLR, CCI, the seam slots)
merged in. Against `origin/feat/db-fix` the kit therefore also carries those
crates/costs, crates/indicators, crates/cli, crates/runner and docs/04-07 changes.

It contains NO market data: no tick, candle or price files, no store files,
no run outputs, nothing under `state/`. The binaries read their inputs from a
local store that is not part of this repository, so a cloud session can build,
test and profile the code (`cargo build -p psrb_search`, `cargo test -p
psrb_search`) but cannot run a real search here. Speed changes must keep the
output byte-identical; the main session checks that on the Mac against the
real store.

These crates are scratch preview code, not the PSRB result.

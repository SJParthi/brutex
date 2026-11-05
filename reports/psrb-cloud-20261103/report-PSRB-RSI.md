STATUS: IN PROGRESS

# PSRB-RSI cloud report (token PSRB-RSI-20261103-1)

Branch `cloud/psrb-rsi-20261103`, cut from `origin/cloud/psrb-kit-20261103` (`56f3043dd4ca`).
START = `git rev-parse origin/cloud/psrb-kit-20261103~1` = `35234cead4009299a93fcd79a3d1fffd773dcc16` (PSRB-SEAM, parent
`1ad6e24ef8e0`).

## Kit check

- `git diff --name-only $START origin/cloud/psrb-kit-20261103` lists only the 13 files under `reports/psrb-cloud-20261103/`.
- `sha256sum MANIFEST.md` = `85b873ebc2a7cc86713701cbc7619bb3b14058b99225e4249fdbfb403c889ad8`, the figure in the kit
  commit's subject.
- Every MANIFEST row matches: `CLOUD.md` `19dff357...`, `PARTS.md` `1a96fc9b...` (whole files); `d3-recipe.txt`
  `9ed425e3...`, `d3-route-b.sh` `f3d70ac2...`, `d3-route-b-cloud.sh` `f032b525...`, `d3-covset.mjs` `ecef3bdc...`,
  `sim.mjs` `c78e72b8...`, `fixtures.mjs` `e4818033...`, `fix2.mjs` `d03d3ceb...`, `rsi-run.mjs` `a99a83ad...`,
  `min-run.mjs` `3c18cb3c...`, `faithful-edge.mjs` `2ed69004...` (each extracted with MANIFEST.md's own `awk` line).

## START baseline (before the first code commit)

The code tree at the OWNER commit equals START outside `reports/` (`git diff --stat $START HEAD -- . ':(exclude)reports'`
prints nothing). On the pinned stable `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo test -p indicators --locked`:
22 test binaries, 504 passed, 0 failed, 0 ignored, exit 0 (02:30:54-02:31:09 UTC, 5 Nov 2026). So this host has no
indicators red to carry.

## Toolchains

- Stable: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)` (rust-toolchain.toml's pin).
- Coverage nightly: installed as `nightly-2026-08-08`; `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`;
  its own `llvm-cov --version` and `llvm-profdata --version` print `LLVM version 23.1.0-rust-1.99.0-nightly`.
- Machine: Linux x86_64 cloud container, 4 CPUs, 15 GiB memory.

NEXT: write `crates/indicators/src/rsi.rs`, `src/rsi_tests.rs`, `benches/ratio/psrb_rsi.rs`, the `lib.rs` line, the two
bench-slot insertions and the four ledger blocks; then the gates, Route B and the bench runs.

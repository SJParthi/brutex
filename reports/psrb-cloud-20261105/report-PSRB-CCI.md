STATUS: IN PROGRESS

# PSRB-CCI cloud report (token PSRB-CCI-20261105-1)

Branch `cloud/psrb-cci-20261105`, cut from `origin/cloud/psrb-kit-20261105` (`fdc091ba69c9`).
START = `git rev-parse origin/cloud/psrb-kit-20261105~1` = `deaa0f5c0606464f2ec79dc9e94de03792ece326` (PSRB-SEAM-2,
parent PSRB-SEAM `35234cead400`, its parent `1ad6e24ef8e0`).

## Kit check

- `git diff --name-only $START origin/cloud/psrb-kit-20261105` lists only the 13 files under
  `reports/psrb-cloud-20261105/`.
- Every MANIFEST row matches (each fenced file extracted with MANIFEST.md's own `awk` line): `CLOUD.md` `004a7f9b...`,
  `PARTS.md` `edcfdb27...`, `PARTS-WILLR-CCI.md` `0c502de2...` (whole files); `d3-recipe.txt` `9ed425e3...`,
  `d3-route-b.sh` `f3d70ac2...`, `d3-route-b-cloud.sh` `f032b525...`, `d3-covset.mjs` `ecef3bdc...`, `wr-cci.mjs`
  `ea3d831f...`, `pops.mjs` `261a5697...`, `cpr-wr6.mjs` `0daa7c91...`, `repair1.mjs` `a2f7bd48...`, `repair3.mjs`
  `d85ca768...` (fenced). `sha256sum MANIFEST.md` = `2ad1450d...`.
- Decision ids: `grep -n -E '^#{2,4} +D-86(4|5)[0-9]' docs/05-decisions.md` prints nothing (exit 1): no clash with
  D-8650..D-8659. No `D-865x` appears anywhere outside `reports/`.
- The four ledger anchors hold at START: docs/04 :8671, docs/05 :87133, docs/06 :21624 and docs/07 :929 are the
  headings PARTS-WILLR-CCI.md 3.4 names, each with a blank line above it.

## START baseline (before the first code commit)

The code tree at the OWNER commit equals START outside `reports/` (`git diff --stat $START HEAD -- . ':(exclude)reports'`
printed nothing). In a detached worktree at START, on the pinned stable `rustc 1.97.1 (8bab26f4f 2026-07-14)`,
`cargo test -p indicators --locked`: 22 test binaries, 504 passed, 0 failed, 0 ignored, exit 0 (5 Nov 2026, ~13:50 UTC).
This host has no indicators red at START.

## Toolchains and machine

- Stable: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)` (rust-toolchain.toml's pin).
- Coverage nightly: `rustup toolchain install nightly-2026-08-08 --profile minimal --component llvm-tools-preview`;
  `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`; its own `llvm-cov --version` and
  `llvm-profdata --version` print `LLVM version 23.1.0-rust-1.99.0-nightly`. No `RUSTC_BOOTSTRAP`.
- Machine: Linux 6.18.44 x86_64, 4 vCPU `Intel(R) Xeon(R) Processor @ 2.10GHz`, 15 GiB memory, running as root;
  TMPDIR on a tmpfs (`/dev/shm`); one CARGO_TARGET_DIR per toolchain.

NEXT: write `crates/indicators/src/cci.rs`, `src/cci_tests.rs`, `benches/ratio/psrb_cci.rs`, the `lib.rs` line and the
two `ratio.rs` insertions; then the four ledger blocks; then the gates, the red checks, Route B and the benches.

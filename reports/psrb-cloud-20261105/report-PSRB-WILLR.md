STATUS: IN PROGRESS

# PSRB-WILLR: Williams %R in exact integers (`indicators::williams_r`)

Session PSRB-WILLR of the 5 Nov 2026 PSRB second wave, TOKEN `PSRB-WILLR-20261105-1`, branch
`cloud/psrb-willr-20261105` cut from `origin/cloud/psrb-kit-20261105` at `fdc091ba69c9b632e4cd80388ff8b6d5ee681682`.

## Kit check

- START = `git rev-parse origin/cloud/psrb-kit-20261105~1` = `deaa0f5c0606464f2ec79dc9e94de03792ece326` (PSRB-SEAM-2),
  whose parent is PSRB-SEAM `35234cead4009299a93fcd79a3d1fffd773dcc16`, whose parent is
  `1ad6e24ef8e05e8836e511873e8352071dd98fa0`: the chain the brief names.
- `git diff --name-only $START origin/cloud/psrb-kit-20261105` lists only the 13 files of
  `reports/psrb-cloud-20261105/`.
- Every MANIFEST row checked: `CLOUD.md`, `PARTS.md` and `PARTS-WILLR-CCI.md` hashed whole, the nine fenced files
  extracted with MANIFEST's own `awk` line and hashed; all 12 sha256 equal the MANIFEST column.
- Decision-id clash grep on docs/05 at START: `grep -nE '^#{2,4} +D-864[0-9]'` and `grep -nE '^#{2,4} +D-865[0-9]'`
  print nothing (PARTS-WILLR-CCI.md 2.1).
- The four ledger anchors of PARTS-WILLR-CCI.md 3.4 sit at the stated lines at START: docs/04 :9038, docs/05 :86873,
  docs/06 :22007, docs/07 :1084 (`grep -n` of each heading).

## START baseline (before the first code commit)

The tree at the OWNER commit holds START's code exactly (`git diff --stat $START HEAD -- . ':(exclude)reports'` prints
nothing), so this run is START's.

    CARGO_TARGET_DIR=/home/user/tgt/stable TMPDIR=/dev/shm/tmp-willr cargo test -p indicators --locked

rustc 1.97.1 (8bab26f4f 2026-07-14): EXIT 0; 504 tests passed, 0 failed, 0 ignored (393 unit, the integration
binaries, 2 doc tests). No host red at START.

## Toolchains

- Stable: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)` (the repository's pin,
  `rust-toolchain.toml`).
- Nightly for Route B: installed in this session with
  `rustup toolchain install nightly-2026-08-08 --profile minimal --component llvm-tools-preview --no-self-update`;
  `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`; `llvm-cov --version` and
  `llvm-profdata --version` print `LLVM version 23.1.0-rust-1.99.0-nightly`: the pins of CLOUD.md rule 6.

## Machine

Linux x86_64 cloud container, 4 vCPU, 15 GiB RAM; `CARGO_TARGET_DIR` one per toolchain (`/home/user/tgt/stable`,
`/home/user/tgt/nightly-cov`), `TMPDIR=/dev/shm/tmp-willr` (tmpfs).

NEXT: write `crates/indicators/src/williams_r.rs`, its tests, the bench wrapper and the slot lines.

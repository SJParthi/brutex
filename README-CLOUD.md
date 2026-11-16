# README-CLOUD: engine v3.4, code only, for data-free cloud jobs

Branch `cloud/spikefade-v34int-20261116`, one orphan commit. **CODE ONLY.** The repository is public: every push publishes.

Labels: MEASURED = a number from a named run on the operator's Mac or on a cloud box. ESTIMATE = measured inputs times a
model. EXACT = a count. No figure here is a trading result.

## 1. What this branch is

This is the spike-selling engine `spikefade` v3.4: round D's `integrate:D` final source, recorded on 16 Nov 2026.
- That source's manifest (integrate:D's own method) has 236 files and sha256
  `5f1de014ff57431e2884c279dc70fe278d50dda07c75e84817c8cc7f6ebabeed`.
- v3.4 = v3.3 final, plus:
  - build:D-high's v3.4 items;
  - the shard kit;
  - data:F3's dated stock freeze reader;
  - data:F7's bands and protection model, through the fill crate v2.6 at `pinned/common-fill-v26`.
- It is not round D's fixed tree. The 5 fixers of fix:D work in their own copies.

**Two jobs use it:**

| job | file | boxes | results |
|---|---|---|---|
| J1 | `JOB-LINUX-TESTS.md`: the engine's test suite on Linux, against the Mac's list `MAC-TESTS.txt` | 1 | `cloud-results/v34-tests-0-of-1` |
| J2 | `JOB-MUTANTS.md`: a full mutation run of v3.4 | 30 | `cloud-results/v34-mutants-<k>-of-30` |

**Provenance of the 151 code files (EXACT, checked file by file before the push):**
- **139** are byte-identical to `scratch/spikefade-v34` at the same path.
- **2** changed by one line each:
  - `Cargo.toml`: `members` adds `"crates/greeks"`;
  - `crates/spikefade/Cargo.toml`: `greeks = { path = "../greeks" }` in place of the Mac's absolute path to the app's crate.

  These are the edits the first cloud branch made. The root `Cargo.toml` equals that branch's byte for byte.
- **10** (`crates/greeks/**`) are the app's own `greeks` crate, byte for byte. It is the tree the Mac's v3.4 build links.
- `crates/core`, `crates/costs` and `crates/greeks` are byte-identical to the first cloud branch
  `cloud/spikefade-shard-20261116` (commit `15fd4f47`). `crates/spikefade` and the fill crate are v3.4's.

**Dependencies:**
- There is no crates.io dependency. `Cargo.lock` lists path crates only: core, costs, common_fill, greeks and spikefade.
  `--offline` works.
- `pinned/` is excluded from the workspace. The fill crate is a path dependency, not a member.

## 2. What is NOT here, and must never be added

Only `.rs`, `.toml`, `.md` and `Cargo.lock` files are allowed, plus `MAC-TESTS.txt`, which holds test names only.

**Left out on purpose:**
- **`crates/spikefade/data/`:** the exit-rule parameter tables `EXITS-DYN-MENU-v1.tsv`, `EXITS-DYN-SETS-v1.tsv` and
  `EXITS-DYN-SETS-v33-1D.tsv`, and their generator.
  - The tables hold no market data. The build does not need them.
  - 7 tests read them.
- **data:F3's and data:F7's tables** (the NSE freeze table, the protection readings, the session-to-contract-file manifest)
  and the NSE contract files.
  - 3 tests of `tests34i.rs` read them at a Mac path.
- **The fill crate's `vectors/`, `matrix/`, `SHA256SUMS*` and `CHANGES-*`.**
  - Only that crate's own tests read them. Those tests are not part of these jobs.
- **The older fill crates** v2.1, v2.1-fa1, v2.2, v2.3, v2.4 and v2.5. None of them is a dependency.
- **`keep/`** (old snapshots) and **every target directory**.

**Never commit any of these:**
- market data;
- tables, plans and inputs manifests;
- batch files, checkpoints and run results (job results go only to the jobs' own `cloud-results/*` branches);
- tickets, signed links and credentials;
- anything under `runtime/`.

## 3. Toolchain

- `rust-toolchain.toml` pins **1.97.1**: components rustfmt, clippy and llvm-tools-preview, target wasm32-unknown-unknown,
  profile minimal. rustup installs it on first use, which needs the network.
- Expected `rustc --version`: `rustc 1.97.1 (8bab26f4f 2026-07-14)`.

## 4. Verify the checkout

Run this from the branch root. The code manifest covers the 151 code files, and none of the job or readme files.

```sh
LC_ALL=C find Cargo.toml Cargo.lock rust-toolchain.toml crates pinned -type f ! -path '*/target/*' \
  | LC_ALL=C sort | xargs sha256sum > ../SOURCE-SHA256-v34int.txt
wc -l < ../SOURCE-SHA256-v34int.txt                 # must print 151
sha256sum ../SOURCE-SHA256-v34int.txt | cut -c1-64  # must print 80fa426f2370a97377d5d35600db260af36b30f488526920f6e539cc84a7a34a
```

## 5. The Mac's reference test run (MEASURED, integrate:D, 16 Nov 2026 17:58 IST)

**The command** (log `v34/integrate/build/t11.log`), run on the final source `5f1de014...`:

```
cargo test --release --offline -p spikefade -- --test-threads 4
```

- **Result:** 216 passed, 0 failed, 1 ignored, finished in 2.55 s.
- **The 216 passed** are v3.3's 145, build:D-high's 28 (`tests34.rs`), build:shard's 22 (`tests34s.rs`) and integrate:D's 21
  (`tests34i.rs`).
- **The ignored test** is v3.3's measurement test `split18::tests::v32b_split_cost_97_months`.
- `MAC-TESTS.txt` holds the 217 names, sorted.

## 6. Known on Linux before any box ran

- **zstd path.** With PORTABLE off, `kit.rs` line 470 calls zstd at the Mac's `/opt/homebrew/bin/zstd`. So two kit tests
  that run with every switch off fail on a box without that path:
  - `tests34s::root_digest_same_for_every_form`
  - `tests34s::sfz_roundtrip_all_rows`

  This is the cause of the first wave's Linux C2 "DIFFER". It is fixed in a later tree (the determinism fix), not in v3.4.
- **Tools at fixed paths.** A box needs these present:
  - with PORTABLE off: `/usr/bin/shasum`, `/usr/bin/unzip`, `/usr/bin/gzip`, `/bin/ps` and `/bin/hostname`;
  - always: `/bin/date`, `/bin/ps` (the memory guard) and `/usr/bin/unzip` (one bhav path).
- **sysctl.** The memory guard's swap reading uses `/usr/sbin/sysctl -n vm.swapusage`, a macOS key. On Linux that reading
  is 0.

## 7. Not proven

- No build of this exact tree has been made on the Mac. On 16 Nov the WD_BLACK disk was below the build floor.
  - Its code is v3.4's byte for byte, except the two one-line path edits of section 1.
- No Linux build of v3.4 has been made. J1's box is the first.

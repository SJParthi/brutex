# README-CLOUD: build and run a spike-selling shard on a cloud box

Branch `cloud/spikefade-shard-20261116`. **CODE ONLY.** The repository is public: every push publishes.

Labels: MEASURED = a number from a named run on the operator's Mac. ESTIMATE = measured inputs times a model. EXACT = a count.
No figure here is a trading result.

## 1. What this branch is

- The spike-selling engine `spikefade`: v3.3 final plus the v3.4 **shard kit** (round D, build:shard, Addendum 30.3).
  The kit adds shard mode (`--shard on`), an explicit batch plan, per-batch files, `ckpt-merge`, segment slices, transport
  files and synthetic days.
  - Every kit switch is OFF by default. With all of them off the binary replays v3.3 final byte for byte (proof P-S0).
- **Source.** This is build:shard's final source (manifest sha256 `74b675db...`, 148 files) minus its 12 non-code files.
  The app's `greeks` crate is copied in as a workspace member (`crates/greeks`), and the engine points at `../greeks`.
  - Each of the 146 files is byte-identical to the file at the same path in the proven code-only kit tree (binary kit2).
  - No manifest holds an absolute path.
  - There is no crates.io dependency. `Cargo.lock` lists path crates only: core, costs, common_fill, greeks, spikefade.
- **Not in this binary:** the v3.4 round-D engine switches (`--fix34`, halts, ELM pre-AEL, stamp, STRICTSPAN, bands,
  the F3 freeze table). They belong to a separate build.

## 2. What is NOT here, and must never be added

Only `.rs`, `.toml`, `.md` and `Cargo.lock` files are allowed.

Left out on purpose:
- **`crates/spikefade/data/`:** `EXITS-DYN-MENU-v1.tsv`, `EXITS-DYN-SETS-v1.tsv`, `EXITS-DYN-SETS-v33-1D.tsv` and their
  generator script.
  - These are exit-rule parameter tables and hold NO market data.
  - The build does not need them. A run takes them through `--menu` and `--sets`.
  - 7 tests read them (section 5).
- **`pinned/common-fill-v24/vectors/` and `matrix/`:** test vectors, a test matrix and their generators. One vectors file
  is a calibration table taken from broker margin screens. Only that crate's own tests read them, and the build does not.
  Its `SHA256SUMS*` files and its CHANGES note are left out too.
- **`pinned/common-fill-v21`, `-v21-fa1`, `-v22` and `-v23`:** older copies. None is a dependency.
- **`keep/`:** old source snapshots.
- **Every target directory.**

Never commit any of these:
- market data: cubes, slices, transport files;
- tables, plans and inputs manifests;
- batch files, checkpoints and results;
- tickets, presigned links and credentials;
- anything under `runtime/`.

Before any commit or push from a box, refuse if any tracked or staged file holds a presigned-link signature. This command
must print nothing (the marker is split in two so that this file does not match it):

```sh
git grep -I -l --cached -e "X-Amz-""Signature"; git grep -I -l -e "X-Amz-""Signature"
```

## 3. Toolchain

- `rust-toolchain.toml` pins **1.97.1**: components rustfmt, clippy and llvm-tools-preview, target wasm32-unknown-unknown,
  profile minimal.
  - rustup installs it on first use, which needs the network.
- Expected `rustc --version`: `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
  - Every shard of one run must report the SAME string.
  - `ckpt-merge` refuses a batch file whose provenance differs in source, rustc, inputs or plan.
- **Tools on PATH:** `zstd` and `unzip` (also `gzip`, `ps`, `hostname`), or pass `--tool-<t> PATH` or `SF_<T>`.
  - The start probes check that zstd and unzip answer and that the sha256 self-test passes.

## 4. Verify the checkout, then build

```sh
git clone --branch cloud/spikefade-shard-20261116 --single-branch --depth 1 https://github.com/SJParthi/brutex.git sf
cd sf
# the source manifest: every file but .git, target, the engine's data folder and this README
LC_ALL=C find . -type f ! -path './.git/*' ! -path './target/*' ! -path './crates/spikefade/data/*' ! -name README-CLOUD.md \
  | LC_ALL=C sort | xargs sha256sum > ../SOURCE-SHA256-cloud-branch.txt
wc -l < ../SOURCE-SHA256-cloud-branch.txt          # must print 146
SRC=$(sha256sum ../SOURCE-SHA256-cloud-branch.txt | cut -c1-64)
[ "$SRC" = c3722928015fb27164c88ad687f64a7529c4c003dc8bc14825c82477984fbf8e ] || { echo "checkout differs: STOP"; exit 1; }
SF_SOURCE_SHA256=$SRC SF_RUSTC="$(rustc --version)" nice -n 10 cargo build --release --locked --offline -p spikefade
sha256sum target/release/spikefade                 # report it; it is not expected to equal the Mac's
```

- **`--offline` works** because `Cargo.lock` holds only path crates.
- **`SF_SOURCE_SHA256` and `SF_RUSTC` are baked into every batch file's provenance** (`PROV source=... rustc=...`).
  - The Mac's merge checks the source with `--expect-source c3722928...`.
  - Left unset, the binary bakes `unset`, and the merge refuses its files.
- **The binary differs between machines** (paths, target). The batch BODIES must not differ, and they are what every proof
  compares.

## 5. C1 (data-free): the box, then the engine's test suite

```sh
uname -a; nproc; grep MemTotal /proc/meminfo; cat /sys/fs/cgroup/memory.max 2>/dev/null; df -P -BG .
grep -m1 'model name' /proc/cpuinfo; rustc -V
```

The test suite covers the engine only. The other crates' own tests need files of the app repository, so they are not run
here.

```sh
# Copy these 3 parameter tables from the package into crates/spikefade/data/ (never git add them). Expected sha256:
#   EXITS-DYN-MENU-v1.tsv      bd01157aae8d244a83f488bfec823b9a04c9fd94d4fbe58128d28530d3b65fc8
#   EXITS-DYN-SETS-v1.tsv      279150b2308d14604fb3f5b94a1903efa93bc2d01283956b12112e1e8dce3013
#   EXITS-DYN-SETS-v33-1D.tsv  5ead880f47bfa5c04c5f4ae1275a799d209ccec811c288236c5def46fc54612b
SF_SOURCE_SHA256=$SRC SF_RUSTC="$(rustc --version)" cargo test --release --locked --offline -p spikefade
```

- **Expected (MEASURED on the Mac by build:shard, on this source):** 167 passed, 0 failed, 1 ignored. The ignored one is a
  measurement test.
- **Without the 3 tables, the 7 tests that read them fail** (read from the code, not run):
  - 5 in `menu.rs`;
  - `sf3_ttp_replaces_tp_close` in `tests3.rs`;
  - `v33_search1d_sets_load_and_count` in `tests33s.rs`.

## 6. C2 (data-free): the synthetic canary

Run this before any market data moves.

**Inputs besides the binary**, all from the package and none of them market data:
- the public NSE tables `C_VAR1_30102026_1.DAT`, `nse-holidays.tsv`, `nse-holidays-2020-2025.tsv` and
  `SPECIAL-SESSIONS-NSE.tsv`;
- the menu and the v1 sets above;
- the Mac's synthetic plan `PLAN-syn.tsv` (sha256 `d73fdf30ed716da41ca2f02f8443d4a1fc3bc3bc77c716687e08751838b69ec2`).

**Use that plan file. Do not write a new one:** the plan's sha256 is part of every batch body.

Run it OUTSIDE the checkout, so nothing it writes can be committed or enter the source manifest:

```sh
B=$PWD/target/release/spikefade; T=<absolute path of the folder holding the inputs above>
mkdir -p ../c2 && cd ../c2 && mkdir -p syn
$B synth syn/main syn/extra --seed 20261116 2023-03-28 2023-03-29 | tail -1
#   expected: SYNTHETIC SET sha256 (over every file's sha256 in order) 7712d2a26cfa59b49061db1eebe72edf14f25c7c75029d76fff48162df46b452
$B cube-slice syn/main syn/transport/main idx --transport 2023-03-28 2023-03-29
$B cube-slice syn/extra syn/transport/extra idx --transport 2023-03-28 2023-03-29
A="--var $T/C_VAR1_30102026_1.DAT --holidays $T/nse-holidays.tsv --holidays2 $T/nse-holidays-2020-2025.tsv \
 --menu $T/EXITS-DYN-MENU-v1.tsv --sets $T/EXITS-DYN-SETS-v1.tsv --set S0 --liq 0,7 --shares 10,25 --cut v3 --segs idx \
 --day-major --v33 --sess-cal dated --special-sessions $T/SPECIAL-SESSIONS-NSE.tsv --batch-accounts 100000"
$B run3 syn/transport/main syn/sh 2023-03-28 2023-03-29 --cube-extra syn/transport/extra $A --threads 4 \
 --batch-plan $T/PLAN-syn.tsv --shard on --batch-set 0-3 --checkpoint syn/sh-ck --cube-transport
$B bck-check syn/sh-ck idx | awk -F'\t' '$1=="batch"{print $2"\t"$4}'
```

**Expected batch bodies.** These are the Mac's reference values from binary plain2. The Mac build of this branch gives
the same when it runs this section verbatim (section 9):

| batch | body sha256 |
|---|---|
| 0 | `6d11c2ea0e3414c870295c4b275deaabacd918f18c2c0f824201c475a954d3c3` |
| 1 | `89f6bc992d9047cb2ddf2e3abb38d08462956ce8c98915f3684a459ff5c87677` |
| 2 | `1aa6fc820b7ca103d7cc0c8f211d4851626e50b84ac93d223ab14fd466525ab8` |
| 3 | `d43bedb8916aba25d7bf38f0a295fb4d4bd93022c81a9122c3068486d0c4f663` |

If all 4 are equal, the box computes exactly as the Mac. If any differs, STOP and report it: no real shard may run.

## 7. A real shard (only after the operator's yes and the upload)

The package arrives through the session's ticket, never through this repository. It gives:
- the transport roots (main and extra);
- the tables;
- `PLAN-idx.tsv`;
- the expected `idx.inputs.tsv`;
- the EXACT run arguments the plan was written with;
- this shard's batch set.

`<OUT>` must be outside the checkout.

```sh
nice -n 10 ./target/release/spikefade run3 <PKG>/main <OUT>/sh<K> <the run's days> \
  --cube-extra <PKG>/extra --margin-extra <PKG>/margin-extra.tsv \
  --margin-manifest <PKG>/margin-manifest.tsv --margin-futures <PKG>/futures-all.tsv \
  --special-sessions <...> --freeze <...> --freeze <...>@2026-11-02 --ban-dir <...> --ban-dir <...> \
  --bhav-dir <...> --bhav-dir <...> --tbill <...> --var <...> --holidays <...> --holidays2 <...> \
  --menu <...> --sets <...> <the run's switches, exactly as shipped> \
  --threads <T> --batch-plan <PKG>/PLAN-idx.tsv --shard on --batch-set <ids> --checkpoint <OUT>/sh<K>-ck \
  --cube-transport --expect-inputs <PKG>/idx.inputs.tsv --tool-zstd "$(command -v zstd)" --tool-unzip "$(command -v unzip)"
./target/release/spikefade bck-check <OUT>/sh<K>-ck idx
```

**Batch set:** `--batch-set` takes `0,4,8`, `0-3,7` or `all`.

**What the flags require:**
- `--shard on` needs `--batch-plan`, `--batch-set` and `--checkpoint`, and it turns on PORTABLE and STRICT.
- `--cube-transport` needs `--shard on`.
- A cumulative checkpoint left in the shard's folder is refused.

**What is refused before the first batch:**
- **The plan check** refuses any difference in segment, days, settings, order, lists, list digest or argument digest. The
  argument digest covers every result-bearing flag and value.
  - Path VALUES do not count: a path flag counts by its role, and the contents are checked through the inputs manifest.
    So the box uses its own paths.
  - `--cube-extra` counts by role. The plan, every shard and the resume must all agree on using it or not.
- **`--expect-inputs`** refuses when a day's slice digest, a root's day presence or absence, or a table's content differs
  (critic F4).

**The margin manifests** (TSV: session, label, kind, effective_utc_us, path, sha256) need only their path column rewritten
to the box's paths.

**Output and restarts:**
- The shard writes `<OUT>/sh<K>-ck/idx.b000NNN.bck` per batch plus `idx.inputs.tsv`, and ends with `SHARD DONE`.
- Running the same command again walks only the missing or damaged batches, so it is safe after a kill.

**Send back** every `idx.b*.bck` and the `idx.inputs.tsv` with the ticket's results link. Never `git add` them.

**Memory: check C1's RAM before a real shard.** Every shard recomputes the whole set-up.
- MEASURED on the Mac, v3.3 every-year index set-up, no `--cube-extra`, 6 threads: max RSS 13.39 GB, peak memory
  footprint 36.17 GB.
- MEASURED on the 2-day proofs below (4 threads): max RSS 0.61 GB for stock and 0.95 GB for index.

## 8. On the Mac only: the merge and the ONE resume (for reference)

```sh
spikefade ckpt-merge --seg idx --plan PLAN-idx.tsv --out <dir>/merged \
  --expect-source c3722928015fb27164c88ad687f64a7529c4c003dc8bc14825c82477984fbf8e <shard dirs...>
spikefade ckpt-canon <dir>/merged/idx.ckpt --tie52 on|off
spikefade run3 <whole cube> <out> <days> <same args> --threads 12 --batch-plan PLAN-idx.tsv --batch-files on --checkpoint <dir>/merged
```

- **The merge refuses:**
  - a gap;
  - a damaged or truncated file;
  - two different bodies for one batch;
  - any file whose provenance (source, rustc, inputs, plan) differs.
- **A shard run on the Mac** must therefore use a binary built from THIS branch with the same two values. The Mac build of
  this branch qualifies: source `c3722928...`, rustc as above.
- **Gated v3.3 resume:** `ckpt-merge ... --fingerprint v33` (uniform plan only), then the gated v3.3 binary with
  `--checkpoint <dir>/merged33`.
- **Finding 7.1 (MAIN rules on it):** with `--cube-extra`, the RESULTS of a resumed or sharded run differ from a single run
  in ONE counter line, "merged into N day loads". Every data file is identical.

## 9. Proof on the Mac, 16 Nov 2026 (branch:shard)

**Clean build of this exact tree.**
- The command: a fresh target directory, `nice -n 10 cargo build --release --locked --offline -j 4 -p spikefade`, with the
  two values of section 4.
- The result: rc 0 in 3 min 14 s (MEASURED; loaded Mac).
- The binary: 4,864,984 bytes, sha256 `e35c2bf0329aaee127c31d46c5e3c84377e5cd99dfac1723fd5878f46a2708c8`.
  - It holds no path of the app's tree, and it holds the source value once.
  - The build gave 54 warnings, the same count as kit2's build.

**The kit's 2-day shard proof, re-run with that binary.** This is build:shard's E5 proof. ONE shard, at 4 threads, walked
EVERY batch of build:shard's plan from its E9 transport files. The index slice used the extra-index root's transport files
as well.

| check | stock (2019-12-31 + 2020-01-01) | index (2023-03-28 + 29, `--cube-extra`) |
|---|---|---|
| batch bodies vs build:shard's 12-thread reference (plain2) | **32 of 32 IDENTICAL** | **41 of 41 IDENTICAL** |
| batch bodies vs the kit2 shard | 32 of 32 IDENTICAL | 41 of 41 IDENTICAL |
| batch-file bytes up to and including the BODY line vs the kit2 shard's files | 32 of 32 | 41 of 41 |
| inputs manifest (E4) vs the kit2 shard's | IDENTICAL | IDENTICAL |
| reads outside the loaded slice | 0 | 0 |
| its folder merged alone with `--expect-source c3722928...` | rc 0 | rc 0 |
| canonical merged checkpoint vs plain2's 4-shard merge and kit2's merge | IDENTICAL (`0eb67dea...`) | IDENTICAL (`d14b5915...`) |
| provenance of every file | source `c3722928...`, `rustc 1.97.1 (8bab26f4f 2026-07-14)` | the same |
| wall / max RSS (MEASURED) | 3 min 56 s / 0.61 GB | 3 min 55 s / 0.95 GB |

**Section 6 (C2), run verbatim with that binary:**
- the synthetic set sha256 was `7712d2a2...`, as expected;
- the shard of batches 0-3 ended with `SHARD DONE`, with 0 reads outside the slice;
- its 4 bodies are IDENTICAL to the table in section 6;
- wall 1 min 9 s (MEASURED).

## 10. Not proven yet

- **No Linux x86_64 build has been made or run** (C0, C1, P-S9). The first box's C1 and C2 are the first.
- **Not run:**
  - P-S5, flagged rows at run level;
  - P-S6, a SEARCH-sized session;
  - critic F6-F11: the bucket protocol, the re-walk, the comparison rewrite, gates on the shard binary, the operator list,
    the memory go / no-go.
- **No PGO build.**

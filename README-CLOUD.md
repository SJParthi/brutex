# README-CLOUD: build and run a spike-selling shard on a cloud box

Branch `cloud/spikefade-shard-20261116-det`: the determinism revision of `cloud/spikefade-shard-20261116`. **CODE ONLY.** The
repository is public: every push publishes.

Labels: MEASURED = a number from a named run on the operator's Mac. READ = a fact read from the code. ESTIMATE = measured inputs times
a model. PREDICTED = follows from MEASURED and READ facts; not yet run on Linux. EXACT = a count. No figure here is a trading result.

## 1. What this branch is

- The spike-selling engine `spikefade`: v3.3 final plus the v3.4 **shard kit** (round D, build:shard, Addendum 30.3), plus the
  **determinism fix** (fix:determinism, 16 Nov 2026; independent check by check:determinism: PASS WITH FOLLOW-UPS).
  - The kit adds shard mode (`--shard on`), an explicit batch plan, per-batch files, `ckpt-merge`, segment slices, transport
    files and synthetic days.
  - Every kit switch is OFF by default. With all of them off the binary replays v3.3 final byte for byte (proof P-S0, and P0
    again for this revision: section 9).
- **What the determinism fix changes.** The first boxes' C2 "DIFFER" came from `cube-slice` taking zstd from the Mac-only path
  `/opt/homebrew/bin/zstd`.
  - **DET-1, tools.** Every subcommand takes `--portable on|off`, `--portable-math on|off` and `--tool-<t> PATH`. They are
    removed before the subcommand's own parser sees them. `cube-slice --transport` implies PORTABLE, as `--shard on` does for
    run3.
  - **DET-2, math.** A new kit switch, PMATH (`--portable-math on`). Under it, every `exp` / `ln` of the Greeks crate and
    `gk.rs`'s `ln_1p` go through ONE pure-Rust implementation, libm 0.2.16, pinned under `pinned/libm-0.2.16`. The same inputs
    then give the same bits on every 64-bit platform.
    - **Shards turn PMATH on**, and a shard cannot turn it off.
    - `--batch-files on` never implies it, so **the Mac's ONE resume keeps the platform libm and v3.3's results unless
      `--portable-math on` is added**.
    - `--portable-math` is a runtime flag, outside the argument digest.
    - A batch body holds integers only and a shard runs no closing pass, so PMATH moves no batch body. It moves only report
      Greeks, in their last bits (MEASURED on 30,000 rows: IV at most 4.8e-13 relative, every Greek at most 3.3e-12).
- **Source: 310 files.**
  - The first shard branch's 146 (13 of them changed), plus `crates/greeks/src/hostmath.rs`, `crates/spikefade/src/det.rs`,
    `crates/spikefade/src/testsdet.rs`, the 3 tables in `crates/spikefade/detdata/`, `pinned/libm-0.2.16/` (156 files),
    `pinned/libm-0.2.16.PIN.txt` and `pinned/libm-0.2.16.SHA256SUMS`.
  - The app's `greeks` crate is copied in as a workspace member (`crates/greeks`), and the engine points at `../greeks`. It now
    carries `hostmath`, through which a host may route `exp` / `ln`; with nothing installed it uses the platform's functions, as
    before.
  - `pinned/libm-0.2.16` is exactly the content of the published `.crate` (sha256
    `b6d2cec3eae94f9f509c767b45932f1ada8350c4bdb85af2fcab4a3c14807981`), plus cargo's unpack marker `.cargo-ok`. It was copied from
    a local cargo registry. Its licence is MIT (`LICENSE.txt`). It is used with default features off: no arch-specific code, soft
    floats only.
  - No manifest holds an absolute path.
  - Nothing comes from crates.io. `Cargo.lock` lists path crates only: core, costs, common_fill, greeks, libm
    (`pinned/libm-0.2.16`), spikefade.
- **Not in this binary:** the v3.4 round-D engine switches (`--fix34`, halts, ELM pre-AEL, stamp, STRICTSPAN, bands,
  the F3 freeze table). They belong to a separate build.

## 2. What is NOT here, and must never be added

Only `.rs`, `.toml`, `.md` and `Cargo.lock` files are allowed, plus exactly these 9 files, allowed by name:

| file | what it is |
|---|---|
| `pinned/libm-0.2.16/.cargo-ok` | cargo's unpack marker (`{"v":1}`) |
| `pinned/libm-0.2.16/.cargo_vcs_info.json` | from the published `.crate`: the upstream commit and path |
| `pinned/libm-0.2.16/Cargo.toml.orig` | from the published `.crate`: its manifest before cargo normalised it |
| `pinned/libm-0.2.16/LICENSE.txt` | from the published `.crate`: its MIT licence |
| `pinned/libm-0.2.16.PIN.txt` | the pin record: version, licence, `.crate` sha256, features |
| `pinned/libm-0.2.16.SHA256SUMS` | the sha256 of each of the 156 files of `pinned/libm-0.2.16` |
| `crates/spikefade/detdata/DET-C2G-FN.tsv` | every distinct (function, input) of the C2G rows, with the pinned libm's output, as exact IEEE-754 bits |
| `crates/spikefade/detdata/DET-C2G-GREEKS.tsv` | the exact bits of every Greeks figure of the C2G rows under `--portable-math` |
| `crates/spikefade/detdata/DET-C2G-ROWS.tsv` | the 602 Greeks inputs of the C2 SYNTHETIC days (section 6's set, run once with the Greeks on) |

- The det tests read the 3 `detdata` tables at run time (section 5). They hold NO market data: the rows come from the synthetic
  days, and the rest is math.
- No other file that is not `.rs`, `.toml`, `.md` or `Cargo.lock` may be added.

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
- tables, plans and inputs manifests (the 3 `detdata` test tables above are the only tables allowed);
- batch files, checkpoints and results;
- tickets, presigned links and credentials;
- anything under `runtime/`.

Before any commit or push from a box, refuse if any tracked or staged file holds a presigned-link signature. This command
must print nothing (the marker is split in two so that this file does not match it):

```sh
git grep -I -l --cached -e "X-Amz""-Signature"; git grep -I -l -e "X-Amz""-Signature"
```

## 3. Toolchain

- `rust-toolchain.toml` pins **1.97.1**: components rustfmt, clippy and llvm-tools-preview, target wasm32-unknown-unknown,
  profile minimal.
  - rustup installs it on first use, which needs the network.
- Expected `rustc --version`: `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
  - Every shard of one run must report the SAME string.
  - `ckpt-merge` refuses a batch file whose provenance differs in source, rustc, inputs or plan.
- **Tools on PATH:** `zstd` and `unzip` (also `gzip`, `ps`, `hostname`), or pass `--tool-<t> PATH` or `SF_<T>`.
  - **Under PORTABLE this holds for EVERY subcommand** (`--portable on`, implied by `run3 --shard on` / `--batch-files on` and
    by `cube-slice --transport`). In the first shard branch it held for run3 only: `cube-slice --transport` took zstd from
    `/opt/homebrew/bin/zstd`, which stopped the first boxes' C2.
  - Under PORTABLE the only fixed tool path left is `/bin/ps`, for the RSS print; a failure there is ignored.
  - With PORTABLE off, a subcommand behaves as before: tools from the Mac paths (zstd from `/opt/homebrew/bin/zstd`), plus
    `/bin/date` and `/usr/bin/shasum` (a few switch-off tests call `shasum`, as v3.3 does).
  - The start probes check that zstd and unzip answer and that the sha256 self-test passes.

## 4. Verify the checkout, then build

```sh
git clone --branch cloud/spikefade-shard-20261116-det --single-branch --depth 1 https://github.com/SJParthi/brutex.git sf
cd sf
# the source manifest: every file but .git, target, the engine's data folder and this README
LC_ALL=C find . -type f ! -path './.git/*' ! -path './target/*' ! -path './crates/spikefade/data/*' ! -name README-CLOUD.md \
  | LC_ALL=C sort | xargs sha256sum > ../SOURCE-SHA256-cloud-branch.txt
wc -l < ../SOURCE-SHA256-cloud-branch.txt          # must print 310
grep -c -E '  \./(pinned/libm-0\.2\.16/(\.cargo-ok|\.cargo_vcs_info\.json|Cargo\.toml\.orig|LICENSE\.txt)|pinned/libm-0\.2\.16\.(PIN\.txt|SHA256SUMS)|crates/spikefade/detdata/DET-C2G-(FN|GREEKS|ROWS)\.tsv)$' \
  ../SOURCE-SHA256-cloud-branch.txt                # must print 9: section 2's named files
SRC=$(sha256sum ../SOURCE-SHA256-cloud-branch.txt | cut -c1-64)
[ "$SRC" = 36ac16ea55b16dff38206a3bca0fbe20afefbab96e46473bf3cea002ed5c63a9 ] || { echo "checkout differs: STOP"; exit 1; }
SF_SOURCE_SHA256=$SRC SF_RUSTC="$(rustc --version)" nice -n 10 cargo build --release --locked --offline -p spikefade
sha256sum target/release/spikefade                 # report it; it is not expected to equal the Mac's
```

- **`--offline` works** because `Cargo.lock` holds only path crates, libm included.
- **`SF_SOURCE_SHA256` and `SF_RUSTC` are baked into every batch file's provenance** (`PROV source=... rustc=...`).
  - The Mac's merge checks the source with `--expect-source 36ac16ea...`.
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

- **Expected (MEASURED on the Mac by fix:determinism and again by check:determinism, on this source):** 175 passed, 0 failed,
  2 ignored. That is build:shard's 167 plus the 8 det tests. The ignored ones are v3.3's measurement test and `det_gen_tables`,
  the generator that wrote the 2 det bit tables.
- **Without the 3 parameter tables, exactly the 7 tests that read them fail** (MEASURED by check:determinism):
  - 5 in `menu.rs`;
  - `sf3_ttp_replaces_tp_close` in `tests3.rs`;
  - `v33_search1d_sets_load_and_count` in `tests33s.rs`.
- **When the tables are absent, skip those 7.** Expected then: 168 passed, 0 failed, 2 ignored, 7 filtered out.

```sh
SF_SOURCE_SHA256=$SRC SF_RUSTC="$(rustc --version)" cargo test --release --locked --offline -p spikefade -- \
  --skip menu::tests:: --skip tests3::sf3_ttp_replaces_tp_close --skip tests33s::v33_search1d_sets_load_and_count
```

- **The 8 det tests alone** (`-- testsdet`): `8 passed; 0 failed; 1 ignored; 0 measured; 168 filtered out`.
  - `det_c2g_fn_table_bits` and `det_c2g_greeks_rows_bits` read `crates/spikefade/detdata/`.
  - They are the proof that the box's build computes the pinned libm's bits. They pass on the Mac's arm64 and x86_64 builds.

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

**Expected batch bodies.** The determinism fix does not change them: the Mac's reference with the switch on
(`C2-REFERENCE-PORTABLE.tsv`) holds the same 4 bodies as v3.4. The Mac build of this branch gives them, also with
`/opt/homebrew/bin/zstd` denied (section 9):

| batch | body sha256 |
|---|---|
| 0 | `6d11c2ea0e3414c870295c4b275deaabacd918f18c2c0f824201c475a954d3c3` |
| 1 | `89f6bc992d9047cb2ddf2e3abb38d08462956ce8c98915f3684a459ff5c87677` |
| 2 | `1aa6fc820b7ca103d7cc0c8f211d4851626e50b84ac93d223ab14fd466525ab8` |
| 3 | `d43bedb8916aba25d7bf38f0a295fb4d4bd93022c81a9122c3068486d0c4f663` |

- `cube-slice --transport` takes zstd from PATH (DET-1). The shard implies PORTABLE, STRICT and PMATH, and its log names
  PORTABLE MATH.
- **Expected provenance** (the `PROV` line of each batch file; these are the fields `ckpt-merge` compares):
  - `source=36ac16ea55b16dff38206a3bca0fbe20afefbab96e46473bf3cea002ed5c63a9`;
  - `rustc=rustc 1.97.1 (8bab26f4f 2026-07-14)`;
  - `inputs=b64f6cf68d6387d8299ccee96f9f8673ca6c080dd7c05a26baa99b7caf4a3e6e` (decoded slices and table contents, never
    compressed bytes);
  - `plan=d73fdf30ed716da41ca2f02f8443d4a1fc3bc3bc77c716687e08751838b69ec2`.
  - The other fields (`target=`, `threads=` and the like) name the box and the run; they are not compared.

If all 4 bodies are equal, the box computes exactly as the Mac. If any differs, STOP and report it: no real shard may run.

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
- `--shard on` needs `--batch-plan`, `--batch-set` and `--checkpoint`. It turns on PORTABLE, STRICT and PMATH; a shard cannot
  turn PMATH off.
- `--cube-transport` needs `--shard on`.
- A cumulative checkpoint left in the shard's folder is refused.

**What is refused before the first batch:**
- **The plan check** refuses any difference in segment, days, settings, order, lists, list digest or argument digest. The
  argument digest covers every result-bearing flag and value.
  - Path VALUES do not count: a path flag counts by its role, and the contents are checked through the inputs manifest.
    So the box uses its own paths.
  - `--cube-extra` counts by role. The plan, every shard and the resume must all agree on using it or not.
  - `--portable-math` is a runtime flag and never counts.
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
- MEASURED on the 2-day proofs (4 threads): max RSS 0.61 GB for stock and 0.95 GB for index.

## 8. On the Mac only: the merge and the ONE resume (for reference)

```sh
spikefade ckpt-merge --seg idx --plan PLAN-idx.tsv --out <dir>/merged \
  --expect-source 36ac16ea55b16dff38206a3bca0fbe20afefbab96e46473bf3cea002ed5c63a9 <shard dirs...>
spikefade ckpt-canon <dir>/merged/idx.ckpt --tie52 on|off
spikefade run3 <whole cube> <out> <days> <same args> --threads 12 --batch-plan PLAN-idx.tsv --batch-files on --checkpoint <dir>/merged
```

- **The merge refuses:**
  - a gap;
  - a damaged or truncated file;
  - two different bodies for one batch;
  - any file whose provenance (source, rustc, inputs, plan) differs.
- **A shard run on the Mac** must therefore use a binary built from THIS branch with the same two values. The Mac's det build
  qualifies: source `36ac16ea...`, rustc as above.
- **Files of the first shard branch** (source `c3722928...`) cannot be merged with files of this branch: the merge refuses a
  source difference.
- **The ONE resume** (`--batch-files on`) keeps the platform libm and v3.3's results. With `--portable-math on` added, its
  Greeks texts say so.
- **Gated v3.3 resume:** `ckpt-merge ... --fingerprint v33` (uniform plan only), then the gated v3.3 binary with
  `--checkpoint <dir>/merged33`.
- **Finding 7.1 (MAIN rules on it):** with `--cube-extra`, the RESULTS of a resumed or sharded run differ from a single run
  in ONE counter line, "merged into N day loads". Every data file is identical.

## 9. Proof on the Mac, 16 Nov 2026 (fix:determinism, then check:determinism)

**The det binary.**
- Built from this exact 310-file tree with `--locked --offline`, nice 10, `-j 4`, the manifest sha256 baked in as the
  provenance `source`.
- sha256 `ad1f1b8644d73a9f0e3e19155eaa62e5732b0d0e5b4e8c013bc4ba75470a3974`. It holds no path of the app's tree.
- The same source built for x86_64-apple-darwin: `b5752e6a...`, run under Rosetta 2.

| check | result |
|---|---|
| C2 (section 6) with the det binary, on the Mac and "Linux-like" (`/opt/homebrew/bin/zstd` denied, zstd only on PATH) | **IDENTICAL**: the 4 bodies above (MEASURED: 26 s each) |
| C2, Linux-like, at 1, 2 and 3 threads; with the transport recompressed at zstd level 19 (other compressed bytes); with the x86_64 build | **IDENTICAL** in every run (check:determinism, runs A, B and C, and its Linux job script's dry run at 4 and 2 threads) |
| C2, Linux-like, with the first shard branch's binary `e35c2bf0` | **FAILED** at `cube-slice`: the first boxes' failure |
| P0, every switch off: the 2-day index and stock proof slices vs the gated v3.3 binary and vs the kit's plain2 | 5 of 5, 3 of 3 and 8 of 8 IDENTICAL, each |
| P0 on v3.3's 8-day slice (8d-bod-banknifty) | 5 of 5, 3 of 3 and 8 of 8 IDENTICAL; with the switch on, 0 of 679 trade rows differ |
| P0 on new inputs: `greeks-tsv` on 30,000 new rows and the 602 C2G rows; run3 on a new synthetic set (seed 4242) with the Greeks on | the first shard branch's binary's output on every row (timing column masked); run3 data files byte-identical, RESULTS differs only in the run id and MEASURED timing |
| P-S2 / P-S3 with the switch on (on `cb090b62`, the det source before one PMATH-only report label) | 41 of 41 index and 32 of 32 stock bodies IDENTICAL from 4 shard processes at 1-4 threads; merges canonically identical (`d14b5915...`, `0eb67dea...`) |
| `greeks-tsv --portable-math on`, arm64 vs x86_64 | identical on every row tried (30,602 and 3,430 rows) |
| The test suite | 175 passed, 0 failed, 2 ignored; the 8 det tests also pass on x86_64 |
| Mutants | fix:determinism 12 of 12 KILLED; check:determinism 2 of 3 KILLED, 1 survived (a test gap: section 10) |

The first shard branch's own proof (binary `e35c2bf0`, source `c3722928...`) is in that branch's README.

## 10. Not proven yet

- **Nothing of this revision has run on Linux.** Linux C2 = IDENTICAL is PREDICTED from the Mac runs above, the x86_64 build and
  a code review that found no platform-dependent call on the shard path.
  - **The C2 gate:** until a Linux box reproduces the 4 bodies of section 6 byte for byte, NO cloud shard result may be used.
- **Not run:**
  - P-S5, flagged rows at run level;
  - P-S6, a SEARCH-sized session;
  - critic F6-F11: the bucket protocol, the re-walk, the comparison rewrite, gates on the shard binary, the operator list,
    the memory go / no-go.
- **No PGO build.**
- **With PMATH on, report Greeks are not bit for bit the live app's `/greeks.json`.** That proof (8b) holds with PMATH off.
  They are report-only figures; the MEASURED distance is in section 1.
- **Open follow-ups (check:determinism, non-blocking):**
  - No named test pins "`--batch-files` never implies PMATH" yet. A mutant that breaks it survived the whole suite; the code
    itself is right.
  - A data-free C2, with inputs built in code, would let a box clear the gate with no input files.
  - RESULTS' "Greeks solve time ... over N instants" counts only instants measured above 0 ns, so N is MEASURED: gates must
    mask the whole line.

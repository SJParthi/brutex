# README-CLOUD: build and run a spike-selling shard on a cloud box

Branch `cloud/spikefade-shard-20261116-mem`: the memory revision (E10, "ship the set-up") of `cloud/spikefade-shard-20261116-det`.
**CODE ONLY.** The repository is public: every push publishes.

Labels: MEASURED = a number from a named run on the operator's Mac. READ = a fact read from the code. ESTIMATE = measured inputs times
a model. PREDICTED = follows from MEASURED and READ facts; not yet run on Linux. EXACT = a count. No figure here is a trading result.

## 1. What this branch is

- The spike-selling engine `spikefade`: v3.3 final plus the v3.4 **shard kit** (round D, build:shard, Addendum 30.3), the
  **determinism fix** (fix:determinism; branch `cloud/spikefade-shard-20261116-det`, commit `8d74dc01`) and **E10, "ship the set-up"**
  (build:shard-memory, repair:shard-memory, check:shard-memory-r2: PASS WITH FOLLOW-UPS; folded onto the determinism fix by
  ship:shard-memory).
  - Every switch is OFF by default. With all of them off the binary replays v3.3 final byte for byte (P0, MEASURED again for this
    revision: section 9).
- **Why E10.** The every-year index run's set-up (pass 0, the readings, the per-config lists) peaked at 27,999 MiB on the Mac (MEASURED,
  a lower bound) and every shard of the det branch recomputes it, so a 13,674 MiB box cannot run a shard. E10 computes the set-up ONCE on
  the Mac and ships it; a shard reads it and never runs the set-up.
  - **SHIP_W, the writer** (Mac only): `run3 ... --write-plan PLAN --write-setup DIR` is the plan run that ALSO writes the set-up
    store `DIR` (`HDR.bin`, `SETUP.tsv`, one `ev/<YYYY-MM>.sfe.zst` section file per month: every run day's event records and every
    config's day list) and writes its digest `v2` as the plan's `SETUP` line. The plan without that line is the det branch's plan byte
    for byte. Its memory switches: R1 `--bhav-cache N` (the previous-session bhavcopy cache as an LRU of N files) and R2
    `--levels-stream on` (the causal level samples streamed); neither changes a plan byte.
  - **SHIP_R, the reader** (a shard): `run3 ... --shard on --batch-plan PLAN --setup-from DIR`. Before batch 1 it checks the store
    against the plan: `v2` = the plan's `SETUP` line, `V1` = the plan's `DIGEST`, `HDR.bin`, every section's `E_d` / `L_d` and every
    list's `M_u`, and the run's own FLAGS / SEG / DAYS / NCFG / NEV / LIQS / TFDEP / ARGS / INPUTS. Then it walks its batches with a
    one-day event window; the walked parts of every list are re-hashed against `M_u` before each batch file. Any violation voids the file.
    A SHIP_R shard never runs pass 0, the readings or the list stage, and never opens a margin TABLE file (the inputs manifest reads only
    the margin manifests' rows).
  - `--setup-audit on` (proof only): the shard also builds the set-up in memory and compares every window with the store.
  - Two subcommands: `setup-check STORE PLAN [--threads N]` (the whole store against a plan) and
    `setup-stats STORE PLAN [--threads N] [--cube DIR] [--cube-extra DIR]` (per batch its largest batch-day union `U_k*`, the memory
    canary's batch `KSTAR` and the largest-cube day `CUBEPEAK`).
  - The RUN.log (and stderr) of a SHIP_R shard: `E10 MEM <seg> start-up: ...` once before batch 1 (after the E4 inputs manifest, the
    store checks and the start check) and `E10 MEM <seg> batch <k>: ...` after every batch file: `VmHWM` / `VmRSS` from
    `/proc/self/status` on Linux, the `ps` RSS elsewhere. Diagnostics only, never in a body. The batch file's provenance adds
    `setup=<v2>`, `hwm_kb=<VmHWM>` and `arena=<GLIBC_TUNABLES>`.
- **What the determinism fix changes** (unchanged in this revision). The first boxes' C2 "DIFFER" came from `cube-slice` taking zstd
  from the Mac-only path `/opt/homebrew/bin/zstd`.
  - **DET-1, tools.** Every subcommand takes `--portable on|off`, `--portable-math on|off` and `--tool-<t> PATH`. They are
    removed before the subcommand's own parser sees them. `cube-slice --transport` implies PORTABLE, as `--shard on` does for
    run3. The E10 subcommands `setup-check` and `setup-stats` take them too, but do not imply them (section 3).
  - **DET-2, math.** A kit switch, PMATH (`--portable-math on`). Under it, every `exp` / `ln` of the Greeks crate and `gk.rs`'s
    `ln_1p` go through ONE pure-Rust implementation, libm 0.2.16, pinned under `pinned/libm-0.2.16`.
    - **Shards turn PMATH on**, and a shard cannot turn it off.
    - `--batch-files on` never implies it (a named test now pins this: `det_pmath_switch_and_shard`), so **the Mac's ONE resume keeps the
      platform libm and v3.3's results unless `--portable-math on` is added**.
    - `--portable-math` is a runtime flag, outside the argument digest.
    - A batch body holds integers only and a shard runs no closing pass, so PMATH moves no batch body.
- **Source: 312 files.**
  - The det branch's 310, 7 of them changed (`crates/spikefade/Cargo.toml`, `bhav.rs`, `kit.rs`, `main.rs`, `report3.rs`, `tests34.rs`,
    `testsdet.rs`), plus 2 new files: `crates/spikefade/src/e10.rs` and `crates/spikefade/src/tests_e10.rs`.
  - The app's `greeks` crate is a workspace member (`crates/greeks`, with `hostmath`), and the engine points at `../greeks`.
  - No manifest holds an absolute path. Nothing comes from crates.io. `Cargo.lock` lists path crates only: core, costs,
    common_fill, greeks, libm (`pinned/libm-0.2.16`), spikefade.
- **Not in this binary:** the v3.4 round-D engine switches (`--fix34`, halts, ELM pre-AEL, stamp, STRICTSPAN, bands, the F3 freeze
  table) and the later engine rulings (Addenda 38 onward). They belong to a separate build (MAIN's open item M8).

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
  generator script. Exit-rule parameter tables with NO market data; the build does not need them; a run takes them through `--menu`
  and `--sets`; 7 tests read them (section 5).
- **`pinned/common-fill-v24/vectors/` and `matrix/`**, its `SHA256SUMS*` files and its CHANGES note; **`pinned/common-fill-v21`,
  `-v21-fa1`, `-v22` and `-v23`** (none is a dependency); **`keep/`**; **every target directory**.

Never commit any of these:
- market data: cubes, slices, transport files;
- tables, plans, set-up stores (`SETUP.tsv`, `HDR.bin`, `*.sfe.zst`) and inputs manifests (the 3 `detdata` test tables above are
  the only tables allowed);
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
  profile minimal. rustup installs it on first use, which needs the network.
- Expected `rustc --version`: `rustc 1.97.1 (8bab26f4f 2026-07-14)`.
  - Every shard of one run must report the SAME string.
  - `ckpt-merge` refuses a batch file whose provenance differs in source, rustc, inputs or plan.
- **Tools on PATH:** `zstd` and `unzip` (also `gzip`, `ps`, `hostname`), or pass `--tool-<t> PATH` or `SF_<T>`.
  - Under PORTABLE this holds for EVERY subcommand (`--portable on`, implied by `run3 --shard on` / `--batch-files on` and by
    `cube-slice --transport`). A SHIP_R shard reads the store's `.sfe.zst` sections with the zstd it finds this way.
  - **`setup-check` and `setup-stats` do NOT imply PORTABLE: on a box always add `--portable on`.** Without it they take zstd from the
    Mac path and refuse (MEASURED on the Mac with that path denied: section 9).
  - Under PORTABLE the only fixed tool path left is `/bin/ps`, for the RSS print on a machine without `/proc`; a failure there is
    ignored.
  - With PORTABLE off, a subcommand behaves as before: tools from the Mac paths (zstd from `/opt/homebrew/bin/zstd`).
  - The start probes check that zstd and unzip answer and that the sha256 self-test passes.

## 4. Verify the checkout, then build

```sh
git clone --branch cloud/spikefade-shard-20261116-mem --single-branch --depth 1 https://github.com/SJParthi/brutex.git sf
cd sf
# the source manifest: every file but .git, target, the engine's data folder and this README
LC_ALL=C find . -type f ! -path './.git/*' ! -path './target/*' ! -path './crates/spikefade/data/*' ! -name README-CLOUD.md \
  | LC_ALL=C sort | xargs sha256sum > ../SOURCE-SHA256-cloud-branch.txt
wc -l < ../SOURCE-SHA256-cloud-branch.txt          # must print 312
grep -c -E '  \./(pinned/libm-0\.2\.16/(\.cargo-ok|\.cargo_vcs_info\.json|Cargo\.toml\.orig|LICENSE\.txt)|pinned/libm-0\.2\.16\.(PIN\.txt|SHA256SUMS)|crates/spikefade/detdata/DET-C2G-(FN|GREEKS|ROWS)\.tsv)$' \
  ../SOURCE-SHA256-cloud-branch.txt                # must print 9: section 2's named files
SRC=$(sha256sum ../SOURCE-SHA256-cloud-branch.txt | cut -c1-64)
[ "$SRC" = daed6a59826490245213796062a895a635171ea19773348f212887ff8e06ed7d ] || { echo "checkout differs: STOP"; exit 1; }
SF_SOURCE_SHA256=$SRC SF_RUSTC="$(rustc --version)" nice -n 10 cargo build --release --locked --offline -p spikefade
sha256sum target/release/spikefade                 # report it; it is not expected to equal the Mac's
```

- **`--offline` works** because `Cargo.lock` holds only path crates, libm included.
- **`SF_SOURCE_SHA256` and `SF_RUSTC` are baked into every batch file's provenance** (`PROV source=... rustc=...`).
  - The Mac's merge checks the source with `--expect-source daed6a59...`.
  - Left unset, the binary bakes `unset`, and the merge refuses its files.
- **The binary differs between machines** (paths, target). The batch BODIES must not differ, and they are what every proof compares.

## 5. C1 (data-free): the box, then the engine's test suite

```sh
uname -a; nproc; grep MemTotal /proc/meminfo; cat /sys/fs/cgroup/memory.max 2>/dev/null; df -P -BG .
grep -m1 'model name' /proc/cpuinfo; rustc -V
```

The test suite covers the engine only. The other crates' own tests need files of the app repository, so they are not run here.

```sh
# Copy these 3 parameter tables from the package into crates/spikefade/data/ (never git add them). Expected sha256:
#   EXITS-DYN-MENU-v1.tsv      bd01157aae8d244a83f488bfec823b9a04c9fd94d4fbe58128d28530d3b65fc8
#   EXITS-DYN-SETS-v1.tsv      279150b2308d14604fb3f5b94a1903efa93bc2d01283956b12112e1e8dce3013
#   EXITS-DYN-SETS-v33-1D.tsv  5ead880f47bfa5c04c5f4ae1275a799d209ccec811c288236c5def46fc54612b
SF_SOURCE_SHA256=$SRC SF_RUSTC="$(rustc --version)" cargo test --release --locked --offline -p spikefade
```

- **Expected (MEASURED on the Mac, this source with the 3 tables):** 195 passed, 0 failed, 2 ignored. That is the shard kit's 167,
  the 8 det tests and the 20 E10 tests. The ignored ones are v3.3's measurement test and `det_gen_tables`.
- **Without the 3 parameter tables, exactly the 7 tests that read them fail** (5 in `menu.rs`, `sf3_ttp_replaces_tp_close` in
  `tests3.rs`, `v33_search1d_sets_load_and_count` in `tests33s.rs`; MEASURED on this code-only tree: 188 passed, 7 failed, 2 ignored).
  When the tables are absent, skip those 7. Expected then (MEASURED): 188 passed, 0 failed, 2 ignored, 7 filtered out.

```sh
SF_SOURCE_SHA256=$SRC SF_RUSTC="$(rustc --version)" cargo test --release --locked --offline -p spikefade -- \
  --skip menu::tests:: --skip tests3::sf3_ttp_replaces_tp_close --skip tests33s::v33_search1d_sets_load_and_count
```

- **The 8 det tests alone** (`-- testsdet`): 8 passed, 1 ignored. `det_c2g_fn_table_bits` and `det_c2g_greeks_rows_bits` read
  `crates/spikefade/detdata/`; they are the proof that the box's build computes the pinned libm's bits.
- **The 20 E10 tests alone** (`-- tests_e10`): 20 passed. They need no file.

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

**Expected batch bodies** (neither the determinism fix nor E10 changes them; the Mac build of this branch gives them, also with
`/opt/homebrew/bin/zstd` denied: section 9):

| batch | body sha256 |
|---|---|
| 0 | `6d11c2ea0e3414c870295c4b275deaabacd918f18c2c0f824201c475a954d3c3` |
| 1 | `89f6bc992d9047cb2ddf2e3abb38d08462956ce8c98915f3684a459ff5c87677` |
| 2 | `1aa6fc820b7ca103d7cc0c8f211d4851626e50b84ac93d223ab14fd466525ab8` |
| 3 | `d43bedb8916aba25d7bf38f0a295fb4d4bd93022c81a9122c3068486d0c4f663` |

- **Expected provenance** (the `PROV` line of each batch file; the fields `ckpt-merge` compares): `source=daed6a59...` (this branch),
  `rustc=rustc 1.97.1 (8bab26f4f 2026-07-14)`, `inputs=b64f6cf68d6387d8299ccee96f9f8673ca6c080dd7c05a26baa99b7caf4a3e6e`,
  `plan=d73fdf30ed716da41ca2f02f8443d4a1fc3bc3bc77c716687e08751838b69ec2`.

If all 4 bodies are equal, the box computes exactly as the Mac. If any differs, STOP and report it: no real shard may run.

## 6b. C2-ship (data-free): the E10 path on the synthetic set

The same synthetic days, read through E10: the Mac's SHIP_W writer wrote a set-up store and a plan for them once; the box checks the
store and walks batches 0-3 as ONE SHIP_R shard on its own transport files.

**Inputs** (the run's `c2/` files, delivered with the ticket, never through this repository; synthetic, NO market data):
- `PLAN-syn-e10.tsv` (sha256 `ba097d9eb2b515869e03cfdfe6209c90235785248c095d03578a317e18df1b12`): `PLAN-syn.tsv` plus one `SETUP`
  line (the store's `v2`, `7cefdcaa700c80a55dd36535befb515bcb44123730ebbb7370cad66845459326`);
- `SETUP-syn-e10.tar` (sha256 `4b08f8a977abf59305f063274cd43f4fd334a46e55e88dd1fe057b2da6b5b970`, ustar): the store `SETUP-syn/`
  (`HDR.bin`, `SETUP.tsv`, `ev/2023-03.sfe.zst`).

```sh
# in the c2 folder of section 6, after its synth and cube-slice lines (a fresh synthetic set gives the same bytes)
tar -xf $T/SETUP-syn-e10.tar                       # makes SETUP-syn/
$B setup-check SETUP-syn $T/PLAN-syn-e10.tsv --threads 4 --portable on | tail -1     # SETUP CHECK PASSED ...
$B run3 syn/transport/main syn/shs 2023-03-28 2023-03-29 --cube-extra syn/transport/extra $A --threads 4 \
 --batch-plan $T/PLAN-syn-e10.tsv --shard on --batch-set 0-3 --checkpoint syn/shs-ck --cube-transport --setup-from SETUP-syn
$B bck-check syn/shs-ck idx | awk -F'\t' '$1=="batch"{print $2"\t"$4}'
```

**Expected batch bodies** (the Mac's, MEASURED with this branch's build, also with `/opt/homebrew/bin/zstd` denied: section 9; they
differ from section 6's because the plan's sha256 is part of every body):

| batch | body sha256 |
|---|---|
| 0 | `d81de91a2bc931a3e30ab286b97b6e473e713f1cd1c91e460106aa254a264a6b` |
| 1 | `35607d336b4b2d32c519c9597ae9c5a3995404fe3314b28cad9ef3860dfe3efb` |
| 2 | `644a7a1763435b788befb2db448a400249a986b7b52b993836b946f8693a67c2` |
| 3 | `46951bd6a70611ee773f19f0329bfb8b0390f7e06875fe33d512a80940009347` |

The shard's stderr and RUN.log must also say `E10 SHIP_R: ... violations 0`. If any body differs, STOP: no E10 shard may run.

## 7. A real shard (only after the operator's yes, the Mac's plan step and the upload)

The package arrives through the session's ticket, never through this repository. With E10 it gives:
- the transport roots (main and extra) and the tables; the margin TABLE objects may be skipped (a SHIP_R shard never opens them);
- `PLAN-idx.tsv`, the plan the Mac's SHIP_W writer wrote, with its `SETUP` line;
- the set-up store (`SETUP.tsv`, `HDR.bin`, the `ev/` section files);
- the expected `idx.inputs.tsv`, the EXACT run arguments the plan was written with, and this shard's batch set.

`<OUT>` must be outside the checkout. Check the store first: `spikefade setup-check <PKG>/setup <PKG>/PLAN-idx.tsv --threads <T>
--portable on` must print `SETUP CHECK PASSED`.

```sh
GLIBC_TUNABLES=glibc.malloc.arena_max=2:glibc.malloc.mmap_threshold=4194304 \
nice -n 10 ./target/release/spikefade run3 <PKG>/main <OUT>/sh<K> <the run's days> \
  --cube-extra <PKG>/extra --margin-extra <PKG>/margin-extra.tsv \
  --margin-manifest <PKG>/margin-manifest.tsv --margin-futures <PKG>/futures-all.tsv \
  --special-sessions <...> --freeze <...> --freeze <...>@2026-11-02 --ban-dir <...> --ban-dir <...> \
  --bhav-dir <...> --bhav-dir <...> --tbill <...> --var <...> --holidays <...> --holidays2 <...> \
  --menu <...> --sets <...> <the run's switches, exactly as shipped> \
  --threads <T> --batch-plan <PKG>/PLAN-idx.tsv --shard on --batch-set <ids> --checkpoint <OUT>/sh<K>-ck \
  --cube-transport --expect-inputs <PKG>/idx.inputs.tsv --setup-from <PKG>/setup \
  --tool-zstd "$(command -v zstd)" --tool-unzip "$(command -v unzip)"
./target/release/spikefade bck-check <OUT>/sh<K>-ck idx
```

- `--shard on` needs `--batch-plan`, `--batch-set` and `--checkpoint`. It turns on PORTABLE, STRICT and PMATH.
- `--setup-from DIR` needs `--shard on` and a plan whose `SETUP` line names the store; it refuses `--write-plan` and a positional run.
- **What is refused before the first batch:** the plan check (segment, days, settings, order, lists, list digest, argument digest);
  `--expect-inputs` (a day's slice digest, a root's day presence, a table's content); and E10's start check (v2, V1, HDR.bin, every
  `E_d` / `L_d` / `M_u`, the run's own facts). Path VALUES never count; `--setup-from`, `--portable-math` and the thread count are runtime
  flags.
- **Output and restarts:** `<OUT>/sh<K>-ck/idx.b000NNN.bck` per batch plus `idx.inputs.tsv`, then `SHARD DONE`. Running the same command
  again walks only the missing or damaged batches.
- **Send back** every `idx.b*.bck`, `idx.inputs.tsv` and the RUN.log's `E10 MEM` lines with the ticket's links. Never `git add` them.

**Memory.** With E10 a shard holds no set-up: its peak is the walk (stage A / B on the batch's days) plus the start-up (the E4 inputs
manifest and the store checks).
- ESTIMATE: <= ~10,400 MiB per shard at 523 lists a batch (`--batch-accounts 6000000`), worst batch-day; the tickets' memory gate is
  that ESTIMATE + 1,024 = 11,424 MiB, under a 13,674 MiB box. **No full-scale per-shard peak is MEASURED yet**: the Linux memory canary
  (batch `KSTAR` of `setup-stats` and one more, in ONE process, `GLIBC_TUNABLES` as above) measures `VmHWM` after each batch; its pass
  line is VmHWM <= 11,000 MiB at the end of every batch and `oom_kill` 0.
- MEASURED on the Mac: the 2-day proof shards (SHIP_R, 1-4 threads) 0.4-1.95 GB; never a gate for the full run.

## 8. On the Mac only: the plan step, the merge and the ONE resume (for reference)

- **The plan step (SHIP_W)**: the every-year plan run with `--write-setup <store> --strict on --bhav-cache 16 --levels-stream on`; it
  writes `PLAN-idx.tsv` (with `SETUP`) and the store; then `setup-check` and `setup-stats` on the Mac. Boxes never run it.

```sh
spikefade ckpt-merge --seg idx --plan PLAN-idx.tsv --out <dir>/merged \
  --expect-source daed6a59826490245213796062a895a635171ea19773348f212887ff8e06ed7d <shard dirs...>
spikefade ckpt-canon <dir>/merged/idx.ckpt --tie52 on|off
spikefade run3 <whole cube> <out> <days> <same args> --threads 12 --batch-plan PLAN-idx.tsv --batch-files on --checkpoint <dir>/merged
```

- **The merge refuses:** a gap; a damaged or truncated file; two different bodies for one batch; any file whose provenance (source,
  rustc, inputs, plan) differs. Files of the first shard branch (`c3722928...`) or of the det branch (`36ac16ea...`) cannot be merged
  with files of this branch.
- **The ONE resume** (`--batch-files on`, in-memory set-up) recomputes v1 and checks it against the plan's `DIGEST` (G7); it keeps the
  platform libm and v3.3's results unless `--portable-math on` is added.
- **Gated v3.3 resume:** `ckpt-merge ... --fingerprint v33` (uniform plan only), then the gated v3.3 binary with
  `--checkpoint <dir>/merged33`.
- **Finding 7.1 (MAIN rules on it):** with `--cube-extra`, the RESULTS of a resumed or sharded run differ from a single run in ONE
  counter line, "merged into N day loads". Every data file is identical.

## 9. Proof on the Mac, 16-17 Nov 2026 (ship:shard-memory)

**The Mac build of this exact tree.**
- A fresh target directory, `nice -n 10 cargo build --release --locked --offline -j 4 -p spikefade`, with the two values of section 4:
  rc 0 in 2 min 48 s (MEASURED; loaded Mac).
- The binary: 5,258,232 bytes, sha256 `378cc5bbaac32bc6abd69e7b52f24206a34f0027a6426b0028c1e1f5d592b700`. It holds no path of the
  app's tree.
- The suite: 195 passed, 0 failed, 2 ignored with the 3 parameter tables; on this code-only tree without them, 188 passed with
  exactly the 7 named tests failing, and 188 / 0 / 2 / 7 filtered out with section 5's skips.

**How this tree was made.** The determinism fix was folded into E10's tree by a 3-way merge (base: the shard kit's tree; `git merge-file`
and `diff3 -m` agree; E10's own patch applied to the det files gives the same files), then cut to this code-only form: every one of the
312 files equals the folded tree's, and the 303 not touched by E10 equal the det branch's.

| check (binary `378cc5bb`, one heavy run at a time, nice 10) | index (2023-03-28 + 29, `--cube-extra`) | stock (2019-12-31 + 2020-01-01) |
|---|---|---|
| P0, every switch off, vs the gated v3.3 binary `8d9a5b67` and vs the kit's `plain2` | 5 of 5, 3 of 3, 8 of 8 IDENTICAL, each | 5 of 5, 3 of 3, 8 of 8 IDENTICAL, each |
| the kit's plan with batch files, every E10 switch off: bodies vs `plain2`'s reference | 41 of 41 IDENTICAL | 32 of 32 IDENTICAL |
| in-memory plan with R1, R2, both, neither: = the kit's plan; `levels-causal.tsv` = plain2's | 4 of 4 | 4 of 4 |
| SHIP_W writer: its plan minus `SETUP` = the kit's plan; `setup-check` | IDENTICAL; PASSED (v2 `255ef05a...`) | IDENTICAL; PASSED |
| G7: the in-memory set-up checked against the writer's plan (R2 off) | IDENTICAL | IDENTICAL |
| 4 SHIP_R shards at 1-4 threads (whole cubes with `--setup-audit on`, E8 slices, E9 transport, margin TABLE files unreachable) vs the reference with the E10 plan | 41 of 41 bodies IDENTICAL; violations 0, 0, 0, 0; slice guard 0 | 32 of 32 IDENTICAL; violations 0; slice guard 0 |
| `setup-stats` U_k\* per batch vs the shards' own E10 MEM lines | 41 of 41 IDENTICAL | 32 of 32 IDENTICAL |
| merge of the 4 shard folders (`--expect-source daed6a59...`), canonical, vs the reference | IDENTICAL `3d52a5a1...` | IDENTICAL `058f990e...` |
| ONE resume of the merge with this binary, and on the gated v3.3 binary | RESUMED, 0 walked; 5 of 5, 8 of 8 (finding 7.1 masked) | the same |
| refusals (P-E7) and interruptions (P-E8: writer killed, `.part` refused, re-run with the same plan, shard killed) | 20 of 20 PASS; 4 of 4 PASS | 20 of 20 PASS; 4 of 4 PASS |
| MEASURED memory of the SHIP_R proof shards (max RSS; `E10 MEM ... start-up` RSS) | 1.25-1.50 GB; start-up 70-730 MB | 0.43-0.63 GB; start-up 170-370 MB |

**C2 and C2-ship.**
- C2 (section 6) verbatim: **IDENTICAL** (the 4 bodies) on the Mac (47 s) and "Linux-like" (`/opt/homebrew/bin/zstd` denied by
  `sandbox-exec`, zstd only on PATH; 41 s).
- C2-ship on the shard kit's synthetic set (the SHIP_W writer, then 4 SHIP_R shards at 1-4 threads, one on E9 transport files): the
  writer's plan minus `SETUP` = `PLAN-syn.tsv`; `setup-check` PASSED (v2 `7cefdcaa...`); 25 of 25 bodies = the reference with the E10
  plan; violations 0; the merge canonical IDENTICAL; the ONE resume 5 of 5. The same again Linux-like.
- Section 6b verbatim (the run files above): every step rc 0, `SETUP CHECK PASSED`, violations 0 and the 4 bodies of section 6b, on the
  Mac and Linux-like. The first Linux-like try refused at `setup-check` (zstd from the Mac path): hence `--portable on` in section 3,
  section 6b and the cloud worker script.

**Tests and mutants.** Mutants, each built into the test binary and its named tests run: **54 of 54 KILLED**, 0 survived (the 2 added in
this revision, `m_det_bfiles_implies_pmath` and `m_e10_mem_now_batch_facts`; E10's 21; the shard kit's 19; the determinism fix's 12).

## 10. Not proven yet

- **Nothing of this revision has run on Linux.** C2 and C2-ship = IDENTICAL on Linux are PREDICTED from the Mac's "Linux-like" runs.
  The det branch's Linux re-check (Variant B) is MAIN's gate before any canary of this branch.
- **The full-scale store does not exist yet**: the every-year SHIP_W plan step has not run (MAIN rulings pending: the UDiFF bhavcopy
  folder ends 2026-09-30; this build is v3.3 + kit + E10, not round D's v3.4 final; the Mac writer's memory allowance).
- **No full-scale per-shard memory is MEASURED**; every full-scale figure above is an ESTIMATE until the Linux memory canary.
- **Not run:** P-S5, P-S6, critic F6-F11 at full scale; no PGO build.
- **With PMATH on, report Greeks are not bit for bit the live app's `/greeks.json`** (the 8b proof holds with PMATH off).

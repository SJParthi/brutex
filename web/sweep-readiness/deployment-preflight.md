# Read-only staged-release preflight

`deployment-preflight.rs` verifies the exact files named in an operator-reviewed
release manifest. Its offline mode performs no network operation; its live
phase takes four bounded read-only status snapshots. It does not
launch a process, modify configuration, acquire a writer lease, open a browser,
restart the service, resume a paused job, or start a sweep. A passed artifact
check is **not** permission to replace the active service.

The native observer links already-built Rust libraries. The backend never
builds, imports, or executes this tool. No new crate dependency is introduced.
It supports the same verified final-component file-open flags as the immutable
readers: macOS and Linux x86_64/aarch64. Other targets refuse compilation.

## Build and run

Choose `api`, `brutex_core`, `serde` with derive enabled, and its matching
`serde_json` rlibs from the **same reviewed native build**. Do not select an
arbitrary last glob match when different feature variants exist in `deps/`.
The following is the retained build used for the initial standalone tests;
relink to the new final release before its activation check:

```sh
rustc --edition=2024 -D warnings web/sweep-readiness/deployment-preflight.rs \
  -L dependency=/private/tmp/brutex-qualified-final-build-20260907/debug/deps \
  --extern api=/private/tmp/brutex-qualified-final-build-20260907/debug/deps/libapi-c3f16203ecf03252.rlib \
  --extern brutex_core=/private/tmp/brutex-qualified-final-build-20260907/debug/deps/libbrutex_core-dafd3a937446e6fc.rlib \
  --extern serde=/private/tmp/brutex-qualified-final-build-20260907/debug/deps/libserde-6d432c369340d0ab.rlib \
  --extern serde_json=/private/tmp/brutex-qualified-final-build-20260907/debug/deps/libserde_json-485fba120653ff77.rlib \
  -o target/sweep-audit-20260906/deployment-preflight
```

Add `--test` to compile its tests and run that executable with
`--test-threads=2`. Original focused tests: thirteen passed, comprising eight
preflight tests, three shared observation-budget tests and two required search
replay-budget tests (`deployment-preflight-tests-v2.log` under the ignored audit directory).
These are finite guard tests, not complete coverage/mutation clearance.
The first D-0554 extension had **24 passing native tests** on its scoped source
(`deployment-gates-tests-final.log`), and a clean standalone Clippy invocation
with warnings denied (`deployment-gates-clippy-final-v2.log`). Those counts
include the five reused configuration-parser tests. This is not a measurement
of whole-crate coverage or a full mutation census of this staging utility.

The subsequent terminal-settlement and caught-phase fixes had **31 passing
native tests**, including the same five reused configuration tests
(`deployment-phase-tests.log`), plus clean standalone Clippy with warnings denied
(`deployment-phase-clippy.log`). Two bounded isolated mutations were caught:
removing the final settlement recheck failed both late-change tests
(`deployment-settlement-replay/terminal-mutant.log`), and ignoring caught-phase
validation failed both caught-summary refusal tests
(`deployment-phase-replay/phase-mutant.log`). Both mutated sources and failure
logs are retained separately. These two selected replays do not establish a
full-module mutation census or whole-crate coverage.

The duplicate-key parser fix has **35 passing native tests**
(`deployment-json-final-tests.log`) and clean standalone Clippy with warnings denied
(`deployment-json-final-clippy.log`). A separate mutation restoring last-key-wins
object decoding compiled and failed the three affected duplicate-input tests
(`deployment-json-replay/json-mutant.log`). This is one additional selected
fault replay, not a complete mutation or coverage measurement.

Invoke the tool with exactly one JSON plan file, optionally preceded by
`--offline` and `--require-release-gates`. The first flag prevents every network
callback. The second requires mandatory release evidence before even observing
live status. JSON plans are local ignored
artifacts or live under `web/`; they are never backend configuration files.
The environment must contain the same effective observation budget and explicit
`BRUTEX_BOOLEAN_SEARCH_REPLAY_NODES` allowance as the
manifest, using the actual `BRUTEX_BOOLEAN_OBSERVATION_BYTES` parser. For example,
the previous qualified-result proof used `1610612736` bytes, derived from its
declared producer receipt budget. This is a **serialized ancestry limit**, not
RAM reserved or peak RSS. A new run must retain its own measured requirement.

Exit **1** means artifact/configuration refusal. Exit **3** means staged bytes
passed but required release evidence is missing, stale, incomplete or failing.
Exit **2** means staged bytes
were verified but coordinated service handoff is still required; the structured
output records active/paused/unknown observations. There is deliberately no exit
code meaning “restart automatically.” Standard output is the only output file
stream; redirect it to an ignored evidence file to retain the observation.

## Version-one manifest

The version-one root object has exactly these fields; unknown and duplicate fields refuse.
All byte counts are canonical positive decimal **strings**, retaining exact
integers. `source_commit` is the full lowercase 40-character source stamp.

| Field | Required meaning |
|---|---|
| `schema_version` | Integer `1`. |
| `source_commit` | The reviewed clean build's commit. This is declared provenance; the tool does not infer an embedded commit by searching executable strings. |
| `artifact_bytes` | Complete scan ceiling for all listed artifact bytes. |
| `observation_bytes` | Actual planned server aggregate Boolean observation limit. |
| `replay_nodes` | Required positive search-history replay node allowance, independently configured on the server. |
| `required_evidence_bytes` | Previously authenticated complete ancestry requirement for the intended receipts, measured with their actual reader. The manifest cannot invent this measurement. |
| `web_root` | Existing absolute frontend root; its `build/` directory is checked completely. This is the value supplied to `BRUTEX_WEB`, not the nested `build/` directory. |
| `source_store` | Existing absolute historical input store. Merely checking the directory does not re-attest its bars. |
| `output_root`, `server_store` | Existing absolute evidence output and dashboard store roots. Their canonical paths must match. No fallback folder is searched. |
| `masters_root`, `logs_root` | Existing absolute runtime directories. This read-only check does not perform a write or sync test. |
| `address` | Explicit nonzero loopback socket, for example `127.0.0.1:8080`. |
| `artifacts` | At most 4,096 entries. Exactly one each has `role` equal to `api`, `cli`, or `checks`; every remaining entry has `role` equal to `web`. Each entry also has absolute `path`, exact `bytes` string, and raw-file lowercase 64-character `blake3`. |

The `checks` artifact is the retained verifier's `evidence.js`, not a rewritten
summary or a focused test log. It must say complete and stable and contain the
seven successful full checks in their exact order. The check artifact's own
bytes are pinned in the reviewed manifest. Its FNV source-change fingerprint
is not upgraded into a cryptographic source attestation by this tool.

Every regular file beneath `web_root/build/` must appear once in the manifest.
Duplicate aliases, missing/extra files, final symlinks, nonregular entries,
directory depth above sixteen, or more than 4,096 total directory entries
refuse. `index.html`, `backtest.html` and `_app/version.json` are required.
The root's trusted clean build creates this list; do not regenerate expected
digests from a suspect deployment and present the new manifest as verification
against the original reviewed release.

## Mandatory acceptance evidence (D-0554)

The seven ordinary checks are necessary but do not imply whole-crate line/branch
coverage or full-module mutation closure. Every result now contains
`mandatory_gates` and `activation_admission`. Missing proof says `blocked`,
including for historical version-one stage plans. Historical stage-only
generation is unchanged and remains useful. It never becomes activation approval.

For a release-admission check, use:

```sh
deployment-preflight --offline --require-release-gates PLAN.json
```

Version **two** adds `mandatory_gates` with these required fields:

| Field | Meaning |
|---|---|
| `source_root` | Canonical clean source snapshot that produced the reports. |
| `crates` | Nonempty exact reviewed list of touched crate directory names. |
| `modules` | Nonempty exact reviewed list of touched Rust module paths relative to `source_root`. |
| `sources` | Complete source census: root `Cargo.toml` and `Cargo.lock`, each named crate's `Cargo.toml`, and every `.rs` file recursively beneath each named crate. Each entry has relative `path`, canonical decimal `bytes`, and lowercase raw-file `blake3`. |
| `evidence` | Pin of the measurement receipt: absolute `path`, decimal `bytes`, and `blake3`. |

The source scan verifies bytes with held handles and requires exact census
equality. Missing files, added files, aliases, symlinks, changed generations and
oversized inputs refuse. The reviewed scope is a coordinator responsibility:
the observer cannot infer every touched crate from an arbitrary checkout or
upgrade a deliberately incomplete scope declaration into full release proof.
Do not choose the scope from whichever tests happened to pass.

The measurement receipt has exactly `schema_version: 1`, `source_commit`,
`scope_digest`, `complete: true`, `coverage`, and `mutations`. The source commit
must equal the plan's full source stamp. `scope_digest` is produced by the native
`gates::scope_digest` helper before measurement. Its BLAKE3 input is the
length-framed UTF-8 domain `brutex-deployment-gates-v1`, then the commit; the
crate count and its framed strings; the module count and its framed strings;
the source count and each source's framed path/byte-string/hash. Counts and
string lengths are little-endian u64. Preserve the reviewed ordering exactly.
The source inventory ties this receipt to exact source bytes; trusted capture
of the tool runs remains necessary, just as for the ordinary seven-check receipt.

Raw LLVM reports, mutation census/outcomes, the seven-check projection and status
response JSON reject duplicate object keys at every nesting level. Escaped keys
are compared after decoding, so `covered` and `\u0063overed` cannot overwrite
each other. Unambiguous extra tool metadata remains supported. Signed and
unsigned 64-bit integers retain their exact values, including values above
JavaScript's precise-integer range; decimal metadata retains serde_json's prior
floating representation. Coverage counts still require integer values: equal
floating, exponent-form, negative or overflowing counts do not become 100%
coverage through rounding. Existing
input-byte caps and serde_json's finite default recursion limit remain enabled,
and trailing JSON content refuses. A duplicated status field becomes unknown;
it cannot turn an active observation into an inactive one.

Each coverage entry has `crate_name`, `full_crate: true`,
`branch_instrumented: true`, and a pinned `report`. The report must be a native
LLVM JSON export. Every source Rust file must appear exactly once under the
declared snapshot path. Every file's integer line and branch `count` must equal
its integer `covered`; a rounded percentage of 100 is ignored. At least one
line must be measured per crate. A missing source file, missing metric,
line-only instrumentation, partial crate, or omitted crate blocks admission.
Files LLVM did not report are unproved here, even if a reviewer expects them
to contain only declarations; this observer has no silent exclusion list.

Each mutation entry has `module`, `full_module: true`, pinned `census` and
`outcomes`, and `compiler_failures` (an array, empty when unnecessary). The raw
`cargo mutants` emitted census and outcome identities must match exactly,
without duplicate, missing, foreign or extra cases. A clean baseline must
contain successful Build and Test phases. Every viable case must be
`CaughtMutant`. A missed mutant, timeout, error or unknown classification blocks.
The caught label alone is insufficient: it requires exactly an ordered successful
`Build`, then completed `Test` with `{"Failure":101}`. This is the genuine native
`cargo test` shape in the retained grammar and search-allocation cargo-mutants
outcomes. Normal duration/argument metadata is preserved. Missing, truncated,
reordered or extra phases, a successful test, timeout/signal/error status, or an
unsupported failure code blocks. The check authenticates recorded phase results;
trusted capture of the underlying run and its failure cause remains necessary.

The newer native mutation driver's TSV is a different evidence format. It does
not record a separate Build/Test pair or numeric test exit, so it cannot be
silently converted into this cargo-mutants format. It requires a separately
reviewed named adapter with its own provenance and completion checks; until then
this admission path refuses it. A raw driver's passing summary does not provide
fields that the driver never measured.
An equivalence assertion is not a success classification: retain that review,
but resolve the guard/refactor/test or retain the release block. Do not relabel
an equivalent candidate as caught without its actual replay failing.

Compiler-invalid mutations are separately accountable. An `Unviable` outcome
requires exactly one completed Build failure (101), plus a pinned log in
`compiler_failures` with its exact `mutant_name`. That log must contain the
same mutation header, a Rust `error[Edddd]` diagnostic at the exact mutated
source line, and the completed failure trailer. Missing diagnostics, foreign
lines/names, timeouts, known infrastructure failures and unused/duplicate
compiler-evidence entries refuse. Compiler-invalid cases are not called caught
or surviving. Unsupported compiler diagnostics remain unresolved; they cannot
be cleared by a free-text explanation.

Physical limits are explicit: 4,096 scope/source entries; 64 MiB total source
bytes; one MiB receipt; 64 MiB per report/log; and 256 MiB total measurement
receipt/report/log bytes. These are serialized admission bounds, not reserved
RAM or filesystem latency guarantees. Artifact, source and report handles remain
retained across all phases until the final JSON response is printed. Complete
source and dashboard inventories are rescanned at terminal settlement, so a new
unmeasured `.rs` file or unpinned build file also refuses. Each inventory retains
its initial entry and depth limits, with at most two rescans after initial
capture in a preflight invocation. Overall work is bounded scanning, not O(1).
Expected digests and inventories are never refreshed to excuse a late change.
No coverage tool, mutation tool, shell process,
service or browser is launched by this verifier.

The standalone tests cover generated source and report fixtures, not a new
whole-crate coverage measurement. The current real version-one stage was
checked offline: 85 pinned artifacts / 34,464,306 bytes passed, then mandatory
evidence correctly blocked with exit 3 (`deployment-gates-existing-stage-final.log`).
It made no live request. This does not claim that the newly edited source has
the old stage's commit or ordinary gate receipt.

## What is and is not checked

Files are hashed with a fixed 64 KiB buffer, checked against exact length and
BLAKE3, and retain their original handles and filesystem generations until the
complete response has been settled. Terminal checks cover late file changes and
the complete inventories. These observations are not an atomic writer lease and
do not freeze future edits. Directory ancestry must remain
trusted; this is not an `openat` sandbox. Filesystem/device blocking latency
cannot be guaranteed by user-space read time limits.

The tool uses GET only for `/pull/run.json`, `/autopilot.json`,
`/backtest/run.json`, and `/pull/recovery.json`. Each request has a five-second
wall-clock read budget and a one-MiB complete-response ceiling; network reads
have explicit timeouts. Non-200, malformed, duplicate-length, partial, oversized
or transfer-encoded responses become **unknown**, not idle. Only fixed state
labels are projected; raw response bodies and private runtime details are not
printed or copied into the plan.

Recovery's existing endpoint is a recent-event view, not an authoritative
writer census. A null sweep status also does not prove every external command
idle. Even a paused scheduler may coexist with a direct pull. These limitations
are preserved in the output. A final coordinated check with the active work's
owner is necessary before replacing its process; the observer never sends a
STOP/Resume or edits durable pause intent to manufacture an idle result.

## Reversible activation contract

The coordinator first finishes the newly required full checks and captures a
clean source/binary/bundle manifest. Stage those artifacts in a new immutable
release directory. Copy and pin the current executable **and its complete current
frontend tree** as rollback artifacts; retaining only the executable is not a
frontend rollback. Preserve the existing store, masters, log roots, local
credential-path configuration, and explicit pause settings without displaying
credentials or private parameter paths.

Run this preflight over the new stage. Keep the current service alive until the
ingestion/recovery owner confirms a safe handoff and the fresh status checks do
not contradict that confirmation. Use the existing service manager to replace
its executable and `BRUTEX_WEB` root as one coordinated change; do not run a
second API against the same store or bypass its `serve.lock`. Startup currently
invokes recovery resume, so a second “preview” server is not an inert observer.

After authorized activation, verify the exact running executable, loopback
listener, frontend artifact paths and all selected real receipt endpoints. A
failed activation rolls back the **service artifacts/configuration only**;
append-only store and journal history is never reverted or deleted. This tool
does not implement that activation step or claim it has occurred.

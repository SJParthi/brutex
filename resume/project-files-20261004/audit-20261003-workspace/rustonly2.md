# rustonly2: is brutex Rust-only outside web/ at 1087e54? (CLAUDE.md §2)

**Verdict.** Every tracked file at HEAD complies. All 659 tracked files outside web/ (969 in total) have an allowed extension for where they live:
- .rs 590, .md 46, .toml 18, .lock 1, .gitignore 1
- .yml 2, both under .github/workflows/
- .json 1, .claude/launch.json

All 969 tracked files are mode 100644. There are no symlinks, no gitlinks, no submodules, no .gitattributes and no LFS. No `.rs` file outside web/ opens with a shebang.

The binaries and the dependency graph are also compliant:
- On x86_64-linux the host build graph (`cargo tree -e normal,build,dev`, 203 packages) holds no native-code crate, no FFI-to-runtime crate and no interpreter crate.
- The built `api` and `cli` binaries list only libgcc_s, libm, libc and ld-linux as NEEDED.
- No build script left a .o, .a or .so file.
- No crate includes or `#[path]`s into web/.
- The one tracked build script (crates/cli/build.rs) starts no process.

Four of the prior pass's five findings are fixed or documented. rustonly-2 is half fixed: the fingerprint now hashes dependency edges, but a version-only change still leaves it unchanged.

The weak points are the guards. They pass non-compliant input I built:
- Gate 1 accepts a `.rs` file whose content is not Rust when it sits under `.github/` or in a crate directory that is not a workspace member.
- Gate 0 refuses `node -e` but accepts 8 other ways of running an inline program.
- Gate 2's token check misses build scripts that cause a program to run without naming `Command`.
- Gate 1g's environment regex misses `RUSTFLAGS` linker settings, `CARGO_HOME` and nextest `--config-file`.
- Nothing refuses `sh` or an interpreter called by absolute path from test or production code.

None of these is exploited in the tree today.

## Re-verification of the prior report (prior/rustonly.md @ bc53131)

| prior id | prior claim | at 1087e54 | evidence |
|---|---|---|---|
| rustonly-1 | `no_declared_native_dependency_is_compiled_any_more` cannot fail (literal `compiled_on_this_target: false`) | **FIXED-since-prior** (D-1435) | crates/vocab/tests/workspace_is_rust.rs:576-632 now walks Cargo.lock edges from the 13 members (`reachable_from_workspace` :254, `declared_native_reached` :305). Only the 3 named `LOCK_ONLY_EDGES` (:174-194) are skipped. It has a self-test that can fail: `a_synthetic_lock_where_a_member_reaches_ring_is_reported` (:666) asserts `["ring","cc"]`. Ran it: 7/7 ok. |
| rustonly-2 | The fingerprint hashes names only, so a feature flip or version bump passes; gate 13 passes with ring compiled | **Partly FIXED.** Feature flip: FIXED (fingerprint now hashes each package's `dependencies` list, :320-338, pin `0xA685_8949_AA8A_D9CE` :549; gates 13b/13c added, ci.yml:6986-7112). **Version-only bump: still undetected** (rustonly2-6). | Probe output below. |
| rustonly-3 | 06-limits said no build script exists | **FIXED-since-prior** (D-1204) | docs/06-limits.md:6135-6140 now names `crates/cli/build.rs` and D-0348. It is guarded by `cli::limits_doc_drift::section_96_names_the_one_build_script_gate_13_allows` (docs/04-invariants.md:5348). |
| rustonly-4 | `api serve` spawns `xdg-open` (a sh script on xdg-utils hosts) and no doc says so | **DOCUMENTED:D-1202 choice 5** | crates/api/src/server.rs:17976-17992 (doc `# On Linux the handler may be a script, and that is allowed (rustonly-4, D-1202)`); :18038 `BrowserHost::Other => ("xdg-open", &[])`; :18058 `std::process::Command::new(program)`; docs/05-decisions.md:51587. |
| rustonly-5 | ci.yml has inline JS (`node -e`, 86 lines) plus npm/node | **FIXED-since-prior** for the inline program (ET-rust-only-purity-9, D-1100). Gate 0 refuses `node -e`. Node is now run only by path on web/ files (ci.yml:7712 `node --test web/tests/*.test.js`, :7785 `node web/ci/css-comments.mjs`). The guard has holes: rustonly2-2. | `grep -n 'node -e' ci.yml`: none. |

## Inventory at HEAD (commands run)

- `git ls-files | wc -l`: 969. Outside web/: 659.
- Outside web/ by extension: `.rs 590, .md 46, .toml 18, .yml 2, .lock 1, .json 1, .gitignore 1`.
- Per area:
  - root: md 5, toml 3, lock 1, .gitignore 1
  - .claude: json 1
  - .config: toml 1
  - .github: rs 3 (source_scan.rs is new since prior), yml 2
  - config: toml 1
  - docs: md 40
  - crates/*: rs + one Cargo.toml each
  - crates/store: also md 1
- `.yml` outside web/: only `.github/workflows/{auto-merge,ci}.yml`. `.json`: only `.claude/launch.json`. No `.yaml`.
- `git ls-files -s | awk '{print $1}' | sort | uniq -c`: `969 100644`. There are no 120000 (symlink), 160000 (gitlink) or 100755 entries.
- `.gitmodules` and `.gitattributes` are absent. `git lfs ls-files` prints nothing.
- `git status --porcelain --ignored` at the start printed only `!! target/`. Nothing was untracked and not ignored. (Later, other workers' zz_audit probes appeared; none of them are mine.)
- web/: js 209, svelte 38, html 22, css 14, rs 10, mjs 5, json 4, md 4, svg 2, ts 1, .gitignore 1. Exempt by D-0052/D-0053.
  - The 10 web/*.rs files (`web/sweep-readiness/*.rs`, `web/saved-backtest/viewer.rs`) are not workspace members. No crate includes them.
- Shebangs outside web/: no tracked `.rs` starts with `#!` other than `#![`. Every `#!/` hit is inside a string literal:
  - .github/source_scan.rs:2264 and :2762 are test fixtures.
  - crates/cli/build_provenance.rs:2861-3098 writes test fixture bytes `b"#!/bin/sh\n"`.
  - ci.yml:283 is gate 1e's stub generator.
- Non-UTF-8 or binary content outside web/: none. I ran `file --mime` over all 659 files.

## Processes started from crates/** and .github/*.rs

| Program | Where | Production? |
|---|---|---|
| `xdg-open` / `open` / `explorer.exe` | crates/api/src/server.rs:18058 | yes. DOCUMENTED D-1202 |
| `mkfifo` | api/src/census.rs:1349 (in `mod tests` from :1099); pull/src/ingest.rs:3606 `/usr/bin/mkfifo` (in `mod tests` from :3329); cli/src/readonly_file.rs:98 (tests from :63); cli/src/checksum_receipts_tests.rs:306; store/src/checksum_audit_tests.rs:129; store/tests/fifo.rs:102 | test only |
| `git` | core/tests/findings.rs:488,522,592,626,679,719; store/tests/cited_commits.rs:195,223,239,623,666,754 | test only |
| its own test binary (`current_exe()` / `CARGO_BIN_EXE_*`) | about 30 sites in api and cli | test only |

- No crate spawns `sh`, `bash`, python, node, npm, perl or ruby (grep `Command::new\("(sh|bash|/bin/sh|/bin/bash)"`: 0 hits).
- crates/cli/build.rs (102 lines) and its modules start no process. It reads git object files to stamp the commit and skips web/ (build_provenance.rs:731-742).

## The web/ toolchain boundary

- No `include_str!`, `include_bytes!` or `include!` reaches into web/. The only hit is a doc comment at crates/api/src/assets.rs:829 that says the include *used to* exist.
- `#[path]` stays inside crates (for example cli/src/lib.rs:78,80 and pull/src/lib.rs:218).
- `api` reads the tracked `web/build` at request time, which §2 allows.
- The build script excludes `web/` from its inputs (build_provenance.rs:742 `path.starts_with("web/")`).

## Dependency graph (run after BUILD_DONE)

- Host: `cargo tree --workspace --locked --offline -e normal,build,dev --prefix none --format '{p}' | sort -u` gives 203 packages.
  - grep for `ring|cc|cmake|bindgen|pyo3|napi|mlua|v8|deno|wasm-bindgen|js-sys|web-sys|openssl|aws-lc|libz|*-sys`: **no hits**.
- All targets: `cargo tree --target all ...` gives 276 lines. Native or binding crates appear only for other targets:
  - `cc v1.4.0`
  - `iana-time-zone-haiku` (Haiku)
  - `core-foundation-sys` (macOS: iana-time-zone ← chrono ← parquet ← lake)
  - `windows-sys`
  - `wasm-bindgen`, `js-sys`, `web-sys`, `wit-bindgen` (wasm)
  - `ring` is absent from the all-target tree.
- Proc-macros on the host: displaydoc, seq-macro, serde_derive, tokio-macros, yoke-derive, zerocopy-derive, zerofrom-derive, zerovec-derive. All are Rust and none is a workspace member.
- Build scripts compiled on the host (`target/debug/build`):
  - ahash, cli, crc32fast, crossbeam-{deque,epoch,utils}, generic-array, getrandom, httparse, icu_{normalizer,properties}_data, libc, libm, num-traits, proc-macro2, quote, rayon-core, rustls, serde, serde_core, serde_json, zerocopy, zmij.
  - Those that call `Command::new` in their registry `build.rs`: crc32fast, getrandom, httparse, libc, proc-macro2, quote, serde, serde_core, zerocopy, zmij.
  - Most run `rustc`. libc also names `freebsd-version` and `emcc`, both on other targets only. DOCUMENTED:06-limits §96 (docs/06-limits.md:6102-6141).
- `find target/{debug,release}/build -name '*.o' -o -name '*.a' -o -name '*.so' ...`: nothing.
- `readelf -d target/debug/{api,cli} | grep NEEDED`: `libgcc_s.so.1, libm.so.6, libc.so.6, ld-linux-x86-64.so.2` only.
- No member manifest has `links =` or `build =`. Gate 2 refuses both keys (ci.yml:1797-1817).
- deny.toml `[bans] deny` names 31 crates, including ring, aws-lc-rs, aws-lc-sys, cc and openssl-sys. Its `[sources]` refuse unknown registries and git sources. Its graph targets are x86_64-linux, aarch64-darwin and wasm32. UNVERIFIED by run: `cargo deny` is not installed.

## Guards for §2 at HEAD

| Gate | What it checks (ci.yml line) | Bypass found? |
|---|---|---|
| 0 | builds source_scan; workflow hygiene and inline interpreters (:28) | YES, rustonly2-2 |
| 1 | extension by path, mode 100644, content (NUL, UTF-8, `.rs` shebang, browser code in html/css), orphans (:66) | YES, rustonly2-1, rustonly2-5 |
| 1e | build and test with 19 tool names shadowed on PATH (:230) | `sh`, `bash` and absolute paths are not shadowed, rustonly2-4 |
| 1b | .json/.yml only under .github/, .claude/, web/ (:367) | no (`.claude/sub/x.json` and `.github/ISSUE_TEMPLATE/x.yml` pass, which the letter of §2 allows) |
| 1g | no tracked .cargo/, key allowlists for rust-toolchain and nextest, workflow env regex (:398, regex :461) | YES, rustonly2-7 |
| 1f | browser code in Rust string literals (:470) | not re-probed (v2 audit: FIXED) |
| 2 | token scan of the build-script closure; `build =`/`links =` keys (:1713) | YES, rustonly2-3 |
| 13 / 13a / 13b / 13c | names in manifest/lock; wasm bindings unreachable; banned names in host `cargo tree`; build scripts' `linked_libs` and objects in `out_dir` (:2023, :6967, :6986, :7053) | 13c reads only `linked_libs` and `out_dir` objects (info, rustonly2-8) |
| vocab `workspace_is_rust` | forbidden names; pinned count 192 plus edge fingerprint; edge walk | version-only change is invisible, rustonly2-6 |

## Probes and experiments (all in scratch, except one deleted vocab probe)

**P1: Gate 1 on a synthetic repo.**
- Method: I extracted ci.yml:68-228 unchanged into `ro2/gate1.sh` and ran it with `SOURCE_SCAN` set to a scanner built by `rustc --edition=2024 -D warnings .github/source_scan.rs`. The repo was a fresh `git init` in scratch.
- Round 1 files: `crates/x/.gitattributes` (Python text), `crates/x/src/tool.md` (Python shebang), `.claude/sub/deep.json`, `.github/ISSUE_TEMPLATE/x.yml`, `foo.RS`, `docs/CODEOWNERS` (shell text), `.github/extra.rs`. Output:
  ```
  FORBIDDEN FILE  foo.RS  (extension RS, outside web/)
  read 9 tracked file(s)
  ... rc=1
  ```
  Only the uppercase extension was refused.
- Round 2: removed `foo.RS`, added `.github/evil.rs` containing `import os\nos.system("echo pwned")`, then replaced the tests file with `crates/zz/src/lib.rs` holding Python text and no Cargo.toml. Output:
  ```
  read 10 tracked file(s)
  OK — every tracked file is in the allowlist for where it lives, has
       an ordinary mode, carries no content its name hides, and every
       .rs outside web/ is compiled.
  rc=0
  ```
  `source-scan orphans` also returned rc=0.

**P2: Gate 0 workflow scan.** Ran `source-scan workflow` on a synthetic workflow (`ro2/wf_bypass.yml`). Only the control was flagged:
```
wf_bypass.yml:5: `node -e` runs a program written inline
rc=1
```
These 8 forms were not flagged:
- a python3 heredoc (`python3 - <<'PYEOF'`)
- `perl -ne`
- `node --eval='..'`
- `python3 -Bc`
- `echo .. | node`
- `awk 'BEGIN{..}'`
- `"node" -e`
- `node -pe`

**P3: Gate 2 `source-scan build`.**
- Control (`std::process::Command::new("npm").status()`) was refused with 3 findings, rc=1.
- All of these gave rc=0:
  - `b_cfgwrite.rs`: `std::fs::write(<manifest>/../../.cargo/config.toml, "[build]\nrustc-wrapper = \"/bin/sh\"")`
  - `b_dep.rs`: `duct::cmd("sh", ["-c","echo hi"]).run()`
  - `b_linkarg.rs`: `println!("cargo::rustc-link-arg=-fuse-ld=/tmp/anything")`

**P4: Gate 1g environment regex.** Ran grep with the exact pattern from ci.yml:461 over 7 lines. Only `RUSTC_WRAPPER=x cargo build` and `cargo build --config build.rustc-wrapper=x` matched. These did not:
- `RUSTFLAGS: "-C linker=/bin/sh"`
- `RUSTFLAGS="-Clinker=./x.rs" cargo build`
- `CARGO_ENCODED_RUSTFLAGS=...`
- `cargo nextest run --config-file crates/x/n.toml`
- `CARGO_HOME=crates/x cargo build`

**P5: vocab fingerprint, version-only bump.**
- Method: `crates/vocab/tests/zz_audit_rustonly2_1.rs` `include!`s the test body (with the `//!` lines stripped). It changes ring's `version = "0.17.14"` to `0.17.99` in an in-memory copy of Cargo.lock.
- Run: `cargo test -p vocab --test zz_audit_rustonly2_1 zz_probe -- --nocapture`:
  ```
  base=0xa6858949aa8ad9ce after_ring_version_bump=0xa6858949aa8ad9ce equal=true
  declared_native_reached after bump: Ok(0)
  test zz_probe_version_only_bump ... ok
  ```
- The same binary ran the 6 real tests: `7 passed; 0 failed`.
- Both probe files were deleted. Final `git status --porcelain` shows none of my files; the `?? zz_audit_*` entries listed there belong to other workers.

## Findings

| id | sev | crate | file:line | What is wrong | Evidence | Status |
|---|---|---|---|---|---|---|
| rustonly2-1 | low | ci | .github/source_scan.rs:1897 (`[".github", file] => file.ends_with(".rs")`), :1898 (`["crates", _, "src", "lib.rs" \| "main.rs"] => true`); ci.yml:224-227 | Gate 1's orphan check counts **every** `.github/*.rs` as compiled, and every `crates/*/src/lib.rs` too, even in a directory that is not a workspace member. CI compiles only 3 named `.github` tools (ci.yml:40,42,4985,5003,7448,7450,7585 are the only `rustc .github/` lines), and the root Cargo.toml `members` is an explicit list of 13. So a tracked `.github/notes.rs` or `crates/zz/src/lib.rs` holding Python passes gate 1, which then prints "every .rs outside web/ is compiled". The content is inert (nothing runs it), but it is a non-Rust file under a Rust name, and the gate's success line is false. | P1 round 2, rc=0 with OK message | NEW (ET-rust-only-purity-3 fixed the shebang and orphan case; this is the case where the file is never compiled yet still counts as a root) |
| rustonly2-2 | low | ci | .github/source_scan.rs:1855-1866 (`workflow_findings` inline matcher) | Gate 0 says a program handed to an interpreter inline "is a second language in a tracked file outside web/" (ci.yml:51-53). The matcher catches only the exact word pairs `node -e/--eval/-p/--print`, `perl -e/-E`, `php -r`, `python* -c` and `deno eval`. Heredocs, stdin pipes, combined flags (`-ne`, `-pe`, `-Bc`), `--eval=`, a quoted program name and awk programs all pass. ci.yml at HEAD has none of these except awk (73 non-comment awk lines). | P2: 8 bypasses, 1 caught | NEW (partial fix of KNOWN:ET-rust-only-purity-9) |
| rustonly2-3 | low | ci | .github/source_scan.rs:1066-1103 (`build_findings`); ci.yml:1713-1765 | Gate 2's token check refuses only *naming* process APIs. A build script can still cause a program to run without them: (a) write `.cargo/config.toml` at the workspace root (cargo reads it from ancestors) naming a `rustc-wrapper`, which the *next* cargo command runs; gate 1g checks tracked files only. (b) `cargo::rustc-link-arg=-fuse-ld=<path>` makes the linker run another program; gate 13c reads `linked_libs`, not link args. (c) A build-dependency that spawns on the script's behalf (`duct`, `xshell`). (c) also changes Cargo.lock, so the vocab count/fingerprint pin goes red until it is re-pinned. (a) and (b) need an edit to the allowlisted crates/cli/build.rs or build_provenance.rs. | P3: all three rc=0 | NEW |
| rustonly2-4 | low | ci / all crates | ci.yml:280-282 (stub list), :276-279 comment | Gate 1e shadows 19 names, but not `sh` or `bash`, and its comment says an absolute path "is still not shadowed here, which is why gate 2's token check is the one that carries the rule". Gate 2 reads only build scripts. So `Command::new("sh").arg("-c")` or `Command::new("/usr/bin/python3")` in **test or production** code passes every gate. None exists today: grep for `Command::new\("(sh\|bash\|/bin/sh\|/bin/bash)"` in crates/ and .github/*.rs gives 0 hits, and the only non-self spawns are mkfifo, git and xdg-open (table above). | ci.yml:280-282 quoted list; grep | KNOWN residual of ET-rust-only-purity-0 (fix-queue_lane2-b.md:7; fixed for build scripts only) |
| rustonly2-5 | info | ci | ci.yml:123 (`any_depth_names='\.gitignore\|\.gitattributes'`), :116-121; source_scan.rs:1935-1959 | `.gitignore` and `.gitattributes` pass at any depth with any content. A `.md`, `.toml` or CODEOWNERS file is checked only for NUL bytes and UTF-8. So `crates/x/.gitattributes` holding Python, or `crates/x/src/tool.md` with a Python shebang, passes gate 1, and the OK line says the tree "carries no content its name hides". All of these are inert: a filter or diff driver needs an untracked `.git/config` entry. | P1 round 1 | NEW |
| rustonly2-6 | low | vocab | crates/vocab/tests/workspace_is_rust.rs:320-338 (`fingerprint` hashes `p.name` and `p.deps`; `version` only sorts) | A version-only change to a package (ring 0.17.14→0.17.99) leaves the pinned fingerprint and the count unchanged. A new release of a crate that starts vendoring C under the same name is not flagged by this test. Gates 13b/13c would still catch it if the crate is *built* (13c by objects/linked libs). | P5: `equal=true` | KNOWN residual of prior rustonly-2 (version-only variant) / ET-rust-only-purity-5 |
| rustonly2-7 | low | ci | ci.yml:461 (gate 1g env regex) | Gate 1g says "The workflows themselves may not set the environment-variable or `--config` forms of the same settings". The regex misses `RUSTFLAGS`/`CARGO_ENCODED_RUSTFLAGS` with `-C linker=` (the same door as `target.<t>.linker`), `CARGO_HOME=<tracked dir>` (whose `config.toml` is any allowed `.toml` and is not under a `.cargo/` path), and `cargo nextest --config-file <file>` (a nextest config with `[scripts.setup]` under another name; the name check at :439-440 matches only `nextest.toml`). Each needs a workflow edit, and a workflow can run bash anyway, so impact is limited. | P4: 5 of 7 lines not matched | NEW |
| rustonly2-8 | info | ci | ci.yml:7053-7112 (gate 13c) | Gate 13c reads only `linked_libs` and object files in build-script `out_dir`s. Native code linked through a `#[link(name=..)]` attribute in a dependency's Rust source (no build script), or through a prebuilt archive outside `out_dir`, emits neither, so only 13b's name list stands in the way. Today: binaries NEED only libc, libm, libgcc_s and ld-linux, and no objects exist under target/*/build. | readelf output; find output | NEW (theoretical) |
| rustonly2-9 | info | docs | docs/06-limits.md:6105-6107 | §96 still says gate 2 "greps every tracked `build.rs` for `Command::new` and `std::process`". Since D-1100/D-1101 gate 2 is a token scan of the build script's whole module closure (ci.yml:1718-1765). | quoted lines | NEW |
| rustonly2-10 | info | law | crates (macOS target): core-foundation-sys ← iana-time-zone ← chrono ← parquet ← lake | On the operator's aarch64-darwin target the graph compiles `core-foundation-sys`, a Rust FFI declaration crate for Apple's C framework (the same kind as `libc`/`windows-sys`). It vendors no C and is not on any banned list. Whether §2's "vendored binding to another language" covers FFI declarations to OS libraries is a law question; `libc` is the same case on Linux and is clearly tolerated. | `cargo tree -i core-foundation-sys --target aarch64-apple-darwin` | NEW (info only) |

## CI YAML: embedded non-Rust counts (HEAD vs prior)

| measure | prior (bc53131) | HEAD (1087e54) |
|---|---|---|
| `run:` steps in ci.yml | 55 | 59 |
| `shell:` keys | 0 | 0 (so GitHub's default bash) |
| awk (lines mentioning / non-comment) | 78 | 85 / 73 |
| sed | 61 | 64 / 62 |
| node | 7 | 14 / 9 (mostly the gate 1e stub list and banned-name lists; invocations are :7712 and :7785, both on web/ files) |
| npm | 11 | 9 / 8 (Gate W: :7690, :7694, :7717, :7868) |
| inline `node -e` program | 1 (86 lines, :8138) | **0** |
| perl / ruby / python invocations | 0 | 0 (only as names in stub and banned lists) |
| heredocs | n/a | 1 (`read -r total zero <<EOF`, :7350, bash data, not a program) |
| auto-merge.yml | 1 bash step calling `gh` | 1 `run:` step (:102), no node, npm, awk or python invocation |
| .github/*.rs tools built by rustc in CI | 2 | 3 (source_scan.rs is new) |

§2 allows `.yml` under .github/. The embedded bash, awk and sed are the gates themselves. Whether shell in YAML is "an interpreted runtime as a tool" is a law question; the prior pass raised it and it is not re-reported here.

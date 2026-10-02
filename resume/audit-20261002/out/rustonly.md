# rustonly: is brutex Rust-only outside web/? (CLAUDE.md §2)

Repo /home/claude/brutex @ bc53131. 930 tracked files. Untracked or ignored at the top level: only `target/` (`git status --ignored --porcelain` prints `!! target/`). The final `git status --porcelain` and `git diff --stat` are both empty.

**Verdict.** At the file level the tracked tree is compliant. Every tracked file outside web/ has an allowed extension or name. On the host target no dependency compiles C or C++, and no dependency is a binding to another language. Three things weaken the claim. (1) The guards are weaker than they say: one test cannot fail, and both the lock fingerprint and gate 13 miss native code that is switched on by a feature. (2) CI YAML embeds bash, awk, sed and a node program. (3) The `api` binary starts `xdg-open` at runtime.

## Per-area table (tracked files)

| Area | Files (ext:count) | Verdict |
|---|---|---|
| (root) | md:5 toml:3 lock:1 .gitignore:1 | COMPLIANT |
| .config/ | toml:1 (nextest.toml, overrides only, no setup scripts) | COMPLIANT |
| config/ | toml:1 | COMPLIANT |
| .github/ | yml:2 (workflows), rs:2 (invariant_paths.rs and mutation_gate.rs, built by `rustc` in CI) | COMPLIANT by extension. The yml files embed shell and JS (see below). |
| .claude/ | json:1 (launch.json) | COMPLIANT (D-0210). It runs `sh -c` and `npm` as operator tooling. |
| docs/ | md:40 | COMPLIANT |
| crates/api | rs:86 toml:1 | COMPLIANT |
| crates/cli | rs:206 toml:1 (includes build.rs, build_provenance.rs, commit_stamp.rs) | COMPLIANT |
| crates/core | rs:16 toml:1 | COMPLIANT |
| crates/costs | rs:17 toml:1 | COMPLIANT |
| crates/engine | rs:8 toml:1 | COMPLIANT |
| crates/greeks | rs:9 toml:1 | COMPLIANT |
| crates/indicators | rs:29 toml:1 | COMPLIANT |
| crates/lake | rs:14 toml:1 | COMPLIANT |
| crates/pull | rs:52 toml:1 | COMPLIANT |
| crates/runner | rs:59 toml:1 | COMPLIANT |
| crates/store | rs:29 toml:1 md:1 | COMPLIANT |
| crates/telemetry | rs:12 toml:1 | COMPLIANT |
| crates/vocab | rs:16 toml:1 | COMPLIANT |
| web/ | js:206 svelte:38 html:22 css:14 rs:10 json:4 md:4 mjs:4 svg:2 ts:1 .gitignore:1 | exempt (D-0052/0053) |

- **Files outside web/ with an extension or name that is not allowed:** none.
- **Shebangs outside web/:** none in any file. The only `#!/` is inside a printf string at ci.yml:160, which is gate 1e's stub generator.
- **Processes started outside web/:**
  - `api` spawns `xdg-open`, `open` or `explorer.exe` (server.rs:16733-16755). This is production code.
  - Tests spawn `git` (core/tests/findings.rs, store/tests/cited_commits.rs), `mkfifo` (cli/readonly_file.rs:98, cli/checksum_receipts_tests.rs:299, store/checksum_audit_tests.rs:122) and their own test binary.
  - Nothing outside web/ starts python, node, npm or `sh -c`, apart from .claude/launch.json and ci.yml.
- **No crate depends on the web/ toolchain:**
  - `api` reads the tracked `web/build` (57 files) at request time.
  - `BUILD_COMMAND` (api/src/assets.rs:1021) is a string that is printed to the operator, never run.
  - No `include_str!` or `#[path]` points into web/.

## CI YAML (.github is allowed by extension)

- ci.yml has 55 `run:` steps and no `shell:` key, so they use GitHub's default bash.
- Interpreters ci.yml calls:
  - bash, awk (78 refs), sed (61), grep, git, cargo, rustc (it builds .github/*.rs)
  - npm/node in Gate W (ci.yml:8043-8074, 8290)
  - an 86-line inline `node -e` JavaScript program at ci.yml:8138 (QUEUED:ET-rust-only-purity-9)
- auto-merge.yml has 1 bash step that calls `gh`.
- §2 allows the `.yml` extension under .github/. It says nothing about embedded languages, and no gate reads YAML content. That makes this a law question, not a gate failure.

## Native and foreign code in the dependency graph

These are the lock packages that ship or compile non-Rust code.

| Crate | Pulled by (`cargo tree -i --target all`) | Compiled for x86_64-unknown-linux-gnu? |
|---|---|---|
| ring 0.17.14 | optional dependency of rustls-webpki, feature not enabled | NO (`nothing to print` on host and with `--target all`) |
| cc 1.4.0 | iana-time-zone-haiku <- iana-time-zone <- chrono <- parquet <- lake | NO (Haiku only) |
| iana-time-zone-haiku | chrono <- parquet <- lake | NO |
| wit-bindgen 0.46 | wasip2 <- getrandom 0.3 <- ahash/graviola | NO (wasm only) |
| wasm-bindgen / js-sys / web-sys | iana-time-zone, reqwest (wasm cfg) | NO (also checked by gate 13a) |

- Not in the lock at all: pyo3, napi, openssl-sys, libz-sys, cmake, bindgen, aws-lc-*. The workspace dependencies `aws-config` and `aws-sdk-ssm` are not used by any member.
- No member manifest has `links =` or `build =`.
- The only `[build-dependencies]` are in crates/cli: sha1 and flate2.
- crates/cli/build.rs and its modules start no process. The only process API is `std::process::id()` at build_provenance.rs:1159, which is in test code.
- graviola has no build.rs and no .c files.
- On the host, 22 dependency build scripts are compiled. Several of them run `$RUSTC` as a probe: crc32fast, libc, proc-macro2, quote, serde, zerocopy, zmij, httparse. This is DOCUMENTED:06-limits §96.

## Per-guard table

| Guard | Claims | Can it fail? | Experiment and output |
|---|---|---|---|
| CI gate 1 (ci.yml:28) | Every tracked file has an allowed extension or name for its path | YES for tracked files | I planted `crates/core/src/zz_probe.py`, `foo.sh`, `crates/core/zz_probe.json`, `zz_probe.yml` and `Makefile`. Untracked: rc=0, because the gate reads `git ls-files` only. After `git add -N`: rc=1, `FORBIDDEN FILE` for all five. A planted `crates/core/src/zz_probe_smuggle.rs` holding `#!/usr/bin/env python3` passed. The gate checks names only (QUEUED:ET-rust-only-purity-3). |
| CI gate 1b (ci.yml:244) | json/yml only under .github/, .claude/, web/ | YES | After `git add -N`: rc=1, `CONFIG OUTSIDE ITS HOME: crates/core/zz_probe.json, zz_probe.yml` |
| CI gate 2 (ci.yml:1436) | No build script spawns a process; no `build =` or `links =` key | YES, but it can be bypassed | `std::process::Command::new("npm").status()` added to build_provenance.rs: rc=1. Raw identifiers `p::r#Command::r#new("/usr/bin/node").r#status()`: rc=0 (QUEUED:ET-rust-only-purity-0). A manifest with `build = "gen.rs"`: rc=1. |
| CI gate 13 (ci.yml:1827) | No foreign runtime declared, resolved or built | YES for listed names. NO for ring re-enabled. | Baseline rc=0. With ring switched on (pull → `rustls-webpki` feature `ring`; `cargo tree -i ring` shows `rustls-webpki` ← `pull`/`rustls`; 30 ring `.o` files compiled): rc=0, "OK — no foreign runtime is declared, resolved, or built for." |
| CI gate 1e (ci.yml:116) | Build and test pass with 12 JS tools shadowed | Reviewed only, not run: a full workspace build was too costly. It reads the exit status and an invocation log. Absolute paths and non-JS interpreters get past it (QUEUED:ET-rust-only-purity-0). | UNVERIFIED by experiment |
| `workspace_is_rust::no_foreign_runtime_reaches_the_workspace` | No pyo3/napi/etc. in the lock | YES | Lock plus `pyo3` gives FAILED: `the lockfile holds: ["pyo3"]` (:162) |
| `workspace_is_rust::the_dependency_set_has_not_moved_without_review` | Dependency set pinned (count 192 plus FNV hash of names) | YES for a name change. NO for a feature or version change. | Adding `libz-sys` gives FAILED at :317 (count). Renaming ring gives FAILED at :204 (declared crate missing). Changing ring's version only (0.17.14→0.17.99): ok. Deleting every `dependencies = [...]`: ok. Ring switched on and compiled (lock diff is one line, `+ "rustls-webpki",` under pull): ok. |
| `workspace_is_rust::no_declared_native_dependency_is_compiled_any_more` (:348) | Fails if declared native code is compiled again | **NO** (CONFIRMED, QUEUED:ET-rust-only-purity-5 / UC-7 / UC-13) | The filter is `d.compiled_on_this_target && packages.contains(d.name)` and every row has a literal `false` (:62, :80, :87, :94). It passed in all of these cases: libz-sys added; ring renamed; pyo3 added; version bumped; dependency arrays stripped; **ring actually compiled for the host**. Its only failure was the empty-lock case, and that failure is the helper assertion at :126, `lockfile is not readable`, not a native-code check. Editing Cargo.lock or a manifest cannot make it fail through its own assertion. |
| deny.toml bans (ring, aws-lc-*, cc) via gate 3 | Refuses ring/cc in the graph | — | UNVERIFIED: `cargo deny` is not installed here (`error: no such command: deny`) |

How the experiments were contained:
- Gate 1/1b plantings were made in the live tree. They were removed with `git rm --cached -f` plus `rm`, and the tree was checked clean afterwards.
- The lock and manifest mutations were made in a `git archive HEAD` copy in scratch with its own `CARGO_TARGET_DIR`. The copy and its target were deleted afterwards.
- The live tree's `cargo test -p vocab --test workspace_is_rust` gave 3 passed, both at baseline and with the planted files present (it reads only Cargo.lock).

## Findings

| id | sev | crate | file:line | What is wrong | Evidence | Status |
|---|---|---|---|---|---|---|
| rustonly-1 | medium | vocab | crates/vocab/tests/workspace_is_rust.rs:348-375 | The native-compile test cannot fail | See the guard table: it stayed green with ring compiled (30 .o files) | QUEUED:ET-rust-only-purity-5 (also UC-7, UC-13) |
| rustonly-2 | medium | vocab / ci | workspace_is_rust.rs:226-326; ci.yml:1827 | The fingerprint hashes package NAMES only. A version bump or feature flip (ring) passes it, and gate 13 also passes with ring compiled. The only remaining defence is cargo-deny, which was not verified here. | Version-bump experiment: ok. Ring-on experiment: vocab 3/3 ok, gate13 rc=0 | QUEUED:ET-rust-only-purity-5 (feature flip). The version-only variant is part of the same defect. |
| rustonly-3 | low | docs | docs/06-limits.md:6012 | Says "No build script in this repository exists at all (gate 13 layer 3)". crates/cli/build.rs exists, and gate 13 allow-lists it: `allow_build='crates/cli/build.rs 1 / build_provenance.rs 1 / commit_stamp.rs 1'` (ci.yml:2008-2010). | `git ls-files '*build.rs'` → crates/cli/build.rs | NEW |
| rustonly-4 | low | api | crates/api/src/server.rs:16733-16755, called at :17092 | On Linux, `serve` spawns `xdg-open`. In xdg-utils that is normally a `/bin/sh` script, so the shipped binary starts a shell interpreter at runtime. §2 forbids "any interpreted runtime ... as a tool". This is a law-interpretation question: it is the OS URL handler, and it can be turned off with `NO_OPEN_ENV`. No doc mentions it (grep of docs/ for xdg-open or open_in_browser: 0 hits). | `BrowserHost::Other => ("xdg-open", &[])`. UNVERIFIED that xdg-open is a script on the target host: it is absent here (`which xdg-open` is empty). | NEW |
| rustonly-5 | low | ci | .github/workflows/ci.yml:8138, 8043-8074 | CI YAML carries inline JavaScript plus npm and node calls outside web/. No gate reads YAML content. | grep counts: node 7, npm 11, awk 78, sed 61 | QUEUED:ET-rust-only-purity-9 |

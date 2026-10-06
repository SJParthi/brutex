# Rust-only scan of PR #74 head 969493e1 (2026-10-06, coordinator session)

Commands were run from the repo root on a clean checkout of 969493e1; outputs summarised.

| Check | Result | Evidence |
|---|---|---|
| Every tracked file vs CLAUDE.md §2 | **1041 files, 0 violations** | `git ls-files` classified by extension/path: outside web/ 624 .rs, 46 .md, 18 .toml, 3 .yml (all under .github/), 1 .json (under .claude/), Cargo.lock, .gitignore, CODEOWNERS |
| web/ (the one exception, D-0052/D-0053) | 346 files, unrestricted by law | 242 .js, 39 .svelte, 23 .html, 15 .css, 10 .rs, 5 .mjs, 4 .json, 4 .md, 2 .svg, 1 .ts |
| Shebang scripts outside web/ | **0** | `grep -lE '^#!/'` over tracked files (the `#!` hits are Rust `#![...]` attributes) |
| build.rs in the repo | **1** (crates/cli/build.rs), spawns no process | grep for Command/process/spawn: no match |
| .cargo/config.toml aliases or runners | **none tracked** | `git ls-files` |
| Native code compiled into the workspace | **none** | `cargo tree --workspace --locked -i ring --target all` and `-i cc -e build --target all`: "nothing to print"; full `cargo build --workspace --all-targets` produced 0 ring/cc artifacts; build scripts that ran: ahash cli crc32fast crossbeam-* generic-array getrandom httparse icu_*_data libc libm num-traits proc-macro2 quote rayon-core rustls serde serde_core serde_json zerocopy zmij |
| TLS crypto | rustls + rustls-graviola 0.4.0 (graviola 0.4.1); `ring` closed by D-0211 | `ring` 0.17.14 still appears in Cargo.lock as an optional dep of rustls-webpki that is never activated |
| Interpreter / foreign-binding crates in Cargo.lock (195 packages) | **none** (no pyo3, lua, rhai, deno/v8, boa, quickjs, wasmtime, napi, neon, jni, cxx, bindgen, cmake, pkg-config) | name scan of Cargo.lock |
| Stale claim | crates/pull/Cargo.toml comments (about lines 93-108 and 124-131) still say ring is linked ("72 ring_core symbols", "The binary is not free of C") although D-0211 removed it | handed to lens L4 (attack/one-authority) to fix with a drift test |

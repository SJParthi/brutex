# Pass 13: tests, docs and security — "Rust only, in every corner" (CLAUDE.md §2), at 1f4de71

Checkout: /home/claude/wt/zero3, detached at 1f4de71. Audit only; nothing in any checkout was edited.
Method: source reading; `git ls-files`; `cargo tree --offline` (read-only, no compile); the registry sources under
/root/.cargo/registry; a bash replay of gate 1's loop, extracted verbatim from ci.yml, in a throwaway `git init`
repository in scratch (now removed). No npm or node was run (no package installs).

**New findings: 5 (1 medium, 4 low).** Pass 6 already covered this ground (P6 theme 1), and P1-07-*, rustonly2-*
and hunt-ci-* are recorded. None of those is repeated here.

---

## Survey (items 1-8)

| # | Check | Result at 1f4de71 |
|---|---|---|
| 1 | Tracked files (982, all mode 100644) | Outside web/ there are 600 `.rs`, 46 `.md`, 18 `.toml`, 1 `.lock`, `.gitignore` and `CODEOWNERS`. There are 3 `.yml` files, all under `.github/workflows/`, and 1 `.json`, `.claude/launch.json`. **Holds.** web/ holds 311 files, 10 of them `.rs` (web/sweep-readiness/*.rs, web/saved-backtest/viewer.rs: ~6,000 lines that cargo never compiles; D-0053 allows them). |
| 2 | `build.rs` | Only `crates/cli/build.rs` (+ `build_provenance.rs`, `commit_stamp.rs`). It names no `Command`. **Holds.** |
| 3 | Dependencies | 154 packages in the native graph. Proc macros: serde_derive, tokio-macros, displaydoc, seq-macro, yoke/zerofrom/zerovec/zerocopy-derive. **None spawns a process.** No `cc`/`cmake`/`*-sys` is built (ring and cc are locked but not built). graviola uses Rust `asm!` (D-0074 era entry, docs/05:16664). **13 dependency build scripts launch `rustc`** (see P13-04): crc32fast, getrandom, httparse, libc, proc-macro2, quote, serde, serde_core, zerocopy, zmij, plus ahash and generic-array through version_check, and num-traits through autocfg. libc, getrandom and autocfg run `RUSTC_WRAPPER` when it is set. |
| 4 | `Command::new` in crates | Production has exactly one: `api` server.rs:18867, the browser opener (`open`/`explorer.exe`/`xdg-open`; D-1202 item 5). Tests: `current_exe()` re-exec (about 35 sites), `CARGO_BIN_EXE_*`, and `git` in core/tests/findings.rs and store/tests/cited_commits.rs, which skip when git cannot run. `mkfifo` appears at 6 sites, one of them by absolute path, pull/src/ingest.rs:3665 `/usr/bin/mkfifo`. AFE-06 names git and mkfifo as allowed. No shell or interpreter is spawned. Gate 0 `spawns` reads only a literal passed to `Command::new` (stated limit, 06-limits:14174). |
| 5 | cargo depending on web/'s toolchain | No `include_str!`/`include_bytes!` reads web/. `api` reads web/build at run time only (assets.rs:127-142). No test reads web/build output or node_modules. Gate 1e shadows 18 tools on PATH for both build and test. **Holds.** |
| 6 | Generated non-Rust source checked in | `web/build/` (57 files) and `web/saved-backtest/build/` (3 files) are generated bundles. Committing them is licensed by D-0053/D-0064 (web/.gitignore). **web/build is stale: P13-01.** |
| 7 | What CI runs besides Rust | See the list below. |
| 8 | Gate 1 bypasses | Already recorded and still open (verification rows below): a newline in a name (P1-07-03), C-quoted paths in 1g (P1-07-04), and `build` key spellings (P1-07-01). New: `.claude/` admits any `.json` at any depth (P13-02). Checked and sound: case variants (`.RS`, `WEB/`, `.GitHub/` are all refused, because matching is case-sensitive and the lists are lower-case), symlinks and gitlinks outside web/ (mode check), `.gitmodules` (extension refused), extensionless files (refused unless LICENSE or CODEOWNERS at their places), trailing dot or space, and homoglyph extensions. |

### Item 7: exactly what runs in CI, for the owner's D-1602 question (not decided here)

- **Shell:** every `run:` step uses GitHub's default `bash` on ubuntu-24.04, and most begin with `set -euo pipefail`.
- **Text tools in ci.yml run blocks (non-comment token counts):**
  - `grep` 190, `printf` 220, `awk` 72 (several multi-line inline awk programs), `sed` 66, `tr` 36, `sort` 22.
  - `mktemp` 18, `xargs` 13, `cut` 12, `wc` 11, `head` 10, `diff` 8, `tail` 6, `uniq` 3, `paste` 2, `tee` 2, `find` 2.
  - `cmp`, `basename`, `chmod`, `sha256sum` and `date` once each.
- **VCS and Rust tools:**
  - `git` 83 and `rustc` 9 (compiles `.github/*.rs`).
  - `cargo` 44 and `rustup` 1.
  - `cargo install`: cargo-deny 0.19.0, cargo-llvm-cov 0.8.4, cargo-mutants 26.2.0, cargo-nextest 0.9.145.
- **Node, web job only (ci.yml:7791-8005):**
  - `npm ci --prefix web`, `npm --prefix web run build` (twice), `npm --prefix web run check`.
  - `node --test web/tests/*.test.js` and `node web/ci/css-comments.mjs`.
  - The names perl/ruby/php/lua/py… appear only as gate 1e stub names.
- **JavaScript actions (they run on the runner's Node, in every job including `language-purity`):**
  - `actions/checkout` (7), `actions/cache` (2), `actions/upload-artifact` (3), `actions/download-artifact` (1).
  - `actions/setup-node` (1) and `Swatinem/rust-cache` (3).
  - `dtolnay/rust-toolchain` (5) is a composite action that runs bash and rustup.
  - These are classified from the actions' own `runs.using`. I did not run them.
- **auto-merge.yml and main-check.yml:** bash with `gh` (11 and 3 uses), `gh --jq` (jq programs evaluated inside gh), `grep`, `tr`, `sort`, `head`, `comm` and `printf`.
- **Operator tooling outside CI:** `.claude/launch.json` starts the product through `sh -c` (P13-03).

---

## Findings

### P13-01 medium: the committed `web/build` bundle is stale, Gate W1 is red at 1f4de71, and three front-end fixes are not in what the binary serves

**Where:**
- web/build (last rebuilt at 9e7a2c4)
- Commits that changed web/src after that rebuild without rebuilding:
  - 3f22aed: web/src/routes/db/+page.svelte:4878
  - 9c6284c: web/src/routes/backtest/+page.svelte:3337 and :4783, web/src/routes/db/+page.svelte:4885-4889, web/src/lib/instrument.js
  - ad14eac: web/src/routes/mapping/+page.svelte:201-212, web/vite.config.js
- The gate: ci.yml:7806-7821 (Gate W1)

**Evidence (ran, grep):**
- `git log -- web/build`: the last rebuild is 9e7a2c4. For each of 3f22aed, 9c6284c and ad14eac, `git show --name-only` lists web/src files and no web/build file.
- The new source strings are absent from the bundle, and the old ones are still there:
  - `grep -rlF 'first bar of the earliest month' web/build` finds nothing. The replaced text `first bar in its month file` is still in web/build/_app/immutable/nodes/6.s8AmQQlg.js.
  - `the bar before this one failed its checksum` (the D-1769 `previous_unreadable` code, which `api` emits at bars.rs:707) is in web/src only.
  - `The refresh keeps running on the server` (D-1974) is not in web/build.
- The P3-02-02 fix (`sweptSymbolOf`) is not served. The bundle still joins on the last `-` segment:
  `n=t.split(`-`).pop()??``` and `String(t.instrument??``).split(`-`).pop()===e.underlying`
- The fingerprint is still `{"version":"1c14f9c9a873646f"}`.

**Why it is wrong:**
- D-0064 (web/.gitignore, docs/05:4899-4902) requires "`web/build` is regenerated and committed in the same commit as any change under `web/src`".
- Gate W1 (`npm --prefix web run build` then `git diff --exit-code -- web/build`) therefore fails on this head, with "The committed bundle is not what this source builds".
- The Rust binary serves web/build, so the operator still gets the pre-fix pages:
  - `BAJAJ-AUTO` and `NAM-INDIA` are still offered as `AUTO` and `INDIA` (P3-02-02).
  - The bar-why texts for `previous_unreadable` and `overflow` are missing.
  - The /mapping timeout does not re-read the masters (D-1974).
- Pass 4 recorded P3-02-02 as FIXED from web/src alone.

**Repro:** `git -C /home/claude/wt/zero3 grep -c sweptSymbolOf -- web/src` returns hits, and `grep -rl sweptSymbolOf web/build` returns none. Gate W1 itself was not run, because it needs `npm ci` and this pass may not install packages.

**Fix:** `npm --prefix web ci && npm --prefix web run build`, then commit web/build in the same commit as the source.

### P13-02 low: `.claude/` accepts any `.json` at any depth, so a tracked `.claude/settings.json` whose hooks run an interpreter passes gates 1, 1b and 1g

**Where:** ci.yml:191 (`.claude/*) list="$claude_allowed"`, meaning `rs|toml|md|lock|html|css|json`), :407-408 (gate 1b exempts `^\.claude/`), :441-488 (gate 1g reads only cargo, rustup and nextest configuration). CLAUDE.md §2: "Exactly one tracked file uses it, `.claude/launch.json`".

**Why it is wrong:**
- Claude Code reads the project file `.claude/settings.json`. Its `hooks` (and `statusLine.command`, `apiKeyHelper`) are shell commands that it runs on the operator's machine.
- That is "an interpreted runtime ... as a tool" arriving through the one path §2 opened for "operator tooling". It is the same door D-1105 closed for `.cargo/config.toml`, `nextest.toml` and `rust-toolchain.toml`.
- No gate pins the `.claude/` file count to the one file §2 names. `.gitignore:46-47` ignores everything but `launch.json`, but `git add -f` overrides it.

**Repro (ran):** a scratch repository tracking `.claude/settings.json` = `{"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"python3 -c \"print(1)\""}]}]}}`. Gate 1's loop, extracted verbatim, printed `bad=0`. Gate 1b's regex exempts it, and gate 1g has no clause for it.

**Fix:** in gate 1 (or 1b), require `git ls-files -z -- .claude` to equal exactly `.claude/launch.json`. Any wider `.claude/` content would then need a CLAUDE.md edit and a decision entry, as D-0210 did.

### P13-03 low: the "press Run on THIS one" configuration starts the product through `sh -c`, whose only reason was removed

**Where:** .claude/launch.json:6-10: `"runtimeExecutable": "sh", "runtimeArgs": ["-c", "exec cargo run --release -p api -- serve"]`.

**Why it is wrong:**
- Before ad14eac (D-1970) the shell existed to evaluate `BRUTEX_COMMIT=$(git rev-parse HEAD)`. ad14eac deleted the substitution and kept the `sh -c` wrapper.
- The product's own launch now goes through a shell interpreter running an inline program for no reason. No gate reads `launch.json`'s program keys: gate 0's `inline_programs` scans workflows only, and its interpreter list excludes `sh`.
- This is a concrete instance for the open D-1602 question.

**Repro:** `git show ad14eac -- .claude/launch.json` (ran).

**Fix:** `"runtimeExecutable": "cargo", "runtimeArgs": ["run","--release","-p","api","--","serve"]`.

### P13-04 low: the repository states both that dependency build scripts launching `rustc` violate §2 and that §2 is not held against dependencies, and its inventory of them is incomplete

**Where:**
- docs/06-limits.md §96 (:6135-6185): "the ban as written cannot be held at the dependency level". Its table has 13 rows but the text says "the fourteen" and "already contains fourteen of".
- docs/05-decisions.md:23240-23256 (invoking the rule against one crate is "picking an argument to fit a conclusion").
- docs/07-plan.md:690-694: "`rustix` 1.1.4 ... its unconditional `build.rs` launches `rustc`; that violates the repository's no-external-process build-script law".
- D-0555 (docs/05:34252-34276): "pending a compliant solution for the blocked dependency graph".

**Why it is wrong:**
- The later texts refuse rustix on exactly the ground §96 and the earlier decision call selective.
- Measured now (ran: grep of each build script in the native `cargo tree`), 13 packages already in the graph launch rustc: crc32fast, getrandom, httparse, libc, proc-macro2, quote, serde, serde_core, zerocopy, zmij, ahash and generic-array (through version_check), and num-traits (through autocfg, `ac.emit_expression_cfg`, which compiles probes with rustc).
- §96's table omits num-traits and autocfg. It lists wasm-bindgen-shared, which is not in the native graph.
- A reader cannot tell whether the current dependency graph is compliant with §2.

**Fix:**
- Make 07-plan:693 cite §96 instead of asserting a violation.
- Add num-traits/autocfg to §96 and correct "fourteen".
- Either add a §2 sentence scoping "any build.rs" to tracked files, or record the dependency probes as an open owner question beside D-1602.

### P13-05 low: `web/sweep-readiness/verify.rs --purity` is a second Rust-only checker that is wider than gate 1, and it prints "passed" on trees gate 1 refuses

**Where:** web/sweep-readiness/verify.rs:434-477. web/sweep-readiness/README.md:28 says it "separately checks the extension boundary".

**Evidence:**
```
Some("LICENSE" | "CODEOWNERS" | ".gitignore" | ".gitattributes")   // any depth
Some("rs" | "toml" | "md" | "lock" | "html" | "css") => true,
```
There is no mode check and no content or orphan check.

**Why it is wrong:**
- D-1104 restricted `LICENSE`/`CODEOWNERS` to the root (plus `.github/` and `docs/` for CODEOWNERS), and refused symlinks, gitlinks and executables outside web/.
- `--purity` accepts `crates/x/LICENSE` holding a script, and a symlinked `.rs`, then prints "Extension boundary: passed".
- Gate 1's own comment (ci.yml:93-97) says two checkers that disagree on one rule is not defence in depth.

**Repro:** not run (it reads the live tree). By source: `crates/a/LICENSE` gives `named = true`, so the path is never refused.

**Fix:** mirror gate 1's placement and mode rules, or delete `--purity` and point the README at gate 1.

---

## Verification rows

| ID | Status | Evidence |
|---|---|---|
| P1-07-01 | NOT FIXED | ci.yml:1843 and :2472 still use the regex `':[[:space:]]*(build|links)[[:space:]]*='`. `source_scan` (:2232) reads `package.build` TOML-aware only to find roots for `orphans`, not to refuse. |
| P1-07-03 | NOT FIXED | ci.yml:155 and :178 are still line-oriented here-string greps. **Ran:** tracked `$'.gitignore\npayload.py'` and `$'run.py\nmd'` give gate 1's loop `bad=0 n=3`. |
| P1-07-04 | NOT FIXED | ci.yml:441 still has `listing=$(git ls-files)` with no `-z` or `core.quotePath=false`. |
| P3-02-02 | PARTIAL | Fixed in web/src (backtest/+page.svelte:3337 and :4783 use `sweptSymbolOf`). The served web/build still uses `split('-').pop()` (P13-01). |
| P3-01-04 | PARTIAL | The server side is detached (pass 4). The page half (mapping/+page.svelte:201-212, re-read and message) is not in web/build (P13-01). |

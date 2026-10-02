# Cloud fix lane 2, second set

Handoff only. Branch `fix-queue` is never merged. Source: the batch-2 audit queue, 2 Oct 2026. Base every fix on origin/main. Do this file after lane2.md. Decision numbers for this lane come from the D-0910 to D-0959 range shared by the cloud lanes.

**Done means:** a test that fails before the fix and passes after; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` and the CI gates green; for a cost finding, the operation is O(1) or the honest bound is named in docs/06-limits.md; extreme cases (empty, one, max, overflow, rerun, corrupt input) covered by tests. One branch and one PR per item, named `fix/cloud-<id>`.

## ET-rust-only-purity-0 · high bug · ci

**Where:** `.github/workflows/ci.yml:1470`

**Finding:** The build-script process ban is a regex that valid Rust gets past. Raw identifiers (r#Command/r#new/r#status), a macro_rules! path ($t::$n), `#[path = "other.rs"] mod x;`, `pub(crate) mod x;` and `include!("x.rs")` all go unnoticed. The last three also keep a fourth file out of Gate 13's three-file build-script allowlist. Gate 1e shadows only 12 JS tool names on PATH, so absolute paths and non-JS interpreters get through. Net result: crates/cli/build.rs ran Node and a Python interpreter with every shell gate green.

**Evidence:**
  - Gate 2's process detectors are ci.yml:1491-1495 (`Command[[:space:]]*::[[:space:]]*new`, `::[[:space:]]*Command`, `process[[:space:]]*::[[:space:]]*Command`, `\.[[:space:]]*(spawn|output|status)[[:space:]]*\(`) plus the std::process member scan ci.yml:1509-1512. This directly disproves Gate 2's own comment at ci.yml:1496 ("nothing defeats the type's own name").
  - A scanned file containing `use std::process as p;` then `p::r#Command::r#new("/usr/local/bin/node").arg(..).r#status()` matches NONE of Gate 2's five patterns (ci.yml:1499-1502 hit set + :1521-1522 pm catch-all): grep hits-rc=1, pm-rc=1.

**Expected fix and test:** None

## W3-runner3-6 · high bug · runner

**Where:** `crates/runner/src/grid.rs:2319`

**Finding:** evaluate_timed / enumerate_variants (crates/runner/src/grid.rs:2319): Levels.ratios=true builds ratio_bitmap by exact equality against single_far_target's quantile rung, so every cell carrying both a fixed stop and a fixed target is refused Code path: RUN on sessions(8), Levels{ratios, ..derived(4)}. Long: cells 56 -> 44 and SL+TP cells 12 -> 0 (target [71], stops [46,47,50]). Short: 12 -> 0 (target [53], stops [64,65,68]). Path: :2227 ratio_set non-empty; :2273 targets = single_far_target; :1727-1731 `want == i128::from(target)`; :2468-2479 `if !admitted { continue; }`. Production passes ratios:true (cli lib.rs:8154, pool.rs:656, expression_pricing.rs:80).

**Evidence:**
  - :2227-2231 `let ratio_set = if ratios { derived_ratios(&favourable, stops.rungs()) }`. `derived_ratios` at :1655-1660 returns empty only when `tightest <= 0 || best <= 0`. :2273 `let targets = single_far_target(&favourable).unwrap_or_else(...)`. `single_far_target` at :1641 returns the 90th-percentile favourable move as a one-rung Ladder. :2319-2334 builds `ratio_bitmap` with `pairs_at_a_ratio`.
  - :2227: `ratio_set = if ratios { derived_ratios(&favourable, stops.rungs()) }`. :2273: `let targets = single_far_target(&favourable).unwrap_or_else(|| ... The target is one rung at the 90th percentile of favourable moves (:1641-1653). :2319-2335: the bitmap is still filled by `pairs_at_a_ratio`. That check (:1727-1731) is exact equality: `want == i128::from(target)`, where `want = stop * r / 100`.

**Expected fix and test:** None

## GAP14-56 · medium test-gap · 

**Where:** `.github/workflows/ci.yml:7858, 7860`

**Finding:** Gate 18 (changed-line mutants) can never mutate the build-time provenance code: its plan diff is limited to crates/*/src/*.rs, and cargo-mutants does not see build_provenance.rs in cli 

**Evidence:**
  - f
  - i
  - x

**Expected fix and test:** Widen the Gate 18 pathspec to cover crates/*/*.rs outside src/ (at least build.rs, build_provenance.rs, commit_stamp.rs). Make build_provenance.rs a module cargo-mutants walks, for example `#[cfg(test)] #[path = "../build_provenance.rs"] mod build_provenance;` in crates/cli/src/lib.rs in place of tests/build_provenance.rs.

Add a gate self-test: a synthetic diff touching crates/cli/build_provenance.rs must produce a non-empty `cargo mutants --list --in-diff` plan. That list is empty today and non-empty after the fix.

## ET-rust-only-purity-2 · medium bug · ci

**Where:** `.config/nextest.toml:None`

**Finding:** A setup script declared in the tracked nextest config runs arbitrary shell before the tests. That is an interpreted runtime used as a tool, and no gate sees it.

**Evidence:**
  - -test-tool nextest` (ci.yml:7948-7955), with nextest pinned to 0.9.145 (ci.yml:7910-7918). D-0629 (docs/05-decisions.md:36387, around line 36405) moved nextest into that required job, which overtakes D-0620's "Nextest remains optional local tooling" (line 36303). Gate 1: exit 0, "OK — every tracked file is in the allowlist for where it lives." (`.toml` is allowed anywhere, ci.yml:44)
  - Gate 1 (ci.yml:28-114) checks file extensions only, and `.toml` is allowed everywhere (`allowed='rs|toml|md|lock|html|css'`, ci.yml:45). binaries on PATH (ci.yml:159) and runs `cargo test` (ci.yml:213). Test code can already spawn processes: `crates/core/tests/findings.rs:488` runs git and `crates/cli/src/readonly_file.rs:98` runs mkfifo.

**Expected fix and test:** None

## ET-rust-only-purity-1 · medium bug · ci

**Where:** `.github/workflows/ci.yml:None`

**Finding:** A tracked .cargo/config.toml (an allowed .toml) can set build.rustc-wrapper to a tracked executable shell script named .rs. Cargo then runs that script for every rustc invocation, in CI and on every developer machine.

**Evidence:**
  - .github/workflows/ci.yml:43 sets allowed='rs|toml|md|lock|html|css'. Lines 76-99 take the text after the last dot of each `git ls-files` entry and test only that. `grep -n -i -E 'cargo/|rustc-wrapper|RUSTC_WRAPPER|config\.toml'` over ci.yml matched only the cache paths `~/.cargo/bin/cargo-mutants|nextest` at lines 7800-7801 and 7899-7900.
  - It sets allowed='rs|toml|md|lock|html|css' (ci.yml:44) and loops over `git ls-files` (ci.yml:103) looking only at `${base##*.}` and the path prefix. (2) Gate 2 only reads `*build.rs` files and the modules they declare (ci.yml:1461-1462), plus `build=`/`links=` keys in `*Cargo.toml` (ci.yml:1572). (3) Gate 1f only scans `crates/*.rs` (ci.yml:304).

**Expected fix and test:** None

## ET-rust-only-purity-3 · medium bug · ci

**Where:** `.github/workflows/ci.yml:28`

**Finding:** The allowlist checks names only. Non-Rust content under .rs, the four allowed names at any depth, executable modes and symlinks all pass. Combined with bugs 1-3, every payload was staged together and run-all-gates changed no verdict.

**Evidence:**
  - *How Gate 1 decides (C2 .github/workflows/ci.yml:74-100).** It only looks at the path and the file name: The only hit is Gate 15's comment at ci.yml:2400, which says a shebang line is refused by nothing. docs/04-invariants.md:839 X-03 claims only "Every tracked file has an allowed extension — CI gate 1".
  - CODE: Gate 1 is at .github/workflows/ci.yml:28-114. The four allowed names are matched on the basename alone (line 80: `^(LICENSE|CODEOWNERS|\.gitignore|\.gitattributes)$`), and the extension is the text after the last dot (lines 81-84). WHAT IS ALREADY DOCUMENTED: ci.yml:2362 says Gate 1 "sees the characters after the last dot".

**Expected fix and test:** None

## ET-rust-only-purity-4 · medium bug · ci

**Where:** `.github/workflows/ci.yml:305`

**Finding:** Browser-code scan misses case variants, inline handlers, anything on a line containing src=, and concat!-split literals. It stops at the first #[cfg(test)], which today hides the production code of 48 files (385 pub items) even though the gate's own comment says no such file exists.

**Evidence:**
  - Method: I copied the Gate 1f `run:` body verbatim (.github/workflows/ci.yml:273-329, YAML indent stripped) into a script and ran it from the worktree root. ci.yml:311 is a case-sensitive `grep -E`, and its pattern has no `on[a-z]+=` and no `javascript:` term. ci.yml:312 `grep -v 'src='` drops the whole line when `src=` appears anywhere on it.
  - What I ran: I copied the Gate 1f script from ci.yml lines 273-329, unchanged, into my throwaway worktree at 9f5c13c7. ci.yml:311 is a case-sensitive `grep -E`, and its pattern has no `on*=` or `javascript:` alternative. ci.yml:312 is `grep -v 'src='`, which drops every line containing `src=` anywhere on it. ci.yml:305 runs awk that stops reading the file at the first column-0 `#[cfg(test)]`.

**Expected fix and test:** None

## UC-5 · medium bug · ci

**Where:** `.github/workflows/ci.yml:328`

**Finding:** Claimed: 'OK — no <script> body, no browser API and no inline handler appears in any .rs production region' Actual: The patterns have no on*= clause and are case-sensitive, and any line containing src= is exempt, so inline handlers are never looked for.

**Evidence:**
  - Code (tree fix/c2-cli @ 9f5c13c7): Gate 1f in .github/workflows/ci.yml:303-316 uses a single case-sensitive pattern, `<script|document\.(getElementById|querySelector|createElement)|await fetch\(|addEventListener|\.innerHTML`, followed by `grep -v 'src='`. The success line at ci.yml:328-329 still prints "OK — no <script> body, no browser API and no inline handler appears in any .rs production region under crates/."
  - `.github/workflows/ci.yml:328-329` at 9f5c13c7 prints "OK — no <script> body, no browser API and no inline handler appears in any .rs production region". The only patterns are at `ci.yml:311`: `grep -E '<script|document\.(getElementById|querySelector|createElement)|await fetch\(|addEventListener|\.innerHTML'`. `ci.yml:312` then drops any line that contains `src=` anywhere, not only a `<script` tag's own `src`.

**Expected fix and test:** None

## UC-9 · medium bug · ci

**Where:** `.github/workflows/ci.yml:305`

**Finding:** Gate 1f — no browser code inside a Rust string is partial: The control `<script>alert(1)</script>` is refused. These all passed: <SCRIPT>, onclick=, `<script data-src=...>alert(document.cookie)`, concat!("<scr","ipt>..."), `<img src=x onerror=alert(1)>`, and production code after a #[cfg(test)] line. Measured: 48 files hold 385 ungated top-level pub items after their first #[cfg(test)] line, which Gate 1f never reads (for example crates/cli/src/lib.rs, first #[cfg(test)] at L43; pub items from L46, `pub fn is_canonical_commit_stamp` at L123). Present tree: no inline handler, `javascript:`, fetch( or innerHTML in the scanned regions.

**Evidence:**
  - The gate script is .github/workflows/ci.yml lines 273-329. `<script data-src=x>alert(document.cookie)`, because `grep -v 'src='` at ci.yml:312 also matches `data-src=`. The pattern at ci.yml:311 has no inline-handler, `javascript:` or case-insensitive branch. Even so, the success message at ci.yml:328 prints "no inline handler appears in any .rs production region".
  - So the OK message at ci.yml:328, "no inline handler appears in any .rs production region", claims coverage the regex does not have. So the ci.yml:301 comment, "Nothing in the tree does that today", is false. A case-insensitive scan of the scanned regions finds only the two src= loaders, render.rs:1168 and mastersrun.rs:813, and the `fetch(` hits are Rust functions.

**Expected fix and test:** None

## UC-10 · medium bug · ci

**Where:** `.github/workflows/ci.yml:1470`

**Finding:** Gate 2 — no build script shells out is partial: The control (plain std::process::Command::new in build.rs) is refused. These passed Gate 2 AND Gate 13: raw identifiers (`use std::process as p; p::r#Command::r#new(..).r#status()`), a macro (`std::process::$t::$n(..).$s()`), `#[path = "provenance_extra.rs"] mod evil_path;`, `pub(crate) mod evil_vis;`, and `include!("evil_inc.rs")`. Compiled and run, the build script executed all seven spawns: marker lines rawident, macro, pathattr, pubcrate, include, node-by-absolute-path, interpreter-by-absolute-path.

**Evidence:**
  - Gate 2 is at .github/workflows/ci.yml:1436-1591. The mod-closure regex (ci.yml:1470) only matches a line that starts with `mod NAME;` or `pub mod NAME;`. The spawn regexes (ci.yml:1499-1502) need the plain tokens `Command::new`, `::Command` or `.spawn|output|status(`. Gate 13 layer 3 (ci.yml:2138-2225) copies the same closure regex and only checks that files EXIST against a three-file allowlist (ci.yml:2008-2010).
  - (A) `p :: r#Command :: r#new("id").r#status()` (raw idents + `use std::process as p` alias + spacing) appended to the already-allowlisted crates/cli/build_provenance.rs — Gate 2's scan regexes (ci.yml:1499-1502) miss the r# spelling and Gate 13 counts the file "1 of 1 allowed".

**Expected fix and test:** None

## AC-gates-law-0 · medium bug · ci

**Where:** `.github/workflows/ci.yml:3680`

**Finding:** TOML ignores whitespace around the dots of a key, and a table header is a key. So `[dependencies . indicators]` followed by `path = "../indicators"` is an ordinary dependency to cargo.

Five checks miss it:
- **Gate 22 A1** refuses only `^\[dependencies\.` (no space allowed before the dot).
- **Gate 22 A2** reads only the flat `[dependencies]` range, which ends at the new header.
- **Gates 9 and 9b** share the A1/A2 code.
- **graph.rs `declarations`/`declared_deps`** and **engine `declared_dependencies`** compare untrimmed `rsplit('.')` segments, so `"dependencies " != "dependencies"` and the `path` line is skipped.

The result: CLAUDE.md §5's rule that `indicators` and `engine` "may not name each other" can be broken with every named guarantee green. The same goes for core and greeks "depending on nothing".

Gate 22 clause C refuses imports only of `store|pull|lake|api|telemetry`, so a later `use indicators::` in engine is not refused either. A quoted key (`"store" = {...}`) also gets past the shell A2 and graph.rs; the engine test does catch that one.

**Evidence:**
  - They are at fix/c2-final:.github/workflows/ci.yml:1638-1639, 1739-1740 and 3680-3681. - **Gates 9, 9b and 22 A2** read only `sed -n '/^\[dependencies\]/,/^\[/p'` and then `grep -oE '^[A-Za-z0-9_-]+ *='` (ci.yml:1659, 1758, 3703). - **graph.rs `named_by_header`/`keys_are_deps`** (crates/core/tests/graph.rs:123-135) and **engine `declared_dependencies`** (crates/engine/src/lib.rs:3476-3494) compare untrimmed `rsplit('.')` segments.
  - **Gate 22 clause A passes it.** I ran fix/c2-final:.github/workflows/ci.yml:3647-3716 verbatim over the real vocab, indicators and engine manifests. **Gates 9 and 9b pass it.** I ran ci.yml:1627-1682 and 1729-1769 verbatim.

**Expected fix and test:** None

## AC-gates-law-1 · medium bug · ci

**Where:** `.github/workflows/ci.yml:1417`

**Finding:** Gates 1d and 24 (and 17, 23, 10, 12) drop every line whose first non-blank character is `*`, treating it as block-comment continuation. But `*slot = ...;` is a normal, rustfmt-stable Rust statement.

Gate 1d's comment claims the filter "cannot hide one", arguing only about `//`. So:
- A segment literal assigned through a deref (`*slot = "acmeorg";`) is never extracted. That is CLAUDE.md §8: no org, env or vendor literal in crates/pull.
- A safe `*slot = Some(memmap2::MmapOptions::new().len(4096).map_anon()?);` never reaches Gate 24's BANNED_MAP. That is §4: a writable memory mapping may never appear.

`map_anon` is a safe fn (memmap2 0.9.8 src/lib.rs:608), so `#![forbid(unsafe_code)]` does not stop it.

**Evidence:**
  - **The filter.** Gate 1d uses the same filter on all four refs: - fix/c2-final:.github/workflows/ci.yml:1423 - origin/main 96194c11 :1418 - fix/c2f-r-api :1423 - fix/c2f-r-cli :1423 - Some of those lines assign a string literal through a deref, for example crates/cli/src/boolean_search_command_tests.rs:144 `*batches = "7";`.
  - **The filter.** fix/c2-final:.github/workflows/ci.yml:1423 (Gate 1d) and :3917 (Gate 24) both run `grep -vE '^[[:space:]]*(//|\*)'`. **The false sentence.** Gate 1d's own comment (ci.yml:1418-1420) says "a Rust STRING LITERAL cannot sit on a line that begins with `//`, so this cannot hide one". It quoted `crates/pull/src/lib.rs:216: let org = "acmeorg";` and then "Either the literal is invented — add it above…", which is the b

**Expected fix and test:** None

## AC-gates-law-2 · medium bug · ci

**Where:** `.github/workflows/ci.yml:4576`

**Finding:** Gate 10 takes the first `::` segment as the crate. When `crates/<seg>/Cargo.toml` is not tracked, it counts the reference as `pending` and `continue`s. The comment says: "The ONLY reason a row is not checked: the crate it names is not a tracked member."

All 13 crates are members now, so every one of today's 52 pending references is a module-first path such as `results::tests::...`, `server::tests::...`, `merge::tests::...` or `vwap::tests::...`. It is not a planned crate.

`.github/invariant_paths.rs` checks only references written "`name` in `crates/...rs`", and none of the 52 rows uses that form. So these references are verified by nothing. Renaming or deleting any of those tests, or citing a test that never existed, leaves Gate 10 green.

X-10 ("Every reachable row in this file names a test that exists") says "1 skipped" and that it is proven for "every row but three". That is §9's rule that every invariant sits beside the test that proves it.

**Evidence:**
  - I could not refute the mechanism. On fix/c2-final:.github/workflows/ci.yml, line 4575 takes `c="${t%%::*}"`. Lines 4579-4581 then do `if ! git ls-files --error-unmatch "crates/${c}/Cargo.toml" ...; then pending=$((pending + 1)); continue`, and that happens before the fn table is consulted. Line 4599 prints the skipped tokens as "the crate they name is not a workspace member". All 13 crates are tracked (git ls-tree fix/c2-final crates/), so every skipped token is a module-first path such as ...
  - **Code (fix/c2-final:.github/workflows/ci.yml:4574-4582).** The gate takes `c="${t%%::*}"` as the crate.

**Expected fix and test:** None

## AC-gates-law-3 · medium bug · ci

**Where:** `.github/workflows/ci.yml:6343`

**Finding:** Rule 6 (`\.iter\(\)\.(find|position|rposition)\(`, §3 rule 4) is grepped line by line over the corpus. For any chain longer than 100 columns, rustfmt puts `.iter()` and `.find(` on separate lines, and the rule never sees it.

So the ratchet ("A RISE is a refusal") holds only for short expressions. Any long expression can add a linear search on a production path without raising a declared allowance or writing a limits entry. It is not an adversarial spelling: it is the formatter's default.

**Evidence:**
  - **How the rule matches.** fix/c2-final:.github/workflows/ci.yml:5958 has `check()` doing `hits=$( { grep -E "$pattern" "$src" || true; } )`. The repo already fixed this same class for rule 5c: ci.yml:6397-6405 says "`[^)]*` cannot cross a newline, so every multi-line attribute — the shape `cargo fmt` produces … was invisible", and replaced that check with a bracket walk. **Measured.** In a throwaway worktree at 19acd032, I extracted gate 11 from ci.yml:4630-6487 unchanged, except for one line: ...
  - **What the gate does** - fix/c2-final:.github/workflows/ci.yml:6343 runs `check 6 '\.iter\(\)\.(find|position|rposition)\('`. - `check()` (ci.yml:5956-5988) runs `grep -E "$pattern"` over a corpus written one source line per record (`print F ":" no ":" line`, strip awk at ci.yml:5811).

**Expected fix and test:** None

## AC-gates-law-5 · medium bug · ci

**Where:** `.github/workflows/ci.yml:237`

**Finding:** The step sets `set -euo pipefail`. `grep -q` exits at its first match, printf then takes SIGPIPE/EPIPE, pipefail makes the pipeline non-zero, and `if` reads that as "no match".

That means the check that exists to catch a toolchain reached by absolute path ("a crate could reach a toolchain by an absolute path the stub directory does not shadow", §2) stays silent for any match outside the last pipe-buffer of output. `cargo test --workspace` prints one line for each of 5,686 `#[test]`s, which I estimate (not measured) at well over 64 KiB.

Gates 12 and 14 use the same shape as `printf "$scrub" | grep -qE "$trigger" || continue`, which would silently skip a doc block larger than a pipe buffer. That is latent today: the largest doc block is 11,201 bytes (crates/pull/src/rate.rs:1).

Gate 1d's own comment records this exact EPIPE behaviour on the runner, but only for the opposite polarity.

**Evidence:**
  - **The code.** fix/c2-final (19acd032):.github/workflows/ci.yml:118 opens the Gate 1e step with `set -euo pipefail`.
  - **The code.** fix/c2-final:.github/workflows/ci.yml:237 is `if printf '%s' "$tout" | grep -qiE 'node_modules|yarn\.lock|pnpm-lock|package\.json'; then`.

**Expected fix and test:** None

## AC-gates-law-6 · medium bug · ci

**Where:** `.github/workflows/ci.yml:4008`

**Finding:** The "other direction" check greps `^overflow-checks[[:space:]]*=[[:space:]]*false` over root Cargo.toml only.

It does not see:
- An indented key (TOML ignores indentation) inside `[profile.release.package.engine]`.
- A quoted key.
- A tracked `.cargo/config.toml` `[profile.release]`, which cargo lets override the manifest (`.toml` is allowed anywhere by Gate 1).

Each one makes a release build wrap while Gate 25 prints "present no profile sets overflow-checks = false". That is exactly the silent-wrap failure §4 bans, which Gate 25 exists to refuse.

**Evidence:**
  - I tried to refute this and could not. The finding is real, and Gate 25 is byte-identical on all four refs. I extracted the block from "Gate 25 — the release profile" to "Gate 27" and hashed it. The sha1 is 2d037752ec63bbc9adc682e6b2f3bbcb4670707c on origin/main (96194c11), fix/c2-final (19acd032), fix/c2f-r-api (16736f23) and fix/c2f-r-cli (43998f33). Only the line numbers differ: main 3936, c2-final 3941. It is not a duplicate. backlog.json's only overflow-checks mentions are ...
  - **The code.** fix/c2-final:.github/workflows/ci.yml:4009-4010 is the whole "other direction" check: `off=$( { sed 's/#.*//' Cargo.toml | grep -nE '^overflow-checks[[:space:]]*=[[:space:]]*false' || true; } )` - It is anchored at column 0.

**Expected fix and test:** None

## AC-gates-o1-1 · medium bug · ci

**Where:** `.github/workflows/ci.yml:4575`

**Finding:** Gate 10 takes the first `::` segment as the crate. If `crates/<seg>/Cargo.toml` is not tracked, it increments `pending` and continues, and pending is never a failure. The comment says 'The ONLY reason a row is not checked: the crate it names is not a tracked member of this workspace'. All 13 crates are members, so every pending hit is a module path.

In docs/04-invariants.md, 52 of 1,520 backticked references start with a module:
- server 23; csv, merge, vwap 3 each; ingest, evaluator, research_policy, boolean_catalog_command 2 each
- one each: results, research, coverage, cash_auction, ledger_v6, ledger_all, audited_stored, column, anchored, strict_range_knobs, sweeprun, identity

Examples: docs/04-invariants.md:4310 `vwap::tests::all_twenty_known_positions_follow_their_actual_reference_and_tolerance` and :3987 `merge::tests::native_ids_accept_duplicates_and_keep_vendors_separate`.

None of these rows contains " in `crates/", so `.github/invariant_paths.rs` skips them as well: `references()` returns early unless `line.contains(" in `crates/")`. The named tests of about 50 rows can be renamed or deleted with Gate 10 green. All 52 names exist today, so no row dangles yet.

**Evidence:**
  - - fix/c2-final:.github/workflows/ci.yml:4575 sets `c="${t%%::*}"`.
  - At fix/c2-final:.github/workflows/ci.yml:4574-4582, Gate 10 takes `c="${t%%::*}"`.

**Expected fix and test:** None

## AC-gates-cx-0 · medium bug · ci

**Where:** `.github/workflows/ci.yml:1636`

**Finding:** Gates 9, 9b and 22A read dependencies with sed -n '/^\[dependencies\]/,/^\[/p', plus refusals for '^\[dependencies\.', dev-, build- and target. graph.rs starts with table="" and skips any key while !keys_are_deps(&table). TOML also allows `dependencies.store = { path = "../store" }` as a root key before [package], and `["dependencies"]` as a quoted header. Cargo accepts both. None of the three checks sees either spelling. These are the checks CLAUDE.md §5 names as the proof that core and greeks depend on nothing, and that a bar cannot reach vocab, indicators or engine (gate 22 ships no allowlist).

**Evidence:**
  - READ (fix/c2-final): - Gate 9 (ci.yml:1636-1665) builds `code=$(sed 's/#.*//' "$f")`. - graph.rs:138 starts with `table = String::new()`, and graph.rs:166 does `if !keys_are_deps(&table) { continue; }`. - engine/src/lib.rs:3476-3526 (`declared_dependencies`, the parser Gate 22 names as authoritative) has the same shape.
  - **Code (fix/c2-final).** - Gate 9 (ci.yml:1638-1661) refuses only `^\[dependencies\.`, `^\[dev-dependencies`, `^\[build-dependencies` and `^\[target\.`. - graph.rs:143-170 starts with `let mut table = String::new();`, skips `if !keys_are_deps(&table) { continue; }`, and `keys_are_deps` compares t

**Expected fix and test:** None

## AC-gates-cx-1 · medium bug · ci

**Where:** `.github/workflows/ci.yml:2623`

**Finding:** Gate 16's rule is "every crate root forbids unsafe outright", and its comment argues deny is not enough because one #[allow(unsafe_code)] switches deny off. It only checks crates/<c>/src/{lib,main}.rs. Every tests/*.rs, benches/*.rs and build.rs is its own crate root. Its WHAT-IT-CANNOT-SEE list names only [lib]/[[bin]] path overrides, which it says end up as a red build, and never names these roots. Gate 5 lets up to three allow(unsafe_code) through with no decision entry, and the tree currently uses zero. In build.rs an unsafe extern "C" system() call starts a shell with no Command or std::process token, so Gate 2's patterns (1498-1524) and Gate 13 layer 3 (an existence count) see nothing.

**Evidence:**
  - **Gate 16 looks at only two files per crate.** fix/c2-final:.github/workflows/ci.yml:2623-2624 loops over `for base in lib main; do f="crates/${c}/src/${base}.rs"`. **Gate 5 would not stop it.** Gate 5 (ci.yml:7502-7508) only fails when the count is above 3, so one `allow(unsafe_code)` passes without a decision entry.
  - docs/05-decisions.md:8105-8106 repeats the claim ("every crate root is `#![forbid(unsafe_code)]` under gate 16").

**Expected fix and test:** None

## AC-gates-cx-3 · medium bug · ci

**Where:** `.github/workflows/ci.yml:3500`

**Finding:** In state st==1, a line that contains '{' switches to st==2, which skips everything up to the next '}' in column 0. `#[cfg(test)]\nuse std::{fs, io};` is a single, rustfmt-stable line with braces, so the walker drops the whole next production fn or impl, including any OpenOptions::new, File::open or fs::read in it. The exit-3 and surviving-cfg(test) soundness checks do not fire. The only trace is a smaller "window" count in the log.

**Evidence:**
  - Code (fix/c2-final:.github/workflows/ci.yml:3509-3525).
  - I could not refute it. I checked it myself. 1. The code. In fix/c2-final:.github/workflows/ci.yml, around lines 3508-3516, state st==1 has two lines. The first is `if (index($0, "{") == 0 && t ~ /;[ \t]*$/) { st = 0; next }`, so a line that ends in `;` ends the item only if it has no brace. The second is `if (index($0, "{") > 0) { st = 2 }`. In state st==2, only a line matching `/^\}/` sets st=0. So `use std::{fs, io};` after `#[cfg(test)]` enters st==2, and the walker drops every line up to ...

**Expected fix and test:** None

## W3-runner2-7 · medium bug · runner

**Where:** `crates/runner/src/excursion.rs:1053`

**Finding:** crossings_with (with grid.rs:3947 blocks_without_pricing) (crates/runner/src/excursion.rs:1053): look-ahead in path refusal: a refused bar AFTER a variant's level exit (stop, target or trail) un-prices that variant's trade and blocks the position until time_exit Code path: The walk always runs `for offset in 0..=to.saturating_sub(from)` to the time exit, and admit() does `out.refused = out.refused.saturating_add(1)` for any refused bar. grid.rs:3947 `if c.block_only || c.cross.refused() > 0 { *open_until = Some(c.time_exit); return true; }` runs for every variant in one_variant (grid.rs:3993) and never compares the variant's exit offset with where the hole is.

**Evidence:**
  - excursion.rs:1053 `for offset in 0..=to.saturating_sub(from)` always walks the candidate to its time exit. admit() (excursion.rs:914-933) runs `out.refused = out.refused.saturating_add(1)` for any refused bar, wherever it sits in the path. grid.rs:3947 `if c.block_only || c.cross.refused() > 0 { *open_until = Some(c.time_exit); return true; }` is called for every candidate inside one_variant (grid.rs:3993).
  - The walk `for offset in 0..=to.saturating_sub(from)` (excursion.rs:1053) always runs to the time exit. admit() does `out.refused = out.refused.saturating_add(1)` (excursion.rs:930) for any refused bar, wherever it sits. grid.rs:3947 `if c.block_only || c.cross.refused() > 0 { *open_until = Some(c.time_exit); return true; }` runs for every variant from one_variant (grid.rs:3993).

**Expected fix and test:** None

## W3-runner3-1 · medium cost · runner

**Where:** `crates/runner/src/grid.rs:3475`

**Finding:** levelled, reached through per_trade (2706) and materialize_cell (2778) (crates/runner/src/grid.rs:3475): per per exit cell replayed. cli candidate_universe.rs:4861-4898 calls materialize_cell for EVERY cell of every population member and side streamed by Sweeper::run_prepared_population_by_reporting, the cost is O(B + signals + C*(span+L)) per cell, so O(cells*B) per member-side, where O(C) is needed; it grows with bars B (SliceFacts), signals (walk_over), and candidate paths times span plus crossing width (crossings_checked). All of it is paid again for every cell. Auditor verdict: undocumented-scan. Documented: Contradicted. per_trade's doc (grid.rs:2693-2695) says 'Called on the handful of combinations a report names, never on the millions a sweep weighs', and docs/06-limits.md §113 says the SliceFacts builds 'are per-slice co ...

**Evidence:**
  - *What each call does (code).** `materialize_cell` (crates/runner/src/grid.rs:2768) calls `per_trade` (:2778). `per_trade` calls `levelled` (:2706). `levelled` runs `let facts = crate::trade::SliceFacts::of(bars, column);` on every call, with no hoisting (:3475). `levelled_over` runs `crate::trade::walk_over(...)` (:3518).
  - In `append_validated_grid_rows`, candidate_universe.rs:4861 loops `for ordinal in 0..evaluated.grid().cells.len()`. At :4890 it calls `materialize_cell(execution_bars, execution_column, &member.item.mask, ...)` once per cell. `produce_candidate_universe_v1` feeds every closed mask to it through `sweeper.run_prepared_population_by_reporting(...)` (:2812). It is a live path: step3_orchestrator.rs:3977 calls it.

**Expected fix and test:** None

## W3-runner3-2 · medium cost · runner

**Where:** `crates/runner/src/grid.rs:2546`

**Finding:** evaluate_resolved_policy_v1 (and evaluate_resolved_expression_policy_v1 at 2566) (crates/runner/src/grid.rs:2546): per per run, meaning per candidate mask or program and side. The population sink reaches it through exit_grid_policy evaluate_training_grid_attested (cli candidate_universe.rs:4800), and cli boolean_candidate_v1.rs:673-680 t ..., the cost is O(B) expected per candidate, where O(signals + C*(span+L) + cells*C) is needed; it grows with bars B: a HashMap of B timestamps, two prefix vectors of length B+1, the forced-exit table, and the median-step sample. Auditor verdict: false-o1-claim. Documented: Falsely claimed absent. exit_grid_policy.rs:1991-1995: 'This is the per-candidate half ... so no bar is re-read and no byte is re-hashed here;

**Evidence:**
  - grid.rs:2546 and :2566 each run `let facts = crate::trade::SliceFacts::of(bars, column);` on every call. Both functions are reached once per run: through evaluate_with_attested (exit_grid_policy.rs:2018) and through evaluate_expression_with_attested (expression_execution.rs:312). SliceFacts::of (trade.rs:406-458) is linear in B.
  - grid.rs:2544-2546 `let exact = resolved.ladders()?; let cells = reserve_policy_cells_v1(resolved.cell_count())?; let facts = crate::trade::SliceFacts::of(bars, column);`. grid.rs:2564-2566 repeats this for expressions. SliceFacts::of (trade.rs:407-455) reads every bar. exit_grid_policy.rs:2018 (evaluate_with_attested) and expression_execution.rs:312 both call it.

**Expected fix and test:** None

## W3-runner3-3 · medium cost · runner

**Where:** `crates/runner/src/identity.rs:567`

**Finding:** data_digest_with_daily_reference / data_digest (crates/runner/src/identity.rs:567): per per candidate identity. cli boolean_candidate_v1.rs:721-725 calls it inside produce_side, which runs for every program and every side (loop at :673-680)., the cost is O(B) per candidate (O(1) per bar), where O(1) per candidate is possible; it grows with signal plus one-minute plus daily bar count, hashed again for each candidate over identical bytes. Auditor verdict: undocumented-scan. Documented: Only the per-bar claim ('Both are O(1) per bar', identity.rs:59). Nothing documents the per-candidate recomputation, and 06-limits has no match.

**Evidence:**
  - The loop is at crates/cli/src/boolean_candidate_v1.rs:673-689: `for (program_index, program) in request.programs.iter().enumerate() { for resolved in &resolutions { ... Inside produce_side, :721-725 calls `runner::identity::data_digest_with_daily_reference(&source.data.signal.bars, &source.data.exact_minute.bars, reference(source))`.
  - produce_side(...)` (:673-680). Every `produce_side` call runs `let data_digest = runner::identity::data_digest_with_daily_reference(&source.data.signal.bars, &source.data.exact_minute.bars, reference(source))` (:721-725). That call hashes signal, minute and daily bars (identity.rs:567, :571, :573).

**Expected fix and test:** None

## W3-runner3-0 · medium cost · runner

**Where:** `crates/runner/src/outcome.rs:1673`

**Finding:** edge (Newey-West pair loop) (crates/runner/src/outcome.rs:1673): per per measured hit, inside a per-candidate call (rank.rs:653 calls edge once per frequent itemset), the cost is O(min(H-1, hits in the last H bars)) per hit; O(rows + n*min(H,n)) per candidate, which is O(n^2) when H >= n; it grows with the horizon H, which accepts any u32: Horizon::bars rejects only 0, and cli knobs.rs:203-208 parses BRUTEX_HORIZON_BARS as a u32. Once H is at least the span of the hits, it also grows with the number of measured hits, so the cost per candidate becomes quadrat .... Auditor verdict: undocumented-scan. Documented: No. The doc at outcome.rs:1436-1443 says only that the working set is O(H), 'Constant in the data, linear in a number the operator chose. Still one pass'.

**Evidence:**
  - :1503 sets `let horizon_bars = forward.horizon().as_bars() as usize;`. :1666-1672: the queue is drained only when `source.saturating_sub(older) >= horizon_bars`. :1673-1685: `for &(older, x_older) in &recent { ... :1538-1541: the capacity clamp `horizon_bars.min(forward.bars_len.saturating_add(1))` bounds the reservation only, not the loop. :318-320: `Horizon::bars` rejects only 0.
  - The queue drains only on bar distance against H, at :1666-1672: `while let Some(&(older, _)) = recent.front() { if source.saturating_sub(older) >= horizon_bars { recent.pop_front(); } else { break; } }`. Every surviving entry is then visited for each measured hit, at :1673-1685: `for &(older, x_older) in &recent { ...

**Expected fix and test:** None

## W3-runner3-4 · medium cost · runner

**Where:** `crates/runner/src/outcome.rs:517`

**Finding:** RangeExtremes::of (called once per forward, :590) (crates/runner/src/outcome.rs:517): per per bar of every forward build (every ranked run and walk-forward fold), the cost is Theta(n log n) time and memory per forward, i.e. amortised Theta(log n) per bar; it grows with log2(bars). Memory is two i64 tables of about n*log2(n) entries. Auditor verdict: undocumented-scan. Documented: Only in the code. outcome.rs:487-503 says 'Build is O(n log n) once per forward, which is per RUN, not per candidate and not per bar', and claims 'A monotonic deque ... cannot answer a variable one'.

**Evidence:**
  - *Build cost.** In crates/runner/src/outcome.rs, `RangeExtremes::of` (lines 512-548) runs `while width.saturating_mul(2) <= n`. The query (`over`, :551-569) is O(1). `forward()` builds the table unconditionally at :590. It is called once per ranked run: runner lib.rs:401, cli lib.rs:3350, :10428, :18457. It is called once per walk-forward train fold: validate.rs:4613, hoisted outside the `par_iter`.
  - At crates/runner/src/outcome.rs:512-548, `RangeExtremes::of` runs `while width.saturating_mul(2) <= n`. `forward` (outcome.rs:590) builds the table on the full slice every time. It is called once per ranked run (runner lib.rs:401, cli lib.rs:3350/10428/18457) and once per walk-forward fold (validate.rs:4613). 1,222,791 bars is a real per-instrument span in this repo (runner lib.rs:949, engine benches/ratio.rs:44).

**Expected fix and test:** None

## W3-runner3-5 · medium bug · runner

**Where:** `crates/runner/src/outcome.rs:1790`

**Finding:** edge (crates/runner/src/outcome.rs:1790): Horizon at or above the span of the hits: the Bartlett bandwidth exceeds the sample, so the long-run variance collapses to rounding noise and a fabricated huge |t| survives the m2<=0 guard. rank::walk sorts by |t| and p_value clamps to 0 Code path: RUN (release, empty mask, under load): H=15 gives t=88.2 / 264.0 / 563.3 at 8 / 32 / 128 sessions. H=u32::MAX gives t=24,673.4 and 116,747.9 at 8 and 32 sessions, and 0.000 at 128 sessions (negative residue, NaN, zero). Reachable: Horizon::bars accepts any nonzero u32 (outcome.rs:318-320) and cli knobs.rs:203-208 parses BRUTEX_HORIZON_BARS as a u32.

**Evidence:**
  - `Horizon::bars` refuses only zero (outcome.rs:318-320: `if bars == 0 { None } else { Some(Self(bars)) }`). `cli::knobs::horizon_count` (knobs.rs:203-208) parses any u32. `horizon_for` (cli lib.rs:17419-17479) passes it through unchanged by design ("TAKEN VERBATIM"). An explicit 375 ("a whole session") is a tested value (lib.rs:26229).
  - `Horizon::bars` refuses only zero: `if bars == 0 { None } else { Some(Self(bars)) }` (outcome.rs:318-320). `horizon_for` returns `crate::knobs::horizon_count(&asked)` (cli lib.rs:17471), and its test `an_explicit_count_is_taken_verbatim` accepts 375. The browser boolean launch caps `horizon_bars` only at `u32::MAX` (api booleanlaunch.rs:138).

**Expected fix and test:** None

## W3-runner3-7 · medium bug · runner

**Where:** `crates/runner/src/outcome.rs:1667`

**Finding:** edge (crates/runner/src/outcome.rs:1667): Session boundary: hits on different IST days are paired by bar-index gap, although every forward window is truncated at that day's 15:10 exit and the two windows cannot overlap Code path: The drain is only `if source.saturating_sub(older) >= horizon_bars { recent.pop_front() }` and there is no day check. forward (:653-670) ends every window at min(horizon, the day's forced bar). A 15:08 hit and the next 09:15 hit are 22 bars apart on a full session, so H >= 23 pairs them with w=1-22/H>0.

**Evidence:**
  - The drain is `while let Some(&(older, _)) = recent.front() { if source.saturating_sub(older) >= horizon_bars { recent.pop_front(); } else { break; } }` (outcome.rs:1666-1672). The pair loop then weights every queued entry with `w = 1.0 - (gap as f64) / (horizon_bars as f64)` (:1681).
  - In crates/runner/src/outcome.rs, `edge` pairs two hits using only the bar-index gap: `if source.saturating_sub(older) >= horizon_bars { recent.pop_front(); }` (:1667-1668), then `let w = 1.0 - (gap as f64) / (horizon_bars as f64);` (:1681). There is no day check, and `edge` could not make one: `Forward` (:335-375) carries no day and no exit index. `SessionBounds::same_day` exists (:260) but `edge` never calls it.

**Expected fix and test:** None

## W3-runner4-0 · medium cost · runner

**Where:** `crates/runner/src/trade.rs:407`

**Finding:** SliceFacts::of (as reached per candidate from grid::evaluate_resolved_policy_v1 / evaluate_resolved_expression_policy_v1 / materialize_expression_cell) (crates/runner/src/trade.rs:407): per one candidate mask or Boolean program priced on a resolved exit grid (per closed population member x side, per program x side), the cost is O(B) time plus O(B) allocation per candidate: HashMap<i64,usize>::with_capacity(bars.len()) (trade.rs:433), two Vec<u64> prefixes of B+1 (434-435), Vec<Option<SquareOff>> of B via forced_exits_with_step (456); it grows with B = execution bars in the slice (independent of the candidate). Auditor verdict: false-o1-claim. Documented: Contradicted, not documented. docs/06-limits.md §113 (6417-6422) says "These are per-slice costs, not per-candidate costs"; exit_grid_policy.rs:1990-1995 calls evaluate_with_attested "the per-candidate half ...

**Evidence:**
  - Three production functions call `let facts = crate::trade::SliceFacts::of(bars, column);` in their own bodies, so the facts are rebuilt on every call: evaluate_resolved_policy_v1 (grid.rs:2546), evaluate_resolved_expression_policy_v1 (grid.rs:2566) and materialize_expression_cell (grid.rs:2830).
  - trade.rs:406-407 defines `pub fn of(bars, column)`. It allocates `HashMap::with_capacity(bars.len())` (:433), two `Vec<u64>` of length B+1 (:434-435) and `forced_exits_with_step` (:456). grid.rs:2546 `let facts = crate::trade::SliceFacts::of(bars, column);` in evaluate_resolved_policy_v1 grid.rs:2566 in evaluate_resolved_expression_policy_v1 grid.rs:2830 in materialize_expression_cell

**Expected fix and test:** None

## W3-runner5-0 · medium cost · runner

**Where:** `crates/runner/src/validate.rs:2705`

**Finding:** walk_forward_exact_grid_v4 (OOS replay loop over `pending`) (crates/runner/src/validate.rs:2705): per one policy-selected (mask, side) candidate replayed on the fold's test window, the cost is O(S_prefix + E_prefix) per candidate: 2 BLAKE3 passes over signal_upto, 5 BLAKE3 passes over trade_test, about 5 more linear validation passes and one E_prefix-sized SliceFacts HashMap build, all in a sequential `for` loop; it grows with S_prefix = signal bars 0..fold.test.end and E_prefix = execution minutes 0..last test signal close + horizon. Both grow with the fold index, and on the last fold they reach the whole span. This is multiplied by the pending count (up to 2 x closed masks).. Auditor verdict: undocumented-scan. Documented: Not in docs/06-limits.md. A grep for ExecutionRunV1, replay_selected, anchored.search, with_levels_over and SliceFacts found no statement about this loop.

**Evidence:**
  - validate.rs:2705 reads `for (ordinal, candidate) in pending.iter().enumerate() {`, a plain `for` with no par_iter. It is reached through step3_orchestrator.rs:3080 `.anchored_search_v4(request.sweeper)`, then candidate_universe.rs:1094, then validate.rs:2215. `pending` gets one entry per (mask, side) whose `select` returns Some (validate.rs:2625).
  - The loop: validate.rs:2705 is a plain serial `for (ordinal, candidate) in pending.iter().enumerate() {`, not a par_iter. `pending` gets one entry per (mask, side) whenever `select` finds an admitted cell (exit_grid_policy.rs:2121-2152, a `max_by_key` over the admitted cells). So it can hold up to 2 x closed.kept.len() entries (reserved that size at :2483).

**Expected fix and test:** None

## W3-runner5-1 · medium cost · runner

**Where:** `crates/runner/src/validate.rs:2572`

**Finding:** walk_forward_exact_grid_v4 / evaluate_one (training population, per candidate x side) (crates/runner/src/validate.rs:2572): per one (closed mask, side) training grid evaluation, the cost is O(S_train + 2*E_train) BLAKE3 per candidate x side, inside the par_iter; it grows with S_train (signal bars 0..train_end) + E_train (execution minutes before test_open), multiplied by 2 x closed masks. Auditor verdict: false-o1-claim. Documented: Not in docs/06-limits.md. The source comment says the opposite. validate.rs:2522-2524: "The per-run door checks the run's five identity terms against the resolution's own copies ...

**Evidence:**
  - validate.rs:2572 `let execution_run = ExecutionRunV1::new(&run, train, Some(trade_train))?;` sits inside `evaluate_one` (:2557). That closure is called twice per closed mask from `closed.kept.par_iter()` (:2592), once for Long and once for Short. The chain is reached from `cli/src/candidate_universe.rs:1094` via `walk_forward_projected_prepared_anchored_search_v4`.
  - validate.rs:2572 `let execution_run = ExecutionRunV1::new(&run, train, Some(trade_train))?;` sits inside `evaluate_one`, which `closed.kept.par_iter()` calls twice per mask (2590-2598). exit_grid_policy.rs:274-275 calls `crate::identity::data_digest_with_execution(signal_bars, execution_bars)`. identity.rs:406-407 then runs `term(.., &data_digest(signal))` and `term(.., &data_digest(execution))`.

**Expected fix and test:** None

## W3-runner5-2 · medium cost · runner

**Where:** `crates/runner/src/validate.rs:2573`

**Finding:** walk_forward_exact_grid_v4 / evaluate_one -> ResolvedExitGridV1::evaluate_with_attested -> grid::evaluate_resolved_policy_v1 (crates/runner/src/validate.rs:2573): per one (closed mask, side) training grid evaluation, the cost is O(E_train) SliceFacts build (a HashMap reserved at E_train with E_train inserts, two Vec<u64> of E_train+1, a Vec<Option<SquareOff>> and a median-step pass) per candidate x side; it grows with E_train (execution minutes in trade_train), multiplied by 2 x closed masks. Auditor verdict: false-o1-claim. Documented: Not in docs/06-limits.md. §113 says the SliceFacts costs "are per-slice costs, not per-candidate costs", and that is false on this path.

**Evidence:**
  - `evaluate_one` runs `resolved.evaluate_with_attested(attested, execution_run)?` (validate.rs:2573). *What each call rebuilds.** evaluate_with_attested (exit_grid_policy.rs:2018) calls `crate::grid::evaluate_resolved_policy_v1(attested.bars, attested.column, ...)`. That function always runs `let facts = crate::trade::SliceFacts::of(bars, column);` (grid.rs:2546).
  - It is reached through step3_orchestrator.rs:3080 -> candidate_universe.rs:1094 -> walk_forward_projected_prepared_anchored_search_v4 (validate.rs:2161) -> walk_forward_exact_grid_v4 (:2383). At validate.rs:2573, `resolved.evaluate_with_attested(attested, execution_run)?` reaches exit_grid_policy.rs:2018 `crate::grid::evaluate_resolved_policy_v1(attested.bars, attested.column, ...)`.

**Expected fix and test:** None

## W3-runner5-3 · medium cost · runner

**Where:** `crates/runner/src/validate.rs:4887`

**Finding:** walk_forward_core (OOS pass, scored.par_iter) (crates/runner/src/validate.rs:4887): per one scored candidate re-priced on the fold's test window, the cost is O(prefix rows) per candidate, where O(test rows) would do; it grows with rows of the column over signal bars 0..fold.test.end (the anchored prefix), not the test window. Only rows with source >= test.start can fire.. Auditor verdict: undocumented-scan. Documented: Not in docs/06-limits.md. The function doc at validate.rs:1729-1733 says "one trade-walk per DISTINCT combination ...

**Evidence:**
  - validate.rs:4769 `let signal_upto = bars.get(..fold.test.end).unwrap_or(bars);` :4777 `let confined_signal = restricted(&full, fold.test.start);` restricted (validate.rs:4997-5000) clones the column and calls `clear_before(from)`. indicators column.rs:695-707 `clear_before` only sets `*bits = ConditionMask::ZERO; *known = ConditionMask::ZERO;` where `source < from`.
  - validate.rs:4769 `let signal_upto = bars.get(..fold.test.end).unwrap_or(bars);`. For Rolling, the train slice is warm..boundary (split.rs:239), but the OOS prefix still starts at 0. :4777 `restricted(&full, fold.test.start)` = `column.clone(); confined.clear_before(from)` (:4997-5000). The doc at :4980-4991 says outright: "the shape is preserved and the bits are cleared instead".

**Expected fix and test:** None

## W3-runner5-5 · medium bug · runner

**Where:** `crates/runner/src/validate.rs:4694`

**Finding:** walk_forward_core / every_candidate_the_sweep_produced_is_priced_and_none_is_skipped (crates/runner/src/validate.rs:4694): Count not moving when output changes: `priced` is no longer counted from the loop, so R-01's assertion `priced == considered` is a tautology, and a reintroduced candidate cap would pass it Code path: validate.rs:4378 `let considered = u64::try_from(closed.kept.len()).unwrap_or(u64::MAX);` and :4694 `let priced: u64 = u64::try_from(closed.kept.len()).unwrap_or(u64::MAX);` are the same expression, so they are equal by construction. The field doc (:133-135) claims "this counts what the loop visited".

**Evidence:**
  - :4378 `let considered = u64::try_from(closed.kept.len()).unwrap_or(u64::MAX);` :4694 `let priced: u64 = u64::try_from(closed.kept.len()).unwrap_or(u64::MAX);` Neither reads `assessed` (built at :4620-4623 by `closed.kept.par_iter().map(..).collect()`) or the flatten loop that follows. The field doc at :133-135 says "this counts what the loop visited". The loop comment at :4568-4569 says "`priced` is a count".
  - crates/runner/src/validate.rs:4378 sets `let considered = u64::try_from(closed.kept.len()).unwrap_or(u64::MAX);`. :4694 sets `let priced: u64 = u64::try_from(closed.kept.len()).unwrap_or(u64::MAX);`. That is the same expression, computed after the `par_iter` at :4620-4677 and independent of it. Nothing inside the loop or the flatten fold (:4695-4728) touches `priced`.

**Expected fix and test:** None

## LATE-gates-and-ci:correctness-and-extremes#10 · low None · 

**Where:** `:None`

**Finding:** Gate 23 clause C allowlists four fixed temp paths in cli/src/results.rs with a reason its own header and results.rs refute 

**Evidence:**


**Expected fix and test:** 

## LATE-gates-and-ci:correctness-and-extremes#11 · low None · 

**Where:** `:None`

**Finding:** Gate W6 passes a masters.js that calls refresh and verify on page load, contradicting its own comment 

**Evidence:**


**Expected fix and test:** 

## LATE-gates-and-ci:correctness-and-extremes#13 · low None · 

**Where:** `:None`

**Finding:** Gates 9, 9b, 17 and 22 (clauses B-D) report success when their subject is absent 

**Evidence:**


**Expected fix and test:** 

## LATE-gates-and-ci:correctness-and-extremes#5 · low None · 

**Where:** `:None`

**Finding:** Gates 17 and 23 word-split tracked paths, so a .rs file with a space in its name (legal through #[path], and allowed by Gate 1) is never sca 

**Evidence:**


**Expected fix and test:** 

## LATE-gates-and-ci:tests-bite#3 · low None · 

**Where:** `:None`

**Finding:** Gate 22 filesystem ban is bypassed by an aliased import group (`use std::{fs as f}`) 

**Evidence:**


**Expected fix and test:** 

## LATE-gates-and-ci:tests-bite#4 · low None · 

**Where:** `:None`

**Finding:** Gate 21's probe list omits File::create, fs::write and File::options, so the logger can write an arbitrary path and Gate 21 still says 'open 

**Evidence:**


**Expected fix and test:** 

## LATE-gates-and-ci:tests-bite#8 · low None · 

**Where:** `:None`

**Finding:** Gate 14 claims to use Gate 12's cost-claim trigger 'character for character', but its scrubs differ 

**Evidence:**


**Expected fix and test:** 

## GAP16-26 · low law · 

**Where:** `crates/runner/src/outcome.rs:852-972, 1307-1386, 1599`

**Finding:** runner::outcome::Edge carries seven paisa money quantities as f64 (three single-move extrema and four sums), and the sums are persisted and served as IEEE bits, with no D-entry sanctioning a float for them 

**Evidence:**
  - C
  - o
  - d

**Expected fix and test:** Fix: hold the extrema and sums as i64 (the sums as i128 or checked i64) in Sides and Edge, and convert to f64 only inside payoff_bp, path_ratio_bp and worst_reward_risk_bp. Store the i64 values in a new sweep-evidence RankedRow version (§3 rule 8: append a version, never mutate one). Alternatively, the operator records a D-entry that sanctions f64 for these fields with the 2^53 bound argued, and the ci.yml rationale lists them.

Test in outcome.rs tests: Sides::observe fed an i64 move of 9_007_199_254_740_993 (2^53+1) must report max_win_paisa == 9_007_199_254_740_993 exactly. It fails today (it reads ...992 through `as f64`) and passes after.

## LATE-whole-hot-path:tests-bite#8 · low None · 

**Where:** `:None`

**Finding:** A test named for the long-run-variance refusal never calls `edge`; it asserts that `f64::sqrt` of a negative is NaN 

**Evidence:**


**Expected fix and test:** 

## ET-rust-only-purity-6 · low bug · ci

**Where:** `.github/workflows/ci.yml:2108`

**Finding:** The manifest regexes miss four valid TOML spellings that cargo resolves as the banned crate. The lock layer still catches the resolved package once Cargo.lock is updated, so this is defence-in-depth loss only.

**Evidence:**
  - The code: .github/workflows/ci.yml:2108-2115 holds the three layer-1 patterns. How I tested it: I copied the step body (ci.yml:1829-2353) into a scratch script, removed the 10-space indent, and changed only `mktemp -d` so it writes to a directory the sandbox allows. deny.toml:104 also bans `{ name = "pyo3" }`, and Gate 3 runs cargo deny.
  - How I tested: I copied the Gate 13 run block (ci.yml:1829-2353) into a script, changing only mktemp to point at a scratch directory. CI builds with --locked in Gate 1e (ci.yml:167, same language-purity job) and Gate 4 (ci.yml:7483), so this state cannot pass CI. The gate then exits 1, with layer 2 reporting "REFUSED Cargo.lock — 5 occurrence(s)" starting at `Cargo.lock:961:name = "pyo3"`.

**Expected fix and test:** None

## ET-rust-only-purity-7 · low bug · ci

**Where:** `.github/workflows/ci.yml:1950`

**Finding:** Three independent banned-runtime lists disagree. Gate 13 (the job that runs first) misses napi, cpython, rustpython-vm, rhai, mozjs and more; only the vocab test's exact-name list or its fingerprint catches them.

**Evidence:**
  - Gate 13 `banned` (.github/workflows/ci.yml:1950-1958) has 20 names, each also matched with its `-*`/`_*` family: pyo3, mlua, rlua, hlua, lua-src, luajit-src, rquickjs, quickjs-rs, duktape, boa_engine, deno_core, v8, rusty_v8, neon, node-bindgen, jni, ruby-sys, magnus, rutie, ext-php-rs.
  - Gate 13 (.github/workflows/ci.yml:1950-1957) bans pyo3, mlua, rlua, hlua, lua-src, luajit-src, rquickjs, quickjs-rs, duktape, boa_engine, deno_core, v8, rusty_v8, neon, node-bindgen, jni, ruby-sys, magnus, rutie and ext-php-rs. The vocab test's FORBIDDEN list (crates/vocab/tests/workspace_is_rust.rs:102-121) uses exact names. deny.toml:103-127 lists 11 runtime names plus ring, aws-lc-rs, aws-lc-sys and cc.

**Expected fix and test:** None

## ET-rust-only-purity-8 · low bug · ci

**Where:** `.github/workflows/ci.yml:2477`

**Finding:** Any tracked file containing a NUL byte is skipped as binary, so the banned word hidden in such a file passes.

**Evidence:**
  - .github/workflows/ci.yml:2477 has `if ! The count at ci.yml:2481 (`grep -IoiF`) and the echo at 2512 (`grep -IniF`) also use -I. ci.yml:2474-2476 says "There are none today". The gate's "WHAT IT CANNOT SEE" list (ci.yml:2390-2406) does not mention binary or NUL-bearing files. How I ran it: I extracted the Gate 15 `run:` block (ci.yml:2356-2547) and ran it with bash in the worktree.
  - The gate's "WHAT IT CANNOT SEE" list (lines 2390-2405) does not mention binary or NUL-bearing files. Gate 1 (lines 28-115) only checks extensions, so a `.md` file containing a NUL byte passes it. I extracted the gate's run block from ci.yml lines 2356-2547 without changes, except one line: `work=$(mktemp -d)` became `mktemp -d "$TMPDIR/g15.XXXXXX"`, because the sandbox refuses /var/folders.

**Expected fix and test:** None

## ET-rust-only-purity-9 · low bug · ci

**Where:** `.github/workflows/ci.yml:8115`

**Finding:** An inline 86-line JavaScript program (`node -e '... require("fs") ...'`) lives in a tracked file outside web/. CLAUDE.md §2 limits the front-end exception to web/ ('and nowhere else'), and no gate reads .yml content for embedded languages. Whether CI-job glue that serves web/ counts under the exception is a law question for the operator.

**Evidence:**
  - `git grep -n 'require("fs")\|process\.argv\|process\.exit(' -- ':!web'` finds matches only at ci.yml:8116, 8117 and 8198. CLAUDE.md:50 reads "Rust is the only language in this repository." CLAUDE.md:73 reads "Under `web/` — and nowhere else — the front end is unrestricted." CLAUDE.md:93 forbids "any interpreted runtime, as a dependency, a dev-dependency, or a tool".
  - I extracted the body (8116-8200, 85 lines by `wc -l`) and `node --check` (v24.1.0) accepted it as valid JavaScript. Gate 1f only reads `git ls-files 'crates/*.rs' 'crates/**/*.rs'` (ci.yml:271-329). Gate 13 reads manifests and the lock (ci.yml:1827+).

**Expected fix and test:** None

## ET-rust-only-purity-10 · low bug · ci

**Where:** `.github/workflows/ci.yml:84`

**Finding:** A legal Rust file with a non-ASCII name is refused, because git ls-files quotes the path and the parsed extension becomes `rs"`.

**Evidence:**
  - At lines 82-87 the extension is taken as `base="${f##*/}"; ext="${base##*.}"`, and at lines 92-97 the path is matched with `case "$f" in web/*)...`.
  - I extracted Gate 1's script from .github/workflows/ci.yml lines 30-114 exactly as written, ran it with `bash -e`, and removed the worktree afterwards. The loop runs from ci.yml:81 (`while IFS= read -r f; do`) to ci.yml:103 (`done < <(git ls-files)`). The loop is at ci.yml:81-103, not 84-104.

**Expected fix and test:** None

## UC-8 · low bug · ci

**Where:** `.github/workflows/ci.yml:1879`

**Finding:** Claimed: 'A C or Lua source tree vendored in WITHOUT a manifest entry. Gate 1's extension allowlist is what refuses that, and it already does.' Actual: Gate 1 checks names only. Non-Rust content under a .rs name, under an allowed name (LICENSE) anywhere, or behind a 120000 symlink passes, and so does an executable bit.

**Evidence:**
  - Gate 1 (.github/workflows/ci.yml:28-114) decides everything from the basename. Line 82 takes `base="${f##*/}"`, line 83 lets `LICENSE|CODEOWNERS|\.gitignore|\.gitattributes` through at any depth, and lines 84-98 compare only the text after the last dot against the list for that path. I copied Gate 1's run body verbatim from lines 30-114 and ran it:
  - Code: Gate 1 (.github/workflows/ci.yml:28-114) reads only `git ls-files`. It exempts `allowed_names='LICENSE|CODEOWNERS|\.gitignore|\.gitattributes'` by basename anywhere in the tree (ci.yml:77,80). It then checks only the text after the last dot (ci.yml:81-98). My grep found only a 1e fixture at ci.yml:160 and a comment at ci.yml:2400.

**Expected fix and test:** None

## UC-11 · low bug · ci

**Where:** `.github/workflows/ci.yml:2108`

**Finding:** Gate 13 — no foreign runtime (layers 1-4) is partial: Baseline 'ok Gate 13'. The control `pyo3 = "0.29"` is refused, and a lock stanza named pyo3 is refused (layer 2 held). Layer 1 was evaded by four valid TOML spellings that `cargo metadata --no-deps` reads as a pyo3 dependency: "pyo3" = ..., `pyo3 . version = ...`, `package = 'pyo3'`, and [dependencies."pyo3"]. Unlisted names passed: napi, cpython, rhai, rustpython-vm, mozjs, wasmtime. Layer 3 missed #[path], pub(crate) mod and include! files.

**Evidence:**
  - I extracted Gate 13 (ci.yml:1829-2352) and Gate 2 (ci.yml:1438-1591) and ran them in the throwaway worktree U-11-0, which I have since removed. A declared dependency has to be in Cargo.lock, because CI runs `cargo fetch --locked`, `cargo build --locked` and `cargo test --workspace --locked` (ci.yml:7460, 7483, and 7425+120).
  - I pulled the Gate 13 run block out of ci.yml:1827-2352 and ran it as-is, changing only the mktemp dir so the sandbox allowed it. Gate 2 (ci.yml:1436) was pulled out and run the same way. LAYER 1 (manifest scan, d1/d2/d3 at ci.yml:2108/2112/2115). CI builds and tests with `--locked` (ci.yml:7483, 7544), so pyo3 cannot build unless it is in Cargo.lock, and layer 2 catches it there. deny.toml:104 also bans pyo3.

**Expected fix and test:** None

## UC-12 · low bug · ci

**Where:** `.github/workflows/ci.yml:2477`

**Finding:** Gate 15 — the banned word is partial: Control docs/zz-nul.md with the word: 'REFUSED docs/zz-nul.md'. The same file with one NUL byte: '2 skipped as binary' and 'OK — the banned token appears in no tracked file.'

**Evidence:**
  - I pulled Gate 15's run body out of .github/workflows/ci.yml, lines 2356-2547, and ran it with /bin/bash and BSD /usr/bin/grep. Code: the skip is at ci.yml:2477: `if ! The skipped count is printed at :2494 but never fails the step. The step then ends at :2545 with `OK — the banned token appears in no tracked file.`
  - I reproduced it myself at 9f5c13c7, in a throwaway worktree at scratchpad/wt/U-12-1 that has since been removed. C2 is untouched: HEAD is still 9f5c13c7 and the tree is clean. What the code does: Gate 15 is in .github/workflows/ci.yml, lines 2354 to 2547. At line 2477 it runs `if ! grep -Iq . - "$f"` and, on failure, does `skipped=$((skipped+1)); continue`.

**Expected fix and test:** None

## AC-gates-law-7 · low bug · ci

**Where:** `.github/workflows/ci.yml:2288`

**Finding:** The awk inspects only `if:` lines in the step. A step-level `continue-on-error: true` on Gate 3 (or on the Gate 8 bench step) lets cargo-deny fail while the build job, and therefore ci-ok, still succeeds. Gate 13 nevertheless reports the security gate as present and unconditional.

The repair comment says "A line's PRESENCE is not a line's EXECUTION". A non-blocking execution is the same hole.

**Evidence:**
  - **What the code does.** The check is `runs_unconditionally`, at fix/c2-final:.github/workflows/ci.yml:2288-2324 (Gate 13) and :7356-7392 (Gate 14). In the whole tree the only mentions are the comment at ci.yml:2969 and docs/06-limits.md, so no other gate refuses the key either.
  - What the check reads: in `runs_unconditionally` (fix/c2-final:.github/workflows/ci.yml:2288-2324 for Gate 13, :7356-7392 for Gate 14), `finish()` looks only at lines matching `/^[ \t]+if:[ \t]/`. It finds only a comment at ci.yml:2969 ("no step sets `continue-on-error`") and a line in docs/06-limits.md:3582.

**Expected fix and test:** None

## AC-gates-law-8 · low bug · ci

**Where:** `.github/workflows/ci.yml:352`

**Finding:** `pull::config::check_segment` (config.rs:384-389) allows `a-z A-Z 0-9 - _` in every position, including the first. But:
- Gate 1c needs `/[A-Za-z0-9]` before the env segment.
- Gate 1d extracts only `"[a-z0-9][a-z0-9_-]{0,31}"`.

A real org segment that starts with `_` or `-` is therefore invisible to both §8 gates. docs/06-limits.md §18 describes Gate 1d as covering every literal that "could *be* a segment — lower case, `[a-z0-9_-]`".

**Evidence:**
  - On fix/c2-final, crates/pull/src/config.rs:384-389 `check_segment` refuses a byte only through `!matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_')`. The parser applies it to every configured segment at config.rs:1145 (`check_segment(text).map_err(|fault| ConfigError::Segment { line, key, fault })`). I ran the exact patterns from ci.yml in $TMPDIR, without a build, against a file holding `"_acmeorg"`, `"-acmeorg"`, `"/_acmeorg/prod/dhan/token"`, and controls `"acmeorg"` and ...
  - - Where: fix/c2-final:crates/pull/src/config.rs:374-398. - Every configured segment goes through it via `segment()` at config.rs:1044, 1063, 1116 and 1145: org, env, region, vendor and fields. - Gate 1c, ci.yml:352: `grep -nE "/[A-Za-z0-9][A-Za-z0-9_.-]*/(${envs})/"`.

**Expected fix and test:** None

## AC-gates-law-10 · low bug · ci

**Where:** `.github/workflows/ci.yml:103`

**Finding:** These gates read `git ls-files` through process substitution, `for f in $(...)` or `|| true`. That swallows git's failure, the loops read nothing, and each prints success. Gates 12, 16, 24, 26 and 27 already refuse a silent zero ("A gate that reads no file is not a gate"); the §2 and §8 gates do not.

In CI the checkout is always a repository. Under the gate runner, a pruned or moved worktree, or git refusing a dubiously owned directory, turns the Rust-only and credential gates into vacuous passes.

**Evidence:**
  - I tried to refute this and could not. I confirmed it myself with a non-repo run and a control run. **Code (fix/c2-final:.github/workflows/ci.yml).** Each gate reads `git ls-files` in a way that `set -euo pipefail` does not check: - Gate 1, line 103: `done < <(git ls-files)`. - Gate 1b, lines 264-265: `bad=$(git ls-files '*.json' '*.yml' | grep -Ev ... || true)`. - Gate 1f, line 304: `for f in $(git ls-files 'crates/*.rs' 'crates/**/*.rs')`. - Gate 1c, lines 351-353 and 362: `git ls-files -z | ...
  - I tried to refute this and could not. When `git ls-files` fails, every gate named in the finding passes. **Scope: the code is on main, not new in C2.** The C2 diffs (`git diff origin/main fix/c2-final|fix/c2f-r-api|fix/c2f-r-cli -- .github/`) touch only three things: Gate 1d's `pull_fixture` word list (lines 1184-1205), Gate 23's `declared` list, and Gate 11's `server.rs 7 -> 8`. None of them touches the file walks. So the walks are identical on origin/main 96194c11 and on fix/c2-final ...

**Expected fix and test:** None

## AC-gates-law-11 · low bug · ci

**Where:** `.github/workflows/ci.yml:4516`

**Finding:** The allowlist entry keeps the identity invariant X-01 ("Run identity changes if any loaded bar differs by one field", §3 rule 3) exempt. Its reasons are that "no run-identity function exists in any crate ... nothing hashes CLAUDE.md section 3 rule 3s nine inputs", that blake3 is taken by no member, and that vocab, indicators and engine are not members.

X-10 says the three exemptions exist "because the code the row describes does not exist". crates/runner/src/identity.rs now implements the nine-term hash, using core's own `brutex_core::blake3`. The exemption is stale, and Gate 10 only flags an exemption whose id is no longer a row. So the identity row can stay ✗ and exempt indefinitely on a false premise.

**Evidence:**
  - - docs/04-invariants.md:837 says there is no run-identity function, and that "crates/vocab, crates/indicators and crates/engine are not workspace members".
  - The exemption's premise is false.** - fix/c2-final:.github/workflows/ci.yml:4516 says: "X-01 no run-identity function exists in any crate ... - fix/c2-final:crates/runner/src/identity.rs:647 has `pub fn identity(run: &Run<'_>) -> RunId`. - docs/04-invariants.md:852 (X-14) cites `runner::identity::two_feeds_with_byte_identical_bars_do_not_collide` and `every_term_changes_the_identity`.

**Expected fix and test:** None

## AC-gates-o1-0 · low bug · ci

**Where:** `.github/workflows/ci.yml:7025`

**Finding:** Layer 3 counts `ratio(` lines against min_points. The comment claims these are 'PINNED AT WHAT THE BENCH CARRIES TODAY at HEAD ... DELETING one is a red build'. A later comment admits they are 'pinned at what each bench carried when its row was added'. At 19acd032, engine is pinned at 4 while its bench has 13 such lines (12 call sites plus `fn growth_ratio(`, which the `fn[ \t]+ratio\(` exclusion misses). Other gaps: vocab 12 vs 7, store 9 vs 4, indicators 8 vs 5, telemetry 8 vs 6, pull 6 vs 4. Layer 4 says it checks 'the bench really prints it', but it runs `grep -qF -- "$r" "$b"` over the whole file, comments included. Every C-E id occurs on at least one `///` line: C-E-01 at engine ratio.rs:600 and C-E-02 at :509, both inside other rows' docs. Result: the mask-evaluation rows that CLAUDE.md §3 rule 4 says 'bind the corrected live method' can be neutralised with every text gate green. Gate 8 then runs a bench whose rows are constant verdicts.

**Evidence:**
  - crates/core/tests/cost_invariants.rs:192-206 (`printed_ids`) takes each id from the first word of a string literal in the bench, not from a comment. Neither the C-E-01 row nor the C-E-02 row (docs/04-invariants.md:1914-1915) names another cost id, so neither can pass through a reference.
  - (1) ci.yml:7021-7029 says min_points are "PINNED AT WHAT THE BENCH CARRIES TODAY at HEAD ... The gate's own :7105 comment and docs/04-invariants.md:1814 ("Gate 14's floor is stated as 7 because that is what its own counter sees" – it now sees 12) show the pin is stale. (2) The `/ratio\(/ && !/fn[ \t]+ratio\(/` counter also counts engine ratio.rs:133 `fn growth_ratio(` as a p

**Expected fix and test:** None

## AC-gates-o1-2 · low bug · ci

**Where:** `.github/workflows/ci.yml:6618`

**Finding:** `allow_claim` holds `crates/cli/src/lib.rs 2`, and its rationale names exactly two blocks: ':7299 base_win_rate_bp' (English 'flat') and ':12922 note_grid_progress' ('A REAL CLAIM AND A REAL GAP'). The note_grid_progress block now says UNVERIFIED (cli/src/lib.rs:17759), so it is no longer refused. Its slot has gone to cli/src/lib.rs:4478, the doc of `window_range_percentile` (:4587). That block says '# O(1) amortised per bar' (:4580) and '... an amortised O(1) step — the bound §3 rule 4 requires' (:4584). It names no test, row, bench path or UNVERIFIED. The gate compares only the count, prints no locations for an allowed file (6912), and warns 'loose' only when the file has zero refusals (6922). So a real per-bar cost claim ships unproven, and the log says nothing about it.

**Evidence:**
  - **The allowlist is keyed by file and count alone.** At fix/c2-final:.github/workflows/ci.yml:6632-6634, `allow_claim` holds `crates/cli/src/lib.rs 2`.
  - The allowlist is the same `crates/cli/src/lib.rs 2` everywhere (fix/c2-final:.github/workflows/ci.yml:6634-6636). The comment at ci.yml:6628-6633 calls `note_grid_progress` "A REAL CLAIM AND A REAL GAP". But its block already says "**UNVERIFIED — both halves of that cost sentence are read off the source**", on origin/main at cli/src/lib.rs:17351 and on fix/c2-final at :17759.

**Expected fix and test:** None

## AC-gates-o1-3 · low bug · ci

**Where:** `.github/workflows/ci.yml:7013`

**Finding:** Gate 14 says 'layer 5 proves gate 8 still executes them'. `runs_unconditionally` refuses a step only when its `if:` can be false. `continue-on-error` does not occur anywhere in ci.yml, and no gate reads it. Adding `continue-on-error: true` to the 'Per-operation cost at 1x / 10x / 100x' step would leave layer 5 printing 'present'. The benches would still run, but a BREACH exit would no longer fail the complexity job, so ci-ok, which accepts only 'success', stays green. The same helper guards gate 3's `cargo deny check` (2288-2332) and has the same gap. Separately, 7365-7366 says 'gate 8 carries `if: always()`', but the Gate 8 step at 7983-8009 has no `if:`.

**Evidence:**
  - In fix/c2-final:.github/workflows/ci.yml:7374 the `runs_unconditionally` awk skips every step line except `if (L[i] !~ /^[ \t]+if:[ \t]/) continue`. The copy at ci.yml:2288-2323 (Gate 13, guarding gate 3's deny step) is the same. I pulled the helper out (ci.yml:7356-7392) and ran it against copies of ci.yml with one line added.
  - I tried to refute this and could not. I confirmed it by running the helpers against fixtures; I did not run anything on GitHub. 1) The helper never reads `continue-on-error`. In fix/c2-final:.github/workflows/ci.yml, `runs_unconditionally` appears twice: at 2288-2323 (the gate 3 / cargo-deny guard) and at 7356-7391 (Gate 14 layer 5). Both copies look at one step property only, at 2308 and 7374: `if (L[i] !~ /^[ \t]+if:[ \t]/) continue`. - Layer 5 test: I pasted the layer-5 function from ...

**Expected fix and test:** None

## AC-gates-cx-2 · low bug · ci

**Where:** `.github/workflows/ci.yml:2288`

**Finding:** runs_unconditionally refuses a step only when its if: collapses to a literal false, 0, failure() or cancelled(). The comment says "Everything else, constant or not, is allowed" (2303, 7371), yet it also claims "the line must be there AND its step must be REACHABLE". Two edits disable Gate 3 or Gate 8 while both checks still print present: step-level `continue-on-error: true` (the step runs but cannot fail its job, so ci-ok sees success) and a constant-false expression such as `if: false && true` or `if: github.run_attempt == 0`.

**Evidence:**
  - The function body is identical on all four refs: - fix/c2-final:.github/workflows/ci.yml:2288-2324 (Gate 13) - fix/c2-final:.github/workflows/ci.yml:7356-7392 (Gate 14) - fix/c2f-r-cli, whose ci.yml has the same md5 as fix/c2-final - fix/c2f-r-api, at 7379+ - origin/main:.github/workflows/ci.yml:2283-2319 and 7351+ `git grep continue-on-error` on fix/c2-final finds only a comment (ci.yml:2969) and docs/06-limits.md:3582, so no other gate refuses it. ci-ok (ci.yml:8284-8312) looks only at ...
  - **Reproduced by running it.** I took `runs_unconditionally` from fix/c2-final:.github/workflows/ci.yml:2288-2324.

**Expected fix and test:** None

## AC-gates-cx-4 · low bug · ci

**Where:** `.github/workflows/ci.yml:2812`

**Finding:** All four gates drop lines matching ^[[:space:]]*(//|\*). In Rust, `*slot = …` is an ordinary statement, so string literals (1d), prints (17 and 23) and MmapMut::map_anon (24) on a deref-assignment line are never scanned. Both comments claim the filter "cannot hide one". Separately, Gate 17 greps println!/eprintln! and Gate 23 counts `m!` by name. `use std::eprintln as note;` followed by `note!(…)` compiles, and neither gate counts it. Gate 17 does not look for dbg! at all.

**Evidence:**
  - On main, the same filter is at ci.yml:1418, 2860 and 3912, and the claims are at 1414 and 2782. fix/c2-final:.github/workflows/ci.yml:1423 (Gate 1d), 2865 (Gate 23) and 3917 (Gate 24) use `grep -vE '^[[:space:]]*(//|\*)'`.
  - - fix/c2-final:.github/workflows/ci.yml:1423 and :2865 use `grep -vE '^[[:space:]]*(//|\*)'`. These are deref statements such as api/src/audit.rs:1480 `*dst = byte;`, and 28 of them are in crates/pull.

**Expected fix and test:** None

## W3-runner2-6 · low cost · runner

**Where:** `crates/runner/src/excursion.rs:195`

**Finding:** Ladder::from_excursions (called per candidate from grid.rs:1752, 2195, 2197) (crates/runner/src/excursion.rs:195): per one candidate's derived stop/target/trail ladder (legacy/expression grid path), the cost is O(n log n) per candidate, i.e. O(log n) per trade, plus an O(n) `observed.to_vec()` copy per axis; it grows with n = the candidate's trade (path) count. Auditor verdict: undocumented-scan. Documented: no. grep for quantile/from_excursions in docs/06-limits.md finds no cost statement for the per-candidate ladder.

**Evidence:**
  - *The sort is real.** excursion.rs:195 `pub fn from_excursions(observed: &mut [Ppm], count: usize)` runs `observed.sort_unstable();` at line 199. Each caller copies the sample first: grid.rs:1752 `Ladder::from_excursions(&mut adverse.to_vec(), rungs)`, and 2195/2197 `Ladder::from_excursions(&mut observed.to_vec(), rungs)`.
  - `Ladder::from_excursions` (crates/runner/src/excursion.rs:195) does `observed.sort_unstable();` at :199. After the sort it reads only `count` positions: `Some(d) => observed.len().saturating_sub(1) / d` (:245), then `observed.get(at...)`. grid.rs:1752 in `merged_stops` (stops, when step_ppm is None). grid.rs:2195, the stepped-failure fallback. grid.rs:2197, the `ladder_of` arm when step_ppm is None.

**Expected fix and test:** None

## ET-strategies-trades-ranking-costs-1 · low bug · runner

**Where:** `crates/runner/src/grid.rs:4080`

**Finding:** Grid cell totals clamp silently at i64::MAX, and the drawdown built on the clamped running equity reads 0. No field flags the clamp, and audit prints it as a real total. trade.rs says a saturated fill is refused rather than saturated; the totals do not follow that rule. CLAUDE.md §4 bans a fallback that hides a failure. This needs totals above about ₹9.2e16, so it cannot happen on real index data.

**Evidence:**
  - crates/runner/src/grid.rs:4079-4081: `cell.trades`, `cell.pessimistic` and `cell.optimistic` are accumulated with `saturating_add`. grid.rs:4374-4386 (`accrue_risk`): `*running = running.saturating_add(pess)` and `dip = peak_equity.saturating_sub(*running)`.
  - crates/runner/src/grid.rs:4080-4081 add `pessimistic` and `optimistic` with `saturating_add`. The same pattern is at :4270 (`gross_win`), :4339 (`gross_loss`) and :4379 (`running` inside `accrue_risk`). :4383 computes `dip` with `saturating_sub`. Neither `Cell` (grid.rs:115-384) nor `Grid` (grid.rs:896-936) has any field that flags an overflow or saturation.

**Expected fix and test:** None

## AC-whp-law-3 · low bug · runner

**Where:** `crates/runner/src/identity.rs:30`

**Finding:** The run-identity module is the code form of §3 rule 3. Its crate doc says "nothing persists a `RunId` yet — it is formatted into a report string and never written to the store — so there is no recorded corpus to migrate". The `Run::feed` doc repeats this to argue that adding a term "costs nothing".

Both statements are false on this branch:
- `cli::results::Record.identity` persists the RunId in the append-only ledger and deduplicates on it.
- `sweep_evidence` attempts and the `and-checkpoint-v1` journal are keyed by it.
- Frontier and trade rows carry it.

The same file contradicts itself at :395-397: "Results already persist `RunId`; wrapping the old digest in a new domain would make an unchanged native run look new and append a duplicate result".

The false sentence is the one a maintainer would read before adding a term. Re-keying now splits every recorded run from its rerun, and the ledger then files the same computation twice under two identities.

**Evidence:**
  - How main persists the RunId: - origin/main:crates/cli/src/lib.rs:16346 `fn record_run(...
  - `fix/c2-final:crates/runner/src/identity.rs:33-35` reads "nothing persists a `RunId` yet — it is formatted into a report string and never written to the store — so there is no recorded corpus to migrate". `identity.rs:395-397` reads "Results already persist `RunId`; wrapping the old digest in a new domain would make an unchanged native run look new and append a duplicate result". - On `fix/c2-final`, `crates/cli/src/lib.rs:16752` has `fn record_run(into, id: &runner::identity::RunId, ...)`.

**Expected fix and test:** None

## W3-runner3-8 · low bug · runner

**Where:** `crates/runner/src/outcome.rs:656`

**Finding:** forward (crates/runner/src/outcome.rs:656): A missing minute (no record at the exact deadline, or a timestamp gap inside the path) is counted as a REFUSED record Code path: `let Some(want) = facts.at_timestamp(deadline) else { ret.push(None); refused.push(true); ...`. Also :717-722: path_accepts is false on any broken_prefix gap (trade.rs:529-535), which likewise sets refused=true. Edge::refused is documented (:961-965) as 'Non-zero means the store handed this run a record the engine refuses'. Tests cover duplicate, overflow and corrupt records only (outcome.rs:3257-3481).

**Evidence:**
  - Code: crates/runner/src/outcome.rs:655-662 handles a missing exact-deadline minute (`let Some(want) = facts.at_timestamp(deadline) else { ... Lines 717-722 set refused=true when `!facts.path_accepts(i, exit)`. In trade.rs:519-535, `path_accepts` is false whenever `broken_prefix` changes along the path, and trade.rs:444-448 counts any step that is not exactly `step_micros` as broken.
  - Code (runner/src/outcome.rs): the exact-deadline branch at ~653-661 reads `let Some(want) = facts.at_timestamp(deadline) else { ret.push(None); refused.push(true); ...`. The path check at ~717-722, `if !facts.path_accepts(i, exit) { ... In trade.rs:444-448 `broken_prefix` counts every adjacent step that is not `step_micros`, and path_accepts (trade.rs:519-537) requires `gaps_after == gaps_before`.

**Expected fix and test:** None

## AC-whp-cx-1 · low bug · runner

**Where:** `crates/runner/src/outcome.rs:998`

**Finding:** `payoff_bp` was fixed to read the side from `mean_paisa < 0.0` (the code's own comment says the long-only reading 'ranked shorts in reverse order of merit'). The two sibling statistics that the other two lenses rank on were not fixed. `worst_reward_risk_bp` (Lens::Asymmetry, D-0593) always divides `min_win_paisa`, the smallest UP move, by `max_loss_paisa`, the largest DOWN move. `path_ratio_bp` (Lens::Path) always divides `favourable_sum`, the up excursion (high - entry), by `adverse_sum`, the down excursion. Both are long-oriented because `Sides::observe` and `forward` define win and favourable as upward. cli's `side_of_evidence` trades a mask SHORT exactly when `mean_paisa < 0.0`. For such a mask these keys are the long's asymmetry. An always-down sample, the ideal short, has no up move, so `min_win_paisa == 0` and `worst_reward_risk_bp` returns 0, the floor. `path_ratio_bp` on the same sample is near 0 because the down excursion is large. ByAsymmetry then breaks ties on `max_win_paisa`, the largest up move, which for a short is its worst loss. Worked example: an excellent short with moves [-300,-300,-300,+10] scores 10/300*100 = 3 bp (its true short figure is 300/10 = 3000 bp). 

**Evidence:**
  - D-0593 (docs/05-decisions.md:35276-35334) never me
  - cli's `side_of_evidence` is also the same on main (lib.rs:5249) and on c2-final (lib.rs:5484). Mechanism, confirmed on fix/c2-final: - **Asymmetry key is long-only.** `crates/runner/src/outcome.rs:998-1009` `worst_reward_risk_bp` returns 0 when `min_win_paisa <= 0.0`; otherwise it returns `min_win_paisa / max_loss_paisa * 100.0`. - **Path key is long-only.** outcome.rs:1207 `path_ratio_bp` computes `favourable_sum / adverse_sum * 100.0`, also with no side.

**Expected fix and test:** None

## ET-strategies-trades-ranking-costs-0 · low bug · runner

**Where:** `crates/runner/src/trade.rs:423`

**Finding:** Look-ahead (CLAUDE.md §3 rule 7). On a native, signal-sourced column the entry cadence is the median same-day step of the whole slice, including bars after the signal. Appending later bars changes which earlier signals count as immediate and when their horizon ends, even though the column bits for those earlier bars are identical. Reachable through runner's public trade::walk, SliceFacts::of, grid::evaluate* and outcome::forward on native columns. In the cli that means `pool` (pool.rs:559-578, whose stored_anchored_column is Sourced::Signal per indicators column.rs:592) and base_win_rate_bp (lib.rs:10263-10266). The audit and audit-stored paths reproject onto Sourced::Fill (lib.rs:15429-15457), which fixes the step at 60s, so they are immune. Real one-minute data would need more than half of its same-day steps off cadence to flip the median; that was not measured on stored data.

**Evidence:**
  - crates/runner/src/trade.rs:427-431: `SliceFacts::of` sets `step_micros` to `median_step_micros_over(bars, accepted)` when the column is `Sourced::Signal`. That function (crates/runner/src/outcome.rs:789-808) takes the median of every same-IST-day accepted step in the whole slice, so it includes bars after bar N. `broken_prefix` (trade.rs:444-446)
  - crates/runner/src/trade.rs:427-432: on a Sourced::Signal column, SliceFacts::of sets step_micros = median_step_micros_over(bars, accepted). That value is the median of every positive same-day step across the whole slice (crates/runner/src/outcome.rs:789-808). the immediacy test, trade.rs:785-788 (signal_bar.ts + step_micros == entry_bar.ts) the horizon deadline, trade.rs:1265 forward()'s deadline, outcome.rs:632

**Expected fix and test:** None

## ET-strategies-trades-ranking-costs-8 · low cost · runner

**Where:** `crates/runner/src/trade.rs:309`

**Finding:** Claimed: 'Per signal: one mask test, one same-day comparison, two costs::fill::Bar constructions and two worst_case_fills, each a fixed count of integer operations' Actual: The code calls fills_at with Anchor::Open and Anchor::PrintedExtreme (trade.rs:1124-1133), not worst_case_fills. It also does a HashMap probe per signal (horizon_bar trade.rs:1267-1269), which is expected O(1), not a fixed count. audit.rs:196-207 itself says 'Nothing in this crate calls that function'.

**Evidence:**
  - trade.rs:309-311 says per signal: "two `costs::fill::Bar` constructions and two `worst_case_fills`". trade.rs:1125-1126 builds FillBar::new twice. trade.rs:1127 calls `fills_at(.., Anchor::Open)`, and trade.rs:1128-1133 calls `fills_at(.., Anchor::PrintedExtreme)`. The comment at trade.rs:1100 says "This was `worst_case_fills`".
  - trade.rs:309-311 names "two `worst_case_fills`". The code in round_trip does something else: two FillBar::new calls (trade.rs:1125-1126), then fills_at(.., Anchor::Open) at trade.rs:1127 and fills_at(.., Anchor::PrintedExtreme) at trade.rs:1128-1133.

**Expected fix and test:** None

## W3-runner5-4 · low cost · runner

**Where:** `crates/runner/src/validate.rs:4644`

**Finding:** walk_forward_core (in-sample pass, closed.kept.par_iter) (crates/runner/src/validate.rs:4644): per one closed candidate priced on the training window, the cost is O(train rows) per candidate of redundant work: a second identical walk; it grows with train column rows + trades, for every candidate. The result is kept only for the winner.. Auditor verdict: undocumented-scan. Documented: Not in docs/06-limits.md. The comment at validate.rs:4556-4560 acknowledges "prices a full exit grid over the training bars AND walks them again for the summary" but does not say the second walk duplicates the grid's own ...

**Evidence:**
  - crates/runner/src/validate.rs:4631-4639 calls `crate::grid::evaluate_over(trade_train, train_column, &item.mask, horizon, side_of(own), .., &facts)`. crates/runner/src/grid.rs:2045-2049 converts `side` straight back to `costs::fill::Direction` and runs `let timed = crate::trade::walk_over(bars, column, mask, horizon, direction, facts);`.
  - In crates/runner/src/validate.rs, the closed.kept.par_iter body (from :4620) calls `crate::grid::evaluate_over(trade_train, train_column, &item.mask, horizon, side_of(own), crate::grid::Levels::derived(rungs), &facts)` at :4631.

**Expected fix and test:** None

## AC-whp-tb-1 · low bug · runner

**Where:** `crates/runner/src/validate.rs:6084`

**Finding:** `a >= b || b > a` is true for any two integers, so the assertion can never fail. Its own comment says the opposite: a change that reverts `held_up` to the level-less reading 'has to make this equality false to pass'. The test's other equality compares `held_up()` with a verbatim copy of its own filter. That copy is the same `Some(total) => total > 0, None => worst_case_positive()` rule as validate.rs:389-398, so it checks the function against itself. `held_up` is production: its count is printed by cli lib.rs:19340 and 19352-19353 and by runner audit.rs:1115. CLAUDE.md §4 bans a test that asserts nothing, and this assertion asserts nothing.

**Evidence:**
  - fix/c2-final:crates/runner/src/validate.rs:6129-6132 reads `assert!(counted >= old_rule || old_rule > counted, "unreachable: the two counts are always comparable")`. In it I replaced the `held_up` filter at validate.rs:394-395 with `f.out_of_sample.worst_case_positive()`, which is the exact revert the comment describes.
  - Confirmed by reading: fix/c2-final:crates/runner/src/validate.rs:6127-6130 is `assert!(counted >= old_rule || old_rule > counted, "unreachable: the two counts are always comparable")`. It exited 101 with: - "panicked at crates/runner/src/validate.rs:6124:9: assertion `left == right` failed: held_up disagrees with the chosen variant's own out-of-sample total / left: 0 / right:

**Expected fix and test:** None

# Tests, docs and security pass 15: do the CI gates fail when their rule is broken?

Head audited: `1f4de71` (detached checkout /home/claude/wt/zero3). Audit only; nothing in any checkout was edited.
Tag: tds15. IDs: P15-01 upward.

**Method.**
- I read `.github/workflows/ci.yml` (8036 lines), `auto-merge.yml`, `main-check.yml`, `.github/source_scan.rs`, `.github/invariant_paths.rs` and `.github/mutation_gate.rs`.
- I built `.github/source_scan.rs` with bare `rustc`, as gate 0 does, in my scratchpad. I then ran its `workflow`, `aggregator`, `step-runs`, `browser`, `paths`, `code` and `deps` subcommands against copies of ci.yml and small files, each with one violation planted.
- Where a gate is a shell regex, I replayed that regex on a planted file.
- No cargo was run. The only toolchain use was `rustc` on the single-file gate tool and two 3-line probes (rustc flag spelling, `-D unsafe_code` override).

**Who reviews what.** auto-merge.yml:316 asks for a code-owner approval only when a PR touches `.github/`, `CLAUDE.md` or `CODEOWNERS`.
- So a bypass that needs a **workflow** edit is backed by a human review.
- A bypass that needs only a **crate** edit merges on green with no human (P15-01, -10, -11, -14, -15, -16).
- Severity below reflects that.

**Branch protection** is not visible in the repo. auto-merge.yml says only `ci-ok` is required, and source_scan.rs:1931-1936 says the same. I could not verify either.

---

## Gate-by-gate table

"Lets through" is one concrete violation the rule forbids and the script passes. **ran** means I replayed the gate's own check with the violation planted.

| Gate | Stated rule | What the script actually checks | Lets through |
|---|---|---|---|
| 0 | workflow hygiene: no `continue-on-error`, no `\| grep -q`, no inline interpreter program; ci-ok guards all; no crate spawns a shell | line-prefix matching on `.github/workflows/*.yml`: a logical line beginning `continue-on-error:`, a stage starting `grep` with q, an interpreter word followed by an inline flag, heredoc or `-` | `"continue-on-error": true` (P15-04, ran). `shell: python {0}` with the program as `run:` (P15-07, ran). Step-level `if: false` on any gate (P15-06, ran) |
| 1 | tracked extensions only from §2, by path | per-record mode and extension check, line-wise `grep <<<` (ci.yml:171, :194), then `content` and `orphans` scans | known: newline in a name (P1-07-03), any `.json` at any depth under `.claude/` (P13-02). Both still hold, below |
| 1b | `.json`/`.yml` confined | `git ls-files` + line grep (ci.yml:405-408) | known: P1-07-03 (newline). Still holds |
| 1c | no literal credential path | `grep -niE "/x/(ten env words)/"` over tracked files | known: P1-07-02 (escapes, an 11th environment word) |
| 1d | every segment-shaped literal in crates/pull declared | scanner `strings` vs declared list | known: P1-07-02. P6-01 recorded it red at this head |
| 1e | no crate needs the front-end toolchain to build/test | build + test with 18 tool names shadowed on PATH. **web/ stays in place** | a test reading `web/masters.js` passes, though four texts say 1e detaches web/ (P15-13, by reading) |
| 1f | no browser code inside a Rust string literal | 15 API words + a tag walk, per literal and per `concat!` | `document.getElementsByTagName(..)`, `["<scr","ipt>…</scr","ipt>"].concat()`, `format!("<{t}>", t="script")` (P15-10, ran) |
| 1g | no tracked configuration runs a program | `.cargo/` and stray tool configs refused; workflow env regex | `CARGO_BUILD_RUSTDOC: /x`, `printf '%s=%s' RUSTC_WRAPPER /x >> "$GITHUB_ENV"` (P15-08, ran). Known: C-quoted paths (P1-07-04), still holds |
| 2 | no build script shells out | scanner `closure` + `build` on `*build.rs`; manifest `build=`/`links=` line regex (:1843) | known: P1-07-01 (quoted, dotted or inline `build` key). Still holds |
| 9 | core depends on nothing | scanner `deps` on crates/core/Cargo.toml | `#[path = "../../x/src/y.rs"] mod y;` (same shape as P15-11). Its failure message gives a reason that no longer exists (P15-12) |
| 9b | greeks depends on nothing, names no workspace type | scanner `deps` + `grep brutex_core\|core::(price…)` | `#[path = "../../core/src/blake3.rs"] mod blake3;` (P15-11, ran) |
| 10 | every invariant names a test that exists | crate-first token: `<crate> <fn>` exists anywhere in the crate; invariant_paths.rs for module-first tokens | a crate-first token naming a production fn of that name passes as a "test" (not run). Known red: P4-01, P5-01 |
| 10b / 27 | invariant id unique | `^\| [A-Z]+-[0-9]+[a-z]? ` / `^\| *`?ID`? *\|` | known: P6-04 (49 ids invisible) |
| 27b | a decision number heads one entry | `^#{2,4} +D-[0-9]{4}` | a second `D-0100` headed at level 1 or 5 (`# D-0100`) is not counted (not run) |
| 11 | banned constructs on O(1) paths | per-file regex counts vs allowlist | known: P1-08-04 (`/src/` glob, stem exemption). Known red: P6-02 |
| 12 | every O(1) claim names its proof | doc-comment trigger words → named test | known red: P6-03. Not replayed this pass |
| 13 | no foreign runtime | deny.toml/lock/manifests (layer 3 regex), layer 4 `step-runs 'cargo deny check'` | layer 4 accepts a disarmed deny step (P15-02, P15-03, ran). Layer 3: P1-07-01 |
| 13a/13b/13c, 4, 4a, 6a, 6b, Tests | cargo verdicts | real cargo commands, each under `if: steps.probe.outputs.has_crates == 'true'` | the probe step is unguarded: `-eq 0` → `-ge 0` skips all of them (P15-03, ran on the meta-checks) |
| 5 | unsafe exceptions ≤ 3 | `grep -ohE '(allow\|expect)\(unsafe_code[,)]'` (:7199) | `#[allow(unused, unsafe_code)]` counts 0 (P15-01, ran) |
| 16 | every crate root `#![forbid(unsafe_code)]` | raw `grep -qE '^#!\[forbid\(…unsafe_code'` on lib.rs/main.rs (:2866) | the line inside `/* … */` satisfies it (P15-01, ran; the probe compiled under `-D unsafe_code`) |
| 6c | `.github/*.rs` meet fmt/clippy | rustfmt --check + clippy-driver, test and non-test | none found |
| 6d | native Rust tests under web/ run | four hard-coded files (:7271-7272) | a new `web/**/*.rs` test file is never compiled (P15-17). Known: P12-07 (two skipped by name) |
| 15 | banned word in no tracked file | `grep -aoiF` per file vs allowlist | `concat!("py","thon")` or `\x74` escapes. Textual by design; not reported |
| 17 | nothing logs from inside the sweep | scanner `paths` of crate closure vs log/print/stdio regex (:3102) | `OpenOptions::new().append(true).open("/dev/stderr")` + `writeln!` in runner (P15-14, ran). Known: P1-08-03 (rename) |
| 23 | stderr is not a log; print surface declared | print/stdio path counts vs declared list | same `/dev/stderr` write (P15-14, ran) |
| 21 | logger opens only its own files | `std::(fs\|…)::fn` path counts per file vs declared | repointing one declared `File::open` argument to a bar path keeps every count (not run) |
| 22 | sweep crates cannot read a bar | deps pinned + BANNED regex + paths rule for vocab/indicators/engine | known: P1-08-03. `runner` is deliberately off the list |
| 24 | no writable memory mapping | regex for `MmapMut\|MmapRaw\|map_mut(\|map_anon(\|map_copy(` (:4318) | `memmap2::MmapOptions::new().map_raw(f)?` without naming the type (P15-16, regex ran) |
| 25 | release profile keeps overflow checks | TOML leaves + workflow regex `overflow-checks=(off\|…)` (:4473) | `RUSTFLAGS: … -C overflow_checks=off` (P15-09, ran: rustc accepts the spelling and wraps) |
| 26 | every HTTPS client installs the TLS provider | per-file count of `Client::builder()` vs `ensure_tls_provider()` text | `reqwest::Client::new()`; a comment holding `ensure_tls_provider()` (P15-15, ran) |
| 19 | a recorded failure is logged | `failures.push(`/`failures: vec![` needs an emit/`note_*(` within 12 lines above | `failures.extend([f])` (not run). Known: P1-08-04 |
| 20 | uncovered logger lines declared | llvm-cov report vs declared list | not replayed |
| coverage | 100% per §9 | `--fail-under-lines 90 --fail-under-regions 89` | recorded (hunt-ci-10, D-0677); not re-reported |
| 18 | no surviving mutant on touched modules | cargo-mutants over the diff, sharded, reconciled | known: P1-08-01 (`cfg_attr(miri, mutants::skip)`) |
| 8 | O(1) ratio benches run and pass | `git ls-files --error-unmatch` + `cargo bench --workspace --locked` | known: P1-08-02 (`bench = false`). Disarmable past gate 14 layer 5 (P15-02, P15-03, ran) |
| 14 | every bound claim re-measured; gate 8 armed | table + layer 5 `step-runs` + whole-line grep | P15-02, P15-03 (ran) |
| W1 | web/build matches web/src | rebuild + `git diff --exit-code` | sound. Known red: P13-01 |
| W2-W6 | front-end tests, svelte-check, CSS checks | under web/ (unrestricted by §2) | W6 is text-only by its own comment. Not reported |
| ci-ok | every job must be `success` | `needs` all jobs, `if: always()`, the three loop lines present | `RESULTS=success` or `exit 0` before the loop (P15-05, ran) |
| 7 | (skipped by design, CLAUDE.md §5) | n/a | not reported. Note: "The browser crate actually builds for the browser" (:7281) also skips permanently with `[ -d crates/web ] \|\| { echo "skip"; exit 0; }`; it is the same design |

**Other checks.**
- **`continue-on-error`:** none in any workflow, in any spelling I found (comments only).
- **`|| true`:** every occurrence guards a grep or diff whose no-match or difference is the pass case, or a probe that is checked next.
  - Exit 2 (a grep error) is swallowed along with exit 1 in 1c (:597-598), 9b (:2002-2004), 22 (:4087) and 25. Every input there is a tracked file, so I found no live miss.
- **Missing `set -e`:**
  - Gates 26 and 27 use `set -eu` without pipefail. Gate 27's one pipe fails closed: a missing file gives `n=0`, which is refused.
  - W1, W3 and the mutation-gate self-test step rely on GitHub's default `bash -e {0}`. W3's `| tee || true` is intentional and followed by a "missing" refusal.
  - No live miss.
- **Always-true skip conditions:** only gate 7 and the wasm32 build step above.
- **Wrong file in a failure message:** P15-12.

---

## Findings

### P15-01 medium: gates 5 and 16 both pass a production crate root whose `forbid` is commented out and whose `unsafe` is allowed by a two-lint `allow`, and the change auto-merges without review

**Where:**
- ci.yml:2866: `if grep -qE '^#!\[forbid\([^]]*\bunsafe_code\b' "$f"; then` (gate 16 layer 1, raw text, comments included)
- ci.yml:7199: `pat='(allow|expect)\(unsafe_code[,)]'` (gate 5; it counts only when `unsafe_code` is the first lint)

**Why it is wrong:**
- Gate 16's rule is that every crate root *carries* `#![forbid(unsafe_code)]`. Its own comment says a forbid is "a wall" because a deny "is one edit from gone".
  - The check reads raw lines, so the attribute inside a block comment satisfies it.
- Without the forbid, the workspace `unsafe_code = "deny"` is a lint level. `#[allow(unused, unsafe_code)]` overrides it.
- Gate 5's pattern needs `(unsafe_code` right after `allow`, so that spelling counts 0 of its 3 exceptions.
- Gate 16 layer 1b scans only test, bench, build and bin roots, so nothing else sees the `unsafe` in lib.rs.
- The change touches only `crates/`, so auto-merge.yml:316 asks for no code-owner approval.

**Repro (ran):**
```
/*
#![forbid(unsafe_code)]
*/
#[allow(unused, unsafe_code)]
pub fn peek(p: *const u8) -> u8 { unsafe { *p } }
```
- Gate 16's grep matches ("forbid seen").
- Gate 5's grep counts 0.
- `rustc --edition=2024 --crate-type lib -D unsafe_code` compiles it.

**Minimal fix:**
- Gate 16: run the root through `"$SOURCE_SCAN" code-prod` (comment-stripped) before the grep.
- Gate 5: match `(allow|expect|warn)\([^)]*\bunsafe_code\b`.
- Or run `"$scan" unsafe` over lib.rs/main.rs closures too, as layer 1b does for other roots.

### P15-02 low: `step-runs` (gate 13 layer 4, gate 14 layer 5) accepts a step whose command is disarmed by a one-line function, an EXIT trap, an earlier `exit 0`, or a continuation into `|| true`

**Where:**
- source_scan.rs:1802-1874 `swallowed()`, which judges only the needle line itself (`if let Some(rest) = body.strip_prefix(needle)`, :1820) and block depth.
- Used at ci.yml:2549, :2556, :6943 and :6955.

**Why it is wrong:** the function claims "a line decides the step only when it sits at the top level … not followed by `|`, `&` or `;`". Four shapes keep the line at top level and still decide nothing:
1. `cargo() { :; }` on one line before it. The words `{` and `}` cancel, so depth stays 0, and the shell function shadows cargo.
2. `trap "exit 0" EXIT` before it.
3. `exit 0` before it.
4. `cargo deny check \` followed by a next line `|| true`. The `rest` of the needle line is ` \`.

The scanner's own test (:3143) covers only the multi-line function form.

**Repro (ran):**
- I planted each shape in a copy of ci.yml (1 and 2 on gate 8's `cargo bench --workspace --locked`, 4 on gate 3's `cargo deny check`).
- `scan step-runs` printed `present … unconditionally and blocking`. Gate 14's whole-line grep also matched.
- `bash -c 'set -euo pipefail; cargo() { :; }; cargo bench --workspace --locked; echo after'` → `after`, rc 0.
- `bash -c 'set -euo pipefail; trap "exit 0" EXIT; false'` → rc 0.
- `false \⏎ || true` under `set -euo pipefail` → survives.

**Minimal fix:** in `swallowed()`:
- refuse any line before the needle that defines a function, sets a `trap`, or runs `exit`/`return` at top level;
- treat a needle line ending in `\` as joined with the next line, as `workflow_findings` already does.

### P15-03 low: the build job's probe condition is accepted in any job, and the probe step itself is unguarded, so one edit switches off fmt, clippy, deny, build, test and 13a-c

**Where:**
- source_scan.rs:1771: `"always()" | "success()" | "!cancelled()" | "steps.probe.outputs.has_crates=='true'"`
- ci.yml:6998: `if [ "$n" -eq 0 ]; then` (the probe, which nothing reads)

**Why it is wrong:**
- `condition_is_safe` justifies the probe condition by "on any run that reaches [the build job] the probe is true". It does not check that the step is *in* the build job, or that the probe still computes that.
- Case (a), the condition on gate 8's bench step in `complexity`:
  - `complexity` has no step with `id: probe`, so the expression is `null == 'true'`, which is false.
  - The step is skipped, the job is `success`, and gate 14 layer 5 prints `present`.
- Case (b), editing the probe from `-eq 0` to `-ge 0`:
  - The probe always writes `has_crates=false`.
  - Ten build-job steps skip, with only a `::warning`.
  - Gate 13 layer 4 still prints `present` for `cargo deny check`.

**Repro (ran on the meta-checks; GitHub evaluation not run):**
- (a) `if: steps.probe.outputs.has_crates == 'true'` under `Per-operation cost…` gives workflow 0, aggregator 0, step-runs 0, and gate 14's line grep matches.
- (b) `-eq 0`→`-ge 0` at :6998 gives step-runs for both deny needles `present`, and workflow and aggregator clean.

**Minimal fix:**
- Accept the probe condition only for steps in job `build`.
- Have gate 14 or gate 13 pin the probe's script with an exact-line check, as layer 5 does for the bench line. Or drop the probe, since gate 16 already refuses a crate-less tree.

### P15-04 low: gate 0 and `step-runs` see `continue-on-error` only as an unquoted key at the start of a line

**Where:** source_scan.rs:2100 (`if key.starts_with("continue-on-error")`) and `own_key` :1776-1795 (the same prefix rule).

**Why it is wrong:**
- YAML reads `"continue-on-error": true`, and the flow form `- { name: x, continue-on-error: true, run: … }`, as the same key. Workflows already rely on quoted keys (the common `"on":`).
- Gate 0's comment says `continue-on-error` *anywhere* turns a red step green.

**Repro (ran):**
- I added `        "continue-on-error": true` under gate 8's step.
- `scan workflow`, `aggregator` and `step-runs` all exit 0, and gate 14's grep matches.
- GitHub's acceptance of the quoted key: not run.

**Minimal fix:** in `workflow_findings`, refuse any line containing `continue-on-error` outside a comment (strip quotes and `{`), or parse the YAML keys.

### P15-05 low: the ci-ok shape check proves three lines are present, not that they run on the real results

**Where:** source_scan.rs:1994-2005 (`for want in [...] { if !body.contains(&want) …`).

**Why it is wrong:**
- `aggregator_findings` is the guard that keeps every gate blocking (D-1601).
- A line `RESULTS=success` or `exit 0` above `for x in ${RESULTS}; do` leaves all three wanted lines in place and makes ci-ok green whatever the jobs returned.

**Repro (ran):**
- I inserted `RESULTS=success` before the loop in a copy. `scan aggregator` exits 0.
- `bash -c 'RESULTS="failure skipped"; RESULTS=success; for x in ${RESULTS}; do [ "$x" = "success" ] || exit 1; done; echo all gates green'` prints `all gates green`.

**Minimal fix:** compare ci-ok's whole `run:` body to an exact expected text, and refuse any extra line.

### P15-06 low: only four step lines in the workflow are protected against a skip condition, and every other gate step can be switched off with `if: false`

**Where:**
- source_scan.rs:2081-2140 `workflow_findings`, which has no `if:` rule.
- `step-runs` is called only for `cargo deny check`, `cargo install cargo-deny --version `, `cargo bench --workspace --locked` and the gate 8 `git ls-files` line.

**Why it is wrong:**
- A skipped step leaves its job `success`, and ci-ok accepts that.
- The allowlist argument D-1102/D-1503 made for gates 3 and 8 ("a line's PRESENCE is not a line's EXECUTION") applies to every gate. It is enforced for four lines.

**Repro (ran):** `        if: false` under `- name: Gate 1 — extension allowlist` gives workflow 0, aggregator 0, step-runs 0.

**Minimal fix:** in `workflow_findings`, refuse any step-level or job-level `if:` that `condition_is_safe` rejects, for every job ci-ok needs, outside the steps the build probe guards.

### P15-07 low: gate 0's inline-program rule misses a step whose `shell:` is an interpreter

**Where:** source_scan.rs:2029-2079 `inline_programs`. For `node`, `py…`, `perl` and the rest, the word after the interpreter must be an inline flag, `-`, or a heredoc.

**Why it is wrong:**
- `shell: python {0}` (or `node {0}`, `perl {0}`) makes the `run:` body itself the program. That is the "interpreter handed a program inline" gate 0 refuses.
- `{0}` is neither a flag nor `-`.
- Gate 0 also reads only `.github/workflows/*.yml`, so a composite `.github/actions/*/action.yml` (a `.yml` gate 1 admits under `.github/`) with the same step is not read at all.

**Repro (ran):** a file with three steps, `shell: python {0}` / `node {0}` / `perl {0}`, each with an inline body. `scan workflow` exits 0.

**Minimal fix:**
- Refuse `shell:` values other than `bash`/`sh`.
- Read every tracked `.github/**/*.yml`.

### P15-08 low: gate 1g misses `CARGO_BUILD_RUSTDOC` and a `$GITHUB_ENV` write that does not spell `NAME=`

**Where:** ci.yml:477, the env regex. Its tail is `…|CARGO_BUILD_RUSTC|RUSTC|RUSTDOC|…)[[:space:]]*[:=]`, with `(^|[^A-Za-z0-9_])` before it.

**Why it is wrong:**
- Cargo's `CARGO_BUILD_RUSTDOC` names the rustdoc program `cargo test` runs for doctests. `RUSTDOC` is matched only when it is not preceded by `_`.
- `printf '%s=%s\n' RUSTC_WRAPPER /tmp/z >> "$GITHUB_ENV"` sets the wrapper for every later step, but the variable name is not followed by `=` or `:` in the source.

**Repro (ran):** the gate's grep over a file holding `CARGO_BUILD_RUSTDOC: /tmp/x`, `RUSTC_WRAPPER: /tmp/y` and the printf line reports only the `RUSTC_WRAPPER:` line.

**Minimal fix:**
- Add `CARGO_BUILD_RUSTDOC` to the variable list.
- Refuse any write to `$GITHUB_ENV` whose line names a listed variable in any position.

### P15-09 low: gate 25 misses `-C overflow_checks=off`, which rustc accepts and which wraps

**Where:** ci.yml:4473: `flag_key="over""flow-checks[[:space:]]*=[[:space:]]*(off|no|n|false|0)"` (hyphen only).

**Why it is wrong:**
- rustc normalises `-C` option names, so `overflow_checks` is the same option.
- RUSTFLAGS is appended after cargo's own `-C overflow-checks=on`, and the last flag wins.
- Gate 1g's RUSTFLAGS clause looks only for linker words.

**Repro (ran):**
- `RUSTFLAGS: -D warnings -C overflow_checks=off` in a planted workflow: the gate's grep exits 1 (no match).
- `rustc -C overflow_checks=off` on `let y: u8 = black_box(255) + 1` prints `0`. With `overflow-checks=on` it panics.

**Minimal fix:** `over""flow[-_]checks`, and also refuse `-Coverflow[-_]checks` without the `=` value forms. Or refuse any `-C` overflow option in a workflow.

### P15-10 low: gate 1f passes browser code that uses no listed API word and tags that are not one literal or one `concat!`

**Where:** source_scan.rs:615-685 (`browser_findings`, 15 fixed words plus a per-literal tag walk) and :687-723 (only the `concat!` macro is joined).

**Why it is wrong:** the rule is "no browser code inside a Rust string literal". The scan is a denylist of 15 spellings plus tags within one literal.

**Repro (ran):** `scan browser` exits 0 on:
- `"for (const p of document.getElementsByTagName('td')) { p.textContent = location.hash; fetch('/x').then(r => r.json()); }"`
- `["<scr", "ipt>alert(1)</scr", "ipt>"].concat()`
- `format!("<{t}>alert(2)</{t}>", t = "script")`

This is a crate-only change, so it auto-merges.

**Minimal fix:**
- Also join adjacent literal elements of an array or slice passed to `.concat()`/`.join()`, and the `format!` template with its literal arguments.
- Add `document.` and `location.` member access, `.then(`, `=>` inside a literal, and `fetch(` to the word list (prose rarely carries those inside a literal).

### P15-11 low: gate 9b (and gate 9) passes `#[path]` inclusion of another workspace crate's source

**Where:** ci.yml:2002-2004 (`leaks=$(… | xargs -r grep -nE '\b(brutex_core|core::(price|instrument|universe|vendor))\b' || true)`) plus the manifest `deps` check.

**Why it is wrong:**
- "greeks depends on nothing" (D-0046: it is shared by git URL).
- `#[path = "../../core/src/blake3.rs"] mod blake3;` compiles core's code into greeks with no manifest dependency and no matching name. `core/src/blake3.rs` refers only to `super::` inside its tests.

**Repro (ran):** the leak grep returns rc 1 (no match) on that file, and `scan deps crates/greeks/Cargo.toml` is clean.

**Minimal fix:** run `scan closure` from `crates/greeks/src/lib.rs` (and core's) and refuse any file outside `crates/greeks/` (outside `crates/core/` for gate 9).

### P15-12 low: gate 9's failure message gives a reason and a decision that no longer apply

**Where:** ci.yml:1911: `echo "core is compiled to wasm32 through crates/web. See D-0009."`

**Why it is wrong:**
- `crates/web` does not exist. CLAUDE.md §5 ("The browser is not a crate") says so, and gate 7 skips for that reason.
- D-0009 (docs/05:127) is "The browser crate depends on `core` alone".
- An author who trips gate 9 is sent to a crate and a decision that do not bind core today. The live reasons are the shared-core contract (docs/10) and §5's gate 9.

**Repro:** not run; by reading.

**Minimal fix:** reword to the current reason and cite the decision that keeps core dependency-free now.

### P15-13 low: gate 1e does not move `web/` aside, but four texts say it does and rely on it

**Where:**
- ci.yml:257-263, gate 1e's own comment: "§2 does not require the build to survive `web/` being absent … The old gate moved the whole `web/` tree aside".
- The texts that claim the opposite:
  - CLAUDE.md:65-66: "gate 1e's premise — that the workspace builds with the front end moved aside".
  - ci.yml:7913-7914 (W6): "gate 1e runs the whole workspace with the `web/` tree DETACHED".
  - crates/api/src/mastersrun.rs:1568-1571: "A test that opened that file would go red in the one job whose green is §2's guarantee".
  - crates/api/src/assets.rs:855-857: "the exact coupling gate 1e detaches the tree to find".

**Why it is wrong:**
- A crate test or `include_str!` that reads `web/masters.js` or `web/build/*` passes 1e: the files are present.
- Nothing else refuses it: gate 1's `orphans` exempts `web/`, and gate 22 checks only the sweep crates.
- The api comment's justification for splitting a proof rests on a protection that does not exist.

**Repro:** not run; by reading. 1e's script (:246-338) has no `mv` or `git worktree` of web/.

**Minimal fix:** either:
- restore a detached-web/ test run in 1e, keeping the exit-status checks; or
- correct the four texts and add a scan refusing `web/` paths in crate sources.

### P15-14 low: gates 17 and 23 miss a write to stderr through `/dev/stderr` opened with `std::fs`

**Where:**
- ci.yml:3102, gate 17's `rule` (log crates, print macros, `io::stdout|stderr`).
- Gate 23's awk (:3211-3220).

**Why it is wrong:**
- `std::fs::OpenOptions::new().append(true).open("/dev/stderr")` and `writeln!` produce the same per-trial stderr output.
- Neither path matches: the scanner reports `std::fs::OpenOptions::new` and `writeln!`.
- `runner` (swept, per gate 17) is not on gate 22's std::fs list.

**Repro (ran):** `scan paths` on the 6-line file, then gate 17's rule and gate 23's print/handle patterns: both rc 1 (no match).

**Minimal fix:** refuse `"/dev/stderr"`, `"/dev/stdout"`, `"/dev/tty"` and `"/proc/self/fd/"` literals in swept crates (gate 17), and count them as handles (gate 23).

### P15-15 low: gate 26 counts strings per file, so an unguarded client or a commented guard passes

**Where:** ci.yml:4623-4626 (`grep -c 'Client::builder()'` vs `grep -c 'ensure_tls_provider()'`).

**Why it is wrong:**
- `reqwest::Client::new()` (and `ClientBuilder::new()`, `reqwest::get`) builds a client without the substring.
- A comment `// ensure_tls_provider()` counts as a guard.
- The two counts are per file, not per call site.

**Repro (ran):** the gate's two greps over a file with `Client::new()`, one `Client::builder()` and the guard only in a comment give `n=1 g=1`, which is `ok`.

**Minimal fix:**
- Use `scan paths-prod` and count `reqwest::Client::new`, `ClientBuilder::new` and `reqwest::get` as sites.
- Count only the code-path `ensure_tls_provider`. Better: make `pull::http` the only constructor.

### P15-16 low: gate 24's regex misses memmap2's safe `map_raw`, which returns a writable mapping

**Where:** ci.yml:4318 `BANNED_MAP='MmapMut|MmapRaw|…map_mut\(|…map_anon(_mut)?\(|…map_copy\('`

**Why it is wrong:**
- `MmapOptions::map_raw` is a safe fn whose result is read-write. It needs no `unsafe`, so gates 5 and 16 do not stop it.
- If the type name `MmapRaw` is never written, nothing matches.
- §4 bans a writable mapping without exception.

**Repro:**
- (ran) the regex over the comment-stripped view of `memmap2::MmapOptions::new().len(4096).map_raw(f)?` gives rc 1.
- (not run) a compile against memmap2; it is not in Cargo.lock today.

**Minimal fix:** add `map_raw\(` and `make_mut\(`, or refuse the `memmap2`/`memmap` dependency by name in gate 13 or deny.toml.

### P15-17 low: gate 6d runs four hard-coded files, so a new native Rust test under `web/` is never compiled or run

**Where:** ci.yml:7271-7272 (`for f in web/sweep-readiness/frontend-publish.rs web/sweep-readiness/main-inspector.rs web/sweep-readiness/deployment-preflight.rs web/saved-backtest/viewer.rs`).

**Why it is wrong:**
- The rule is "the native Rust tests under web/ run".
- Gate 1's `orphans` exempts `web/` (source_scan.rs, `!f.starts_with("web/")`), so a new `web/x/y.rs` with a failing `#[test]` is compiled by nothing and is not refused.
- Today `web/sweep-readiness/probes/support_lanes.rs` is compiled by no CI step, and `verify.rs` is built but not run.

**Repro (ran):** `git ls-files 'web/*.rs'` lists 10 files. Four are roots in the loop, three are `#[path]`-mounted, and support_lanes.rs and verify.rs sit outside the loop.

**Minimal fix:** derive the roots from `git ls-files 'web/*.rs'` minus files mounted by `#[path]` (scanner `modules`), and refuse an unlisted root.

---

## Verification of known items (all at the same head 1f4de71)

| ID | Status | Evidence |
|---|---|---|
| P1-07-01 | NOT FIXED | ci.yml:1843 and :2472 still `grep -E ':[[:space:]]*(build|links)[[:space:]]*='` |
| P1-07-03 | NOT FIXED | ci.yml:171 and :194 still `grep -Eq … <<< "$base"` / `<<< "$ext"` (line-wise); 1b :405-408 line-wise |
| P1-07-04 | NOT FIXED | ci.yml:441 `listing=$(git ls-files)`, no `-z`, no `core.quotePath=false` |
| P13-02 | NOT FIXED | ci.yml:126 `claude_allowed="${allowed}|json"`, applied to `.claude/*` at any depth (:191) |
| P13-05 | NOT FIXED | web/sweep-readiness/verify.rs unchanged at this head; CI builds it (:7270) and never runs it |

Gate 7 is not reported (skipped by design, CLAUDE.md §5).

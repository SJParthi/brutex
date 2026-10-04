# Pass 6: tests, docs and security (final/all-fixes-zero @ 1f4de71)

**Verdict at 1f4de71:** the zero-findings head would turn three more language-purity gates red if it were merged into #74. Those gates are 1d, 11 and 12. Gate 10 is already red: P4-01 and P5-01 are still open, and they are the only gate-10 failures. All three new failures are regressions: the same gate scripts pass on PR #74's head, 73441e5. The zero-fix commits that caused them are 9c6284c, ad14eac, df31af3, 092a693 and fd45f9d. None of them is in #74 yet, and no CI run exists for 1f4de71 (`gh api actions/runs?head_sha=1f4de71…` returns 0 runs).

The Rust-only rule (CLAUDE.md §2) holds on every tracked file. The crate graph and the vocabulary documents match the code exactly.

**New findings: 4 (3 medium, 1 low).** P1-07, P1-08, P1-09, P4-* and P5-* are not re-reported.

**Method (ran).**
- Every gate step in `jobs.language-purity` except gate 1e was extracted from ci.yml and run with bash in a detached scratch worktree at 1f4de71. That is steps 0, 1, 1b, 1g, 1f, 1c, 1d, 2, 9, 9b, 10b, 7, 13, 15, 16, 17, 23, 21, 22, 24, 25, 27, 27b, 26, 19, 10, 11, 12 and 14.
- Gate 0 compiled `.github/source_scan.rs` with rustc, as CI does: its 46 tests passed, and its `workflow`, `aggregator` and `spawns` checks passed. Gate 10 compiled `.github/invariant_paths.rs` the same way.
- The scripts that fail were re-run against 73441e5, and all three passed there.
- The environment was this container's mawk 1.3.4 and GNU grep 3.11. D-0065 records that ubuntu runners also use mawk.
- Cargo was used read-only: `cargo metadata --no-deps` and `cargo tree -e normal,build,dev`. Nothing was compiled.
- The scratch worktree has been removed.

---

## Theme 1: the Rust-only rule (CLAUDE.md §2), all 982 tracked files

| Check | Result |
|---|---|
| Extensions outside `web/` (671 files) | 600 `.rs`, 46 `.md`, 18 `.toml`, 3 `.yml`, 1 `.lock`, 1 `.json`, plus `.gitignore` and `CODEOWNERS`. **All are allowed.** There is no `.html` or `.css` outside `web/`. `LICENSE` is not tracked; it is allowed, not required. |
| `.yml` only under `.github/` | 3 files, all `.github/workflows/*.yml`. **Holds.** |
| `.json` only under `.claude/` (outside `web/`) | 1 file, `.claude/launch.json`. **Holds.** |
| `web/` (311 files: js/ts/svelte/mjs/css/html/json/svg) | Unrestricted by D-0052 and D-0053. **Allowed.** |
| File modes and names | All files are 100644. There are no symlinks, no executables and no non-printable or tab characters in names. |
| `build.rs` files | Only `crates/cli/build.rs`, which includes `build_provenance.rs` and `commit_stamp.rs`. It reads `.git` objects through `sha1` and `flate2`. It has no `Command` and does not touch `web/`. Gate 2 lists exactly these three files. **Holds.** |
| build-deps and dev-deps (from `cargo metadata`) | `cli` build: `sha1`, `flate2`. `cli` dev: the same two. No other crate has any. There is no process-running crate (duct, xshell, assert_cmd, escargot). **Holds.** |
| Native code that is actually built | `cargo tree -e normal,build,dev -i ring` prints "nothing to print". No `*-sys`, `cc`, `cmake`, `bindgen` or `pkg-config` is in the host build graph. `ring` and `cc` stay in Cargo.lock as optional and unbuilt (D-0074, D-0211; gate 13b). **Holds.** |
| `cargo` build, test and clippy depending on `web/` | No `include_str!` or `include_bytes!` reads `web/` (assets.rs:855 records the removal). Gate 1e (not replayed) moves `web/` aside. **Holds by source.** |
| `std::process::Command` in production crates | The programs started are `git`, `mkfifo`, `current_exe()` (a self re-exec), and `open`, `explorer.exe` or `xdg-open`. `xdg-open` is allowed by name in D-1202 item 5. No interpreter is started (gate 0 `spawns` passed). |
| `.github/` workflow steps that run another language's tooling | **`web` job:** `actions/setup-node`, `npm ci --prefix web`, `npm run build` and `npm run check`, and `node` running `web/ci/css-comments.mjs` and `web/tests/*.test.js` (ci.yml:7791-8005). Every program it runs is a file under `web/`, so §2's `web/` exception covers it, and gate 0 refuses inline `node -e`. **Allowed.** **All jobs:** steps are bash with awk (72 non-comment uses in ci.yml, including multi-line inline awk programs such as gate 19's `strip=` at ci.yml:4754) and sed (66). `auto-merge.yml` and `main-check.yml` use `gh --jq` (8 and 2 uses). §2 forbids "any interpreted runtime … as a tool", so taken literally this conflicts with the rule. D-1602 (docs/05-decisions.md:54219) records that "`awk` is not refused … whether shell in a workflow is a second language is the law question the prior pass raised and left to the operator". **UNDECIDED. This is an open law question, not a defect a gate can fix.** |

## Theme 2: replay of each `language-purity` gate at 1f4de71

| Gate | What it checks | Ran non-vacuously? (evidence) | Verdict |
|---|---|---|---|
| 0 | Scanner self-test; workflow hygiene; `ci-ok` aggregator; crate code spawning a shell or interpreter | Yes: 46 tests; 3 workflows; 600 `.rs` files | PASS |
| 1 | Extension allowlist by location; file modes; every `.rs` is compiled | Yes: "read 982 tracked file(s)" | PASS |
| 1e | Workspace builds with `web/` and its tools absent | Not replayed (it is a build) | — |
| 1b | `.json` and `.yml` confined | Yes (the listing is guarded against being empty) | PASS |
| 1g | No tracked config names a program | Yes: rust-toolchain.toml has 5 keys, nextest.toml has 21 | PASS |
| 1f | No browser code in a Rust string | Yes: 597 files | PASS |
| 1c | No literal credential path | Yes (guarded against being empty) | PASS (the P1-07-02 bypass is already recorded) |
| **1d** | Every segment-shaped literal in crates/pull is declared | Yes: 55 files, 593 literals | **FAIL: 7 undeclared literals. New: P6-01.** |
| 2 | No build script shells out | Yes: 3 build files | PASS |
| 9 / 9b | `core` and `greeks` depend on nothing | Yes; `cargo metadata` agrees | PASS |
| 10b | Invariant ids are unique (old pattern) | Matches 1,459 ids and has no zero guard | PASS, but its coverage is narrower than gate 27's (P6-04) |
| 7 | `crates/web` depends only on core | **Vacuous by design**: "skip — crates/web does not exist" | SKIP (P1-08-05 already recorded) |
| 13 | No foreign runtime in manifests, lock, build scripts or deny.toml | Yes: 14 manifests, 1,867 lock lines, 29 names | PASS. It warns three times that deny.toml cannot list three banned runtime crates. Those crate names contain the word gate 15 bans in tracked files, so deny.toml cannot spell them. Gate 13b covers what is actually built. Known; designed. |
| 15 | Banned word is absent | Yes: 4 allowed files | PASS |
| 16 | Every crate root has `forbid(unsafe)`; every member inherits the lints | Yes: 15 roots, 113 test roots | PASS |
| 17 | Nothing logs inside the sweep | Yes: 75 files | PASS |
| 23 | stderr is never the only channel; temp paths | Yes | PASS |
| 21 | The logger opens only its own files | Yes: declared equals measured (6 calls) | PASS |
| 22 | Sweep crates cannot read a bar | Yes | PASS |
| 24 | No writable mapping | Yes: 597 files. `memmap2` is declared in the workspace but used by no member and absent from the lock. | PASS |
| 25 | Release profile has overflow checks on and `panic = "abort"` | Yes | PASS |
| 27 / 27b | Invariant-id and decision-number uniqueness | Yes: 1,730 ids; 1,124 headings | PASS (27 is blind to 49 ids: P6-04). Every D-number cited in a tracked file or in a commit message since 331b05c has a heading. The exceptions are D-0051, D-0676 and the renumbered lane ids, which are already recorded by D-0684 and D-1197. |
| 26 | Every HTTPS client installs the TLS provider | Yes: 7 sites in 6 files. No `Client::new()` or `ClientBuilder::new` exists to slip past the needle. | PASS |
| 19 | A failure an operator can see is logged | Yes: 27 sites | PASS |
| **10** | Every invariant names a test that exists | Yes | **FAIL: P5-01 (lines 3442 and 6424) still open. No new miss.** |
| **11** | Banned constructs on the O(1) paths | Yes: 374 files | **FAIL: rule 7 refuses 3 files. New: P6-02.** |
| **12** | Every O(1) claim names its proof | Yes: 460 claim blocks | **FAIL: 2 refused. New: P6-03.** |
| 14 | Every crate that claims a bound re-measures it | Yes: layers 1-5 | PASS |

The same scripts at 73441e5 (PR #74's head): gate 1d rc=0, gate 11 rc=0, gate 12 rc=0.

## Theme 3: documents against the code

| Check | Result |
|---|---|
| CLAUDE.md §5 against `cargo metadata --no-deps` | **Exact match** for all 13 members. core, vocab, greeks and telemetry have no dependencies. costs→core. store and lake→core, telemetry. indicators and engine→vocab. pull→core, costs, greeks, store, telemetry. api→core, pull, store, telemetry, cli, vocab. runner→core, costs, engine, indicators, vocab. cli has its nine arrows. There are no workspace dev-dependency arrows. The root `members` list holds the 13 directories. |
| docs/01-architecture.md table (lines 30-42) | Exact match with the same graph. |
| docs/03-vocabulary.md against `vocab::table::TABLE` | **370 of 370 rows identical** (bit to name, dense 0-369, in order). The retired and void status matches exactly: retired 6, 19, 25; void 235-273. §8's counts are right: 328 live, 3 retired, 39 void, 81 of 97 `Near`, 14 free. `VOCAB_VERSION` = 3. |
| Append-only rule (§3.8) | Every visible revision of table.rs (7 commits since import ffa41c6) maps each index to the same name it has now. Nothing is renumbered or reused. |

---

## New findings

### P6-01 medium: CI gate 1d is red at 1f4de71, because D-1769's new crates/pull tests and code add 7 undeclared segment-shaped literals

**Where:** ci.yml:613 (gate 1d). The literals:
- crates/pull/src/rolling.rs:1212 `r#""junk""#`
- crates/pull/src/rolling.rs:1213 `"1e-7"`
- crates/pull/src/rolling.rs:1216 `"99999999999999999999"`
- crates/pull/tests/folder.rs:911 `scratch.dir("one-wrapper")`
- crates/pull/src/http.rs:5554 `if post { "post_json" } else { "Discovery::get" }`
- crates/pull/src/folder.rs:646 `root_from(None, Some("rel".into())).expect_err("relative HOME")`
- crates/pull/src/folder.rs:647

All come from 9c6284c (D-1769). The http.rs:5554 literal also makes the older 5436 and 5630 sites count.

**Why it is wrong:** gate 1d refuses any segment-shaped literal in crates/pull that its declared lists do not name: "Either the literal is invented — add it above — or it is a real path segment". Merging this head into #74 turns `language-purity`, and so `ci-ok`, red.

**Repro (ran):** run the extracted gate-1d step at 1f4de71. It ends with rc=1 and these 7 lines: `UNDECLARED SEGMENT-SHAPED LITERAL IN crates/pull: 1e-7 | 99999999999999999999 | junk | one-wrapper | post_json | rel | relative`. The same script at 73441e5 gives rc=0.

**Fix:**
- Add `junk 1e-7 99999999999999999999 one-wrapper rel relative` to gate 1d's `scaffold` list.
- Add `post_json` to its vendor-wire or scaffold list.
- Re-run the step.

### P6-02 medium: CI gate 11 rule 7 is red at 1f4de71, because three zero-fix commits each add a `.contains(&…)` without raising the allowlist

**Where:** the rule-7 `allow_member` list at ci.yml:5602-5603 says `crates/api/src/server.rs 4` and `crates/api/src/sweeprun.rs 5`, and has no `crates/lake/src/footer.rs`. The new sites:
- crates/api/src/server.rs:9903 `status.is_some_and(|code| code != 429 && !(200..=299).contains(&code))` (9c6284c, D-1769)
- crates/api/src/sweeprun.rs:861 `let read = reads.contains(&name)` (ad14eac, D-1970..D-1974)
- crates/lake/src/footer.rs:267 `if (BOOL_TRUE..=UUID).contains(&kind) {` (df31af3, D-1980)

ad14eac also replaced sweeprun.rs's only rule-6 `.iter().find/position` site, so ci.yml:5555 `crates/api/src/sweeprun.rs 1` (rule 6) is now loose. The gate warns `Gate 11 allowlist is loose::crates/api/src/sweeprun.rs no longer matches rule 6`.

**Why it is wrong:** every site is bounded: two are `Range` probes and one is a fixed-width `reads` slice. But the gate's law is "a count is raised only by someone who can say why", with the reason recorded in docs/06-limits.md §"Gate 11 allowlist reasons" (line 12596). Neither the count nor the reason was updated, so the gate fails.

**Repro (ran):** the extracted gate-11 step gives `REFUSED crates/api/src/server.rs — 5 occurrence(s), 4 allowed`, `REFUSED crates/api/src/sweeprun.rs — 6 occurrence(s), 5 allowed`, `REFUSED crates/lake/src/footer.rs — 1 occurrence(s), 0 allowed`, then `GATE 11 FAILED.` and rc=1. At 73441e5 it gives rc=0.

**Fix:**
- Set server.rs to 5 and sweeprun.rs to 6, and add `crates/lake/src/footer.rs 1` to `allow_member`.
- Drop the rule-6 entry `crates/api/src/sweeprun.rs 1`.
- Add one reason line for each change under docs/06-limits.md's Gate 11 rule-7 heading (Range probe; fixed `reads` table; Range probe).

### P6-03 medium: CI gate 12 is red at 1f4de71, because two new doc blocks trip its cost-claim trigger and name no proof

**Where:**
- crates/api/src/autopilot.rs:4258-4261 (092a693, D-1767): `/// … and every / later day of the current month was never scanned (CE-23, D-1767).`
- crates/cli/src/boolean_admission_tests.rs:80 (fd45f9d, D-1990/D-1991): `/// Three priced trades on two IST days: +10, -4 and a flat 0.`

**Why it is wrong:** gate 12's trigger is `[Oo]\(1\)|constant[ -]time|never scans?|worst[ -]case|\bflat\b` (ci.yml, gate 12 `trigger=`).
- "never scanned" matches `never scans?` as a prefix.
- "a flat 0" matches `\bflat\b`, and the `claim_scrub` noun list does not remove it.

Neither block names a test path, an invariant id or UNVERIFIED, so the gate refuses both. Neither is really a cost claim, but the gate fails anyway.

**Repro (ran):** the extracted gate-12 step gives `REFUSED crates/api/src/autopilot.rs:4258 (a_caught_up_feed_stays_on_the_month_still_being_written) — a cost claim that names no proof`, the same line for `crates/cli/src/boolean_admission_tests.rs:80 (three_trade_coordinate)`, then `GATE 12 FAILED.` and rc=1. At 73441e5 it gives rc=0.

**Fix:** the smallest change is to reword the two comments, for example "was never visited" and "a zero-PnL trade". The alternative is to add `never scanned` and `flat 0` to `claim_scrub`, recording the measurement as that gate's comment requires.

### P6-04 low: gates 27 and 10b cannot see 49 invariant row ids, so a duplicate among them would pass both

**Where:**
- Gate 27's id pattern is `^\| *`?[A-Z][A-Z0-9-]*-[0-9]{2,3}[a-z]?`? *\|`.
- Gate 10b's is `^\| [A-Z]+-[0-9]+[a-z]? `, and it has no zero-match guard.
- In docs/04-invariants.md, 49 first-column ids match neither pattern, for example `S-30-session`, `S-30-grid`, `RUST-UC7-a/b/c`, `AF-PROBEAPI1-a…e`, `AF-1203-a…f`, `AF-W3S13-a…e`, `CU-SV4-CLOSE-D0961` and `AF-D1204`. The cause is a letter suffix after a hyphen, or a 4-digit or alphanumeric tail.

**Why it is wrong:** gate 27 exists to make every row id name exactly one invariant. D-1608 widened its pattern once already, because 179 rows "were invisible to uniqueness". These 49 rows are the same blind spot, and the gate's "WHAT IT CANNOT SEE" note does not list them. None is duplicated today (checked), so this is a missing guard, not a live collision.

**Repro (ran):** `grep -oE '^\| *`?[A-Z][A-Za-z0-9_.-]*`? *\|' docs/04-invariants.md | grep -vE '<gate-27 pattern>' | grep -vE '\| (ID|Id|Invariant|Point|File) \|' | wc -l` gives 49. Running `uniq -d` over all ids gives no output.

**Fix:**
- Widen gate 27 to `^\| *`?[A-Z][A-Z0-9]*(-[A-Za-z0-9]+)+`? *\|` and exclude the header words.
- Retire gate 10b, which is now a strict subset with no zero guard, or give it the same zero guard.

---

## Earlier findings re-checked at 1f4de71

| ID | Status | Evidence |
|---|---|---|
| P4-01 | NOT FIXED | The gate-10 row loop still lists RS-03, PS-02 and AFD-15 as missing, as in pass 5. |
| P5-01 | NOT FIXED | The gate-10 replica still gives `line 3442: … a_stale_handle_refuses_to_extend_a_ragged_chosen_trade_tail` and `line 6424: brutex_core::knob::tests::…`. |

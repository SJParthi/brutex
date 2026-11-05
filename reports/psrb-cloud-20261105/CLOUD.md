# Cloud sessions for the PSRB second-wave indicator parts (Thu 5 Nov 2026)

The operator's Mac is the bottleneck: 8 build slots, all busy, and part steps wait hours for one. These cloud sessions
build two pure-code parts of the PSRB strategy family (narrow CPR and Pivot R1, Supertrend, Bollinger, RSI, Williams %R,
CCI, EMA 200; tested only on NSE F&O stocks, cash, intraday) that need no data store, so the Mac only checks and lands
them: PSRB-WILLR (Williams %R) and PSRB-CCI (the Commodity Channel Index). On 3 Nov 2026 the operator said yes to cloud
use on this public repository and to full automation. Four fib parts and the three first-wave PSRB parts (PSRB-RSI,
PSRB-BOLL, PSRB-STV) were built this way.

## What you have

- Your kit branch `cloud/psrb-kit-20261105` is `fix/fib-fno-seam` at `1ad6e24ef8e0` (FNO-SEAM, passed) plus the
  PSRB-SEAM commit `35234cead400` (19 comment and blank lines in `crates/indicators/benches/ratio.rs`, PARTS.md 2.3)
  plus ONE PSRB-SEAM-2 commit (18 comment and blank lines: 12 in `crates/indicators/benches/ratio.rs`, 6 in
  `crates/indicators/src/lib.rs`, PARTS-WILLR-CCI.md 3.3) plus this folder.
  **START = the PSRB-SEAM-2 commit = the parent of the commit that added this folder:**
  `START=$(git rev-parse origin/cloud/psrb-kit-20261105~1)`. Check `git diff --name-only $START origin/cloud/psrb-kit-20261105`
  lists only `reports/psrb-cloud-20261105/` files. Measure your D2 set and your coverage against START.
- `PARTS-WILLR-CCI.md`: the design and the bar of this wave. Read sections 0, 2, 3 whole, your part's section whole
  (5 PSRB-WILLR, 6 PSRB-CCI) and 7. Follow it to the letter: the module and file names, the anchors, the API, the
  formulas, the test names, the bench ids, the ledger blocks and the decision ids.
- `PARTS.md`: the first wave's design. Its sections 2.4 and 2.5 bind (PARTS-WILLR-CCI.md 2 says how); read them whole.
- `MANIFEST.md`: each kit file's source and the sha256 of its content. Check it before you use any file.
- `d3-recipe.txt.md`, `d3-route-b.sh.md`, `d3-route-b-cloud.sh.md`, `d3-covset.mjs.md`: the coverage recipe and Route B,
  each as one fenced block. `wr-cci.mjs.md`, `pops.mjs.md`, `cpr-wr6.mjs.md`: the node BigInt models that re-check the
  fixtures of PARTS-WILLR-CCI.md 4.3, 5 and 6 (evidence; your Rust tests prove the Rust by their own brute force).
  `repair1.mjs.md` (the white-box pop counts and the CCI equality clause behind 5.5 and 6.5) and `repair3.mjs.md` (the
  staircase at both generator readings and the CCI ceiling at `u16::MAX`): evidence in the same sense. No model's
  output is here: PARTS-WILLR-CCI.md states every figure a session needs.
- Every non-Markdown file is here as `<name>.<ext>.md` with one fenced block, because the repository tracks `.json`
  only under `.claude/` and only the extensions CLAUDE.md section 2 names.

## Rules

1. **Paths.** The recipe names Mac paths under `/Volumes/WD_BLACK/...`. Read `${W}/wt/DBFIX` and `${W}/wt/P-<NAME>` as
   this checkout. Mac items do not apply: DRAIN, the disk floor, `bin/cargo`, lanes, deep-dir locks, `wt/` paths,
   `preflight.sh`, shadow servers, "never push". Every build rule of the recipe binds.
2. **No store, no app, no shadow, no vendor data.** Nothing in these two parts needs them. A step that seems to is a
   question (rule 11).
3. **Branch.** Your brief names it (`cloud/psrb-willr-20261105` or `cloud/psrb-cci-20261105`, lower-case), cut from
   `origin/cloud/psrb-kit-20261105`. Commit and `git push -u origin <branch>` after every step: the account running you
   may end at any time, and only pushed work survives. Never push any other branch, never merge, never rebase or
   rewrite a pushed commit, never open a pull request, never touch `main`, `feat/db-fix`, any `fix/*` branch or the kit
   branch, never force.
4. **One writer.** Your first commit is `reports/psrb-cloud-20261105/OWNER-<PART>.md` holding your brief's TOKEN. Before
   every push: `git fetch origin <branch>`. If origin's OWNER file holds another token, or your push is refused as
   non-fast-forward, another session owns the branch: commit nothing more and end your turn. A TOKEN is never reused.
5. **Files you may edit:** only your part's Files list in PARTS-WILLR-CCI.md (5.1 or 6.1, and 3.4): your new files,
   your ONE `pub mod` line directly under your slot in `crates/indicators/src/lib.rs`, your lines directly under your
   two slot lines in `crates/indicators/benches/ratio.rs`, and your blocks at your four ledger anchors
   (PARTS-WILLR-CCI.md 3.4). Never `trend.rs`, `vwap.rs`, `evaluator.rs`, `daily.rs`, `rsi.rs`, `bollinger.rs`,
   `dual_band_supertrend.rs`, `crates/indicators/tests/`, `Cargo.toml`, `Cargo.lock`, `CLAUDE.md`, `AGENTS.md`,
   `docs/00-charter.md`, `.github/`, or another part's lines. A need outside your list: `STATUS: BLOCKED OUTSIDE` with
   the exact change.
6. **The bar is unchanged** (PARTS.md 2.4). 100% line and branch coverage of your D2 set by Route B: the pinned nightly
   `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)` (the 3 Nov sessions installed it as `nightly-2026-08-08`; accept it only
   if `rustc -V` prints exactly that), `RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch'`,
   `CARGO_PROFILE_DEV_OPT_LEVEL=0`, `llvm-profdata merge -sparse`, `llvm-cov` with branches, run by
   `d3-route-b-cloud.sh` with `D3_NIGHTLY`, `D3_LLVM_BIN` and `D3_CARGO` pointed at that toolchain (no
   `RUSTC_BOOTSTRAP`). The bench wrapper and `fn main` of `benches/ratio.rs` are measured by running the bench target
   once under the same instrumentation. No such nightly: coverage UNMEASURED, with why. Also, on the repository's pinned
   stable `1.97.1`: `cargo fmt --all --check`, `cargo clippy -p indicators --all-targets --locked -- -D warnings`,
   `cargo test -p indicators --locked`, `cargo test -p core --locked --test cost_invariants --test decision_citations`.
   Each new test shown red under a hand change of the code it guards. Mutants are deferred.
7. **Host reds.** A test that fails because of this host is a host red only with a proof: it fails the same way on
   START with the same command, or it fails only as root and passes as a non-root user. List each with its proof, for
   the Mac. Every other red is yours to fix.
8. **Host.** One CARGO_TARGET_DIR per toolchain; TMPDIR on a tmpfs; `CARGO_BUILD_JOBS=2` on out-of-memory; delete
   nothing you did not create; a full disk is `STATUS: HELD (disk)` with `df -h`.
9. **Benches are advisory here.** Paste every bench run's printed lines whole (the wrapper prints, per size, the best
   round's p50, p99 and max with n, and the median and worst rounds' p99), with the machine and the command. The Mac's
   gate 8 run decides.
10. **Report.** `reports/psrb-cloud-20261105/report-<PART>.md`. Line 1 exactly one of `STATUS: DONE`,
    `STATUS: IN PROGRESS`, `STATUS: HELD (<why>)`, `STATUS: BLOCKED QUESTION`, `STATUS: BLOCKED OUTSIDE` (only items for
    the Mac or the operator are left). Then: the kit check, what you built, files and functions, tests by name, gates
    and coverage with commands and figures (per file: lines a/b, branches c/d), benches, decisions drafted (your block
    only), anything not done, and a NEXT line while in progress. Also `part-<PART>.md` (what the Mac's check reads first:
    files, API, tests, decisions, NEEDS THE MAC) and `result-<PART>.json.md`: a line that is exactly three backticks
    followed by `json`, one json object, a line that is exactly three backticks, and no other fence in the file. Its
    fields: `name`, `branch`, `head`, `committed`, `tests`, `gates`, `coverage`, `benches`, `decisions`, `needs_mac`,
    `held`. **`head` is your code tip:** the sha `git rev-list -1 HEAD -- . ':(exclude)reports'` prints. Write the json in
    a later commit that touches only `reports/`, after the code tip is pushed, and again after any later code commit.
    Before you end your turn, check that the command prints exactly the json's `head` and `jq -e .` accepts the block.
11. **A question** for the main session: `STATUS: BLOCKED QUESTION` with the question; push; end your turn. The answer
    comes in a continuation session's brief.
12. **Never** the Py-language (never use it, never write its name), a credential, a private value, a real vendor row,
    a deletion of anything you did not create in this session, mutants, a merge, a force push, a pull request.

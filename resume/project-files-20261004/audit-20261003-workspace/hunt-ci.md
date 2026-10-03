# hunt-ci: CI workflows, gate tools and engine Apriori at 1087e54

## Verdict

At 1087e54 the `ci-ok` aggregator is sound. It runs under `if: always()`, it `needs` all seven jobs, and it accepts only `success`. The per-step mechanics of the main toolchain gates are honest: fmt, clippy `-D warnings`, `test --locked`, a pinned `cargo deny` and Gate 18's exact mutant reconciliation all fail when they should.

The weak points are around the gates, not inside them.

1. **No reviewer stands between an edit and a merge.** Branch protection on `main` requires only the check name `ci-ok` (from GitHub Actions). That check is produced by the PR's own copy of `ci.yml`. `auto-merge.yml` arms every same-repo PR. No CODEOWNERS file exists, and PRs #17–#20 were merged by `github-actions[bot]` with zero reviews. So a PR that weakens a gate certifies itself. PR #74 (this HEAD) rewrites 3,766 lines of `ci.yml`, has auto-merge armed and has 0 reviews.
2. **Merges done by auto-merge never trigger the `push: main` CI run.** The last push-event run on `main` is ffa41c6d (2026-09-23, failure). The one scheduled run (2026-09-28, 2c209309) also failed.
3. **The only meta-gate that proves a step "runs" checks that a line is present, not that it executes.** `cargo deny check || true` passes it.
4. Smaller gaps: nothing guards `ci-ok`'s own composition; the 3,600 lines of gate-tool Rust under `.github/` sit outside fmt, clippy, coverage and mutation testing (`source_scan.rs` fails default clippy with 10 errors); Gate 27 cannot see 179 invariant rows; W4 has no "silent zero" guard; and the check named "Coverage 100%" enforces 90/89.

§9 mapping: coverage is not 100% and not branch coverage, and mutation testing covers changed lines only. Both are DOCUMENTED.

Engine Apriori (subset prune, determinism, pair budget, resume budget check): re-read at HEAD. **No NEW defect.** The known items are FIXED as v2 reported.

Method: code reading with line numbers at HEAD, GitHub REST reads, and rustc-built runs of `.github/source_scan.rs` against modified copies of `ci.yml` kept in the scratchpad. No cargo was run (BUILD_DONE never appeared), and no tracked file was touched.

## Findings

| id | sev | area | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| hunt-ci-1 | medium | ci / merge policy | `.github/workflows/auto-merge.yml:66-70,277-281`; `.github/workflows/ci.yml:7885-7913`; branch protection | The only merge condition is a job named `ci-ok`, and that job comes from the PR's own copy of `ci.yml`. No review is required, there is no CODEOWNERS, and auto-merge arms every non-fork PR. So any same-repo PR, agent-authored ones included, can weaken or delete a gate in `ci.yml` or `auto-merge.yml` and merge itself with no human look. Every "the gate refuses X" claim in this repo therefore holds only against honest mistakes, not against a change to the gate. | `gh api repos/SJParthi/brutex/branches/main` → `"required_status_checks":{"checks":[{"app_id":15368,"context":"ci-ok"}],...,"enforcement_level":"everyone"}` (no `required_pull_request_reviews` is visible here; the full protection endpoint returned 403, so whether reviews are required is UNVERIFIED from the API, but see next). PRs #17, #18, #19, #20: `merged_by=github-actions[bot]`, `auto_merge.enabled_by=github-actions[bot]`, `/reviews` empty. `ls CODEOWNERS .github/CODEOWNERS docs/CODEOWNERS` → none exist. PR #74 (head 1087e54): `auto=SJParthi`, reviews 0. `git diff --stat origin/main...HEAD -- .github` → `ci.yml | 3766 ++++----`, `source_scan.rs | 2803 +++`. | NEW |
| hunt-ci-2 | medium | ci | `.github/workflows/ci.yml:4-6`; `auto-merge.yml:277` | `on: push: branches: [main]` never fires for merges made by auto-merge. GitHub suppresses workflow runs for events caused by `GITHUB_TOKEN`, and `auto-merge.yml` arms the merge with that token. So the merged result on `main` is never re-verified. The weekly schedule is the only post-merge signal: it ran once and failed, and nothing alerted anyone. | `gh api ".../actions/runs?event=push&branch=main"` → total 14, newest `2026-09-23 CI ffa41c6d failure`. Main commits after it have no push run: 8435d26c (#17), 96194c11 (#18), 2c209309 (#19), bc531316 (#20), 4bbd37df (#22), all `merged_by github-actions[bot]`. Runs for head_sha 4bbd37df are only `workflow_run Auto-merge`. `event=schedule` → total 1: `2026-09-28 2c209309 failure`. | NEW |
| hunt-ci-3 | low | ci | `.github/source_scan.rs:1739-1790` (`step_runs`); used at `ci.yml:2516,2523` | `step-runs` is meant to prove that Gate 3 RUNS `cargo deny check` "so that its failure fails the run". It only checks that some line in the step begins with the needle, plus the step/job `if:` and `continue-on-error`. A line that runs the command but swallows its result passes: `cargo deny check \|\| true`, or the line inside `if false; then … fi`. Gate 0 does not refuse `\|\| true`, which the workflow uses legitimately in about 100 places. | Built with `rustc --edition=2024 .github/source_scan.rs`. On HEAD `ci.yml`: `present 1 step(s) run 'cargo deny check' unconditionally and blocking` rc=0. On a copy with line 7169 changed to `cargo deny check \|\| true`: same output, rc=0; `scan workflow` on that copy: rc=0. On a copy wrapping the line in `if false; then`/`fi`: same output, rc=0. | NEW (the Gate 8 twin of this meta-gate is KNOWN: v2-1 / AC-gates-cx-2, still open: `runs_unconditionally` awk at `ci.yml:6861`) |
| hunt-ci-4 | low | ci | `.github/workflows/ci.yml:7885-7913`; `source_scan.rs:1770-1780` | Nothing guards the aggregator itself. No gate checks that `ci-ok` `needs` every job, keeps `if: always()`, or still requires `success`. The only `needs` check is inside `step_runs`, and only for the job holding `cargo deny`. A future job left out of `needs`, or a job-level `if:` on `ci-ok` that skips it, makes gates advisory with no red. GitHub counts a skipped required job as passing. Today all 7 jobs are listed and the guard is correct. | `ci.yml:7888 needs: [language-purity, build, coverage, complexity, mutant-plan, mutants, web]`, `7889 if: always()`. `grep -n "ci-ok" ci.yml source_scan.rs` shows no other structural check besides `source_scan.rs:1743-1779` (deny job only). | NEW |
| hunt-ci-5 | low | ci / §9 | `.github/source_scan.rs`, `.github/mutation_gate.rs`, `.github/invariant_paths.rs` (3,630 lines) | The Rust that decides a dozen gates is compiled by bare `rustc -D warnings` (`ci.yml:40-42,4985,5003,7448-7450,7585`). It is not a workspace member, so `cargo fmt --all --check`, `cargo clippy --workspace`, `cargo llvm-cov` and `cargo mutants` never see it. Its own unit tests pass, but no coverage or mutant measure applies, and it does not meet the workspace's clippy bar. | `clippy-driver --edition=2024 --test -D warnings .github/source_scan.rs` → 10 errors (e.g. `:136 stripping a prefix manually`, `:360 this if has identical blocks`, `:794 collapsible if`, `:810 very complex type`). mutation_gate 0, invariant_paths 0. `-W clippy::pedantic`: 48/5/2 warnings. `rustfmt --check` on all three: rc=0. Self-tests: source_scan 40 passed, mutation_gate 6 passed. | NEW |
| hunt-ci-6 | low | ci | `.github/workflows/ci.yml:4469-4471` (Gate 27) | Gate 27's row-id pattern `^\| *`?[A-Z][A-Z-]*-[0-9]{2,3}[a-z]?`? *\|` allows no digit inside the prefix. So 179 rows in `docs/04-invariants.md` (`FV4-01`, `PS3-01`, `GR3-01`, `C-O1CLI1-01`, `S-PROBESTORE3-01`, …) are never uniqueness-checked. A second `FV4-01` would pass. There are no duplicates among them today. Gate 10 reads every row, so only uniqueness is blind. | `grep -cE '<gate pattern>' docs/04-invariants.md` → 1309. Rows matching a broader `[A-Za-z][A-Za-z0-9-]*-[0-9]+` but not the gate's pattern → 179. e.g. `docs/04-invariants.md:3836 \| FV4-01 \|`, `:3880 \| PS3-01 \|`. | NEW |
| hunt-ci-7 | low | ci | `.github/workflows/ci.yml:7865-7880` (Gate W4) | W4 runs without `set -o pipefail` (the default shell is `bash -e`), and `npm … build \| tee` hides a build failure. More importantly, a count of zero is accepted with no check that the tool spoke at all. If Svelte's warning wording or stream changes, W4 reads 0 forever. Gates 26, 27 and W3 in the same file refuse exactly this ("A silent zero is not a pass"). A failed build is still caught by W1, so the first half is mitigated. | `CEILING=0; npm --prefix web run build 2>&1 \| tee /tmp/build.log >/dev/null; FOUND=$(grep -F 'Unused CSS selector' … \| wc -l)`. No `set -o pipefail`; no "missing" branch (compare W3 `:7733-7737`). No `defaults: run: shell` in the file (`grep -n "defaults:\|shell:"` → none). | NEW |
| hunt-ci-8 | low | auto-merge | `.github/workflows/auto-merge.yml:221-224, 266-267` | The STOPPED messages say "Auto-merge was not enabled", but an earlier `pull_request` run of this same workflow has usually already armed auto-merge, and a STOP never disarms it. The alarm is accurate about this run and misleading about the PR's state. A human reading "not enabled" may think nothing is armed, and the PR then lands on the next green push without anyone re-reading it. | The `pull_request` trigger reaches `gh pr merge --auto` (`:277`) while gates are pending (section 5 deliberately does not refuse pending). A later `workflow_run` with `failure` reaches `stopped "CI concluded … Auto-merge was not enabled …"` (`:224`) with no `gh pr merge --disable-auto`. | NEW |
| hunt-ci-9 | low | auto-merge | `.github/workflows/auto-merge.yml:148-149` | On `workflow_run` the PR is picked as `[.[] \| select(.state=="open")] \| .[0]` from `commits/{sha}/pulls`. That endpoint also returns open PRs that merely CONTAIN the sha (for example a stacked PR). If one of those sorts first, the head comparison at `:180` takes the quiet `not_yet`, and the PR whose CI just finished is never evaluated by this trigger. | Code as quoted. Not exercised. Low because the `pull_request` trigger usually armed it already. | NEW |
| hunt-ci-10 | info | ci | `.github/workflows/ci.yml:7181` | The check the PR UI shows as green is named `Coverage 100%`, but it enforces `--fail-under-lines 90 --fail-under-regions 89` (`:7258-7259`) and no branch coverage. `docs/06-limits.md:219` also still says region coverage "is the number that is actually 100%". | Quoted lines. | DOCUMENTED: D-0677 (floor), D-0030 / 06-limits §219 (branch). The check name and the 06-limits:219 sentence are stale. |
| hunt-ci-11 | info | ci | `.github/workflows/ci.yml:7154-7156` | A Gate 3 comment says "rust-toolchain.toml pins 1.85.0". It pins 1.97.1. | `rust-toolchain.toml: channel = "1.97.1"` | NEW (doc drift) |
| hunt-ci-12 | info | ci | `.github/workflows/ci.yml` (whole file) | No `permissions:` block, so CI jobs get the repo-default `GITHUB_TOKEN` scope (UNVERIFIED: the Actions-permissions API returned 403 through the proxy). Third-party actions are pinned by mutable ref: `dtolnay/rust-toolchain@stable` ×5 (a branch) and `Swatinem/rust-cache@v2` ×3. | `grep -n permissions ci.yml` → none; `uses:` tally. | NEW (hardening) |
| hunt-ci-13 | info | ci | `.github/workflows/ci.yml:7118-7138` (Gate 5) | Gate 5 counts only the literal `allow(unsafe_code)`. `#[allow(unsafe_code, reason = "…")]` (the style this workspace uses for every other allow) and `#[expect(unsafe_code)]` are not counted. This is redundant in practice: every `src/lib.rs`/`main.rs` carries `#![forbid(unsafe_code)]` (checked: none missing), and Gate 16 layer 1b covers test roots. | `grep -oh 'allow(unsafe_code)'`. | NEW (info) |

## The §9 definition of done, gate by gate

| §9 item | gate | file:line | really enforces? |
|---|---|---|---|
| `cargo fmt --check` | Gate 6a `cargo fmt --all --check` | ci.yml:6959-6961 | Yes for workspace members. `.github/*.rs` is not covered, though it is fmt-clean today (hunt-ci-5). |
| clippy `-D warnings` | Gate 6b `cargo clippy --workspace --all-targets --locked -- -D warnings` | ci.yml:7114-7116 | Yes for members. `.github/source_scan.rs` fails it (hunt-ci-5). |
| `cargo test --workspace --locked` | `Tests` | ci.yml:7171-7173 (also Gate 1e :338) | Yes. |
| `cargo deny check` | Gate 3, pinned 0.19.0 | ci.yml:7140-7169 | Yes. Its meta-check can be fooled (hunt-ci-3). |
| 100% line + branch coverage | `Coverage 100%` | ci.yml:7258-7260 | **No**: 90% lines / 89% regions, branches unmeasured. DOCUMENTED (D-0677, D-0030). |
| no surviving mutant on touched modules | Gate 18 | ci.yml:7416-7638, mutation_gate.rs:105-133 | Changed lines (`--in-diff`), not whole modules. DOCUMENTED (06-limits ~:3179). Survivors and timeouts fail; `unviable` is accepted; it fails loudly above 256×62 = 15,872 cases (mutation_gate.rs:19-21,53-61). On push, schedule or dispatch it tests only `HEAD~1` (:7484-7486). |
| invariants beside tests | Gates 10, 12, 27 | ci.yml:4847, 5842, 4449 | Gate 10 reads every row. Gate 27 skips 179 (hunt-ci-6). |
| decision entry per locked choice | none (judgement) | — | Not mechanisable. 27b checks only that decision numbers are unique. |

Every gate in this table can be removed or weakened by the PR it is judging (hunt-ci-1).

## Security checklist

| question | answer | evidence |
|---|---|---|
| `pull_request_target` with PR-head checkout | Not used anywhere | `grep pull_request_target .github` → none |
| Script injection via `${{ github.event.* }}` in `run:` | None. Every event value crosses into the shell through `env:` | ci.yml `${{` sites: :13, :7422, :7432, :7454 (env), :7540 (name), :7550, :7567, :7588-7589 (env), :7629, :7902 (env); auto-merge.yml :80-97 all `env:` |
| `workflow_run` with write token running untrusted code | No checkout, no PR code executed. A fork PR is STOPPED (`auto-merge.yml:191-192`) | — |
| Secrets | None referenced | `grep secrets\. .github` → none |
| Permissions | auto-merge: `contents: write, pull-requests: write, checks: read` (needed for `gh pr merge`). CI: unset (hunt-ci-12) | auto-merge.yml:66-69 |
| Can auto-merge merge with skipped or neutral checks? | Auto-merge itself ignores skipped, neutral and cancelled checks by design (`:244-253`) and defers to protection. Protection requires `ci-ok`; because `ci-ok` is `always()` with a success-only loop, a skipped or neutral gate job makes it fail. The residual risk is that `ci-ok` itself is skipped or redefined in the PR (hunt-ci-1, hunt-ci-4). Whether protection is "strict" (up to date with main) is UNVERIFIED: the endpoint omits `strict` and the full protection read is 403. The behind-main check runs only when auto-merge is armed (`:228-231`). | — |

## Prior CI items re-checked at HEAD

- **rustonly-6 (ring built under Gate 13): FIXED-since-prior, by code reading.**
  - Gate 13b (`ci.yml:6986-7047`) runs `cargo tree --workspace -e normal,build,dev` against a banned list that includes `ring` and `cc`, with a self-test fixture.
  - Gate 13c (`:7053-7112`) reads build-script `linked_libs`.
  - Not executed here.
- **v2-1 / AC-gates-cx-2 / AC-gates-o1-3: still open (KNOWN).** `runs_unconditionally` awk is still at `ci.yml:6861-6896`. Gate 13 moved to `step-runs`, but that check has its own hole (hunt-ci-3).
- **GAP14-56: FIXED as v2 said.** The self-test that a change to `build_provenance.rs` plans a mutant is at `ci.yml:7516-7533`.

## Engine Apriori at HEAD: no NEW finding

| check | file:line | result |
|---|---|---|
| Prefix join completeness and injectivity | lib.rs:2074-2087 (`keyed` sorted by `(without_highest, mask)`, deduped), :1953-1955 | Correct. S∖{p_k} and S∖{p_{k-1}} share a block, and the pair is recoverable from the candidate. The block order within words can put the higher-highest-bit parent first when the two highest bits are in different words. Harmless: `every_non_parent_subset_is_frequent` uses `popcount` and ascending `set_positions`, and `pair_is_informative` (vocab implication.rs:196) is symmetric. |
| Subset prune skipping the parents | lib.rs:2425-2430 | Correct for prefix-join candidates only, which is what its doc states. `join_screen` is its only caller (:2137). |
| Meaning prune vs anti-monotone prune | lib.rs:2137-2152 | Consistent. A k-set with no dead pair has no (k-1)-subset with a dead pair, so it is never pruned through a missing subset. |
| Pair budget edge | lib.rs:1971-1979 (`>=` before each pair) | Exactly `pair_budget` pairs are walked. A level needing exactly the budget completes, and the next pair halts. `resume.rs:254` now accepts `pairs == budget` when not halted, which matches. KNOWN items FIXED. |
| Ceiling | lib.rs:1719-1733, :2003-2014 | Candidates admitted ≤ ceiling. Checked before any counter moves. |
| Determinism | lib.rs:2358-2394 (disjoint `counts`, serial append in candidate order), :2437-2439 (`sort_canonically`), :1997 | Budget and ceiling checks are sequential and independent of `batch_cap`, so lane count changes only scheduling. Machine-dependent output appears only on a `Memory`/`Workers` breach, which is inherent. |
| Drain-failure rollback | lib.rs:2033-2047 | `generated`/`emitted` are reduced by the uncommitted batch, so `reconciles()` holds. |

## Files

- Report: `/tmp/claude-0/-home-claude-brutex/1e694b3f-0b9b-5fae-8d41-2cb758b4eff1/scratchpad/out/hunt-ci.md`
- Probe copies (scratchpad only, untracked): `scratchpad/hunt-ci/ci_ortrue.yml`, `ci_deadbranch.yml`, `scan`.
- `git status --porcelain` shows no file of mine. The `zz_audit_*` files listed belong to other workers.

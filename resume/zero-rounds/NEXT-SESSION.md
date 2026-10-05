# Zero-findings: resume in a new session (saved 2026-10-05 ~02:00 UTC)

## State when paused
- All zero-findings work so far is ON final/all-fixes (PR #74): PR 74 CI fast-forwarded it to 6db4dfb8, then added 969493e (D-2001, elite TOP check).
- Nothing is unpushed. Branches zero-work and zero/pr74-merge are pushed and fully contained in final/all-fixes.
- Decision numbers used by this workstream: D-1930..D-1939, D-1955..D-1969, D-2660..D-2690. Next free in this block: D-2691 (re-check with `grep -c "^### D-2691" docs/05-decisions.md`).
- Tracker snapshot: resume/zero-rounds/zero-findings.tsv (copy of /mnt/project-files/fix-board/status/zero-findings.tsv). States: found / fixing / branch / pushed / operator.

## What is left
1. Watch PR 74 CI on 6db4dfb8+; fix any failure in zero-work code (merge commit, never force-push).
2. Remaining logged rows in zero-findings.tsv with state `found` or `fixing` (about 300). Many may already be fixed by other lanes: check docs/05-decisions.md and the code for each before fixing. Other owners: Fix Board owns CE-84..94, CE-98..101, conc server/runs/apicache, conc18-*, P17-01, p14num-1/2; O(1) sweep owns P13-04/05, P15-07/08/09/17, P1-19-03, log-3, P1-04-02.
3. User-only items, name them and do not decide: p13num-4 (Finance Act 2023 STT), p14num-3 and run1-1/2 (losing-trade definition), P11-04 (history rewrite), pst-3, clib-1/2, gaps-7.
4. At the end: a comparison-table summary (found vs fixed vs owner-held) for the user.

## How to work (rules)
- Read CLAUDE.md fully first. Rust only (web/ excepted). O(1) or name the bound with numbers. Evidence, never guess.
- One PR only: #74 (final/all-fixes). Open no other PR. Work on a branch (e.g. zero/next), merge-commit only, never force-push, never rewrite history.
- Run tests as non-root: `setpriv --reuid=65534 --regid=65534 --clear-groups <test-binary>` (the store lock tests fail as root).
- Static gates locally: build `.github/source_scan.rs` with `rustc --edition=2024 -D warnings`, run the language-purity steps from ci.yml with SOURCE_SCAN set; run them only after `git add` (unmerged index stages triple-count files).
- Before pushing: cargo fmt --check, cargo clippy --workspace --all-targets -- -D warnings, affected suites green, a docs/05 entry per decision, docs/04 row per invariant.
- Never commit a credential path, GDFL data or web/node_modules. ci.yml must stay under ~520 KB.
- Save state here (resume/zero-rounds/) at 93% usage; stop at 98%.

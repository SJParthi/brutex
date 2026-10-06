# PAUSE — lens L4 one-authority (2026-10-06)

Branch `attack/one-authority`, head `d02e10a38ce045c4f6c39639e6bf721a062401a4` (pushed).
Base: origin/final/all-fixes `969493e` (merged at 0edc1a0; re-merge before next push).

## Done and validated (commit 0edc1a0, pushed)
- D-3500 gate 0 spawn scan: type alias / <Command>::new / Command::new as value / impl..for Command refused (ONEAUTH-01).
- D-3501 stale `ring` manifest comments + graph.rs banned-crate comment check (ONEAUTH-02).
- D-3502 CLAUDE.md §5 / AGENTS.md §5 pictures checked against manifests (ONEAUTH-03).
  fmt, clippy -D warnings, all language-purity static gates (except 1e) green; core/pull/lake tests green as non-root
  (3 core findings history tests fail ONLY as nobody: git dubious ownership; green as root).

## In progress (commit d02e10a, WIP, NOT validated)
- D-3503 gate 10b/27 read `| ID — claim |` (ONEAUTH-04). Tool tests green.
- D-3504 crates/core/tests/citations.rs + D-2710→D-2712, D-1619→D-1614, I-41 row (ONEAUTH-05). green.
- D-3505 pull::rolling::shift_six negative tie = core half-up (ONEAUTH-06). green.
- D-3506 cli population_write_lock + api path_in→Results::path, cli/tests/one_path_authority.rs (ONEAUTH-07). green.
- D-3507 InstrumentKey::swept_surface; api census axis 210; 9 api tests re-derived; api comments fixed (ONEAUTH-08). api lib green.
- Failing red outputs recorded in /tmp (lost on container reclaim): re-run tests against 969493e if needed.

## Exact next steps
1. Gate 1d still refuses literal "-" in crates/pull/src/rolling.rs test `a_negative_tie_rounds_by_the_one_rule_core_gives_a_price`
   (`("-", rest)` in the `authority` closure): rewrite without a bare "-" string literal (e.g. a bool sign + push('-')).
2. Re-run: cargo fmt --check; cargo clippy --workspace --all-targets -- -D warnings; static gates
   (extract jobs.language-purity.steps, skip 1e, RUNNER_TEMP=/tmp/claude-0 SOURCE_SCAN=/tmp/claude-0/source-scan GITHUB_ENV set).
3. Full non-root tests of core, pull, cli, api (cli is heavy; one crate at a time).
4. Gate 18 pre-run: git diff origin/final/all-fixes...HEAD -- crates > /tmp/claude-0/mine.diff;
   cargo mutants --in-diff ... -p core / pull / cli / api one at a time (cargo-mutants 26.2.0 must be reinstalled in a fresh container).
5. Squash nothing; new commit "one-authority L4 round 1 batch 2 validated"; merge origin/final/all-fixes; push.
6. Round 2 (fresh eyes) — open candidates not yet worked: C2 (pool / sweep-all / range-all redo everything after interruption:
   no per-instrument completion journal), B7 (web copies of 15:10 forced exit), lower-impact 09:15/IST helper copies
   (cli stored.rs NSE_OPEN_MINUTE_V2, indicators orb private IST offset, pull gaps ist(), api calendar_of ist()),
   71 inline awk programs in ci.yml (known pinned ratchet), web comments 213→208 (deferred: web/build churn).
7. Already tracked (not mine): P13-03 `.claude/launch.json` runs `sh -c` (audit-helpers → zero-findings).
8. Update resume/attack-lenses-20261006/one-authority.tsv and write RESULT-one-authority.md.

## Restart
git fetch origin attack/one-authority final/all-fixes fix-queue && git checkout attack/one-authority

## Queued during the pause (coordinator, 04:08 UTC) — do after RESUME
9. New gate-hole finding: a live cargo-mutants marker (`/* ~ changed by cargo-mutants ~ */`, e.g. telemetry tail.rs:503 in
   pr74/g18-rest 92bcd1a/99d8217, repaired 0d8c386) passes every static gate. Add a language-purity check in an existing
   .github tool (source_scan `content` is the natural home) refusing that marker in any tracked file, needle built from pieces
   so the tool does not refuse itself; test with a planted marker that fails first; record decision (D-35xx), ONEAUTH row,
   docs/11 bullet. Run `git grep -n "changed by cargo-mutants"` before every commit.

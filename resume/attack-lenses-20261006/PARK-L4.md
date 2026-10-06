# PARK — lens L4 one-authority (12:30 UTC order)

- Branch: `attack/one-authority`
- Head: `ffff5e5` (pushed). Working tree clean; `git grep -n "changed by cargo-mutants"` empty.
- Base: origin/final/all-fixes `969493e` — NOT yet re-merged with the newest final/all-fixes.

## Pushed and validated
- D-3500..D-3515, ONEAUTH-01..15, I-41 row (see RESULT-one-authority.md).
- On 2f4be13: cargo fmt --check, cargo clippy --workspace --all-targets -D warnings, every language-purity gate
  except 1e (run locally), and 77 non-root test binaries of core/pull/indicators/runner/cli/api green
  (core findings history tests fail only as nobody: git dubious ownership; 14/14 as root).
- Web (D-3514) at 7cf031b: W1 build matches, W2 870 pass, W3 svelte-check 0, W4 0, W5 ok.
- ffff5e5 adds one pull test (`the_clock_stamp_is_a_sigv4_date_of_now`) that kills the two Gate 18 survivors in
  `pull::ssm::now_stamp`; pull lib green, clippy clean.

## Gate 18 pre-run (cargo mutants --in-diff, --baseline skip --in-place --timeout 900 --cap-lints true)
- core: 2 tested — 1 caught, 1 unviable.  indicators: 6 — 6 caught.  runner: 15 — 14 caught, 1 unviable.
- pull: 30 — 28 caught, 2 missed → both killed by ffff5e5 (re-run: 2 caught).
- api: 14 of 22 tested before the 2 h background cap stopped it — 13 caught, 1 unviable, 0 missed; 8 untested.
- cli: 27 mutants NOT run.

## Open
- Gate 18 pre-run for api (8 left) and cli (27). Run per crate, sharded so each stays under 2 h:
  `cargo mutants --in-diff mine.diff --baseline skip --in-place --timeout 900 --cap-lints true -p cli --shard k/3`
  (never commit while it runs; restore with `git checkout -- crates` if a run is killed).
- Merge newest origin/final/all-fixes (merge commit), re-run fmt/clippy/gates, push.
- Round 4 (exit round) not run; lens has not yet had a zero round.
- Needs the owner: whether rust-toolchain.toml / Cargo.toml join auto-merge's sensitive paths (D-3510).
- D-0370 / D-0372 duplicate headings: DONE (answered, no change needed) — D-0684 tables both copies and gate 27b already
  refuses every duplicate decision id while pinning these five to exactly two; recorded in D-3515 (pushed).

## Resume
git fetch origin attack/one-authority final/all-fixes fix-queue && git checkout attack/one-authority
then the two Gate 18 runs above, the merge, and round 4.

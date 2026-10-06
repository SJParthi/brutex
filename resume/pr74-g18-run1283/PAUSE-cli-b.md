# PAUSE — cli-b (branch pr74/g18-cli-b)

Head: 9f4914d (pushed). Paused 2026-10-06 ~04:50 UTC on the coordinator's usage guard.

## Done (committed and pushed)
- Kills for all 80 listed survivors: new tests, plus behaviour-preserving restructures where a mutant was equivalent or unreachable:
  - ordered::Turns::ready: the own-slot clause is gone;
  - pool::price_all: `forced: Some(..)`;
  - population discard guard and keep-on-bytes;
  - observations append_synced refresh shape;
  - selection_v6 tail_bytes computed once;
  - readonly_file::regular is one fn with two cfg'd tails;
  - result_set::hash_range is generic.
- Decisions D-2020..D-2031; invariant rows G18-cli-b-01..24.
- Full cli lib suite as uid 65534: 1970 passed, 0 failed, 1 ignored. Command: the `cli` lib test binary run with `--test-threads=3` under setpriv.
- cargo fmt --check: clean.
- Static gates (language-purity steps, gate 1e skipped): all exit 0. Gate 11 first refused a slice `.contains` in population.rs; fixed in 9f4914d, then re-run and passed.
- Targeted cargo-mutants, 14 of the 86 run so far: 14 CAUGHT (live 229 true/false, 231 `&&`; admission_v4 2730, 2739 x2, 2740, 2744; finalization_v4 2454, 2463 x2, 2464, 2468; finalization_v3 2455). 1 UNVIABLE (readonly_file::regular -> Ok(Default::default()); File has no Default). 0 missed, 0 timeout.

## Not yet done
- Targeted mutation of the remaining mutants (~71 + 2 population ones). Saved lists: /tmp/g18bin/{re3,ex3,args3,filter2}.txt; these are lost if the container is reclaimed.
- cargo clippy --workspace --all-targets -- -D warnings: NOT RUN yet (UNVERIFIED).
- Merging newest origin/final/all-fixes, the final re-validation, and RESULT-cli-b.md.
- p50/p99/max for the restructures: UNVERIFIED. No complexity changed; none was measured.

## Restart
```
git fetch origin pr74/g18-cli-b final/all-fixes && git checkout pr74/g18-cli-b
CARGO_BUILD_JOBS=2 cargo mutants --baseline skip --jobs 2 --timeout 300 --cap-lints true -p cli \
  --file crates/cli/src/<each touched file>.rs --re '<survivor regex, see commit messages / survivors-cli-b.md, line numbers shifted>' \
  --exclude-re '<the 15 already done above>' --test-tool nextest --cargo-test-arg=--lib -- -E 'test(/<new test names>/)'
```
Then: clippy -D warnings; merge origin/final/all-fixes (merge commit); fmt, clippy, cli tests as non-root, static gates; push; write RESULT-cli-b.md.

## Added during pause (coordinator, 04:39 UTC): 10 TIMEOUT cases, see timeouts-cli-b.md
These are not started yet. Plan on RESUME:
- **ordered.rs, `Turns::ready`.** Four of these mutants are already covered by the pure-rule test `a_lane_never_waits_on_its_own_slot`, which fails fast. These are false (89), the `||` (95/96), and `-` → `+` or `/` (96:75). Note the own-slot `||` was removed in abda922, so line numbers shift.
- **ordered.rs, `update` → (), `Finished::drop` → (), delete `!` in `turn` (212).** Each needs a fan-out test with a BOUNDED wait: run it on a thread, take its result with `recv_timeout`, assert the order. A deadline miss is a failure, not a hang.
- **admission_v4 2745 and finalization_v4 2469, `+=` → `*=`.** Restructure the exact-prefix loop to a bounded `for index in 0..record_count` (or a `zip`), so the progress no single compound-assignment mutation can stall.
- Prove each with CI's flags plus `--build-timeout 300`: caught or unviable, zero timeouts.

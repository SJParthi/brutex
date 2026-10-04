# Shared brief for every attacker (round R)

Repo: /home/claude/brutex (branch claude/attack-data-pipeline-hgxmw9, = origin/final/all-fixes). Read CLAUDE.md first; it is law.
Several attackers share THIS working tree at once. Rules that keep that safe:
- Edit ONLY the files your section says you own, plus NEW test files you create (name them `crates/<crate>/tests/attack_<area>.rs`, or a `#[cfg(test)] mod attack_<area>` at the end of an owned file). Never touch docs/, Cargo.toml, Cargo.lock, ci.yml, or anything another attacker owns.
- Do NOT git add / commit / stash / checkout / reset. The thread lead commits.
- Do NOT call any mcp__hearthbot__ tool.
- Build only your crate: `CARGO_BUILD_JOBS=2 cargo test -p <crate> --locked <filter>`. The target dir is shared; cargo's lock serialises builds, so waiting on "Blocking waiting for file lock" is normal.
- The box runs as root: store lock/permission tests fail as root. Run store tests as `setpriv --reuid=65534 --regid=65534 --clear-groups cargo ...` if a permission test misbehaves (HOME/CARGO_HOME may need to be readable; if setpriv is awkward just report which tests are root-only failures).
- No new dependencies. No proptest/quickcheck. Property/fuzz tests use a hand-written deterministic PRNG (splitmix64 or xorshift64*) with a fixed seed and a fixed, large case count (e.g. 100_000-1_000_000 where fast), so reruns are byte-identical (CLAUDE.md §3 rule 5). Also enumerate exhaustive small domains where possible, and hand-pick extreme values: 0, 1, -1, i64::MIN/MAX, u64::MAX, f64 NaN, ±inf, subnormals, -0.0, f64::MIN_POSITIVE, f64::MAX, f64::EPSILON, huge/tiny strikes, 0/1-day tenors, expiry at the very stamp, empty slices, one element, duplicate stamps, unsorted input, out-of-session stamps, DST-free IST boundaries (09:15, 15:29, 15:30), month/year boundaries, leap day 2020-02-29/2024-02-29.
- Every test must ASSERT something meaningful (CLAUDE.md §4 bans a test that asserts nothing). A property test asserts an invariant (e.g. put-call parity within a stated tolerance, monotonicity, round-trip, no panic AND a named refusal).
- Repo gates you must not trip: no `.iter().find(` / `.iter().position(` (use loops or `.into_iter()...` alternatives? -- simplest: plain for-loops); no local `#[allow(...)]`; a `#[path]`-included test file must start with `#![cfg(test)]` on line 1; in crates/pull, no new lowercase string literals that look like credential path segments, and no float literals like `1e999999999` in source (build them with format!/repeat); never write the name of another programming language anywhere (Gate 15) — say "interpreted runtime" if you must; Rust only; prices are paisa i64, never floats; no fallback that hides a failure (degrade loudly with a named reason, or refuse).
- `cargo fmt` your files; `cargo clippy -p <crate> --all-targets --locked -- -D warnings` must be clean for your crate.

## What a "finding" is
A real defect you DEMONSTRATED with a failing test on the unmodified code: panic, overflow, wrong number, silent acceptance of bad input, a lookup that answers a neighbour instead of refusing, non-determinism, an O(n) path claimed O(1), an unbounded allocation, etc. Not a style opinion. If you only suspect something, write the test; if it passes, it is a "case tried, no failure".

## Fix
Fix every real finding minimally in your owned files, keep the failing test (now passing) as the regression proof. Keep behaviour of existing tests unless they encoded the defect (say so). Refuse loudly rather than clamp silently.

## O(1)
For the hot per-row/per-lookup operation in your area, add a measurement test or report: time N=10^3, 10^4, 10^5, 10^6 sized inputs, per-op p50/p99/max (std::time::Instant, sorted samples), and state whether per-op cost is flat. Report the numbers you measured; never invent one. If something is O(log n) or O(n), say so plainly with the evidence (file:line) — do not rewrite architecture-wide; propose instead.

## Report (your final message, and also write it to SCRATCH/r<R>/<area>.md)
1. Table: attack | cases tried (count) | failures | fixed? | evidence (test name + file:line of the fix)
2. For each fix: a draft decision entry (title + 3-6 sentence body) using your D-number block, and a draft invariant row `| <ID> | statement | test_name |` using your invariant prefix.
3. The exact cargo commands you ran and their pass/fail counts.
4. The p50/p99/max numbers.
5. Anything that needs real vendor data (GDFL/Zerodha on the user's Mac) to settle — name it, don't guess.
6. Files you modified (full list).

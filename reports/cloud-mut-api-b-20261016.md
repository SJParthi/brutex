# Cloud mutation and coverage pass B — `crates/api`, 2026-10-16

Branch `cloud/mut-api-b-20261016`, base `cloud/base-20261016` (c9e3472, the
snapshot of `feat/db-fix` at 4756cce0). PR SJParthi/brutex#81. The operator
re-applies the branch onto `feat/db-fix`.

**This report holds no authority** (`CLAUDE.md` §10): it is evidence of what
was measured on this date, on this machine.

> **DRAFT — the pass is still running.** Sections marked `PENDING-FINAL` are
> filled in when phase 2 finishes. Nothing in them is a result yet.

## Scope

Seven files in `crates/api/src`, the ones `feat/db-fix` changed:
`audit.rs`, `autopilot.rs`, `emitted.rs`, `import_journal.rs`, `pullrun.rs`,
`scratch.rs`, `server.rs`.

The goal is `CLAUDE.md` §9 for those files: no surviving mutant, and 100%
line coverage. Branch coverage is **not measured**: this toolchain's
`cargo llvm-cov` reports `0/0` branches for every file, so no claim is made
about it.

## What changed

PENDING-FINAL: test counts per module.

- Tests go in a `#[cfg(test)] mod cloud_mutant_tests` at the end of each scoped
  file. For `server.rs` (35,184 lines at base) they go in three sibling files,
  `server_cloud_mutant_tests_{a,b,c}.rs`, declared at its end with `#[path]`,
  one per quarter of the file a sub-agent covered. Two more tests (and a
  helper) went into the existing `census_request_tests.rs`.
- **No `crates/api/tests/cloud_mutants_server.rs`.** The plan put public-item
  tests there. The mutation run uses `cargo mutants … -C --lib`, which does not
  run integration tests, so a killing test in `tests/` would not have counted.
  Every test is in the crate.
- No existing test was weakened, skipped or deleted.
- `emitted.rs`: three assertions in existing tests now compute their message
  values before the `assert!`. Those values were the only lines a passing run
  never executed. Same assertions, same messages.

## Production changes — two real bugs, each with its test

### 1. The spot run's vendor-down breaker could never trip (`server.rs`, 37f1bee)

**Input.** A spot broker run (`POST /pull/spot`, the autopilot's tick, or a
recovery) against a vendor whose own side is down, for example every request
answered with a 5xx.

**Observed before the fix.** One instrument took the whole ~30 s 5xx ladder
and came back with `reached_wire = true` and `vendor_down = false`. Over four
instruments, every request was sent and the run ended with `stopped = None`.
The breaker never tripped.

**Expected.** Invariant A-50 (`docs/04-invariants.md`) says three
*consecutive* instruments lost to the vendor's own side stop the run, so an
outage costs three ladders, not one per instrument. At ~785 instruments and
~30 s each, the defect made an outage cost about 6.5 hours.

**Cause.** The breaker counts the `VENDOR_DOWN` marker (`\u{2}`). Only
`laddered` (F&O discovery, the rolling POST) wrote it. The spot path is
`broker_window → fetch_chunks → with_retry`:

- `with_retry`'s `Step::ServerDown` arm returned its sentence unmarked.
- `fetch_chunks` embedded the reason mid-sentence, where `read_markers`
  never looks.

The A-50/A-51 proof test, `three_consecutive_vendor_failures_stop_the_run_and_a_success_clears_it`,
feeds the breaker synthetic booleans and a hand-built string, so it could not
see this.

**Fix.**

- `with_retry` marks its `ServerDown` refusal.
- `fetch_chunks` lifts the marker exactly as it lifts `CREDENTIAL_DEAD`, and
  puts it back in front only when no prefix of the instrument was written. A
  written prefix makes the instrument a success, which clears the streak.
  Nothing strips the marker from `Chunks::unfetched`, so a re-added one would
  leak there.
- `fetch_chain_chunks` (F&O contracts, where no breaker reads it) strips the
  marker before its `CREDENTIAL_DEAD` check, so it cannot reach a page as an
  invisible control character.

**Test.** `server::cloud_mutant_tests_c::three_instruments_lost_to_the_vendors_own_side_stop_a_spot_run`
drives a real spot run against a loopback vendor that answers 503: three
instruments' ladders, then a stop naming the breaker. Its twin
`the_fno_contract_refusal_carries_no_vendor_down_marker` pins the F&O strip.

### 2. A lost line continuation in a refusal (`server.rs` `price_group`, 8e545dc)

The sentence *"the vendor sent no underlying level beside this bar, so there
is nothing to price it against"* carried 18 spaces mid-sentence: a `\`
continuation had been joined into the literal. Fixed to one space. The test
compares the reason word by word, so it passed before and after; the fix is
visible in the diff.

## A defect found and NOT fixed — it needs the operator's call

**A backend failure the vendor names under a status outside 500–599 gets a
zero-second retry ladder, and its refusal says the vendor failed "0 time(s)".**

**Mechanism.** `step` sends a vendor-named `Disposition::RetryBounded` (Dhan
`DH-908`/`DH-909`/`800`, Groww `GA000`/`GA003`) to `server_ladder`, *"the SAME
two ladders the status path calls"*, whatever the HTTP status. But both
callers count `server_errors` only for statuses `500..=599`:

- `with_retry`, the spot bars path, at server.rs ~8336;
- `laddered`, F&O discovery and the rolling POST, at ~6997.

So under any other status the count stays 0, and the ladder becomes:

- `Again { wait_ms: 1_000 * 0 * 0 }` for attempts 1–5, i.e. no wait at all.
  The AIMD governor's permit is the only pacing, so the six requests come
  back to back. The documented sizing is ~30 s, chosen to outlast a blip.
- Then `ServerDown { answered: 0 }`, whose refusal reads *"its own side has
  now failed 0 time(s) on this chunk, out of 5 allowed"* after six failures.
  That is a count the code never took, stated as fact — the §3 rule 6 shape
  `step`'s own doc names as the defect the two-counter split was built to
  remove.

**Reachable** whenever a vendor sends one of those names under a non-5xx
status. Whether Dhan or Groww actually do is **UNVERIFIED** here:
`step`'s doc notes that Dhan's annexure publishes no HTTP status for its
codes, which is why the named path exists.

**Pinned by two existing cloud-pass tests**, both written against the
behaviour as found:

- `server::cloud_mutant_tests_c::laddered_marks_a_spent_backend_ladder_vendor_down`
  asserts the "failed 0 time(s)" text.
- `three_instruments_lost_to_the_vendors_own_side_stop_a_spot_run` is fast
  only because the ladder waits nothing.

**Proposed patch, not applied.** In both callers, count a vendor-named backend
failure with the 5xx:

```rust
if why.status.is_some_and(|code| matches!(code, 500..=599))
    || matches!(why.named, Some(pull::refusal::Disposition::RetryBounded))
{
    server_errors = server_errors.saturating_add(1);
}
```

with the same change in `with_retry` on its `(status, named)` pair. Then:

- `step`'s doc line "`server_errors` counts only the 5xx answers" becomes
  "the answers the vendor's own side failed".
- The two tests above expect "failed 5 time(s)".
- The breaker test takes ~60 s: the first instrument's ladder alone, then the
  rest three at a time.

**Why it is not applied.** It changes how fast this repository re-asks a live
vendor, and it puts a minute-long test into a suite that every phase-2 mutant
runs whole. Both are the operator's decisions, not a test pass's.

## Method

PENDING-FINAL: per-file mutant table.

**Tools.** cargo-mutants 27.1.0 and cargo-llvm-cov, with toolchain 1.97.1,
offline. Every test runs as the non-root user `mut`, because root ignores
file modes and several refusals are mode-driven.

**Phase 1.** Each file's mutants are split into shards; three lanes each take
one shard at a time from a shared queue. A job runs:

```
cargo mutants --in-place --no-shuffle --baseline=skip --timeout 180 \
  --build-timeout 900 --cap-lints=true -C --lib -f crates/api/src/<file> \
  --shard k/n -- -- <filters>
```

The filters are the file's own test modules plus every module that names the
file's items. For `server.rs` that is `server:: emitted`. Each job also has a
45-minute lane limit. A shard that hit it (rc=124) left its untested mutants
as "unrun", and those go to phase 2.

**Survivors.** Every phase-1 survivor (missed or timeout) was read by hand and
handled one of two ways:

- a killing test, checked against the hand-applied mutant where the kill was
  not obvious; or
- a written reason why no test can kill it (below).

**Phase 2.** Every mutant phase 1 did not settle (missed, timed out, or unrun)
is run again, on the newest committed source, against the **whole** `api` lib
suite, with `--timeout 400`.

**Skipped tests.** Both phases skip three tests:

- `server::tests::the_two_sites_past_the_socket_are_driven_over_a_real_one` (30 s)
- `server::tests::a_refused_chunk_keeps_the_contiguous_prefix_and_names_what_is_missing` (30 s)
- `server::head_deadline_tests::the_served_default_cuts_a_partial_head_that_goes_quiet` (10 s, timing-bound)

Under three concurrent lanes on four cores, a load-induced failure in one of
them would count a mutant as caught when it was not. The skip errs the other
way: a mutant that only one of these three could kill is reported as missed.

## Network safety

No test may send a request anywhere but `127.0.0.1`. Every broker path is
driven one of three ways:

- `Site::load`: `Broker::Refused`, which stops before any socket.
- `Credentials::Scripted`: the `cfg(test)` source whose base URL replaces each
  feed's with a loopback fake vendor.
- A closure, for paths that take one.

`/home/mut/netcheck.sh` runs each module's tests under
`strace -f -e connect,sendto,sendmsg` with every proxy variable unset. It lists
any non-loopback `AF_INET`/`AF_INET6` address. The list is empty for every
module.

**Incident, reported when it happened.** While probing the Groww name walk,
sub-agent B wrote two draft tests that tried
`https://api.groww.in/v1/historical/expiries?…`. `pull::chain::month` builds
discovery URLs from the shipped descriptor, not from the wire's base URL, so a
scripted credential does not redirect it. The requests failed at the
transport. The only credential was the fake token `cloudb`. Both drafts were
deleted, and nothing in the branch builds a non-loopback request. Covering
that walk needs a production seam (below).

## Results

PENDING-FINAL

## Mutants no test can kill

PENDING-FINAL (from /home/mut/p2/unkillable.md and phase 2)

## Lines not covered, and why

PENDING-FINAL

## Gates

PENDING-FINAL

## Not done

PENDING-FINAL

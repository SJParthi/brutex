# tests-docs-security pass 16 (tag tds16): test-suite determinism and isolation

Checkout: /home/claude/wt/zero3 at 1f4de71 (read-only). Proof run in a throwaway worktree at 1f4de71, which has since been removed.

Theme: tests that depend on the wall clock, fixed temp paths or ports, `set_var`, test order or global statics, the host (root, filesystem, `/dev`, path lengths, CPU count, locale), the network, unseeded randomness, or sleeps. Special attention to macOS, because the operator runs on a Mac.

Counts: 2 medium, 2 low. No verification rows were assigned to this pass.

Ranked by how likely each is to fail on the operator's Mac or in CI.

---

### P16-01 medium: `pull` ingest's socket-lock test binds a Unix socket at a path too long for macOS, so it fails on every Mac run

**Where:**
- crates/pull/src/ingest.rs:3708 `a_socket_at_the_lock_path_is_refused_rather_than_run_unlocked`
- its scratch helper at ingest.rs:3422-3431

```rust
let root = std::env::temp_dir().join(format!(
    "brutex-ingest-{tag}-{}-{}",
    std::process::id(),
    std::time::SystemTime::now()...as_nanos()
));
...
let root = scratch("LOCK-SOCKET");
let census = root.join("manifest").join("dhan.man");
let lock_path = census.with_extension("man.lock");
let listener = std::os::unix::net::UnixListener::bind(&lock_path)
    .expect("the socket this test is about can be bound");
```

**Why it is wrong:**
- The bound path is `TMPDIR` + 74 bytes: `brutex-ingest-LOCK-SOCKET-` (26) + pid (5) + `-` + 19-digit nanoseconds + `/manifest/dhan.man.lock` (23).
- macOS `sun_path` is 104 bytes, and the macOS `TMPDIR` (`/var/folders/xx/<30>/T/`) is about 49. That gives about 123 bytes, so `std` refuses the bind with `InvalidInput` "path must be shorter than SUN_LEN", and the `.expect` panics. On Linux CI (`/tmp/`, 79 bytes) it passes.
- The repository already decided this class once. D-0695 (docs/05-decisions.md:43186-43203) moved the three api census tests to `SocketAt`, which binds through a short `/tmp` link, after the same panic made cargo-mutants refuse an unmutated tree. This later test (D-1480) binds directly again.
- It fails whenever any TMPDIR longer than 29 bytes is in effect, including a mutation run's.

**Repro (ran):** at 1f4de71, `cargo test -p pull --lib -- --exact ingest::tests::a_socket_at_the_lock_path_is_refused_rather_than_run_unlocked`:
- With `TMPDIR` set to an 87-byte directory, it FAILED: `panicked at crates/pull/src/ingest.rs:3713:14: the socket this test is about can be bound: Error { kind: InvalidInput, message: "path must be shorter than SUN_LEN" }`.
- With the default `/tmp`, it passed.

**Minimal fix:** bind at a short path and move it into place, the way store/tests/catalog.rs:749-753 does: `UnixListener::bind(temp_dir().join(format!("bis-{pid}")))`, then `fs::rename` to `lock_path`. Or reuse a `SocketAt`-style short link. Add a premise assertion that fails if the bound path is ever ≥ 104 bytes.

### P16-02 medium: the serve-lock stamp test resolves its lock to `/dev/full`, which macOS does not have, so it fails on every Mac run

**Where:** crates/api/src/server.rs:21877 `a_serve_lock_stamp_that_fails_is_cleared_or_refused_never_left_stale`, lines 21934-21941.

```rust
std::os::unix::fs::symlink("/dev/full", &lock_path).expect("the /dev/full name");
let addr: std::net::SocketAddr = "127.0.0.1:1".parse().expect("an address");
let refused = take_serve_lock(&root, addr).expect_err("an unstampable lock refuses");
assert!(refused.contains("could not be written"), "{refused}");
```

**Why it is wrong:**
- `/dev/full` is a Linux device; macOS devfs has none, so on a Mac the symlink dangles.
- `take_serve_lock` (server.rs:18248-18262) opens it with `.create(true)`, which follows the dangling link and tries to create `/dev/full`. An ordinary user gets `EACCES`, and the function returns `"REFUSED: the one-server lock … could not be opened — Permission denied"`.
- That still matches the `expect_err`, but not `contains("could not be written")`, so the test fails on the operator's machine. Linux CI cannot see this.
- The test is not `cfg`-gated. It also leaves its scratch root behind on failure.

**Repro:** not run; the difference is the host itself. On macOS `ls /dev/full` reports no such file. Chain: server.rs:21938, then 18248-18262 (open with create), then the refusal at 18257-18260, then the assertion at 21941.

**Minimal fix:** gate the `/dev/full` leg with `#[cfg(target_os = "linux")]`, or skip it with a printed reason when `Path::new("/dev/full")` does not exist. The pure `stamp_outcome` legs above it already cover ENOSPC on every host. Or reach the same arm portably by stamping through a handle whose `write` fails, if `stamp_serve_lock` can be split to take the file.

### P16-03 low: the 16-collision scratch test predicts the next values of a process-global counter that other tests in the same binary also advance

**Where:** crates/cli/src/operator_boundary_tests.rs:283-308 (in `ledger_self_checks_do_not_delete_an_existing_directory_and_can_run_concurrently`, :248), against `verification_scratch` at crates/cli/src/lib.rs:8012-8030 (`static NEXT: AtomicU64`).

```rust
let first = crate::verification_scratch()?;
let (prefix, serial) = name.rsplit_once('-')...;
for offset in 1..=16 {
    let path = first.with_file_name(format!("{prefix}-{next}"));
    fs::create_dir(&path)?;           // pre-claims serial+1..=serial+16
    ...
}
let refused = crate::ledger_round_trip();
assert!(!refused.held);
assert!(refused.evidence.contains("16 collisions"));
```

**Why it is wrong:**
- `NEXT` is shared with every other `verification_scratch()` caller in the cli lib test binary: the 9 tests built on `results_report_tests::Fixture::new` (results_report_tests.rs:14) and `columns_tests.rs:123`.
- Failing interleaving 1: thread B (a results_report test) calls `verification_scratch` after A has read serial s. B gets s+1 and creates it, and A's `fs::create_dir(s+1)?` returns `AlreadyExists`, so the test errors.
- Failing interleaving 2: B takes s+1 after A has made it, gets `AlreadyExists`, retries upward and eventually claims s+17. A's `ledger_round_trip` then starts at s+18 or later, finds no collision, and `assert!(!refused.held)` fails.
- In the same window, B itself can be the one to exhaust its 16 retries and fail.
- libtest's alphabetical dispatch usually keeps the `operator_boundary_*` and `results_report_*` modules apart in time, so the window is narrow. That is why this is low. Under nextest (one process per test) it cannot happen.

**Repro:** not run (interleaving reasoned from source).

**Minimal fix:** give `verification_scratch` an injectable start (`fn verification_scratch_from(next: &AtomicU64)`), and have the test drive a local counter. Or have the test pre-claim the paths the function will try by holding a test-only lock that every `verification_scratch` caller takes.

### P16-04 low: the process-global failed-request ration is shared by every routed api test, and one test asserts that a 404 line is written

**Where:**
- crates/api/src/logs.rs:1219 `static FAILED_LINES: Mutex<Rations>`, with `LOCAL_FAILED_LINES_PER_WINDOW = 200` per 60 s (:1192, :1216). It is consulted by `note_request` (:1118-1145), which returns without writing once the ration is spent.
- The assertion: crates/api/src/server.rs:31866 `the_run_events_and_the_request_event_reach_the_installed_sink`, :31995-32021.

```rust
let missing = get(addr, "/emit-request-probe.js").await;   // a 404 -> Warn -> rationed
...
.find(|record| crate::emitted::says(record, "path", "/emit-request-probe.js"))
.expect("and so does the one that answered 404");
```

**Why it is wrong:**
- The ration is per process, not per router. Every api lib test that sends a same-origin request through `router_serving` or `audited_router_serving` and gets a status of 400 or more spends from the same 200-line window.
- One test alone, `http_admission_tests::every_audited_route_refuses_another_sites_request_before_the_journal` (http_admission_tests.rs:589-601), spends 63 of the 200: 21 routes × 3 methods with a leading `Sec-Fetch-Site: same-origin`.
- If the binary has spent 200 same-origin failures in the 60 s before the probe, the 404 line is suppressed, and the test fails with the production code correct.
- I did not measure the binary's actual per-minute total, so this stays low rather than medium.

**Repro:** not run.

**Minimal fix:** make the ration a field of the router's state (`Site`/`Loaded`) rather than a process static, so each test server has its own window. Or have the emit test take a test-only reset of `FAILED_LINES` under a lock that every routed test holds.

---

## Checked and found holding (not reported)

- **`set_var`/`remove_var`:** no call anywhere. Every crate forbids `unsafe`, and env-dependent rules go through pure `*_from(..)` resolvers.
- **Fixed ports:** every listener binds `127.0.0.1:0`. The refused-bind test (server.rs:21470) squats a kernel-chosen port.
- **Fixed temp paths:** nearly all scratch names carry the pid plus a serial or nanos.
  - pull/tests/unit.rs:108/775 share the directory `brutex-pull`, but the file names are pid+serial unique, which is pinned by `temporary_configuration_paths_are_process_local_and_unique`.
  - Literal `/tmp/x`-style paths are display-only.
- **Randomness:** no `thread_rng`, `OsRng` or `RandomState::new` in any crate.
- **Wall clock:**
  - Fixed `today()` stand-ins are used in ingest/recovery parsing tests.
  - Real-clock tests compare against the same clock (`autopilot.rs:4383`, a µs-wide IST-midnight race only).
  - `pull::calendar::LAST_DAY` (2026-09-04) is already past, and no test asserts `stale:false` on the live route.
- **Sleeps:** they are either bounded polls with multi-second ceilings (readonly_file.rs:137, checksum_audit_tests.rs:153, census_request_tests.rs:1176 ctime tick, telemetry sink.rs:1902) or deliberate perturbation whose assertions hold under any schedule (ordered_tests.rs).
  - The head-deadline timing tests state a slack (T=400 ms, T_SLACK=350 ms).
  - P5-07 already covers the two slack-less ones.
- **Thread-local fault injectors:** store `SYNC_FAULT`, cli `BarrierWatch` and `COLD_ADMISSIONS` are per thread.
  - The one global they share, `sweep_evidence::FORGOTTEN`, is handled by the memo test's 16-try epoch check (sweep_evidence_tests.rs:551-580).
- **The knob store (`cli::knobs`):** setters hold `knobs::serially()`.
  - The api census row (emitted.rs:783) sets `BRUTEX_TOP` briefly. No in-process api test was found that reads `BRUTEX_TOP` concurrently.
- **macOS symlinked temp dir (`/var` → `/private/var`):** the all-rung V5 canonical-spelling refusal and the results listing are handled by canonical fixtures (step3_orchestrator.rs:5025, results_report_tests.rs:9, all_rung_population_v5.rs:1639, columns_tests.rs:123).
- **macOS case-insensitive APFS:** handled explicitly (pool.rs:1646-1667, batch.rs:925/1262).
- **errno numbers in tests:** 2, 5, 9, 13, 24 and 28 are the same on Linux and macOS. The `ELOOP` difference is per-OS (store tests/open_flags.rs:64-66).
- **FIFO and nonblocking opens:** per-OS flag constants exist for macOS.
- **`/proc` reaping check (server.rs:24830):** vacuous on macOS but cannot fail there.
- **Root:** permission tests re-run as uid 65534 via `where_permission_binds`. CI and the Mac run as non-root.
- **CPU count:** used only as an upper bound (engine sweep_readiness_oracle.rs:234) or with a fixed 4-thread pool (index_stop_search_progress_tests.rs:255).
- **Network:** the masters refresh and SSM paths are not driven by any test. Credential reads are injected (`Credentials::Scripted`, `discover_from`).
- **`git`-dependent tests** (cited_commits, findings) refuse a shallow clone loudly. CI uses `fetch-depth: 0`. Already covered in earlier passes.

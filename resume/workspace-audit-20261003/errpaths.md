# errpaths — error-path audit of crates/** production code at 1087e54

## Verdict

The production tree (282,277 non-test lines after stripping `#[cfg(test)]` items, `*_test(s).rs` files and the six test-only modules) is hard to panic from input. Workspace clippy denies `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `cast_possible_truncation` and `cast_sign_loss`. Only 3 `.expect(` calls and 1 `.unwrap()` remain in shipping code, and all four are on compile-time-true values. Release builds set `overflow-checks = true` (Cargo.toml:181), so an unchecked `+ - *` that overflows panics instead of wrapping silently. A pattern scan of store, lake, telemetry, core, pull csv/archive/fold, api bars/calendar_of and runner resample found no unchecked arithmetic on input-derived values that can overflow. Every hit was bounded by a guard on an earlier line (examples below). The real error-path weaknesses are **silent fallbacks**, CLAUDE.md §4's banned shape. Four are NEW and proven by probes:
(1) `AwsIdentity::discover` ignores `AWS_PROFILE` and quietly falls back to `[default]` when the environment identity is partial;
(2) `result_set::committed_receipt` uses `Path::exists()`, so an I/O fault on the parent ledger is reported as "no receipt";
(3) `runner::grid` turns an invalid caller stop ladder into an empty one with no refusal;
(4) `indicators::Evaluator::new` accepts swapped tolerance widths and silently drops every `near_*` bit.
Further NEW items, read from code but not probed: one mislabelled drop reason in pull ingest, a non-exhaustive match that turns an unknown drop reason into zero, and a family of `.exists()` absence tests in step3_comparison. No lock is held across `.await`. The only tokio mutex is the `REFRESH` serializer, and std guards cannot cross an await in Send futures. One prior finding is re-verified FIXED: fills_at PrintedExtreme `i64::MIN` low, now refused at costs/src/fill.rs:434. A second is re-verified FIXED: CSV signed time, `digits()` at pull/src/csv.rs:400.

## Method

- `errpaths/strip2.py` (scratchpad) lexes each `crates/*/src/**.rs` (strings, raw strings, chars, comments). It drops every item under `#[cfg(test)]`, `#[cfg(all(test…))]` or `#[cfg(any(test…))]` by brace depth. It also excludes files named `*test*.rs` and the test-only modules `api/src/{scratch,isolated,emitted}.rs`, `engine/src/manifest.rs`, `pull/src/emit_sites.rs` and `store/src/emits.rs`. Result: `errpaths/p3.txt`, 282,277 production lines. The first naive pass counted 460 api `unwrap()`s, which were all test files. That pass was discarded.
- Each risky family was read at its call site.
- Probes were placed at `crates/{runner,pull,cli}/tests/zz_audit_errpaths_{1,2,3}.rs`, run, then deleted.

### Counts per crate (production only)

| crate | lines | `.expect(` | `unwrap_or*` | `.ok()` | `let _ =` | `if let Ok/Some` | `as` int cast | `_ =>` | `map_err(\|_\|` | `Err(_) =>` | `PoisonError::into_inner` | `.exists()` |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| api | 36,908 | 1 | 225 | 84 | 247 | 153 | 118 | 92 | 39 | 9 | 39 | 0 |
| cli | 170,971 | 0 | 342 | 138 | 261 | 321 | 607 | 179 | 619 | 10 | 3 | 13 |
| core | 5,574 | 0 | 12 | 5 | 3 | 1 | 7 | 16 | 1 | 1 | 0 | 0 |
| costs | 2,741 | 0 | 0 | 0 | 0 | 4 | 6 | 1 | 3 | 1 | 0 | 0 |
| engine | 1,961 | 0 | 10 | 0 | 1 | 4 | 3 | 4 | 5 | 4 | 0 | 0 |
| greeks | 654 | 0 | 0 | 0 | 0 | 0 | 1 | 1 | 0 | 0 | 0 | 0 |
| indicators | 5,166 | 1 | 36 | 12 | 0 | 20 | 3 | 6 | 1 | 1 | 0 | 0 |
| lake | 1,686 | 0 | 6 | 5 | 0 | 0 | 3 | 7 | 14 | 1 | 0 | 0 |
| pull | 19,016 | 1 | 82 | 50 | 18 | 53 | 117 | 52 | 23 | 8 | 7 | 0 |
| runner | 28,305 | 0 | 146 | 39 | 104 | 70 | 75 | 18 | 75 | 0 | 0 | 0 |
| store | 4,781 | 0 | 18 | 6 | 4 | 8 | 21 | 3 | 5 | 0 | 3 | 0 |
| telemetry | 2,118 | 9* | 20 | 5 | 0 | 14 | 2 | 15 | 0 | 0 | 5 | 0 |
| vocab | 2,396 | 0 | 5 | 6 | 0 | 0 | 6 | 13 | 5 | 0 | 0 | 0 |

\* The telemetry `expect(` hits are the JSON scanner's own `Scan::expect(byte)?` method, not `Option::expect`.

**Every `unwrap`/`expect`/`panic!` left in shipping code**:
- `runner/src/rank.rs:723` `marked.ranked.unwrap()` is the `Ranked1` trait's own method, not `Option::unwrap`.
- `pull/src/ssm.rs:450` `Hmac::new_from_slice(key).expect(..)`: HMAC accepts any key length.
- `indicators/src/trend.rs:1868` `pinned_fib().expect("pinned")`: a const-asserted width.
- `api/src/server.rs:4338` `HeaderValue::from_str(&note_alphabet(..)).expect(..)`: the alphabet is limited to 0x20..=0x7E.

All four are justified. `panic!`/`unreachable!` in production: 0. The two matches are text inside `reason = "…"` strings.

### How the `let _ =` / `.ok()` / `unwrap_or*` families classify (pattern level, every site read in the risky subsets)

| family | sites | verdict |
|---|---|---|
| `let _ = write!/writeln!/fmt::Write::write_fmt` into a `String` | ~560 | legitimate: infallible on `String` |
| `let _ = telemetry::emit(..)` / `emit_if!` (cli/lib.rs:3117, api/mastersrun.rs:137..563, sweeprun.rs:1818..3250, recovery.rs:839/991) | 13 | legitimate: a failed write is `Emitted::Dropped` and recorded in `Sink::health` (telemetry/src/sink.rs:174-185) |
| `let _ = attempt.level(..)` (cli/batch.rs:675, cli/lib.rs:17703) | 2 | legitimate in practice. I/O failures are `remember`ed (sweep_evidence.rs:584-593) and refuse `finish`. Only const-stride encode failures and lock poisoning bypass `remember`. Both are unreachable. Info only. |
| `let _ = fs::create_dir_all(dir)` (pull/ingest.rs:3014), `remove_dir_all`/`remove_file` cleanup (cli/lib.rs:7727, live.rs:381/385) | 4 | legitimate: documented best-effort, and the next step reports the failure |
| `.ok()` on `str::parse` / `Day::new` / `YearMonth::new` feeding `?` or `ok_or_else(refusal)` | ~150 | legitimate: the Option is turned into a named refusal at the boundary, e.g. api/sweeprun.rs:922, 1110, 1127, 2683 |
| `metadata().modified().ok()` / `created().ok()` in generation stamps | ~20 | legitimate: `None` compares unequal and refuses |
| `unwrap_or(u64::MAX)` on `u64::try_from(len)` | many | legitimate saturation of a count (usize ≤ u64 on every target) |
| `lock().unwrap_or_else(PoisonError::into_inner)` | 57 | mostly legitimate. store/file.rs:1833/1920/2364 reset the cache before use ("forget first"). |
| `if let Ok(mut held) = X.lock()/write()` that drops a write on poison (cli/knobs.rs:125/320/360/363, api/autopilot.rs:2140, server.rs:17533/17691, cli/candidate_trades.rs:389) | 8 | latent only: nothing panics while those guards are held. knobs.rs:360 `clear_all` on a poisoned lock would carry one request's knobs into the next run. Info. |
| `vocab::table::set_exact/set_near(..).unwrap_or(mask)` and `if let Ok(..)` in every indicator module (22 sites) | 22 | documented at indicators/src/daily.rs:604-611. It holds for `set_exact`, which is pinned by `the_plan_agrees_with_the_vocabulary`. **It does NOT hold for `set_near`**, whose `WrongBand` outcome depends on the runtime `Tolerance`. See errpaths-4. |
| `Ladder::*(..).unwrap_or_default()` in runner/grid.rs:1655, 2118, 2120, 2131 | 4 | **hidden failure.** See errpaths-3. |
| `Path::exists()` as an absence test (cli/result_set.rs:857, step3_comparison.rs:510/584/671/765/863/947) | 7 | **hidden failure.** See errpaths-2 and errpaths-6. population_admission_v4.rs:3117, population_finalization_v4.rs:2763, population_v6.rs:2501 and selection_v5.rs:2741/2937 use `exists()` only for a "created" flag. That flag is taken under an exclusive flock and rewrites a constant header, so it is legitimate. |

### Panic and wrap on adversarial input, as checked

- **Store geometry** (store/src/layout.rs:338, 361, 457, 521, 561, 586) is plain arithmetic, but `Layout` comes only from a const table selected by version (`find(|l| l.version == version)`, layout.rs:300). No header field reaches it. Safe.
- **`Admission::admit`** at store/src/file.rs:2799 computes `(ts_micros + self.anchor)` only after the `from..until` month check on the line above. Safe.
- **pull csv** `paisa()` (csv.rs:372-390) uses `checked_mul`/`checked_add`, and `ist_seconds()` digits are pre-checked (csv.rs:400-418). Safe.
- **api** `months_of` (bars.rs:761) uses i32 from u16/u8. `seek_page` (bars.rs:852/877) has `filled < limit` and `start <= end` by construction. `page_of` (bars.rs:1112) guards `limit == 0`. Every `asked.offset + n` is iterated only over rows inside the slice. Safe.
- **lake** `with_capacity(rows)` (reader.rs:557/627) is bounded by `rows > self.bytes.len()` (reader.rs:284). page.rs:274 caps the zstd output at `expect + 1`. Safe.
- **Allowed-truncation `as` casts** (46 allow sites) were all checked. Each is either const-bounded (fnv1a `as u32`, which deliberately truncates a hash) or float-to-int after an explicit clamp and NaN check (runner/outcome.rs:1276, 1416, 1496, 1570; cli/population_statistics_v2/v3). Safe.

### Files, TOCTOU and locks

- Atomic publishes do write, `sync_all`, `rename`, then directory `sync_all`: pull/ingest.rs:3297-3306 and pull/masters.rs:877-930. No partial-write defects were found beyond the KNOWN `complete` marker (GAP11-0, search_checkpoint.rs).
- api/autopilot.rs:623 `File::create(probe)` is a write probe that syncs and removes its file. Legitimate.
- `exists()`-then-`open` in the *_v4/v6 ledgers runs under an exclusive flock (population_admission_v4.rs:2559-2583). Not exploitable.
- No `std::sync` guard crosses `.await`; it would not compile in axum's Send futures. The single `tokio::sync::Mutex` (api/mastersrun.rs:40 `REFRESH`) is a deliberate serializer.

## Findings

| id | severity | crate | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| errpaths-1 | medium | pull (reached from api) | pull/src/ssm.rs:307-312 (`discover`), 243 (`from_env`), 321 (`from_shared_file("default")`); caller api/src/server.rs:10115 | **A silently different AWS identity.** `discover()` tries env, then `[default]`. (a) `AWS_PROFILE` is never read anywhere in the repo, so an operator with `AWS_PROFILE=prod` is signed as `[default]`. (b) If `AWS_ACCESS_KEY_ID` is set but the secret is unset, the env error is discarded (`if let Ok(from_env)`), and the file's `[default]` identity is used with no mention of the env attempt. The rustdoc claims the order "follows AWS's own". AWS tooling honours `AWS_PROFILE`, and botocore raises PartialCredentialsError for a half-set env pair (the botocore behaviour is UNVERIFIED here: no source in docs/00-charter.md). | Code: `if let Ok(from_env) = Self::from_env() { return Ok(from_env); } Self::from_shared_file("default")`. `rg AWS_PROFILE crates docs` gives 0 hits. PROBE2: `AWS_PROFILE=prod` gives `Ok(key_id=FILE_DEFAULT_KEY)`, not `[prod]`. Env key set with the secret unset gives `Ok(key_id=FILE_DEFAULT_KEY)`, silently. | NEW |
| errpaths-2 | low-medium | cli (reached from api frontierjson / trades routes) | cli/src/result_set.rs:857 (`committed_receipt_with_limit`) | **An I/O fault on the parent ledger is reported as "no committed run".** `if !Results::path(root).exists() { return Ok(None) }`. `Path::exists` maps every stat error (ELOOP, EACCES, EIO, ENOTDIR) to `false`. Callers `trades.rs:958` and `frontier.rs:1423` then read with a `None` proof, which by design answers "absent/uncommitted" (trades.rs:972-974). The same ledger opened directly refuses. `try_exists()` is already used two functions above (result_set.rs:773/784). | PROBE3: with `results/runs.bin` as a symlink loop, `exists()=false` and `try_exists()=Err(FilesystemLoop)`. `committed_receipt` returns `Ok(false)` (= `Ok(None)`), and so does `committed_receipt_bounded`, while `Results::open_read` returns `Err("… Too many levels of symbolic links")`. | NEW |
| errpaths-3 | low | runner | runner/src/grid.rs:2131 (also 1655, 2118, 2120) | **An invalid caller stop ladder becomes an empty ladder, with no refusal.** `merged(&Ladder::new(stops_ppm.to_vec()).unwrap_or_default(), forced)`. `Ladder::new`'s own rustdoc (excursion.rs:115-121) says an unsorted list is *"Refused rather than sorted for the caller … quietly fixing it would hide that"*, but the grid replaces the refusal with `Ladder::default()` (zero rungs), and the grid prices with no stop axis. Likewise `Ladder::stepped(..)` returning `None` silently falls back to the quantile ladder (2117-2119), so a requested points step can be ignored. Current cli callers build `stops_ppm` from `stop_ladder_ppm`, which emits ascending, positive values for realistic prices, so this is latent at the binaries and live at the `pub` API (`runner::grid::Levels::stops_ppm` is `pub`). | PROBE1: `[80, 40, 20]`, `[40, 40]` and `[-5, 10]` each give `grid.stops=[]` and `cells_with_a_stop=0`, against `[20,40,80]` which gives 42 stop cells. The grid has no refusal field set. | NEW |
| errpaths-4 | low (latent) | indicators | indicators/src/evaluator.rs:594 (`Evaluator::new`), 72-79 (`Widths` with `pub` fields); swallow sites: lib.rs:588, daily.rs:560/635, fib.rs:231/253, gap.rs:480/503, orb.rs:364, session.rs:247/494, trend.rs:874/964, vwap.rs:655 | **Swapped or wrong-base tolerances silently erase every `near_*` condition.** `set_near` returns `VocabError::WrongBand` precisely so a pivot width on a Fibonacci rung is loud (D-entry quoted at docs/05-decisions.md:15496). Every caller then discards that error with `.unwrap_or(mask)` / `if let Ok`. `Evaluator::new` accepts any `Widths` without checking `fib.base()==SessionRange && pivot.base()==CprWidth`. The daily.rs:604-611 justification ("`the_plan_agrees_with_the_vocabulary` proves none of the positions … is any of those") covers the position checks, not the runtime band check. All production callers today use `Widths::pinned()`, so this is latent. | PROBE1: with `Widths{fib: pivot, pivot: fib}` over 30 synthetic sessions, the distinct bits ever known fall from 308 to 234 and the bits ever set from 121 to 93. `census().refused()` is 0 in both runs, so nothing is refused or named. | NEW |
| errpaths-5 | low | pull | pull/src/ingest.rs:1494 (`keep_in_session`) | **An unreadable timestamp is counted under the wrong drop reason.** `Err(_) => census.count(DropReason::BeforeWindow)`. `Window::verdict` returns Err from `IstMoment::from_epoch_secs` both for pre-epoch stamps and for stamps past 9999-12-31 or past `i64` (session.rs:707-735). A far-future corrupt stamp is therefore reported on the page as "before the window", which is opposite to the truth. The comment above it says "counted, never stored, and never silently kept", but the label is the defect: there is no `Unreadable` reason. | Code read (session.rs:1093 `let at = IstMoment::from_epoch_secs(epoch_secs)?;`). Not probed: the function is private. | NEW |
| errpaths-6 | low | cli | cli/src/step3_comparison.rs:510, 584, 671, 765, 863, 947 | Same shape as errpaths-2. `if !primary.exists() { return StageProbe::unmeasured(.., "{} is absent") }`, so a stat failure is reported as "absent / unmeasured" rather than refused. | Quoted code at :510-512, :671-679. Not separately probed: same `Path::exists` semantics as PROBE3. | NEW |
| errpaths-7 | info | api | api/src/audit.rs:478-490 | `DropCensus::of` ends `_ => 0` over `pull::session::DropReason`, which is `#[non_exhaustive]` (session.rs:145). A fifth reason added in `pull` compiles and reports zero on the page with no warning. The source comment states this, but it is not in docs/06-limits.md or docs/11-findings.md. | Quoted comment: "a fifth reason added in `pull` compiles here and reports zero rather than failing to build". | NEW (self-documented in code) |
| errpaths-8 | info | telemetry | telemetry/src/sink.rs:1381-1389 (`resume_point`), 746, 788 | An unreadable newest log file, or one whose last 64 KiB holds no decodable line, restarts `seq` at 0. `reserved_run` (run ids, used by cli/lib.rs:2136 when no operation-audit id exists) is seeded from that `seq`, so run ids already present in older rotated files can be handed out again. The reset is stated in rustdoc (sink.rs:1369-1377) but not reported through `sink.report` at runtime, unlike the torn-tail case (sink.rs:751-753). | Code read. | DOCUMENTED (rustdoc sink.rs:1369) / runtime-silent |
| errpaths-9 | info | runner | runner/src/validate.rs:1671-1676 (`fold_rungs`) | The legacy `walk_forward_shaped` door reads `BRUTEX_GRID_RUNGS`. A garbage, zero or huge value silently becomes `DEFAULT_RUNGS`, with no refusal recorded and no ceiling. The doc above says "A zero is refused rather than obeyed", but the code falls back instead. No binary calls this door (`rg walk_forward_shaped crates/{cli,api}/src` returns 0 non-test hits). | Code: `.and_then(|raw| raw.to_string_lossy().trim().parse::<usize>().ok()).filter(|&n| n > 0).unwrap_or(DEFAULT_RUNGS)` | NEW (latent; docs/11-findings.md:353 mentions `fold_rungs` only as a call site) |

### Prior findings re-verified at HEAD (error-path ones only)

| prior id | claim | HEAD status | evidence |
|---|---|---|---|
| audit-20261002 "fills_at PrintedExtreme low i64::MIN panics at trip.rs:938" | subtract overflow | FIXED-since-prior | costs/src/fill.rs:434-439 `if sell.raw() < TICK.raw() { return Err(CostError::BelowTick {..}) }` before any arithmetic |
| audit-20261002 "CSV time reader accepts +/- signs" | `-9:15:00` stored | FIXED-since-prior | pull/src/csv.rs:400-405 `digits()` requires every byte `is_ascii_digit` (probestore-1, D-1201) |
| fix-queue lane3-b "fold.rs:333 volume saturating_add" | silent cap | KNOWN (not re-checked beyond grep; out of scope) | — |
| fix-queue lane2-b "grid.rs:4080 saturating totals" | silent clamp | KNOWN | — |

## Probe outputs (verbatim)

PROBE1 (runner, `zz_audit_errpaths_1.rs`):
```
running 2 tests
stops_ppm=[20, 40, 80] -> grid.stops=[20, 40, 80] cells=56 cells_with_a_stop=42
stops_ppm=[80, 40, 20] -> grid.stops=[] cells=14 cells_with_a_stop=0
stops_ppm=[40, 40] -> grid.stops=[] cells=14 cells_with_a_stop=0
stops_ppm=[-5, 10] -> grid.stops=[] cells=14 cells_with_a_stop=0
test probe_unsorted_caller_stops_silently_empty ... ok
pinned: distinct bits ever set=121 ever known=308 | swapped: ever set=93 ever known=234 | refused rows pinned=0 swapped=0
test probe_swapped_widths_silently_drop_near_bits ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

EXIT 0
```

PROBE2 (pull, `zz_audit_errpaths_2.rs`):
```
running 2 tests
test probe_discover_fallbacks ... [A: AWS_PROFILE=prod] test zz_child ... CHILD discover -> Ok(key_id=FILE_DEFAULT_KEY)
[B: env key set, secret unset] test zz_child ... CHILD discover -> Ok(key_id=FILE_DEFAULT_KEY)
[C: env complete] test zz_child ... CHILD discover -> Ok(key_id=ENV_KEY)
ok
test zz_child ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

EXIT 0
```

PROBE3 (cli, `zz_audit_errpaths_3.rs`):
```
running 1 test
exists()=false try_exists()=Err(Os { code: 40, kind: FilesystemLoop, message: "Too many levels of symbolic links" })
committed_receipt -> Ok(false)
committed_receipt_bounded -> Ok(false)
Results::open_read -> Err("/tmp/zz_errpaths_root_10799/results/runs.bin could not be opened: Too many levels of symbolic links (os error 40)")
test probe_committed_receipt_masks_io_error_as_absent ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

EXIT 0
```

All three probe files were deleted after running. Final `git status --porcelain` output: no zz_audit_errpaths_* path remains (`git status --porcelain | grep -ci errpaths` → 0); the other untracked zz_audit_* files belong to other workers.

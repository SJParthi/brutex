# AG-06b: Pull runs never start unjournalled, and no tokio worker waits on the journal lock

Cloud specification of ingest + /db part **AG-06b**, written 24 Oct 2026 from the Mac build
launcher and its tables. A cloud session that cannot read any file on the Mac builds from this
file alone. A cloud build that follows it should equal a Mac build. Every block marked
"verbatim" is copied byte for byte from the source and line it names.

Repository: brutex only. "TickVault" is just the name of the cloud environment. Every path in
this file is in the brutex repository (`crates/...`, `docs/...`), or is a Mac path under
`/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925` (shortened to `W` below), which the cloud
README explains.

---

## 1. Header

| Field | Value |
|---|---|
| Name | `AG-06b` |
| Title | Pull runs never start unjournalled, and no tokio worker waits on the journal lock |
| Kind | `nw` (not an `ia` package. Its specification is the package text in section 2, not the ingest-automation design) |
| Source | `state/followups-20261020` (fixb-followups.json [1] AG-06 followups[6]: AF-04's half of server.rs, NOT DONE by design in L-AG06-03 and L-AG06-07. An opening refused after the bound still runs unjournalled, and restart_scan cannot say not_known. A contended append blocks a tokio worker for up to 1 s, or 2 s with the notes lock) |
| Priority rank | **65 of 137** in `__ARGS.priority` (0-based index 64). It sits between `R3-19` (rank 64) and `CC-20` (rank 66). It is not in `EMBED.priority`, because AG-06b is an `__ARGS.extraPackages` part |
| Critical path | **No**: it is not in `__ARGS.criticalPath`. Nothing on the path waits for AG-06b |
| In `.only` | yes (index 100) |
| Decision block | `D-7100..D-7109` (`decisionBlock` field of the package, and `state/decision-blocks.json` `.blocks[51]`) |
| Launcher flags | `noCritic: true` and `noGate: true` hold for the whole run. `maxLive` is 3. No `skipFix`, `verified`, `pendingRepair` or `roundBase` entry names AG-06b |
| Mac shadow port | **8515** (`portBase` is unset, so 8400, plus the part's index 115 in the `.only`-filtered package list). The second agent uses 8615 |
| Mac build pool | `W/tgt/pool-5` (POOL hash of "AG-06b" mod 8 = 5) |
| Tables checked | `state/cloud-20261024/args.json` and `embed.json` are byte-equal (after `jq -S`) to line 6 (`__ARGS`) and line 10 (`EMBED`) of `W/state/fix/db-fix-round-2-run0738.js.fat` |

### Prerequisites

Formula: (package deps + `__ARGS.extraDeps["AG-06b"]`) - `__ARGS.dropDeps["AG-06b"]`.

- package deps: `["IA-10c"]`
- `extraDeps["AG-06b"]`: none
- `dropDeps["AG-06b"]`: none
- So PREREQUISITES = { IA-10c }

`__ARGS.filesOnly["AG-06b"] = ["IA-10c"]`. The launcher filters a FILES_ONLY entry out of the hard
deps, so **IA-10c does not block the start**. Only the merge waits for it.

**Hard prerequisites that block the start: none.**

**FILES_ONLY links (merge first, do not block the start):**

| Part | Status at feat/db-fix e851da1ca | Notes |
|---|---|---|
| IA-10c | **NOT MERGED** (no `Merge IA-10c:` subject in `git log feat/db-fix --first-parent --merges`, and not in `__ARGS.doneExtra`) | Built on branch `fix/db-IA-10c` (tip 56634b002, 23 Oct 23:17 IST, "IA-10c repair 1 ..."). Its second check, `verify-IA-10c-r2`, has a progress file and no report yet. IA-10c is on the critical path. |

**Parts this one builds on (named in the spec; merged):**

| Part | Status |
|---|---|
| AG-06 | merged, `5cec3e7cd Merge AG-06: Pull audit journal appends wait a bounded time for the lock instead of being refused, so no run starts unjournalled under contention` (also in doneExtra) |
| R3-08 | merged, `294b24d4f Merge R3-08: Every read in a named, bounded pool; a reader that meets a writer waits, then says so` (also in doneExtra) |
| AF-04 | merged, `49ad8a060 Merge AF-04: ...` (the server.rs half that AG-06 left to "AF-04" is this package now) |
| IA-9, IA-16 (IA-10c's own deps) | merged (`0dc8361c3`, `c099cf003`) |

**Parts that share files with AG-06b (all NOT MERGED at e851da1ca):** R4-02 (the rest of
server.rs), R4-13 (also edits broker_run_by), PREC-16 (owns `crates/store/src/flock.rs`), R3-27
(owns `pullrun.rs`, and puts the pull journal in monthly pieces), R4-11 (now owns FM-162).
**Downstream:** WP-99 waits for AG-06b (WP-99's specAppend). The 20 Oct follow-up plan also says
"AG-06b must merge before the Zerodha half of the fresh run (the /pull/run full re-pull with kept
answers, mode (i)), because that half writes the pull journal through RunLedger" (main-session.md
item 52(d)). The 23 Oct no-fresh cut (D-7135) turned the "fresh run" into a patch run. That
section still says every patch run needs the opening record first.

**Before building, read section 7.1** (added by the independent check, 24 Oct). It lists what the
change meets that the spec does not say: the telemetry sink's run key, the `pull.run` "started"
event written before the opening, the emit-site count pinned at 96 in `emitted.rs`, and two
source-shape tests that pin the callers' journal-call text. Section 7's IA-10c part also notes
that IA-10c itself edits `autopilot.rs` and `pullrun.rs`.

---

## 2. The package's own text

### 2.1 Title and spec (verbatim)

Source: `__ARGS.extraPackages[name=="AG-06b"].spec`. That is line 6 of
`W/state/fix/db-fix-round-2-run0738.js.fat`, also in `W/state/cloud-20261024/args.json`. It is
7,477 bytes (`jq -j`, no trailing newline).

Line numbers in the spec are **at 453de6989**. Section 7 maps each one to e851da1ca.

```text
DECISION BLOCK D-7100..D-7109 (the main session records it in state/decision-blocks.json before launch, owner AG-06b). Every locked choice of this part is one docs/05-decisions.md entry numbered from this block in order, D-7100 first; ids left unused stay unused. This follow-up run (20 Oct 2026, state/followups-20261020) uses all of D-7000..D-7099: its nine other new packages take D-7000..D-7089 in blocks of 10, and D-7090..D-7099 holds the folds into R4-29, R4-09 and R3-20 and the deferral record D-7094. AG-06b therefore takes the first block after that range. Never use an id from D-6920..D-6999, which is reserved (state/decision-blocks.json). Re-grep docs/05-decisions.md on feat/db-fix and on every fix/* and origin/* branch before committing.

FOLLOW-UP PACKAGE. It comes from the fix B checks of AG-06 (merged 5cec3e7cd, D-6244): the two server.rs halves that AG-06 left NOT DONE by design (docs/06-limits.md L-AG06-03, L-AG06-04, L-AG06-07 and L-AG06-08, at :20003-20008 on feat/db-fix 453de6989), and AG-06's measurement harness. There is no fix-plan.json entry: this text is your package.
Read first:
- state/fix/round2/verify-AG-06-r2.json (followups, notes);
- state/fix/round2/fix-AG-06.json;
- docs/06 rows L-AG06-01..11 (:20001-20011);
- D-6244 (docs/05-decisions.md:75511).
Report one findings[] entry per item, AG06B-1 to AG06B-4.
Two things from the same checks are NOT yours:
- the store::flock items are PREC-16's, because it owns crates/store/src/flock.rs;
- the index-build hold (L-AG06-02) is R3-27's, because it puts the pull journal in monthly pieces.
Never edit flock.rs, audit.rs's build_index or index_locked, or pullrun.rs for them.

OWNED (a region, not whole files):
- crates/api/src/server.rs: struct RunLedger and its impl (:4148 to about :4400 at 453de6989); the opened() constructor (:4619-4640); and the journal calls (RunLedger::open, its flushes and its close) inside broker_run_by (:6074) and fno_walk (:11409), and in every other async caller of RunLedger that git grep finds (name them in notes).
- crates/api/src/audit_lock_wait_measure.rs and crates/api/src/audit_lock_wait_tests.rs (AG-06's, merged).
SHARED EDITS, declared:
- R4-02 owns the rest of server.rs and is told that this region is yours.
- IA-10c (critical path) edits the /pull/* handlers' run-id seat and merges FIRST. This is a FILES_ONLY link: you may start at once, and your merge waits for IA-10c. Rebase onto it and keep its run-id seat.
- R4-13 later edits broker_run_by for its SHAPE_CHANGED stop, after R4-02.
Whichever merges second keeps both sides.
Keep these readers compiling and unchanged:
- crates/api/src/autopilot.rs:4000, which reads opened_refused (the one-command cut keeps autopilot.rs exactly as merged);
- pullrun.rs restart_scan (R3-27's file; call it only).

ITEMS
(1) AG06B-1. NO RUN STARTS UNJOURNALLED (L-AG06-03, L-AG06-04).
Today RunLedger::open (server.rs:4185-4210) keeps opened_refused when journal.open_run is refused after APPEND_LOCK_WAIT (1 s, audit.rs:1414), and the run goes on to fetch and land. The restart scan (pullrun.rs:2042) reads only the journal, so if that run then dies it reads as clean.
Make the landed opening record a precondition of the run. When it is refused, refuse the run before its first vendor request, by name: journal_unavailable, naming the bound, the wait, the attempts, the record kind and the holder in store::flock::holder_note's words. Nothing is fetched, landed or written beyond the existing api.audit Error event and one pull.run refused event. /pull/run.json and the run's receipt carry the refusal.
RunLedger::open may keep its report shape. The existing test at server.rs:34210-34240 (asserting !ledger.opened() && ledger.opened_refused.is_some()) stays green, or is changed with the reason in notes. Never weaken it.
(2) AG06B-2. NO TOKIO WORKER WAITS ON THE JOURNAL LOCK (L-AG06-07, and the worker half of L-AG06-08).
broker_run_by and fno_walk call RunLedger::open and its flushes from async code on a tokio worker, so a contended append blocks that worker for up to 1 s, or 2 s with the notes lock.
Route every such call through the existing off_the_workers (server.rs:5815): block_in_place on the multi-thread runtime, inline on a current-thread one. That is one shared mechanism. Add no spawn_blocking: server.rs:19405-19420 records why a spawn_blocking task can hold a stopping process.
(3) AG06B-3. THE HARNESS CLOSES ITS RUNS.
audit_lock_wait_measure.rs:54 times a close built as Record::refused(Scope::Spot, Outcome::Stored, ...) through close_run. The restart scan does not pair that close with its opening (is_close, pullrun.rs:1520-1523; same_key near :1726), so 221 harness runs read as unclosed on the checker's shadow.
Build the close the way real runs close: Record::of_run (audit.rs:857, as at server.rs:6478), with the opening's source and window.
(4) AG06B-4. LEDGER ROWS.
- docs/06: new rows L-AG06b-NN; never edit AG-06's rows. L-AG06-03/-04 and L-AG06-07 are closed by items (1) and (2), with the test names and the measurements below.
- docs/05, from your block: the opening record is a precondition of a run (1); the journal calls on the pull paths run off the workers (2).
- docs/04: a row beside each test.

FAILING-FIRST TESTS (each red before your change, shown in a log):
- a_run_whose_opening_is_refused_after_the_bound_asks_the_vendor_nothing_and_lands_nothing. Hold the append lock past APPEND_LOCK_WAIT with audit_lock_wait_tests.rs's hold helper (:73). A fixture broker counts 0 requests, the store is byte-identical afterwards, and the refusal names journal_unavailable, the bound and the holder.
- a_contended_journal_append_never_stalls_the_http_surface. Use a MULTI-THREAD runtime with worker_threads(1): with two workers, the second one keeps answering before the fix, so the test would not fail first. Hold the lock 900 ms during a run's opening; 20 concurrent GET /feeds.json must each answer within 100 ms.
- every_journal_call_on_the_pull_paths_goes_off_the_workers. A source-shape test of broker_run_by and fno_walk, in the existing split_once pattern (server.rs:21372).
- the_measure_harness_closes_every_run_it_opens. Not ignored: a 3-run harness journal on a temp root goes through restart_scan, which finds no open run.
EXTREMES, each with a test:
- a zero bound: one attempt, refused at once, nothing fetched;
- a run refused at its opening and asked again at once lands when the holder lets go;
- the journal file missing or read-only: refused by name, nothing fetched;
- a current-thread runtime: the work runs inline, as off_the_workers says.

O(1) AND MEASUREMENTS:
- uncontended open_run, append_member and close_run, p50/p99/max at n >= 200, before and after, within the ratio gate;
- GET /feeds.json p50/p99/max at n >= 200 while a holder keeps the append lock, before and after, on your shadow (state/fix/shadow.sh AG-06b), never the real store.
Every cost left above O(1) is a docs/06 row with p50/p99/max.

DEPS: IA-10c, as a FILES_ONLY link (start now, merge after it). AG-06 and R3-08 are merged. store_writes_needed: none.

FAILURE MATRIX: the main session adds two rows owned by AG-06b (ids at append time): a run whose opening is refused after the bound runs unjournalled; and a contended journal append blocks a tokio worker for 1-2 s. FM-162 is NOT this package's: the main session re-owns it to R4-11. If your refusal also proves part of FM-162's text, say so in notes with the test name, so R4-11's builder can cite it.
```

### 2.2 `__ARGS.specAppend["AG-06b"]`

**None.** No specAppend key names AG-06b. The one-command-cut, no-fresh, options, ledger
follow-up, R4-09b and priorities amendments add nothing to this part. They are already folded into
the spec above (for example "the one-command cut keeps autopilot.rs exactly as merged").

### 2.3 What OTHER parts were told about AG-06b (verbatim, from their specAppend)

Source: `__ARGS.specAppend`, the paragraphs that name AG-06b. These are the far side of the
declared shared edits.

```text
=== WP-99
FOLLOW-UPS 20 Oct 2026 (state/followups-20261020). You are scheduled now and run LAST. Your edges now include every part this follow-up brought into the build (IA-13, R4-29, R4-24, SC-8, SC-11, R3-20, R3-33, R3-34) and every new package (IA-3b, CC-1, CC-3, CC-5, CC-7, CC-8, CC-14, CC-19, VKI-1, and AG-06b from the fix-B follow-ups).
=== R4-02
SERVER.RS REGION (20 Oct 2026 follow-ups). The new package AG-06b owns one region of server.rs:
- struct RunLedger and its impl (:4148 onward at 453de6989);
- the opened() constructor (:4619-4640);
- the journal calls inside broker_run_by (:6074) and fno_walk (:11409).
Its job there: no run starts unjournalled (a refused opening refuses the run with journal_unavailable before any vendor request), and no tokio worker waits on the journal lock (off_the_workers). Do not edit that region. If your merge meets AG-06b's hunks there, keep both sides.
=== R4-13
BROKER_RUN_BY (20 Oct 2026 follow-ups). The new package AG-06b also edits broker_run_by (:6074 at 453de6989): its journal calls go through off_the_workers, and a refused opening refuses the run with journal_unavailable before its first vendor request. Your SHAPE_CHANGED stop is a separate hunk; if your merge meets AG-06b's hunks, keep both sides. A run refused at its opening never reaches your stop.
```

---

## 3. Everything the spec points to

AG-06b is of kind `nw`, so `specStep(p)` sends the builder to the fix-plan entry and its findings
(section 3.1). The spec's own "Read first" list follows in 3.2 to 3.6, and the per-kind sources in
3.7 to 3.10.

### 3.1 fix-plan.json entry and findings

- `jq '[.work_packages[] | select((.name | split(" ")[0]) == "AG-06b")]' W/state/board/fix-plan.json`
  returns `[]`. **There is no fix-plan entry**, as the spec says: "There is no fix-plan.json
  entry: this text is your package."
- `W/state/board/audit-result.json` `.confirmed` has no finding whose id or package names AG-06 or
  AG-06b. The items are the spec's own: **AG06B-1, AG06B-2, AG06B-3 and AG06B-4**. Report one
  `findings[]` entry for each.
- "D-number assignments are in WP-02's decisions_needed" (generic specStep text) does not apply.
  AG-06b has its own block, D-7100..D-7109 (section 5).

### 3.2 Read first (1): `state/fix/round2/verify-AG-06-r2.json`, followups and notes (verbatim)

The second check of AG-06 at edc7870ad. Verdict `pass_with_followups`.

`followups` (an array of 8 strings, one per bullet):

```text
[0] AG-06 / docs/06 L-AG06-01 (store::flock): state the CROSS-PROCESS outcome with its measurement. Against another process only the head of the queue polls, so a holder that re-takes the lock without a gap starves it: 4 of 4 refused after 1 s at 0 gap. Two harness processes on one real-journal clone refused 5 of 3,600 appends. The one-server lock keeps this rare in production, but the limit row should name it. A kernel-blocking take on a helper thread with a deadline, or a lock-file note, would remove it.
[1] store::flock (AG-06 / R3-08): HOLDERS and the queue are keyed by the hash of the path SPELLING, not by (dev, ino). Two spellings of one lock (a symlinked root, relative vs absolute) split the in-process queue, so they fall back to polling. The refusal then says 'held by another process ... this process holds nothing' while this process holds it. Key by device and inode (one fstat), or canonicalise once.
[2] store::flock::holder_note (AG-06): for a lock on a DATA file (pull.journal, pull.notes), the no-registry fallback reads the file's first 512 bytes looking for a month-lock note, and then says 'a reader (readers write nothing)'. Readers of the journal never lock, so the wording misleads there. Name the journal case, or skip note_in for locks that are not month .lock files.
[3] AG-06 / docs/06 L-AG06-09: the registry cost can now be stated as measured. A patient take+release costs p50 875 ns, p99 1.08 us, max 19.7 us, against plain try_lock at 458 ns, 583 ns and 3.25 us (n = 5,000 each, probe vr2_uncontended_cost_of_a_patient_take_vs_try_lock).
[4] AG-06 harness (audit_lock_wait_measure.rs): the 'close' it times is Record::refused(Scope::Spot, Outcome::Stored, ...) through close_run. The restart scan does not count it as a closing record: 221 harness runs read as unclosed on the shadow. The latency figures stand, but a reader of the clone sees hundreds of open runs. Use a closing outcome the scan recognises, or say so in the harness doc.
[5] MERGE: feat/db-fix moved to c152231ad (R3-04, PREC-1) during this check, so `git merge-tree feat/db-fix fix/db-AG-06` now conflicts in docs/04-invariants.md, docs/05-decisions.md and docs/06-limits.md (append-only rows: keep both sides). Code merges cleanly, including feat/db-fix's +333-line audit.rs. In the merged audit.rs (tree a32ac877c) there are 3 try_lock_patiently sites, one of each *_DOING, and no static, Mutex<, LazyLock or thread::sleep, so the_refusal_text_pins_the_bound_constant's source checks still hold; this was checked by grep, not compiled. D-6244 is still free (ledger head D-6243).
[6] AF-04 (server.rs), named NOT DONE in L-AG06-03 and L-AG06-07: a run whose opening is refused after the 1 s bound still runs unjournalled, and restart_scan cannot say not_known. A contended append blocks a tokio worker for up to 1 s (2 s with the notes lock).
[7] L-AG06-02 (pre-existing in kind): a from-scratch index build holds the append lock for about 2-3 s on the 1.08 GB journal (measured 2.06 to 3.09 s on fresh clones here). Appends that meet it are refused after 1 s.
```

`notes`:

```text
Second check of AG-06 at edc7870ad (base feat/db-fix c76f9178c). Verdict: pass_with_followups. All gates green. Both of verifier r1's blocking items are fixed and reproduced. code-5's bounded wait reproduces on real data, before against after and in-process against cross-process. The package's status 'partial' is honest: the server.rs half (AF-04) is named NOT DONE in docs/06 L-AG06-03.

Things to know:
(1) Port 8905 was already held by another verifier's server (pid 64699, tgt/verify-ag05-r2-before/release/api), so my first serve.sh on 8905 failed to bind and its 200 came from that server. I did not use that answer. My shadow server ran on :8915: first pid 41589 (a copy of the fixer's 10:43 release binary), then pid 91914 (my own release build, sha256 65f1ee86...). Every GET figure in this report is from pid 91914, measured before 12:40 IST. At 12:40 the gate script's cp replaced tgt/verify-AG-06-r2-bin/release/api in place with the pool-5 build (sha256 206eb7a0...); the server stayed up.
(2) The before test binary was built into the fixer's target tgt/AG-06-r1-tgt from scratch/verify-AG-06-r2-before. Cargo gave it the same artifact path as the branch's (debug/deps/api-cc35fb44f697c8d6), so that file in the fixer's target is now the feat/db-fix+harness build. Cargo will rebuild it before any later use from wt/P-AG-06. My copies: tgt/verify-AG-06-r2-bin/api-test-before (sha256 c4c1ab24...) and api-test-after (544105d7...).
(3) My first `git -C wt/DBFIX worktree add scratch/...` used a relative path, which resolved under wt/DBFIX. I moved that worktree to scratch/verify-AG-06-r2 with `git worktree move`, which deletes nothing. wt/DBFIX/scratch/ already held other agents' worktrees.
(4) A zsh word-splitting slip in my own script created two stray shadow directories, each holding only a cloned pull.journal.idx: 'shadow/AG-06-v-r2m4 before' and 'shadow/AG-06-v-r2m5 after' (the names contain a space). A background loop then waited for a child that could not start, and I stopped it.
(5) pool-5.lock was taken at 11:33:58 and released by my gate script at 13:05:49.

Left in place, nothing deleted: scratch/verify-AG-06-r2 (the branch plus the probe file crates/store/src/flock_probe_r2.rs and its mod line in flock.rs); scratch/verify-AG-06-r2-before (feat/db-fix c76f9178c plus the harness and its mod line); targets tgt/verify-AG-06-r2-tgt and tgt/verify-AG-06-r2-bin; logs tgt/verify-AG-06-r2-logs (gates.log, test-*.log, clippy.log, deny-direct.log, measure-*.log, m*-held-*.log, xproc-*.log, gets-*.log, probe-store.log, kill-run.log, run.json, and the gets-*.t scratch files); shadows shadow/AG-06-v (the server root; its journal now carries harness records and audit/pull.held*), shadow/AG-06-v-r2m1, -r2m2, -r2m3, -r2m4b, -r2m5b, -r2m6b, -r2m7b, plus the two stray directories above. The older shadow/AG-06-v that shadow.sh moved aside is under shadow/.old/.
```

Which follow-up belongs to whom (from the AG-06b spec and `state/followups-20261020/main-session.md`):

| followups[i] | Owner |
|---|---|
| [0] L-AG06-01 cross-process starvation | PREC-16 (store::flock) |
| [1] HOLDERS keyed by path spelling | PREC-16 (planned matrix row FM-627) |
| [2] holder_note blames "a reader" on a data-file lock | PREC-16 (planned FM-628) |
| [3] L-AG06-09 registry cost, measured | PREC-16 / docs only |
| [4] the harness's close is not counted as a close (221 runs read unclosed) | **AG-06b, item AG06B-3** ("test-only: the harness runs on a cloned journal, never on the real store. No failure-matrix row") |
| [5] MERGE residue, run the_refusal_text_pins_the_bound_constant on the merged head | main session (landed) |
| [6] AF-04 server.rs, NOT DONE: an opening refused after the bound runs unjournalled; a contended append blocks a tokio worker | **AG-06b, items AG06B-1 and AG06B-2** |
| [7] L-AG06-02 index build holds the lock 2.06-3.09 s | R3-27 (planned FM-631) |

### 3.3 Read first (2): `state/fix/round2/fix-AG-06.json` (verbatim fields)

`head` edc7870adbaa6be3322a02f222c9abc082f59e11, `committed` true.

`files_changed`:

```text
crates/api/src/audit.rs
crates/api/src/audit_lock_wait_tests.rs
crates/api/src/audit_lock_wait_measure.rs
crates/store/src/flock.rs
crates/store/src/flock_patient_tests.rs
docs/04-invariants.md
docs/05-decisions.md
docs/06-limits.md
```

`findings` (one entry: id `code-5`, status `partial`):

```text
[
  {
    "id": "code-5",
    "status": "partial",
    "how": "Every journal append (append, append_member, open_run, append_failure, close_run, close_unopened) goes through Journal::appended_within, which takes the lock with store::flock::Flock::try_lock_patiently(file, path, APPEND_LOCK_WAIT = 1 s, APPEND_DOING) instead of try_lock. The .notes lock waits the same way. The index build registers with a zero bound as 'the journal index build'. REPAIR ROUND 1, blocking item 1: there is ONE patience mechanism in store::flock. One wait loop (patiently), called by both try_lock_shared_patiently (R3-08) and try_lock_patiently. One pause (MAX_PAUSE 16 ms). One ceiling (MAX_PATIENCE 1 s): APPEND_LOCK_WAIT is checked against it at compile time and the refusal prints the bound as cut. One registry (HOLDERS, read by holder_note) holds writers, readers and the in-process FIFO turn queue. PATIENT_PAUSE, MAX_PATIENT_WAIT and api::audit's HOLDERS, Holding and holder_of are gone, and api::audit holds no wait loop and no sleep. Blocking item 2: the registry and the kernel lock move together. take_held does the flock(2) grant and the record in one step under the registry's lock, and let_go does the forgetting and the unlock in one step. The Turn hand-off happens after the unlock, so a predecessor's end can never erase its successor's entry, and at no instant can a reader here find the lock held with no holder named. Past the bound the refusal names the bound, the record's kind and outcome, the wait, the attempts, the lockers ahead and the holder. The api.audit Error event is unchanged. After the lock, device and inode are compared with the path, so a journal replaced during the wait is refused by name. Readers never take the lock. Decision renumbered to D-6244 (feat/db-fix ledger head D-6243 + 1). The docs conflicts were resolved keeping both sides' rows. Verifier follow-ups taken: AG06-03 reworded to the production bound; the index build is named in the APPEND_LOCK_WAIT doc and in L-AG06-02; L-AG06-07 covers the blocked calling thread / tokio worker; L-AG06-08 covers the stacked notes wait; the refusal prints the cut bound. PARTIAL for one reason: a run whose opening is refused AFTER the bound still runs unjournalled, because RunLedger::open is in server.rs (AF-04's file). This is NOT DONE by design and named in docs/06 L-AG06-03, and L-AG06-04 corrects FM-162's close_unopened note.",
    "evidence": "Blocking 1: git -C /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX merge-tree --write-tree --name-only --messages feat/db-fix fix/db-AG-06 gives rc=0 with no CONFLICT (tree eaa02a9e3). In that tree, grep -nE 'loop \\{|MAX_PAUSE: |PATIENT_PAUSE|MAX_PATIENCE: |MAX_PATIENT_WAIT|static HOLDERS|static QUEUES' on crates/store/src/flock.rs finds MAX_PATIENCE :86, MAX_PAUSE :93, HOLDERS :216 and two loops: :1064, Turn::drop skipping given-up tickets with no sleep, and :1115, the one wait loop. grep -nE 'static HOLDERS|fn holder_of|struct Holding|thread::sleep|loop \\{' on crates/api/src/audit.rs finds nothing. Blocking 2: tgt/AG-06-r3-logs/store-test-before-repair2 flock::patient_tests::no_reader_here_ever_finds_the_lock_held_and_its_holder_unnamed FAILS on the two-step code: 1,155 and 1,638 held-but-unnamed samples (tgt/AG-06-r6-logs/sampler-before.log). The current store lib test binary tgt/AG-06-r1-tgt/debug/deps/store-782582652b875685 flock::patient_tests:: passes 20 of 20, three runs (sampler-after.log). a_successors_name_survives_its_predecessors_release passes too. code-5: cargo test -p api --locked (in tgt/AG-06-r5-logs/tests.log) runs 17 audit::lock_wait_tests, all ok, among them an_opening_append_waits_for_a_held_lock_and_lands, an_append_held_past_the_bound_is_refused_naming_the_bound, eight_concurrent_appenders_lose_no_record, a_reader_never_waits_on_the_append_bound, the_code_4_barrier_experiment_loses_no_record_in_300_runs and a_run_whose_opening_met_a_held_lock_and_then_died_reads_interrupted. Harness: cd crates/api; BRUTEX_AG06_MEASURE_ROOT=<fresh cp -cR clone of shadow/AG-06/audit> <api lib test binary> audit::lock_wait_measure --ignored --nocapture. Before (tgt/AG-06-r2-bin/api-before, the feat/db-fix try_lock code plus the harness) refuses 183-195 of 200 per op with 2 writers and 175-184 with 8. After (tgt/AG-06-r5-logs/api-test-r5, this tree's code) refuses 0 in every case, two runs (tgt/AG-06-r6-logs/measure-{before,after,before2,after2}.log). Shadow AG-06 :8805 (fresh, release api from this tree, GET only): /pull/run.json says restart clean, which is correct for the cloned journal. A GET cannot make lock contention, so the server write path is exercised by the tests and the harness, not by a POST (none sent)."
  }
]
```

`p50_p99` (the baseline AG-06b's "before" numbers compare against):

```text
- Method: ignored test harness api::audit::lock_wait_measure. 1, 2 and 8 writer threads start on one barrier; n = 200 samples per operation per case, timed with Instant per call. Each run uses its own fresh APFS clone (cp -cR) of shadow/AG-06/audit, the operator's 1.08 GB journal with 4,217,222 records, never the real one. Figures are ms p50/p99/max.
- Pair 2, 11:07 IST, load 17-26. BEFORE (feat/db-fix try_lock + harness): 1 writer open_run 3.4/6.8/2016.9, append_member 1.6/4.0/6.0, close_run 3.8/7.5/10.0, 0 refused. 2 writers refused 184/185/185 of 200. 8 writers refused 175/175/175 of 200.
- Pair 2, 11:07 IST. AFTER (this tree): 1 writer open_run 3.5/7.7/2057.9, append_member 1.8/3.1/3.2, close_run 3.5/6.0/6.0. 2 writers 8.0/18.3/23.6, 4.8/9.3/18.0, 6.8/16.0/22.7. 8 writers 31.2/63.8/66.3, 21.0/50.4/53.6, 24.8/54.0/61.0. 0 refused.
- Pair 1, 10:52 IST, load 19-21, with this package's own gate build running beside it. Before: 2 writers refused 192/195/195, 8 writers 183/183/184; 1 writer append_member 1.4/3.3/4.3. After: 0 refused everywhere, but fsync spikes under the load: 1 writer append_member 2.4/16.3/26.8; 8 writers append_member 21.1/710.3/716.6 and open_run 28.6/248.4/579.5. Every wait stayed inside the 1 s bound.
- The roughly 2 s max of each uncontended open_run is the index rebuilt from scratch on the first append to a fresh clone (a new inode), once per journal file (L-AG06-02). It is the same before and after.
- The wait is bounded by APPEND_LOCK_WAIT = 1 s (= MAX_PATIENCE), plus at most one pause of up to 16 ms. Uncontended cost: one registry lock and hash probe each for the ticket, the take and the release, plus one non-blocking flock call. Contended cost: at most the bound, whatever the queue length (docs/06 L-AG06-01, -09, -10, -11).
- GET, n = 200 sequential, shadow AG-06 :8805 (release api from this tree), 10:51 IST at load about 20, ms p50/p99/max: /pull/run.json 0.60/4.36/110.09, /audit.json?feed=zerodha 3.80/54.48/183.03, /audit.json?feed=gdfl 0.86/8.13/332.74. These read routes take no append lock and are not changed by this package; the tails are machine load.
```

`notes`:

```text
REPAIR ROUND 1 of AG-06. Earlier attempts in this round (cut off by the account switch at 10:21 IST on 20 Oct) had already committed both repairs as WIP commits, and merged feat/db-fix five times, up to c76f9178c (IA-3). This attempt reproduced each blocking item against the current tree. It finished the last docs fix: docs/06 had cited the off-main commits dad1a760d and aab82424b, which failed store tests/cited_commits.rs. It re-ran the docs-sensitive gates and the shadow check, measured before and after again, and squashed the branch. The squash is one commit edc7870ad on top of feat/db-fix c76f9178c, and its tree is identical to the WIP tip 327f8f458. The unsquashed history is kept on branch fix/db-AG-06-wip-r1.

BLOCKING 1 (two mechanisms): fixed. One loop, one pause, one ceiling and one registry, in store::flock. try_lock_shared_patiently calls patiently. The merge with feat/db-fix is clean. D-6244 = feat/db-fix ledger head D-6243 + 1; re-check at landing (A6).

BLOCKING 2 (registry names the wrong holder): fixed at the cause in the shared code. The grant and the record are one step (take_held), and the forget and the unlock are one step (let_go), under the registry's lock. The turn passes on only after the unlock. The test fails before (1,155 and 1,638 unnamed samples) and passes after.

NOT DONE by design (owner AF-04, server.rs): a run whose opening is refused after the 1 s bound still runs unjournalled, and the restart scan cannot say not_known for it (L-AG06-03). The bound does not cover a from-scratch index build (about 1.33 s per GiB, L-AG06-02). A stuck holder costs each append 1 s (L-AG06-06). A contended append blocks its calling thread for up to 1 s; spawn_blocking at the callers is AF-04's (L-AG06-07).

BASE RED, not this package's: core tests/cost_invariants.rs fails on C-HA-01, from R4-09a's store bench, because no docs/04 row declares it. It is red on feat/db-fix the same way.

No POST was sent. :8080, wt/APP and tgt/app* were not touched. The real journal is unchanged: 1,079,608,832 bytes, mtime Oct 13 22:47:50. The shadow server AG-06 :8805 is stopped and the port is free. Builds used this package's private target tgt/AG-06-r1-tgt, not pool-5, so this branch's crates do not reach other worktrees. pool-5.lock was held by another package from 10:33 and was not touched.

LEFT IN PLACE for the main session (nothing deleted):
- Branch: fix/db-AG-06-wip-r1.
- Targets: tgt/AG-06-r1-tgt.
- Logs: tgt/AG-06-r2-bin, tgt/AG-06-r2-logs through tgt/AG-06-r6-logs.
- Shadows: shadow/AG-06, shadow/AG-06-measure-{probe,before,after,before2,after2,r3,r5-before,r5-after,r6-before,r6-after,r6-before2,r6-after2}, shadow/AG-06-r1-{before,after}, shadow/AG-06-r2-before1.
- Scratch worktrees: scratch/AG-06-r1-before, scratch/AG-06-r1-first, scratch/AG-06-r2-before, wt/P-AG-06-before.
```

### 3.4 Read first (3): docs/06-limits.md, the AG-06 section with rows L-AG06-01..11 (verbatim)

At e851da1ca these are lines **20763-20864** of `docs/06-limits.md`. The section header is at
20763 and the rows are at 20854-20864. The spec's ":20001-20011" was at 453de6989. **Never edit
these rows.** AG-06b adds new rows `L-AG06b-NN`.

```text
## The pull audit journal's append lock waits a bounded time (AG-06, D-6244)

**First build, measured 18 October 2026**, with the test harness
`api::audit::lock_wait_measure::measure_append_latency_under_contention`
(`crates/api/src/audit_lock_wait_measure.rs`, `--ignored`), which calls
`Journal::open_run`, `append_member` and `close_run` through the public API
with 1, 2 and 8 writer threads started on one barrier, 200 samples of each
operation in each case (1 writer × 200, 2 × 100, 8 × 25), timing each call
with `Instant`. Each side ran over its own APFS clone (`cp -c`) of the
operator's 1.08 GB journal (4,217,222 records) and its index, on the
WD_BLACK volume where the journal lives, so the real journal was never
written. Before: a test build (the workspace test profile, optimised with
debug info) of `feat/db-fix` at the head this branch started from, with only
the harness file added. After: this branch, the same profile. The shared
machine's load average was 11 to 41. Figures are p50 / p99 / max in
milliseconds; "refused" counts appends refused.

| Writers | Operation | Before p50 / p99 / max | Before refused | After p50 / p99 / max | After refused |
|---|---|---|---|---|---|
| 1 | `open_run` | 6.6 / 16.7 / 2,216.0 | 0 | 5.4 / 12.5 / 2,451.8 | 0 |
| 1 | `append_member` | 3.2 / 7.9 / 11.3 | 0 | 2.6 / 9.1 / 15.1 | 0 |
| 1 | `close_run` | 6.5 / 11.2 / 13.5 | 0 | 5.6 / 11.6 / 13.5 | 0 |
| 2 | `open_run` | 0.6 / 16.6 / 20.0 | 100 | 11.7 / 20.1 / 22.0 | 0 |
| 2 | `append_member` | 0.4 / 8.9 / 11.2 | 100 | 7.4 / 13.7 / 20.3 | 0 |
| 2 | `close_run` | 4.6 / 18.5 / 19.8 | 100 | 10.3 / 17.1 / 31.4 | 0 |
| 8 | `open_run` | 0.05 / 7.8 / 8.9 | 175 | 39.4 / 49.5 / 163.9 | 0 |
| 8 | `append_member` | 0.05 / 5.2 / 6.2 | 175 | 29.3 / 156.1 / 161.7 | 0 |
| 8 | `close_run` | 0.06 / 7.1 / 10.0 | 175 | 30.0 / 48.9 / 50.6 | 0 |

The two sides ran ten minutes apart at a load average of 11 to 13, each on
its own fresh clone. Before, a contended call is fast because it is refused
(17 to 60 µs), and the record is lost: 100 of 200 of each operation with two
writers, 175 of 200 with eight. After, nothing is refused and a contended
call costs the holds queued ahead of it. An earlier pair at a load average of
15 to 41 agreed: before, 199, 199 and 198 of 200 refused with two writers and
194 to 195 with eight; after, none, with eight writers' p99 at 20.7, 17.9
and 22.0 ms and the uncontended p99 at 3.7, 2.2 and 4.3 ms. A first build of
this change that only polled (no queue in the process) refused 1 of 200
`open_run`s with eight writers on the clone, and 102 of 800 appends in
`eight_concurrent_appenders_lose_no_record`: a poller loses to a writer that
takes the lock again the moment it lets go, which is why the queue exists.


The first `open_run` on each clone (the 2.2 s and 2.5 s max of the uncontended rows)
is the index rebuilt from scratch: a clone is a new inode, and the index is
keyed by device and inode, so the first append rebuilt it under the append
lock (L-AG06-02).

**Repairs 1 and 2, measured 20 October 2026.** Repair 1 folded the first
build's own wait loop, pause, 10 s ceiling, queue map and holder registry into
R3-08's (`store::flock::patiently`, `MAX_PAUSE`, `MAX_PATIENCE`, `HOLDERS`),
and moved the holder's registration into the lock guard. Repair 2 made the
`flock(2)` grant and the registration one step under the registry's lock, and
the forgetting and the unlock another (L-AG06-10). The same harness ran over
fresh APFS clones (`cp -cR`) of the `audit/` of this package's shadow
(`shadow.sh AG-06`), never the real journal. Before: a test build of
`feat/db-fix` as of its SC-3 merge (20 October, 00:31 IST) with only the
harness added (saved as `tgt/AG-06-r2-bin/api-before`; no later commit on
feat/db-fix touches `crates/api/src/audit.rs` or `crates/store/src/flock.rs`,
so the `try_lock` code it measures is still the base's), run at 01:43
IST at a load average of 15 to 17. After: this branch with both repairs, run
at 07:44 IST at a load average of 11 to 13, over the clone
`shadow/AG-06-measure-r3`. Figures are p50 / p99 / max in milliseconds, n =
200 per row; "refused" counts appends refused.

| Writers | Operation | Before p50 / p99 / max | Before refused | After p50 / p99 / max | After refused |
|---|---|---|---|---|---|
| 1 | `open_run` | 3.7 / 8.2 / 2,412.4 | 0 | 4.8 / 10.2 / 2,246.6 | 0 |
| 1 | `append_member` | 1.8 / 5.6 / 6.1 | 0 | 2.3 / 8.2 / 19.6 | 0 |
| 1 | `close_run` | 3.4 / 8.5 / 9.5 | 0 | 4.9 / 10.7 / 11.4 | 0 |
| 2 | `open_run` | 0.015 / 6.7 / 8.5 | 170 | 10.9 / 18.2 / 34.3 | 0 |
| 2 | `append_member` | 0.015 / 3.2 / 3.3 | 171 | 6.7 / 12.8 / 32.6 | 0 |
| 2 | `close_run` | 0.015 / 6.4 / 6.8 | 171 | 9.7 / 16.7 / 17.5 | 0 |
| 8 | `open_run` | 0.054 / 4.6 / 6.6 | 186 | 32.2 / 48.9 / 52.0 | 0 |
| 8 | `append_member` | 0.059 / 3.9 / 4.9 | 185 | 22.2 / 35.5 / 47.4 | 0 |
| 8 | `close_run` | 0.057 / 4.0 / 6.6 | 186 | 24.4 / 36.7 / 38.4 | 0 |

As on 18 October, a contended call before is fast because it is refused and
its record lost; after, nothing is refused and a contended call costs the
holds queued ahead of it, bounded by `APPEND_LOCK_WAIT`. The uncontended max
of `open_run` (2.2 s and 2.4 s) is the index rebuilt from scratch by the
first append to a clone (L-AG06-02). The repaired build's uncontended p99 is
within the noise of the 18 October pair (12.5, 9.1 and 11.6 ms then).
Re-run: `BRUTEX_AG06_MEASURE_ROOT=<a fresh cp -cR clone root>
tgt/AG-06-r3-logs/api-test-repair2 --ignored --exact
audit::lock_wait_measure::measure_append_latency_under_contention
--nocapture`; logs `tgt/AG-06-r2-logs/measure-before1.log` and
`tgt/AG-06-r3-logs/measure-after-r3.log`.

| ID | Limit | Why it is not O(1) / not closed | Bound | Package |
|---|---|---|---|---|
| L-AG06-01 | An append that meets another writer's append waits for it | the lock is one file's exclusive `flock(2)`; waiting is what replaced losing the record (code-5, FM-162) | `APPEND_LOCK_WAIT` = 1 s, which is also `store::flock::MAX_PATIENCE`, the one ceiling of every patient lock (a compile-time assertion keeps it at or under it), then refused by name (bound, record kind and outcome, wait, attempts, holder). Uncontended p99 3.7 to 12.5 ms (`open_run`) and 4.3 to 11.6 ms (`close_run`) across the 18 October runs, so 64 queued appenders at that p99 need 0.24 to 0.8 s: the headroom is thin at the top of the measured load, and past it the 64th is refused by name (`sixty_four_appenders_at_once_each_land_or_are_refused_by_name` proves only that none is lost silently, not that all 64 land). In this process waiters queue in the order they asked; against another process the head of the queue polls, 1 ms doubling to `MAX_PAUSE` (16 ms) | AG-06 |
| L-AG06-02 | A from-scratch index build holds the append lock for the whole build | the index is built under the append lock so no append can overtake it (WP-33c) | about 1.33 s per GiB (2.2 to 2.5 s measured for the 1.08 GB clone on this loaded machine), once per journal file: at the first start or append after the index is absent or is not this file's (a restored or cloned journal). An append that meets it waits 1 s and is refused, naming the holder: "doing the journal index build" when `Journal::build_index` runs it, but "doing a pull audit journal append" when the build runs inside an append (the in-append build is not told apart). **NOT DONE**: a bound that covers it would grow with the journal | AG-06 |
| L-AG06-03 | **NOT DONE: a run whose opening append is refused AFTER the bound still runs unjournalled** | `RunLedger::open` (server.rs, batch-1 AF-04's file) keeps `opened_refused` and runs on, and `RunLedger::flush` never retries the opening; the restart scan (`pullrun.rs`) reads only the journal | the window is now a holder past 1 s (a stuck writer, or L-AG06-02), not any few-millisecond append. Follow-on after AF-04: refuse to start a run whose opening could not be journalled, or retry the opening at the first flush; and answer `not_known`, not "clean", when such a refusal is known. The `api.audit` Error "record not appended" and `pull.run started` events are still written | AG-06, AF-04 |
| L-AG06-04 | Correction to the D-4950 note above that a refused opening record "closes behind it written late": `close_unopened` covers a refused opening only when the run reaches its close | a run that dies before its close leaves no opening record, and the restart scan read that journal as clean (code-5) | narrowed by L-AG06-01 to L-AG06-03's window | AG-06 |
| L-AG06-05 | A holder in another process is not named by itself | `flock(2)` does not say who holds a lock; the registry of holders is this process's own, and the journal's writers leave no note in it (a note is for a lock file that holds nothing else) | the refusal says, in `store::flock::holder_note`'s words, "held by another process that left no note in the lock" and that this process holds nothing on it (a `cli` pull, or a second server over this store) | AG-06 |
| L-AG06-06 | A stuck holder costs every append the bound | each append waits its own 1 s before it is refused; nothing remembers that the last one was refused | 1 s per append, each refusal an `Error` event naming the holder; a run with many failed members appends one record per member | AG-06 |
| L-AG06-07 | **A contended append blocks the thread that calls it for up to the bound** | the wait is a blocking poll and a condition-variable wait, and the pull runs call the journal from async code: `broker_run_by` (`server.rs` near line 6070) and `fno_walk` (near line 11405) reach `RunLedger::open` and its flushes on a tokio worker | up to 1 s of one worker per contended append (the 8-writer p99 above is the measured cost); moving those calls to `spawn_blocking` is `server.rs`, batch-1 AF-04's file. **NOT DONE** here | AG-06, AF-04 |
| L-AG06-08 | A member failure whose note was cut can wait twice | its whole note is appended to `pull.notes` under the journal's own lock (so the note's ordinal is the record's), and that file's lock waits its own `APPEND_LOCK_WAIT` | at most 2 s in all. Every writer in this code takes the notes lock only under the journal lock, so the second wait is met only behind a holder outside this protocol (an older build, or a stuck process) | AG-06 |
| L-AG06-09 | The holder registry is paid on every append | one registry lock and hash probe for the ticket, one for the holder, and one each as they end (R3-08's L-R308-02 cost, which no bench times apart) | inside the append latency measured above, uncontended and contended; not timed apart | AG-06 |
| L-AG06-10 | The process-wide holder registry's lock is held across one non-blocking `flock(2)` call at every registered take and every registered release (repair 2) | the registry is this process's memory and `flock(2)` is the kernel's: repair 1 registered after the grant and forgot before the unlock, so at those two instants a refusal read "another process" while this one held the lock. Repair 2 makes each pair one step under the registry's lock (`store::flock::take_held`, `let_go`), so every reader here sees the two move together. The price: every registered take and release in the process (the journal's appends, R3-08's month writers and patient readers) passes through that one mutex for the length of one `flock` call, which never blocks | one `flock` call per hold of the registry's lock, never a blocking one. Its cost is inside the journal append latency measured above (repairs 1 and 2: uncontended `append_member` p50 2.3 ms, p99 8.2 ms, n = 200) and is not timed apart (UNVERIFIED alone, as L-R308-02). What it buys, measured by `no_reader_here_ever_finds_the_lock_held_and_its_holder_unnamed` (8 lockers × 200 takes, a sampler holding the registry's lock): with repair 1's two-step code, 5,964 of 25,038, 6,684 of 11,811 and 5,560 of 10,455 samples that met the lock held found nobody named (three runs of the saved binary `tgt/AG-06-r3-logs/store-test-before-repair2`); with repair 2 the test asserts none, with at least 100 such samples | AG-06 |
| L-AG06-11 | A hand-off wakes every patient locker of the path waiting in this process, not only the next one | the queue (`store::flock`'s `Turns`) is one counter pair and one condition variable per lock path; a hand-off moves `serving` and calls `notify_all`, and each woken waiter re-checks its own ticket under the queue's mutex and sleeps again unless it is next. A waiter that gives up before its turn is kept in an ordered set and skipped when reached (O(log g) for g given up) | per hand-off, O(w) wake-ups for w lockers of that path waiting in this process; w is at most the threads appending to one journal at once, each waiting at most `APPEND_LOCK_WAIT`. The pull runs append from a handful of threads; the tests drive w up to 63 (`sixty_four_patient_lockers_all_take_the_lock_in_turn`, `sixty_four_appenders_at_once_each_land_or_are_refused_by_name`). Its cost is inside the 8-writer rows above (p99 36.7 to 48.9 ms, dominated by the holds' `fsync`) and is not timed apart (UNVERIFIED alone). One condition variable per ticket would make a hand-off wake one waiter; not done, because w is small and bounded | AG-06 |
```

### 3.5 Read first (4): D-6244 in docs/05-decisions.md (verbatim)

At e851da1ca, lines **78517-78640** (the spec's ":75511" was at 453de6989). The next entry, D-6245,
starts at 78641.

```text
### D-6244 — Every append to the pull audit journal waits a bounded time for the append lock instead of being refused at once, through R3-08's one patience mechanism (AG-06; code-5, FM-162) — 2026-10-19

**Number** the ledger's head on feat/db-fix was D-5883 when this entry was
written, D-5902 (AH-05) at this change's merge of AH-05, and D-6243 (R4-09a)
at its merges of R4-09a and of IA-3 (20 Oct; IA-3's eight entries, D-4815 to
D-5903, all sit below the head), so this takes D-6244 (amendment A6: head + 1
at landing; re-checked at landing). It replaces the numbers D-5865 and D-5903
this change carried before; neither was landed as this decision, and D-5903
is now IA-3's own, a different decision.

**Finding** code-5 (14 Oct attack, both skeptics; a consequence of FM-162). The
journal's append lock (`api::audit::Journal::appended_images`, and the whole
note file's lock beside it) was taken with `Flock::try_lock`, so an append
that met another writer's few-millisecond append was refused at once and its
record lost. When the lost record was a run's opening record and the process
then died before the run's close, the restart scan read the journal and
answered "clean" for a run that ran. The code-4 skeptic's 300-run barrier
experiment lost records on every run to the same refusal.

**Decided.**

1. **One patience mechanism, R3-08's, made exclusive.** `store::flock`
   already had R3-08's patient SHARED lock (`with_patience`, `MAX_PATIENCE`,
   `try_lock_shared_patiently`) and its holder registry (`HOLDERS`,
   `holder_note`). The exclusive twin, `Flock::try_lock_patiently(file, path,
   bound, doing)`, reuses every part of it, and keeps none of its own:
   - **one wait loop**, `store::flock::patiently`, which both patient locks
     call (R3-08's own loop was folded into it): ask once, and on contention
     pause 1 ms doubling to `MAX_PAUSE` (16 ms) until the bound, measured as
     the longer of the monotonic clock and the pauses slept, so a clock that
     steps backwards cannot extend it; a host refusal that is not contention
     is returned at once, never waited on;
   - **one pause**, `MAX_PAUSE` (made public so the journal's documents can
     name it);
   - **one ceiling**, `MAX_PATIENCE` (1 s): a shared lock's whole budget in
     one `with_patience` call, and the cut on any one exclusive bound. The
     10 s ceiling the first build of this change kept beside it is gone;
   - **one registry**, `HOLDERS`: a guard taken patiently registers its
     holder (thread, since when, and what its caller said it is doing) IN
     THE SAME STEP as `flock(2)` grants the lock, and forgets it in the same
     step as it unlocks, both under the registry's lock (repair 2;
     R3-08's month guards take the same step), and the refusal names the
     holder in `holder_note`'s words. The in-process queue (point 2) lives in the same
     entry, so a path's holder and its waiters are one record.
2. **In this process, patient exclusive lockers of one path queue in the
   order they asked** (a ticket each, a condition variable, a place given up
   when its waiter's bound runs out), and only the head of the queue polls
   `flock(2)`. A first build that only polled refused 102 of 800 appends from
   eight threads appending in a loop with a one-second bound: the lock is free
   only for the microseconds between one holder's release and the next take,
   and a sleeping poller wakes to a held lock. With the queue none is
   refused. The turn passes to the next locker only after the unlock (the
   guard's `turn` field drops after its `Drop` has unlocked), and an empty
   queue is forgotten with its registry entry.
3. **The bound is `api::audit::APPEND_LOCK_WAIT`, one second**, equal to the
   ceiling; a compile-time assertion refuses a value above it, which would be
   cut silently. It is set from the measured holder time (one append's open,
   `stat`, write, `fsync` and index catch-up on a clone of the operator's
   1.08 GB journal, uncontended p99 3.7 to 12.5 ms, `docs/06-limits.md`
   AG-06). Both append lock sites (the journal and its `.notes` file) use it.
   `build_index` takes the same lock with a zero bound: the old `try_lock`
   answer (busy at once when anyone holds or awaits it), now registered while
   it builds, so an append that waits out its bound behind a long build names
   "the journal index build".
4. **A refusal past the bound names the bound, the record and the holder.**
   The text keeps "cannot take the journal append lock" and "refused rather
   than interleaved", and adds the record's kind and outcome, the bound as
   cut by the ceiling, the wait, the attempts, how many were ahead in this
   process, and the holder: "held by this process (pid, thread, doing a pull
   audit journal append / the journal index build) since …", or, when no
   holder in this process is registered, "held by another process" in
   `holder_note`'s words (`flock(2)` does not say which). The `api.audit`
   Error "record not appended" event is unchanged and carries the whole
   refusal.
5. **The locked file must still be the journal.** The wait widened the moment
   between `open` and the lock, so after the lock the descriptor's device and
   inode are compared with the path's (one `stat`); a journal moved aside or
   replaced meanwhile is refused by name rather than appended to a file no
   reader opens.
6. **Readers never wait on this bound.** No journal reader takes the append
   lock (`look`, `page`); the restart scan's own wait for an index build
   (`pullrun::INDEX_WAITS`) is unchanged.

**Repair 1 (19 Oct), and why.** The first build of this change (its first commit on fix/db-AG-06)
was made before R3-08 merged, and its second check failed it on two counts.
Merged, it left two wait loops, two pause constants, two ceilings and two
holder registries side by side (rule 2 asks for one path); points 1 and 3
are the one path. And its own registry, kept in `api::audit` beside the
guard, named the wrong holder: the guard's explicit `drop` released the lock
and handed the turn on before the registry entry dropped at function exit,
so the predecessor's late removal erased the successor's entry and a refusal
during the successor's hold blamed another process. Registration now
belongs to the guard and follows the lock (point 1), so that ordering cannot
recur; `the_holder_is_named_through_every_in_process_hand_off` samples the
registry through real hand-offs and fails on the first build.

**Repair 2 (20 Oct), and why.** Repair 1 registered after the grant and
forgot before the unlock, which left two instants a reader here could meet:
the lock held by this process and nobody named, read as "another process".
A sampler that holds the registry's lock while it asks the kernel whether
the lock is held found that in 5,964 of 25,038 held samples. Now the grant
and the record are one step under the registry's lock
(`store::flock::take_held`), and so are the forgetting and the unlock
(`let_go`). Chosen over the alternatives because it makes the registry
exact for every reader in this process at the price of one non-blocking
`flock` call under a mutex the take and the release already lock; recording
a holder before the grant would name one that does not hold the lock, and
naming the instants only (repair 1's own note) left the defect.
`no_reader_here_ever_finds_the_lock_held_and_its_holder_unnamed` is the
exact sampler; `docs/06-limits.md` L-AG06-10 states the cost.

**Not done here, and why.** A run whose opening append is refused AFTER the
bound (a holder that keeps the lock longer than one second: a stuck writer, or
the once-per-journal from-scratch index build of about 1.33 s per GiB) still
runs unjournalled, because `RunLedger::open` keeps `opened_refused` and runs on
and `RunLedger::flush` never retries the opening. That is `server.rs`, batch-1
AF-04's file. Follow-on after AF-04: refuse to start a run whose opening could
not be journalled, or retry the opening at the first flush; and let the restart
scan (`pullrun.rs`) answer `not_known` rather than "clean" when such a refusal
is known; and move the append off the async worker (`spawn_blocking`), since a
contended append blocks the thread that calls it for up to the bound.
`docs/06-limits.md` (AG-06) names each, and corrects the earlier reading that
`close_unopened` covers a lost opening record: it does only when the run
reaches its close.
```

### 3.6 docs/04-invariants.md, the AG-06 rows AG06-01..AG06-12 (verbatim, for the row format)

At e851da1ca, lines **8291-8312**. The next section starts at 8313.

```text
### The pull audit journal's append lock waits a bounded time (AG-06; D-6244)

`store::flock::Flock::try_lock_patiently` (R3-08's patience made exclusive)
and `api::audit`'s three lock sites. Proofs in
`crates/store/src/flock_patient_tests.rs` (AG06-07, AG06-08, AG06-10 to
AG06-12) and `crates/api/src/audit_lock_wait_tests.rs` (AG06-01 to AG06-06,
AG06-09, AG06-11). Every fixture is invented.

| Id | Invariant | Proof | |
|---|---|---|---|
| AG06-01 | **An append that meets another writer's append waits for it and lands.** That holds for every append kind (`append`, `append_member`, `append_failure`, `close_run`, `close_unopened`, `open_run`) and for the whole-note file's own lock. | `an_opening_append_waits_for_a_held_lock_and_lands`, `every_append_kind_waits_for_a_held_lock`, `the_whole_note_lock_waits_too` | ✓ |
| AG06-02 | **A lock held past `APPEND_LOCK_WAIT` refuses the append within the bound plus one pause, never a hang.** The refusal names the bound, the record's kind and outcome, the wait, and the holder in `store::flock::holder_note`'s words: this process's thread and what it is doing (an append, the index build) when it is one, else another process. A zero bound is one attempt. Nothing of a refused append lands. | `an_append_held_past_the_bound_is_refused_naming_the_bound`, `a_refusal_names_a_holder_in_this_process_when_it_is_known`, `a_zero_bound_refuses_at_once` | ✓ |
| AG06-03 | **Concurrent appenders lose no record.** 8 threads × 100 appends behind one barrier write 800 whole records, each once. 64 appenders at once with the production bound each land at an ordinal of their own or are refused naming the bound, and the journal holds exactly the ones that landed (whether all 64 fit in one second is the disk's to say: `docs/06-limits.md` L-AG06-01). The code-4 barrier experiment (one walk opens and closes while another opens, 300 runs) loses none. | `eight_concurrent_appenders_lose_no_record`, `sixty_four_appenders_at_once_each_land_or_are_refused_by_name`, `the_code_4_barrier_experiment_loses_no_record_in_300_runs` | ✓ |
| AG06-04 | **A run whose opening met another writer's append, and whose process then died, reads interrupted at the next start, not clean** (code-5). | `a_run_whose_opening_met_a_held_lock_and_then_died_reads_interrupted` | ✓ |
| AG06-05 | **A reader never waits on the append bound.** `look` and `page` take no append lock and answer while one is held. | `a_reader_never_waits_on_the_append_bound` | ✓ |
| AG06-06 | **The extremes refuse by name or land, never silently.** A holder killed while it holds the lock frees it to the waiter. A journal moved aside or replaced while an append waits is refused, and neither file gains a record. A full disk after the lock is refused, cut back, and the lock released. A torn tail met after the wait is refused, not repaired. | `a_holder_killed_while_holding_frees_the_lock_to_the_waiter`, `a_journal_replaced_while_an_append_waits_is_refused_by_name`, `disk_full_on_the_append_after_the_lock_is_refused_by_name_and_releases_the_lock`, `a_torn_tail_after_the_wait_is_refused_not_repaired` | ✓ |
| AG06-07 | **The patient exclusive lock waits, then takes; or expires and names its wait and its holder.** It holds the lock exclusively once taken. 64 patient lockers at once, each holding briefly, all take it in turn within the production ceiling. A file replaced over the path hands the waiter the file it opened (the caller checks identity). | `an_exclusive_patient_lock_waits_then_takes`, `an_exclusive_patient_lock_expires_and_names_its_wait`, `sixty_four_patient_lockers_all_take_the_lock_in_turn`, `a_holder_whose_file_is_replaced_still_hands_the_waiter_the_file_it_opened` | ✓ |
| AG06-08 | **The one wait loop is bounded whatever the clock or the bound does.** A free lock is one attempt and no pause. A zero bound is one attempt. A host refusal is not waited on. Pauses go 1, 2, 4, 8, 16, 16 ms, and the last is cut to the bound. A clock stepping backwards cannot extend the wait, and one jumping forwards ends it early. A garbage bound is cut to `MAX_PATIENCE`. | `a_free_lock_is_taken_at_the_first_attempt`, `a_zero_bound_is_one_attempt_and_no_pause`, `a_host_refusal_is_not_waited_on`, `the_pauses_double_from_one_ms_to_the_patience_step_and_the_last_is_cut`, `a_clock_stepping_backwards_cannot_extend_the_wait`, `a_clock_jumping_forwards_ends_the_wait_early_never_late`, `a_garbage_bound_is_cut_to_the_ceiling` | ✓ |
| AG06-09 | **The journal's lock sites go through `store::flock` only.** The journal and its notes file wait; the index build registers with a zero bound; each names what it is doing; `api::audit` holds no wait loop and no registry of its own. The bound is one second and cannot exceed the ceiling (a compile-time assertion). | `the_refusal_text_pins_the_bound_constant` | ✓ |
| AG06-10 | **Patient lockers of one path in this process take turns in the order they asked, so none is starved.** Eight threads taking the lock in a loop 100 times each with a one-second bound are never refused. One that gives up hands its place on, and the next is served. A zero bound behind a locker in this process refuses at once, naming it. An empty queue is forgotten with its registry entry. | `eight_lockers_appending_in_a_loop_are_never_starved`, `lockers_in_one_process_take_turns_in_the_order_they_asked`, `a_locker_that_gives_up_hands_its_place_on`, `a_zero_bound_behind_a_locker_in_this_process_refuses_at_once`, `a_refusal_behind_a_holder_in_this_process_names_it` | ✓ |
| AG06-11 | **While a holder in this process holds the lock, the registry names it, at every instant a reader in this process can see, through every hand-off; and it never names one that has let go** (repair 1 and repair 2). The `flock(2)` grant and the registration are one step under the registry's lock (`store::flock::take_held`), and so are the forgetting and the unlock (`let_go`); both `flock` calls are non-blocking. The turn passes on only after the unlock, so a predecessor's end never erases its successor's entry; once it lets go nothing of it stays. Repair 1 left two instants uncovered (between the grant and the registration, and between the forgetting and the unlock); repair 2 closed both. Proved exactly by a sampler that, holding the registry's lock as every reader here does, asks the kernel whether the lock is held and the registry whether a holder is named, through 1,600 hand-offs among 8 lockers: zero samples find it held and unnamed (repair 1's two-step code: 5,964 of 25,038 in one run). Through the journal, a sample that meets the lock held and reads "not this process" is checked twice more and is never still wrong. The month locks (R3-08's noted writer and patient reader) take the same one step. | `a_patient_holder_is_named_while_it_holds_and_forgotten_when_it_lets_go`, `a_successors_name_survives_its_predecessors_release`, `no_reader_here_ever_finds_the_lock_held_and_its_holder_unnamed`, `the_holder_is_named_through_every_in_process_hand_off` | ✓ |
| AG06-12 | **One patience mechanism in `store::flock`** (repair 1): one wait loop that the shared and the exclusive patient locks both call, one pause constant, one ceiling, one registry of holders and waiters; no lock sleeps outside the loop. R3-08's shared-lock proofs pass through the shared loop unchanged. | `one_wait_loop_one_pause_one_ceiling`, and R3-08's `a_patient_reader_waits_for_a_writer_then_is_refused`, `a_patient_reader_takes_the_month_a_writer_lets_go_in_time`, `a_requests_patience_is_spent_across_every_month_it_opens` | ✓ |
```

### 3.7 The follow-up source items (verbatim)

`state/followups-20261020/fixb-followups.json` `[1]` (part `AG-06`), `followups[6]`:

```text
AF-04 (server.rs), NOT DONE by design (L-AG06-03, L-AG06-07): an opening refused after the bound still runs unjournalled and restart_scan cannot say not_known; a contended append blocks a tokio worker for up to 1 s (2 s with the notes lock).
```

`state/followups-20261020/main-session.md`, items 50 to 53 (lines 791-833; item 51 is included because it lies between), verbatim. The
decision block in 52(a) was later moved to **D-7100..D-7109** by correction A1 (README.md:29,
main-session.md:13, decision-blocks.json). D-7090..D-7099 holds the folds. Section 5 is the one
that holds.

```text
### 50. fixb-followups.json [1] AG-06 followups[4] (AG-06 harness: its close, Record::refused Stored through close_run, is not counted as a close by the restart scan; 221 runs read as unclosed)

already_covered: false

Owned by the new package AG-06b, item (3) AG06B-3; its spec is in the AG-06 followups[6] item. It is test-only: the harness runs on a cloned journal, never on the real store. No failure-matrix row.

### 51. fixb-followups.json [1] AG-06 followups[5] (MERGE: feat/db-fix moved to c152231ad; merge-tree conflicts in docs/04, 05, 06; merged tree not compiled; D-6244 still free)

already_covered: true

Landed. One residue is left. The merge step compiled the merged tree (cargo check over store, cli, vocab, core, pull and api) and ran only the cited_commits test, and the AG-05 merge after it left no test log. So api's the_refusal_text_pins_the_bound_constant (crates/api/src/audit_lock_wait_tests.rs:688) has not run on the merged head.

Run it with the full workspace test on the feat/db-fix head (453de6989 now), in a detached worktree with the one fixed private target dir, touched first (OPS-10):
1. cargo test -p api --locked --lib the_refusal_text_pins_the_bound_constant;
2. then the workspace run.
Record the result.

### 52. fixb-followups.json [1] AG-06 followups[6] (AF-04 server.rs, NOT DONE by design L-AG06-03, L-AG06-07: an opening refused after the bound still runs unjournalled and restart_scan cannot say not_known; a contended append blocks a tokio worker for up to 1 s, 2 s with the notes lock)

already_covered: false

(a) DECISION BLOCK. state/decision-blocks.json already records D-6900..D-6909 (the cut) and D-6910..D-6919 (R4-19d), keeps D-6920..D-6999 reserved and unused, and has free_from D-7000. AG-06b uses D-7090..D-7099, the last block of this run's D-7000..D-7099.
The assembler gives the other new packages blocks of 10 from D-7000 upward, in order, and must stop at D-7089. The packages area's drafts still carry D-69xx blocks (src/items-01..03: IA-3b D-6910, which is R4-19d's, CC-1 D-6920, CC-3 D-6930, ...), so all of them must be renumbered. If they need more than nine blocks, ask the main session for more ids; never take D-7100 and up unasked.
Record AG-06b's block with the others, and set free_from to D-7100 once this run's blocks are recorded.
(b) MATRIX.
- Re-own FM-162 to R4-11, after the cut's cases.json (the matrix entry above).
- Add two new rows owned by AG-06b, ids the next free at append time: (i) a run whose opening append is refused after the bound runs unjournalled, and the restart scan cannot say not_known (test a_run_whose_opening_is_refused_after_the_bound_asks_the_vendor_nothing_and_lands_nothing); (ii) a contended journal append blocks a tokio worker for up to 1-2 s (test a_contended_journal_append_never_stalls_the_http_surface).
- The flock rows are PREC-16's and the index-hold row is R3-27's (see their items).
(c) ASSEMBLER (followups.jq, applied after build-options.jq | oneclick-cut.jq | r4-19d.jq):
- add AG-06b to .extraPackages, then run slim_launcher.sh;
- add AG-06b to .only;
- add AG-06b to .priority right after R3-19;
- set .filesOnly["AG-06b"] = ["IA-10c"], so the start is not blocked and only the merge waits; the package's deps already hold IA-10c, which the launcher needs because a FILES_ONLY entry is filtered out of the hard deps (db-fix-round-2-run2021.js:306-307);
- append R4-02's and R4-13's spec texts, guarded so that a second application adds nothing.
The new edge AG-06b -> IA-10c makes no cycle: IA-10c's deps are IA-9 and IA-16, nothing depends on AG-06b, and IA-10c is not a dropped part. The critical path is not lengthened, because nothing on it waits for AG-06b.
(d) FRESH-RUN READINESS. AG-06b must merge before the Zerodha half of the fresh run (the /pull/run full re-pull with kept answers, mode (i)), because that half writes the pull journal through RunLedger.

### 53. fixb-followups.json [1] AG-06 followups[7] (L-AG06-02: the from-scratch index build holds the append lock for 2.06 to 3.09 s on the 1.08 GB journal, so appends that meet it are refused after 1 s)

already_covered: false

Add one failure-matrix row; the main session gives it the next free FM id. Scenario: a from-scratch pull-journal index build holds the append lock 2.06-3.09 s on the 1.08 GB journal, so appends that meet it are refused after 1 s. With AG-06b, a run whose opening meets it is refused by name, never run unjournalled. Owner R3-27. Test: an_index_build_on_a_large_journal_holds_the_append_lock_only_for_its_catch_up.

```

### 3.8 Rules audit (`state/design/rules-audit-20261011.md`)

`grep -n '^## .*AG-06b'` and `grep -n '^## .*AG-06'` find **no section**. The audit is dated
11 Oct and the part was made on 20 Oct. Its rules still bind, through the fixPrompt's step 5b
(section 3.11): O(1) at p99, parallel, saved and searchable, common and scalable, and extreme
cases.

### 3.9 e2e failure matrix (`state/fix/e2e-failure-matrix.json`)

**No row has `owner` AG-06b**, and the text "AG-06b" appears nowhere in it. The last id is
FM-620, out of 767 rows. The 20 Oct plan said the main session would append two rows owned by
AG-06b, "ids at append time". They are **not appended yet**. The planned rows, verbatim from
main-session.md lines 77-78:

```text
| FM-629 | AG-06b | a pull run whose opening append is refused after the bound still runs unjournalled, and the restart scan cannot say not_known | a_run_whose_opening_is_refused_after_the_bound_asks_the_vendor_nothing_and_lands_nothing | set at append time |
| FM-630 | AG-06b | a contended pull-journal append blocks a tokio worker for up to 1-2 s | a_contended_journal_append_never_stalls_the_http_surface | set at append time |
```

Treat them as this part's matrix rows. Expected: the test passes. Status: open until AG-06b
merges. FM-631 (index-build hold) is R3-27's.

FM-162 is **NOT this package's** (re-owned to R4-11). The spec asks: "If your refusal also proves
part of FM-162's text, say so in notes with the test name, so R4-11's builder can cite it." The
row, verbatim:

```text
{
  "id": "FM-162",
  "stages": [
    "S5",
    "S9"
  ],
  "scenario": "Records are lost under contention for the pull audit journal's append lock. The lock is a try_lock, so a concurrent append is refused at once ('refused rather than interleaved') instead of waiting. A member-failure or closing record from one writer is lost while another writer holds the lock. close_unopened covers only a lost opening record.",
  "how": "/pull/run legs, the autopilot, /pull/repair and `cli zerodha-repair --from-store` all append to one vendor's journal.",
  "expected": "A bounded blocking wait for the append lock, with the wait time logged. A record that still cannot be written is named on /ingest and kept in a sidecar for replay.",
  "test": "concurrent_journal_appenders_lose_no_record: two writers appending 1,000 records each. Assert 2,000 records. It fails today with refusals.",
  "status": "partly",
  "evidence": "CONC-020: crates/api/src/audit.rs:1715-1730 (Flock::try_lock), :1575-1605 (close_unopened)",
  "severity": "medium",
  "owner": "R4-11",
  "sources": [
    "CONC-020"
  ],
  "verdicts": {
    "CONC-020": "partly/medium"
  },
  "origin": "hunters",
  "owner_note": "Re-owned under the one-command cut D-6900 (decided 20 Oct, applied 23 Oct): from E2E-04 (R4-12) to AG-06. Handled at head. AG-06 merged at 5cec3e7cd (20 Oct 13:10 IST); commit edc7870ad says 'pull audit journal appends wait a bounded time for the lock ... FM-162; D-6244'. The sidecar-for-replay half stays partly open. | Kept with R4-11 under D-7135 (23 Oct). Omitted by the first agent. The open half (the run record must open before any stage) was given to R4-11 by the 20 Oct specAppend; the matrix still names AG-06. Every patch run needs it."
}
```

### 3.10 16 Oct cloud spec and critic

- `git show origin/cloud/db-base-20261016:reports/db-specs-20261016/AG-06b.md`: **does not
  exist**. Neither does `AG-06.md`. The part is from 20 Oct.
- The critic `state/design/critic-round7-20261011.md` has **no part for AG-06b or AG-06**, and
  `noCritic` is true.
- The design `state/design/ingest-automation.md` does not name AG-06 or AG-06b. It is not this
  kind's specification.

### 3.11 The exact prompt the Mac builder of AG-06b is given (rendered)

**Public-repo redaction.** This file is pushed to a public repository, so the four
`NOREPLY-ADDRESS` tokens below and in 3.12 stand for the standard Anthropic noreply e-mail address
of the co-author line. Write the real line exactly as the session's commit attribution gives it.

Source: `fixPrompt(p)` (lines 351-373) and the non-`ia` branch of `specStep(p)` (lines 341-349)
of `W/state/fix/db-fix-round-2-run0738.js.fat`. The values are filled in for AG-06b: port 8515,
pool 5, deps IA-10c. `args.niftyFirst` is unset, so the NIFTY FIRST paragraph is included.

Two parts are left out:

- The leading `drainFirst('fix','AG-06b')` paragraph. It is the Mac launcher's hold latch: one
  Bash command that prints OPEN or DRAINED, and on DRAINED the agent returns `held = true`. A cloud
  session has no latch.
- The `${C}` CONTEXT. It is in 3.12.

On the cloud, read every `W/...` path as "the Mac's" and every Mac-only step (shadow server, pool
lock, `W/state/...` writes) as "describe it for the merge-back" (section 6). Step 2's line "All
are merged into feat/db-fix" is the launcher's generic text. For AG-06b it is false: IA-10c is a
FILES_ONLY link and is not merged (section 1).

```text

YOU OWN WORK PACKAGE AG-06b: "Pull runs never start unjournalled, and no tokio worker waits on the journal lock".

NIFTY FIRST (operator): prove your work first on NIFTY spot and NIFTY options before anything else, on every NIFTY selection your package touches (feed GDFL 1s and Zerodha 1min..1day; segment Spot and expired options; every expiry form 2018-09..2020-09, strikes, CE/PE; dates, time filters, sorts, Go to, paging, export, coverage, moneyness/IV/greeks cells; for import packages: one NIFTY spot day, one NIFTY options day, one special session). Fix in the SHARED, instrument-agnostic code path, never as a NIFTY special case, then repeat the check on one BANKNIFTY selection, one F&O stock and one illiquid strike. The NIFTY evidence so far is in /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/gdfl-board/out/nifty.json.

IF /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/P-AG-06b ALREADY EXISTS (an earlier attempt of this same package was cut off by a usage limit; it may hold commits ahead of feat/db-fix, "WIP ... to be squashed" commits and UNCOMMITTED changes), reuse it: read git log feat/db-fix..HEAD and git diff first, judge every hunk critically against your package, keep what is right, fix what is wrong, finish what is missing; never discard it blindly, never git checkout/reset away its changes. Changed files under web/build/ are generated output of an earlier npm run build: reset them (git checkout -- web/build && git clean -fdq web/build) before you judge the diff; never commit web/build unless your package's row owns it.
IF /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/fix-AG-06b.json or /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/verify-AG-06b-r<n>.json EXIST, an earlier run today already built this package (and a second agent may have checked it): read the newest of each first. Every BLOCKING issue in the newest verify report, and every follow-up in it that names your files, is part of your package now; fix those first, each with a test that fails before, then finish whatever the builder's notes list as NOT DONE. If the branch is committed and nothing is open, re-run the gates once, write your report and return quickly.
SAVE AS YOU GO, RESUME, NEVER REDO (operator 21 Oct 2026: "don't repeat the same work twice; always continue or resume after holding it"). A usage limit, an app restart or a hold can end you at any minute, and whatever is only in your head is then lost. (a) FIRST, if /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/fix-AG-06b-progress.md exists, read it whole: an earlier attempt of this package wrote it. Continue from its last NEXT line. What it records as done stands for every commit still on the branch (git log feat/db-fix..HEAD): redo only what it does not record, and re-run a recorded gate only as part of your one final gate run. Shadow-server checks it records from before your last code commit do not stand: take them again on the final build before your last commit (a running server tests every file at once). If git log --first-parent --format=%s feat/db-fix shows "Merge AG-06b:", that earlier attempt was already merged: its entries are history and its NEXT line does not bind you; append an entry "attempt after merge" and do the task below. (b) Keep that file as you go: append (never rewrite) an entry after every finished piece of work, headed by the output of TZ=Asia/Kolkata date '+%F %T IST': what you did and in which files, each gate or test command with its exact result line, each shadow-server check (URL + value), the WIP commit sha, and a NEXT line that says exactly what comes next. (c) Commit on fix/db-AG-06b at least every 20 minutes of editing and always right before any long test, build or gate run: git add <only your files>; git commit -m "WIP AG-06b: <what> (to be squashed)" -m "Co-Authored-By: Claude Opus 5.5 (1M context) <NOREPLY-ADDRESS>". Never hold more than 20 minutes of edits uncommitted. Do NOT squash these commits yourself, whatever the context says about squashing before the final report (21 Oct): the merge folds every WIP commit into one squash commit with the same tree. Your last commit (step 6) is a normal one. (d) A BRANCH ALREADY MERGED BY A FOLD (review fix 21 Oct): before ANY git merge feat/db-fix into this branch (steps 2 and 4), run B=$(git log -1 --format=%H --grep='^AG-06b: squash of fix/db-AG-06b at ' feat/db-fix). If B is set and git merge-base --is-ancestor $B HEAD fails, take T0=$(git log -1 --format=%s $B | grep -oE '[0-9a-f]{40}' | head -1); if git merge-base --is-ancestor $T0 HEAD succeeds, first run git merge -s ours -m "AG-06b: record fold $B of $T0 (same content, no file changed)" $B. It changes no file (that fold holds exactly the tree of $T0, which your branch already has) and makes the later commits of feat/db-fix merge onto your own lines without conflicts. The commits that fold already merged still show in git log feat/db-fix..HEAD: they are merged, not open work.
RULES AUDIT (11 Oct): /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/rules-audit-20261011.md lists, per owner, the gaps ten agents found against the operator's rules (O(1) at p99, parallel, saved and searchable, common and scalable, extreme cases). Find the section for AG-06b (grep -n '^## .*AG-06b' that file): every gap listed there is part of your package, with the test or bench it names. If a gap there cannot be closed inside your files, say so in notes with the owner it needs.
1. Read your package: jq '[.work_packages[] | select((.name | split(" ")[0]) == "AG-06b")][0]' /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/board/fix-plan.json
   Read the full text of every finding it lists (observed, expected, evidence, location, fix_hint) from /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/board/audit-result.json (key "confirmed", match on "id"). Read the plan's summary and all_sources_view in fix-plan.json if your package touches them. D-number assignments are in WP-02's decisions_needed. [THE PACKAGE SPEC OF SECTION 2 OF THIS FILE, IN FULL]
2. Set up: git -C /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX worktree add /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/P-AG-06b -b fix/db-AG-06b feat/db-fix   (if the worktree or the branch already exists, reuse it, then git merge feat/db-fix into it, resolving conflicts by keeping BOTH sides' intent). Then: ln -s /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/APP/web/node_modules /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/P-AG-06b/web/node_modules (skip if present); export CARGO_TARGET_DIR=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/pool-5. [SHARED POOL, OPS-09 18 Oct: hold its lock around every cargo command: mkdir /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/pool-5.lock before, rmdir it after; while it exists wait 30 s and retry; a lock older than 90 minutes is stale: rmdir it. OPS-10 20 Oct (AG-05, R3-24, PIT-1 and PREC-1 checks): cargo names the artifacts of a workspace crate by its path inside the workspace and trusts mtimes, so in a shared pool a newer build from another worktree reads as fresh and runs as yours. Take the lock ONCE for a whole sequence of cargo commands (gates, release build) and rmdir it after the last of them, not after each. Touch the lock directory right before each cargo command of the sequence, and never from a background loop or a timer: a loop that outlives you keeps the lock of a dead holder fresh, and the 90-minute rule never frees it. Right after taking the lock, and again after any git merge, checkout or reset inside the hold, run (cd <the worktree you build> && git ls-files -z | xargs -0 touch) before the first cargo command. A gate result taken without that touch is not counted. A release api or cli you serve or measure is copied under the lock (mkdir -p, then cp) to /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/bin-AG-06b/release/ (the second agent: /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/bin-AG-06b-v/release/) and served from there, never from the pool] Work ONLY in that worktree.
   Dependencies: IA-10c. All are merged into feat/db-fix, so your branch contains them once you merge feat/db-fix.
3. Do the package completely and precisely, the way its specification says, with tests that fail before and pass after. Edit only your package's owned files (new test files for your own modules are fine). If you truly must touch another file, keep it minimal and list it in files_outside_ownership. /db packages (WP-*) do not edit docs/05-decisions.md, docs/04-invariants.md, docs/06-limits.md, docs/11-findings.md or docs/07-plan.md unless they are WP-99: cite the D-numbers WP-02 assigned in code comments instead. Ingest packages (IA-*) land their own rows in those ledgers in the same change (D-4861): re-grep the highest D-48xx number first and never reuse one.
4. Gates in your worktree (all must pass before you commit; merge feat/db-fix into your branch once more just before them): cargo fmt --all --check; cargo clippy -p <each touched crate> --all-targets -- -D warnings; cargo test -p <each touched crate> --locked; if web/ changed: (cd web && npm run build && npm test), then reset the generated web/build files unless you own them. Anything you add to a hot path must stay O(1) per operation; measure p50, p99 and max for every route or operation you touch (n >= 200 sequential GETs against your shadow server, or the crate's bench) and report them with n and the method. Anything that cannot be O(1) is refused or bounded, never silently slow, and is named for docs/06-limits.md in your notes.
5. Check against REAL data, never on :8080: build in release (cargo build --release -p api -p cli), then
   /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/stop.sh AG-06b; /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/shadow.sh AG-06b && /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/serve.sh AG-06b 8515 /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/bin-AG-06b [SHARED POOL, OPS-09 18 Oct: hold its lock around every cargo command: mkdir /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/pool-5.lock before, rmdir it after; while it exists wait 30 s and retry; a lock older than 90 minutes is stale: rmdir it. OPS-10 20 Oct (AG-05, R3-24, PIT-1 and PREC-1 checks): cargo names the artifacts of a workspace crate by its path inside the workspace and trusts mtimes, so in a shared pool a newer build from another worktree reads as fresh and runs as yours. Take the lock ONCE for a whole sequence of cargo commands (gates, release build) and rmdir it after the last of them, not after each. Touch the lock directory right before each cargo command of the sequence, and never from a background loop or a timer: a loop that outlives you keeps the lock of a dead holder fresh, and the 90-minute rule never frees it. Right after taking the lock, and again after any git merge, checkout or reset inside the hold, run (cd <the worktree you build> && git ls-files -z | xargs -0 touch) before the first cargo command. A gate result taken without that touch is not counted. A release api or cli you serve or measure is copied under the lock (mkdir -p, then cp) to /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/bin-AG-06b/release/ (the second agent: /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/bin-AG-06b-v/release/) and served from there, never from the pool] /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/P-AG-06b
   This serves your build at http://127.0.0.1:8515 over a private shadow of the real store (bar files read through a link, everything else cloned). Confirm YOUR server is the listener: lsof -nP -iTCP:8515 -sTCP:LISTEN -t must print the pid in /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/shadow/AG-06b/test-server.pid. GET requests only against a shadow whose bars are the real store's link: never POST there, never run an import, pull, repair or backfill against it. For code that WRITES (importer, repair, sidecar or index writer, migration, verdict or alert store): exercise the write path only on a store root you own entirely - a temp dir built from synthetic fixtures, or a shadow in which you REPLACED the bars link by a real directory holding copies of the few months you need (cp -c) - and prove it never touched /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store. Reproduce each finding's selection and confirm the right answer (record URL + value as evidence a second agent can repeat). If the page is web-only, curl the API and run the web unit tests; you may open http://127.0.0.1:8515/db in the browser pane only if no other agent is using it. Stop your server with /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/stop.sh AG-06b when done.
5b. SELF-CHECK BEFORE YOU COMMIT (19 Oct speed rule: a failed second check costs a whole repair round plus a re-check): do the second agent's checks yourself first. (i) Read your own full diff against feat/db-fix and judge every hunk against repo law (O(1) per operation: name every loop over months, files, records or instruments that a request or a per-row step can reach; append-only store; no look-ahead; no fallback that hides a failure; prices in paisa i64; Rust only outside web/; no test that asserts nothing; no real vendor row in a fixture) and the operator's rules (one common runtime-dynamic code path, incremental and scalable, every state change saved, audited, logged and searchable; every failure, refusal and extreme value tracked so it can be found, debugged, monitored and seen where the operator looks (/logs, /db or the part's own page) through a read that stays O(1) at p99). (ii) Add tests for at least 8 extreme cases that fit your change, at least 2 of them combinations of extremes at once and at least 1 out-of-the-box case no list names (empty, one-record and maximum inputs; month, year and expiry boundaries; special sessions; renamed symbols; duplicate, out-of-order and late rows; torn or zero-length files; a missing month mid-range; garbage parameters; concurrent requests; a kill mid-write; two writers). (iii) Re-run every finding you call fixed on your shadow server and record URL + value. Fix whatever this finds before you commit, and list in notes what you checked.
6. Commit on fix/db-AG-06b (git add the files you changed; never git add -A at the root, never commit vendor data, binaries, shadow or target dirs). Message: what was fixed and why, finding ids, D-numbers, ending with the line "Co-Authored-By: Claude Opus 5.5 (1M context) <NOREPLY-ADDRESS>". Never push. Never touch :8080, wt/APP, tgt/app*, the real store, or another package's worktree.
7. If something needs a store write on the real store (re-import, recount, migration, re-pull, backfill), implement and test the code path on fixtures and list the exact command under store_writes_needed; do not run it on the real store.
8. Write your structured report also as JSON to /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/fix-AG-06b.json (mkdir -p /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2).
Return the structured report. Be honest: a finding is "fixed" only if a test or a shadow-server check proves it; say NOT PROVED where you could not check.
```

### 3.12 CONTEXT (`EMBED.context`, verbatim)

Source: line 10 of the launcher (`EMBED.context`), also `W/state/cloud-20261024/embed.json`
`.context`. It is 14,914 bytes (`jq -j`, before the redaction). Every step agent reads it. Where its later paragraphs disagree
with its earlier ones, the later ones hold.

```text
CONTEXT (brutex, 7 Oct 2026; RESUMED 9 Oct 2026 on a new Claude account after the weekly usage limit stopped round 1 mid-run). The operator wants /db — and everything it depends on — entirely correct: every selection on Zerodha and GDFL (indices, stocks, expired options with moneyness, intrinsic, extrinsic, IV, greeks, OI) shows exactly the right data; everything searchable, findable, auditable, logged and visualised; O(1) per operation with measured p50/p99; every extreme case covered. A read-only audit confirmed 185 defects (each verified twice); the fix plan splits them into work packages with non-overlapping files.

FILES: plan = /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/board/fix-plan.json ; findings with full evidence = .../state/board/audit-result.json (key "confirmed"). Integration worktree /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX on branch feat/db-fix (from feat/app-all plus the Zerodha minute-calendar fix). The operator's real app (:8080) serves wt/APP (feat/app-all) — never touch it; it is redeployed by the main session after merges.

REPO LAW (CLAUDE.md, binding): Rust only outside web/ (web/ is unrestricted, but no crate may need Node to build/test); no invention — vendor/exchange claims need a source or UNVERIFIED; run identity includes feed; O(1) per operation; append-only store (new versions/generations, never in-place mutation); no look-ahead; fallbacks never hide a failure (degrade loudly with the reason, or refuse); no test that asserts nothing; prices are paisa i64; decisions live in docs/05-decisions.md (WP-02 writes D-3316..D-3343 up front; the Zerodha calendar fix uses D-3344+). Never copy raw vendor rows into the repository; fixtures are synthetic.

OPERATOR ANSWERS (defaults the operator can still override; WP-02 records them):
1. The GDFL options import was STOPPED by the operator at 23:23 IST on 6 Oct, mid-day 2020-09-16. Bars after 2019-11-13 are on disk but uncounted; they get counted by the census v4 recount (no vendor reread), then the import resumes from 2020-09-16.
2. Store writes and API rebuild/restart are approved in principle ("fix and resolve everything"); they run in the main session after the code lands (WP-80), each as a new month/generation, never in place. Agents never write the real store.
3. GDFL closing-auction prints (from 2026-08-03): keep them, as a labelled 'auction' field/row kind — never silently dropped, never mixed into ordinary trades.
4. Spot for an option bar: exact second stays the default (D-2807); add an opt-in 'last spot at or before t' that always shows the spot's age. Never a later spot.
5. Yes: import the GDFL index files (FINNIFTY, MIDCPNIFTY, NIFTYNXT50, NIFTYIT, NIFTYCPSE, NIFTYFPI, INDIA VIX) and the old-name stock files (TATAMOTORS, HDFC, ZOMATO and the others) — stored, not swept — so every option underlying gets spot and greeks.
6. Zerodha 2015–2017 index 1day bars: the operator's window is 2018-01-01..2026-09-30. Hide them by default; a visible toggle shows them labelled 'outside your window'. Never delete.
7. Dhan, Groww and TrueData: removed from every picker (the operator uses only Zerodha and GDFL).
8. NSE circulars for the 2020-03-13 / 2020-03-23 halts, renames and index aliases: WP-02 may look them up (WebSearch/WebFetch on nseindia.com / nsearchives); record the circular number and URL if found, else keep UNVERIFIED and show 'suspected'.
9. 30 May 2019 LTQ=1 marker rows and LTQ=0 price rows: flag only; bars unchanged.
10. OI of a 1s bar: the last row of the second (bar-close semantics), applied to new imports; existing months are re-imported as new generations by WP-80.
11. /db opens on NIFTY · Spot · the feed's finest timeframe · newest first.

HARD RULES: never write the real store /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store; never POST to any server; never start, stop or restart :8080; never kill a process you did not start; never push; never rm -rf outside your own worktree, target dir or shadow dir. Commands that bind ports or write outside the sandbox may need dangerouslyDisableSandbox. The machine is shared by ~12 agents: keep builds to your own CARGO_TARGET_DIR and avoid needless rebuilds.


RESUME FACTS (9 Oct): WP-02 is committed and merged into feat/db-fix (b875f22e), so D-3316..D-3343 exist. WP-70 is done (outside the repo). Every other package's first agent was cut off by the usage limit; six of them (WP-00, WP-01, WP-10, WP-11a, WP-12b, WP-14) left UNCOMMITTED work in their worktrees, backed up as patches in /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/resume-kit/wip-20261009/. The Mac rebooted on 8 Oct; :8080 was restarted by the main session on the same build (tgt/app5) and real store. BUILDS: export PATH=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/bin:$PATH before any cargo command, so builds go through the machine-wide queue (8 at once, 4 jobs each; queued waiting is normal, not a hang). Start long cargo commands in the background writing a log that ends with a line EXIT <code>, then wait with one blocking `until grep -q '^EXIT ' <log>; do sleep 20; done` call (600000 ms timeout, repeated). SAFETY: never rm -rf "$HOME", "$TMPDIR", ~ or any unquoted variable or glob; use literal absolute paths inside your own worktree, target or shadow dir. Never commit or copy a real GDFL data row (Ticker,DD/MM/YYYY,HH:MM:SS,price...) into the repository: fixtures are invented, with prices that cannot be mistaken for real ones.

ROUND 1c (10 Oct, new account): 17 packages are merged (feat/db-fix 3cd0623c). The 18 defects found on 9 Oct are now findings DB-N01..DB-N18 in audit-result.json, added to WP-50, WP-52, WP-53, WP-56 and a new package WP-39 (bars sort index: the blocking full-history 1s sort). Co-author line for commits: 'Co-Authored-By: Claude Opus 5.5 (1M context) <NOREPLY-ADDRESS>'.

ROUND 2 (11 Oct 2026, a new account; where this paragraph and the text above disagree, this paragraph is right). 33 packages are merged on local feat/db-fix (WP-00 WP-01 WP-02 WP-10 WP-11a WP-11b WP-12a WP-12b WP-12c WP-13a WP-13b WP-13c WP-14 WP-15 WP-20 WP-30 WP-31 WP-32 WP-33a WP-33b WP-33c WP-34 WP-35 WP-36 WP-37 WP-38a WP-38b WP-50 WP-51 WP-52 WP-53 WP-54 WP-55); read git -C /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX log --oneline --grep='^Merge ' feat/db-fix for the true state. The confirmed findings are now 343 (DB-*, DB-N01..N26, DB-A01..A53, DB-B01..B79). The operator's live app :8080 runs a frozen copy of a build (tgt/app<N>) over the real store and is redeployed by the main session after merges, never by you. This run also builds the INGEST AUTOMATION packages (IA-*): the operator's core aim is that the app itself imports GDFL from /ingest, fast and self-checking, with no Claude session in the loop; their specification is state/design/ingest-automation.md.

OPERATOR'S STANDING RULES (11 Oct, binding on every package): (1) O(1) per operation at p99 everywhere, MEASURED: report p50, p99 and max with n and the method; anything that cannot be O(1) is bounded or refused out loud and named for docs/06-limits.md. (2) One common, runtime-dynamic, incremental, scalable code path: no instrument or vendor special case, no hard-coded list the data already knows, no work redone that an index or a sidecar can remember. (3) Every state change is saved, audited, logged, searchable and visible on a page; nothing fails silently. (4) Every extreme case is covered by a test that fails before the fix: empty, single, maximum, duplicate, out-of-order, late, torn, truncated, missing-in-the-middle, renamed, special session, concurrent, killed mid-write, disk full, garbage parameters. (5) No claim without evidence a second agent can repeat; say NOT PROVED otherwise. A SECOND AGENT re-runs your gates, reproduces every 'fixed' finding on a fresh shadow server and attacks your change before it is merged.

PORTS AND SANDBOX (11 Oct): test servers left by earlier runs still listen on 8100-8142, 8231, 8347 and 8372; never trust an answer from a port you did not just start, and never stop a server that is not your own package's. Inside the Bash sandbox a direct connection to 127.0.0.1 is refused ('Operation not permitted'). Reach your own shadow port either through the sandbox proxy - curl --noproxy '' --proxy "$HTTP_PROXY" http://127.0.0.1:<port>/... with 127.0.0.1:<port> listed in that command's allowed_domains - or by running the command with dangerouslyDisableSandbox. Starting a server (binding a port) needs dangerouslyDisableSandbox. Never change permission settings; never ask the operator to approve anything: if a command is refused, find another way that stays inside these rules, or report it in notes. Commit co-author line: 'Co-Authored-By: Claude Opus 5.5 (1M context) <NOREPLY-ADDRESS>'.

USAGE BUDGET (11 Oct): the account's usage window is small and an agent cut off by it loses everything it has read. Work economically: grep before you read; read by line range, never whole large files; never print a whole log (tail -40, or grep for the failures); do not re-read what you have read; run each gate once after your last edit, not after every edit; keep throwaway exploration out of your context. If an earlier attempt of your package left commits and uncommitted changes in the worktree, they hold up to 30 minutes of an interrupted agent's work from today: start from them. Commit your work in progress on your own branch at least every 15 minutes (message 'WIP <package>: <what>'; squash before your final report), so a cut-off loses little. Aim to have the package committed within about 45 minutes; if a part cannot be finished well in that time, commit what is complete and proved, and list the rest exactly under notes as NOT DONE.

KNOWN SLOW TEST (11 Oct, measured by a second agent): cli's pool::tests::the_pool_verb_prints_its_whole_page_on_a_generated_store ran for more than 95 minutes on this loaded machine and held up every package that runs the cli suite. Unless your package changes crates/cli/src/pool* or what that test reads, run the cli suite as: cargo test -p cli --locked -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store , and say in your gates line that this one test was skipped as the known slow one. The final Gate run on feat/db-fix runs it. Never wait more than 25 minutes on one test run without looking at which test is still running (the log's last 'test ... has been running for over 60 seconds' lines name it).

NO DELETIONS THAT NEED APPROVAL (12 Oct, learned the hard way): a command with rm -rf, or any deletion run with dangerouslyDisableSandbox, raises an approval card that nobody answers, and the agent that sent it hangs for hours. NEVER run rm -rf. Do not delete browser profiles, scratch worktrees' build output, target directories or shadow stores at the end: leave them where they are and list their paths in your notes; the main session cleans up. This holds inside the sandbox too: find -delete, git worktree remove, git clean and rm of a directory all raised cards. Delete nothing at all; if you need a clean directory, make a NEW one with a new name. Write your report file BEFORE any tidying. The helper scripts state/fix/shadow.sh, serve.sh and stop.sh delete nothing (shadow.sh moves an older shadow of the same name aside): use them as written, and always make a FRESH shadow for a check; never reuse one another agent left. If any command has not returned after 20 minutes with no output, treat it as stuck, do not repeat it, and finish your report without it.

RELAUNCH 13 OCT 2026 (a new session; where this paragraph and the text above disagree, this paragraph is right). 50 packages are merged on feat/db-fix (head 29dc84e6 at launch); git -C /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/DBFIX log --oneline --grep='^Merge ' feat/db-fix is the truth. :8080 runs a frozen build tgt/app18 and is redeployed by the main session only. WP-62 and WP-12f failed their third check (round2/verify-<pkg>-r3.json): their builders repair first. The compressed-store design (state/design/store-compression-20261012.md, section 9) names packages that collide; those are now dependencies (IA-1 after R3-35; R3-06 after SC-0 and SC-2; SC-3 after R3-08; SC-5 after R3-07 and R3-08; R3-22 after SC-4) and each affected package's spec says what to do. DISK (operator's rule): before any step that writes many files (a backfill, an import, a sweep, an index build), project its disk use and file count from a measured sample and write the projection in your report; no import of stock options happens until the compressed form exists. The operator restated the standing rules on 13 Oct, word for word in spirit: O(1) at p99 everywhere, measured; everything in parallel; every state change saved, audited, logged, tracked, searchable, monitored and shown on a page as plain comparison tables; one 100% common, runtime-dynamic, incremental, scalable path; and every extreme permutation and combination of exceptions, errors, situations and conditions covered by a test, including the out-of-the-box ones nobody listed.

ROUND 3 ADDENDUM (14 Oct 2026). The operator's MAIN AIM: one press of Ingest (and a daily timer) runs the whole chain end to end with no human or Claude step: GDFL Dropbox download, verify, compress into the tick store, ticks to 1-second bars; Zerodha daily + 1-minute + the folded 2-60 minute candles; every check; the pages. Everything compressed with O(1) per lookup at p99; a fresh clone plus only the Dropbox link and the daily Kite sign-in rebuilds code and data. Packages R4-01..R4-29 come from /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round3-packages.json (read its notes for SHARED LINES, SEQUENCED EDITS and split items). FAILURE MATRIX /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/e2e-failure-matrix.json: every row whose owner is your package, one of your item ids, or (for round-2 packages) your package id is part of your package: make its test exist and pass, or report it NOT DONE with the reason. AMENDMENTS: A1 IA-12a depends on R4-01 and adds its panel as web/src/routes/ingest/panels/GdflImport.svelte with no +page.svelte edit. A2 SC-8 depends on R4-01 and R4-04, puts its band at web/src/routes/ingest/panels/Storage.svelte and its /db badge at web/src/routes/db/badges/Storage.svelte, edits neither +page.svelte, and uses server.rs for route lines only. A3 IA-6 and IA-16 add their codes' rows to crates/pull/src/reasons.rs if R4-05 merged first. A4 SC-2's sealed summary section carries R3-06's month summary bytes with their version; no R4 package edits sealed.rs. A5 R3-32 ships no tracked config/launchd/*.plist (CLAUDE.md section 2, gate 1). A6 decision numbers are taken at landing from the ledger's head (D-4928 is already used; the head is past D-4950). Never delete any file; never write the real store.
```

---

## 4. Files

### 4.1 Owned (a region of a file, not whole files)

| File | What is owned | At e851da1ca |
|---|---|---|
| `crates/api/src/server.rs` | `struct RunLedger` and `impl RunLedger` (4148-~4400) | lines 4148-4357 (struct 4148-4173, impl 4175-4357) |
| same | `BrokerRun::opened(...)` constructor | 4619-4640 |
| same | the journal calls in `broker_run_by`: `BrokerRun::opened(&journal, ...)` at 6158, `out.ledger.flush(...)` at 6285 and 6315 | fn at 6074 |
| same | the journal calls in `fno_walk` (fn at 11409): `fno_opened(journal, asked, now)` at 11427 (it calls `journal.append(&head)` at 11395), and the outcome record through `FnoPage::say_counted`, which calls `recorded_fact(self.journal, &record)` at 10468 | fno_opened 11379-11407 |
| same | every other async caller of RunLedger (name them in notes). At e851da1ca: `broker_answer` (async, 3532), which calls `close_reaching_none` (3727-3747, `run.ledger.close`) and `landed_answer(..., Some(&mut run.ledger))` (3709; `landed_answer` 9337, `ledger.close`/`rest`/`fact` at 9421-9424); and `recovery_spot` (async, 6442: `journal.append` 6472, `run.ledger.close` 6486, `run.ledger.rest` 6492). Outside server.rs: `autopilot.rs` 3988-4000 (`run.ledger.close`, `rest`, reads `opened_refused`). That reader stays unchanged | |
| `crates/api/src/audit_lock_wait_measure.rs` | whole file (AG-06's, merged) | exists, 99 lines |
| `crates/api/src/audit_lock_wait_tests.rs` | whole file (AG-06's, merged) | exists, 722 lines |
| `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md` | append-only: new AG-06b rows and entries only, from block D-7100..D-7109 | |

A new test file of AG-06b's own beside these is fine (fixPrompt step 3: "new test files for your
own modules are fine"). If it is a `#[path]` module of `audit.rs` the way the two above are
(`audit.rs:3336-3341`), the one mod line in `audit.rs` is a file outside ownership. List it in
`files_outside_ownership`.

### 4.2 Must not touch

| File / region | Owner | Why |
|---|---|---|
| `crates/store/src/flock.rs` | PREC-16 | owns the file; all store::flock items |
| `crates/api/src/audit.rs` `build_index` (3117) and `index_locked` (3155) | R3-27 | the index-build hold, L-AG06-02 |
| `crates/api/src/pullrun.rs` (whole file) | R3-27 | call `restart_scan` (2042) only. IA-10c adds 199 lines to it (`open_landing`, `LandingRun`, ...): keep them as IA-10c merges them |
| `crates/api/src/autopilot.rs` | one-command cut | "keeps autopilot.rs exactly as merged". Line 4000 reads `run.ledger.opened_refused` and must keep compiling unchanged. IA-10c adds its 2 lines (`run: 0,`); "as merged" includes them |
| `crates/api/src/emitted.rs` | not AG-06b's | pins the LIB target's emit-site count at 96 (7.1). Touch it only if a new `telemetry::emit` site is added, and then list it in `files_outside_ownership` |
| the rest of `crates/api/src/server.rs` | R4-02 | told that the region above is AG-06b's |
| the `/pull/*` handlers' run-id seat | IA-10c | merges first; rebase onto it and keep its seat |
| `broker_run_by`'s SHAPE_CHANGED stop | R4-13 | separate hunk, after R4-02; whoever merges second keeps both sides |
| docs/06 rows L-AG06-01..11, docs/04 rows AG06-01..12, D-6244 | AG-06 (merged) | never edit; add new rows |
| `:8080`, `wt/APP`, `tgt/app*`, the real store | operator | never |

---

## 5. Decision numbers

**D-7100..D-7109**, owner "AG-06b (20 Oct follow-ups, fix-B area)". Sources:
`state/decision-blocks.json` `.blocks[51]` and the package's `decisionBlock` field.

- Take ids in order, D-7100 first. Ids left unused stay unused.
- At e851da1ca, `docs/05-decisions.md` has **no D-7100..D-7109** entry. Its highest headings are
  D-7136, D-7137 and D-7138. Re-grep feat/db-fix and every `fix/*` and `origin/*` branch before
  committing. (Checker, 24 Oct: `docs/05-decisions.md` on all 580 refs under `refs/heads/fix` and
  `refs/remotes/origin` in `W/wt/DBFIX` holds no `### D-710x` heading.)
- Never use D-6920..D-6999 (reserved). The registry's `free_from` is D-7205.
- At least two entries are expected (spec item AG06B-4): (1) the landed opening record is a
  precondition of a run; (2) the journal calls on the pull paths run off the workers. A third may
  record the harness close (AG06B-3) if it is a locked choice.

---

## 6. NEEDS THE MAC (step 5 of the fixPrompt, run at merge-back)

The cloud cannot reach the real store, the shadow servers or the shared build pools. On the Mac,
the builder or the merge-back runs the commands below after the cloud branch is fetched into
`W/wt/P-AG-06b` (branch `fix/db-AG-06b`). They are GET-only against a shadow whose bars link to
the real store. Every write path runs on a clone or a temp root. Nothing here touches
`/Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store`, `:8080` or `wt/APP`.

`store_writes_needed`: **none** (spec).

```sh
W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925
# 0. Gates (pool 5, lock held once for the whole sequence, OPS-09/OPS-10)
mkdir $W/tgt/pool-5.lock            # wait 30 s and retry while it exists; older than 90 min = stale
cd $W/wt/P-AG-06b && git ls-files -z | xargs -0 touch
export CARGO_TARGET_DIR=$W/tgt/pool-5
cargo fmt --all --check
cargo clippy -p api --all-targets -- -D warnings
cargo test -p api --locked
cargo test -p store --test cited_commits --locked     # docs/04, docs/06 touched
cargo build --release -p api -p cli
mkdir -p $W/tgt/bin-AG-06b/release && cp $CARGO_TARGET_DIR/release/api $CARGO_TARGET_DIR/release/cli $W/tgt/bin-AG-06b/release/
rmdir $W/tgt/pool-5.lock

# 1. Shadow server on 8515 (GET only)
$W/state/fix/stop.sh AG-06b; $W/state/fix/shadow.sh AG-06b && \
  $W/state/fix/serve.sh AG-06b 8515 $W/tgt/bin-AG-06b $W/wt/P-AG-06b
lsof -nP -iTCP:8515 -sTCP:LISTEN -t      # must equal: cat $W/shadow/AG-06b/test-server.pid
```

Checks and the expected answers:

1. **Real journal unchanged.** Before and after every step:
   `stat -f '%z %m' /Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store/audit/pull.journal`.
   Size and mtime must not change. On 20 Oct the size was 1,079,608,832 bytes with mtime Oct 13
   22:47:50. Take the current value as the baseline.
2. **Restart state on the cloned journal.** Run
   `curl -s http://127.0.0.1:8515/pull/run.json | jq '.restart'`. It must still answer as it did
   before the change. On AG-06's shadow it said `clean` for the cloned journal. It must not say
   `not_known` because of anything AG-06b added.
3. **GET /feeds.json under a held append lock, before and after (spec "O(1) AND MEASUREMENTS").**
   - n >= 200 sequential GETs of `/feeds.json`, p50/p99/max, with no holder.
   - Repeat while a holder keeps the shadow journal's append lock
     (`$W/shadow/AG-06b/audit/pull.journal`) and a run is opening. Hold the lock for 900 ms per
     opening: on the shadow root only, use AG-06b's own hold helper, or a probe test binary pointed
     at that root.
   - Do this on the feat/db-fix build ("before") and on the AG-06b build ("after").
   - Expected after: every GET answers within 100 ms (the spec's test bound). Before: GETs stall
     for up to 1-2 s on a 1-worker runtime. (Checker, 24 Oct: the served `api` builds its runtime
     with `Builder::new_multi_thread()` and no `worker_threads` (`crates/api/src/main.rs:39`), so
     it has one worker per core, and on the shadow server the "before" stall may not show at all.
     That is why the spec's failing-first test uses `worker_threads(1)`. Report what was
     measured; do not claim a stall the shadow did not show.)
   - Report n, the method, and p50/p99/max for both.
4. **Uncontended journal ops, before and after (ratio gate).** Clone the shadow audit dir:
   `cp -cR $W/shadow/AG-06b/audit $W/shadow/AG-06b-measure/audit`. Then run:
   `BRUTEX_AG06_MEASURE_ROOT=$W/shadow/AG-06b-measure <api lib test binary> audit::lock_wait_measure --ignored --nocapture`
   for open_run, append_member and close_run, with n >= 200. Compare with AG-06's figures in 3.3
   (1 writer after: open_run 3.5/7.7/2057.9 ms, append_member 1.8/3.1/3.2, close_run 3.5/6.0/6.0;
   the ~2 s max is the first-append index build on a fresh clone).
5. **AG06B-3 on the clone.** After step 4's harness run on the clone, the restart scan of that
   clone must find **no open harness run**. Check it with
   `/pull/run.json` on a server over that clone, or with the `the_measure_harness_closes_every_run_it_opens` test.
   Before the fix: 221 harness runs read as unclosed on AG-06's shadow.
6. **A refused opening refuses the run (AG06B-1). Write path: temp root only.** On a store root you
   own entirely (a temp dir from synthetic fixtures, as the failing-first test does), hold the
   append lock past `APPEND_LOCK_WAIT` and start a run.
   - Expected: refusal word `journal_unavailable`, naming the bound (1000 ms), the wait, the
     attempts, the record kind and the holder in `store::flock::holder_note`'s words.
   - The fixture broker counts 0 requests, and the store is byte-identical afterwards.
   - One `api.audit` Error event ("record not appended") and one `pull.run` refused event are
     written.
   - `/pull/run.json` and the receipt carry the refusal.
   - Never POST /pull/* against the bars-linked shadow. A /pull/run on a shadow needs a shadow
     whose bars link was REPLACED by copies (`cp -c`) of the months it needs.
7. **The refusal shows where the operator looks.** On that owned root, the `journal_unavailable`
   refusal is visible on `/logs.json` (the `pull.run` refused event) and on `/pull/run.json`.
   Record the URL and value of each.
8. **NIFTY first** (fixPrompt): steps 6 and 7 first for a NIFTY spot request (Zerodha 1day,
   one month), then one BANKNIFTY request, one F&O stock and one F&O walk (fno_walk). The same
   refusal must hold for each, from one shared code path.
9. Stop: `$W/state/fix/stop.sh AG-06b`. Write `$W/state/fix/round2/fix-AG-06b.json` and
   `fix-AG-06b-progress.md`.

---

## 7. Code facts at e851da1ca

From `git show e851da1ca:<file>` with grep. Spec line numbers were at 453de6989. Most still match.

**`crates/api/src/server.rs`** (36,278 lines, exists)

- `pub(crate) struct RunLedger` at **4148**. Fields:
  - `thread: Option<audit::Thread>`
  - `pub opened_refused: Option<String>` (4152)
  - `flushed`
  - `pub written`
  - `pub refused: Option<String>`
  - `window`
  - `source`
  - `head: Option<audit::Record>`

  The type doc above it (to 4146) says a journal that refuses the opening record leads to "nothing
  is appended during the run ... the receipt says the opening record was refused". That is the
  behaviour AG06B-1 replaces.
- `impl RunLedger` at **4175**:
  - `fn open(journal, asked, of, driver) -> Self` at 4184. `match journal.open_run(&head)` at
    **4205-4208**: `Ok(thread) => ledger.thread = Some(thread)`,
    `Err(why) => ledger.opened_refused = Some(why)`.
  - `source()` 4216
  - `close()` 4232. Its arms: `close_run` 4238, `close_unopened` 4239, `append` 4240.
  - `flush()` 4262. It calls `journal.append_failure` at 4273. Its doc still says "The journal's
    lock is taken without waiting". That has been stale since AG-06; fix the doc if you touch it.
  - `opened()` 4281, `const fn`, which is `self.thread.is_some()`.
  - `rest()` 4291. It calls `journal.append_member` at 4305.
  - `write()` 4311
  - `fact()` 4337. It puts `opened_refused` into the receipt line at 4343.
- `fn run_source` 4375 and `fn spot_ask` 4398. They follow the impl and are not in the owned
  region.
- `impl BrokerRun` at 4519. `fn opened(journal, asked, of, withheld, driver) -> Self` at
  **4619-4640**, which calls `RunLedger::open(journal, asked, of, driver)` at 4629.
- `pub(crate) struct BrokerRun` at 3771. It has `pub blocked: Option<Blocked>` (3801), the "never
  opened a socket" refusal. `fn blocked(why, code)` is at 4848 and `fn out_of_order(why)` at 4891
  (409). `struct Blocked { why, code }` is at 4913. `autopilot.rs` and `recovery_spot` already
  branch on `run.blocked`.
- `fn note_run_started(asked, instruments) -> Option<u64>` at **4930**. Its `pull.run` "started"
  Info event is at 4962, and it claims the telemetry sink's run key. `fn note_run_finished(out,
  balanced, claimed)` at 4993 emits `pull.run` "finished" (5001) and is the only place that
  releases that key (`sink.release_run(run)` at 5065). In `broker_run_by`,
  `let claimed_run = note_run_started(asked, targets.len());` is at **6154**, before the journal
  opening at 6158, and `note_run_finished(&out, ..., claimed_run)` is at 6317. No `pull.run`
  "refused" event exists at head. (Checker, 24 Oct: see 7.1.)
- `fn off_the_workers<T>(work: impl FnOnce() -> T) -> T` at **5815** (doc 5807-5814). It uses
  `block_in_place` on MultiThread and runs `work()` inline otherwise.
- `pub(crate) async fn broker_run_by(asked, site, censuses, driver) -> BrokerRun` at **6074**:
  - `let journal = site.journal();` 6157
  - `let mut out = BrokerRun::opened(&journal, asked, targets.len(), withheld, driver);` 6158.
    The source-shape test at 21390 pins this exact text.
  - `out.ledger.flush(&journal, &out.total.failures);` at 6285 (per instrument) and 6315 (end)
- Callers of `broker_run_by`: `broker_run` (6354, Hand), `recovery_spot` (6449, Recovery) and
  `autopilot.rs:3919` (Autopilot).
- `fn fno_opened` 11379-11407. It is a plain `journal.append(&head)` of an `Outcome::Opened`
  record. It is **not** a RunLedger, and on refusal "The walk still runs". `async fn fno_walk` is
  at **11409** and calls `fno_opened` at 11427. It is called from `async fn fno_answer` (10337) at
  10365, which `pull_fno` (14141, the `/pull/fno` handler, route 16190) calls at 14251.
- Lines 19411-19420 are the doc of the stop's bounded wait: "that drop *waits forever* for every
  `spawn_blocking` task". This is why no spawn_blocking is added. Existing spawn_blocking sites:
  20072, 25118, 25479, 26328, 28618.
- Source-shape test pattern: `the_sweep_withholds_per_instrument_probes_the_store_and_says_the_horizon_once`,
  with `.split_once("pub(crate) async fn broker_run_by(")` at **21372**.
- The test the spec says must stay green:
  `a_run_whose_opening_record_was_refused_closes_behind_it_written_late` at 34202 (test fn line). It holds
  the lock with `held.try_lock()`, then calls `RunLedger::open`, and asserts
  `!ledger.opened() && ledger.opened_refused.is_some()` at 34224-34227. It opens with the lock held
  and releases it only after `open` returns, so under AG-06 the open waited 1 s and was refused.
- Routes: `/feeds.json` 16056, `/pull/run.json` 16215 (`pull_run_json` 9902), `/audit.json` 16264,
  `/logs` and `/logs.json` 16274-16275.

**`crates/api/src/audit.rs`** (5,417 lines; read only, except a possible mod line)

- `Record::refused` 755. It builds `kind: Kind::Run` with `from_days: 0, to_days: 0`, so it carries
  no window unless `.with_window(...)` (788) is called.
- `Record::of_run(scope, at, elapsed_micros, source, window, done)` **857**.
- `Record::member_failure` 932, `Record::opened` 992.
- `pub const APPEND_LOCK_WAIT: Duration = from_secs(1)` **1414**. There is a compile-time check
  against `store::flock::MAX_PATIENCE` at 1418-1421. `APPEND_DOING` is 1425.
- `Journal::append` 1500, `append_member` 1542, `open_run(&self, head) -> Result<Thread, String>`
  1560, `append_failure` 1591, `close_run` 1620, `close_unopened` 1643.
- `appended_within` 1753, with `Flock::try_lock_patiently(..., APPEND_LOCK_WAIT, APPEND_DOING)` at
  1795. The refusal text is at 1808: "... cannot take the journal append lock for a {} record ({})
  within its {} ms bound (APPEND_LOCK_WAIT): ...". The notes lock waits the same way (1888-1903).
- `fn note_append_failed` 2118. It emits `telemetry::Event::error("api.audit", "record not
  appended")` with path, kind, scope, outcome, source, at_unix_secs, bars_stored and why. This is
  the "existing api.audit Error event".
- `build_index` 3117 and `index_locked` 3155: not yours.
- `#[path = "audit_lock_wait_tests.rs"] mod lock_wait_tests;` at 3336-3337 and
  `#[path = "audit_lock_wait_measure.rs"] mod lock_wait_measure;` at 3340-3341. Both files are
  child modules of `api::audit` (`use super::*`). Line 2054 calls
  `lock_wait_tests::take_write_refusal()`, a test hook.

**`crates/api/src/audit_lock_wait_measure.rs`** (99 lines, exists)

- One `#[ignore]` test, `measure_append_latency_under_contention` (31-33). It reads env
  `BRUTEX_AG06_MEASURE_ROOT`, and runs 1, 2 and 8 writer threads behind a Barrier with 200/writers
  iterations.
- Per iteration:
  - `Record::opened(Scope::Spot, at, &src, window, 1, 0xa5)` (52) through `journal.open_run`
    (54, timed)
  - `append_member` (59)
  - the close, built at **61** as `Record::refused(Scope::Spot, Outcome::Stored, at, &src, "done")`
    with **no `.with_window`**, through `close_run`, or `close_unopened` on a refused open (63-66)
- **Why the scan does not pair it:**
  - `pullrun.rs::is_close` (1520-1523) is `kind == Run && outcome not in {Opened, Refused}`. That is
    TRUE for this record.
  - `same_key` (1526-1528) compares scope, `window_of` and source. `window_of` is `None` for a
    record whose from_days and to_days are 0 (1514-1516), and the opening has the window, so the
    keys differ.
  - The fix the spec names: `Record::of_run(Scope::Spot, at, micros, &src, window, &Ingested::default())`
    with the opening's source and window, as `recovery_spot` does at server.rs 6478-6485.
- It prints `AG06 writers=.. op=.. n=.. refused=.. p50_us=.. p99_us=.. max_us=..`.

**`crates/api/src/audit_lock_wait_tests.rs`** (722 lines, exists)

- Helpers:
  - `refuse_next_write` 33
  - `scratch` 43
  - `window` 50
  - `at` 55
  - `opened` 59
  - `closed` 63 (the same windowless `Record::refused(Outcome::Stored)` close)
  - `member` 67
  - `journal_with_one_record` 72
  - **`hold(journal) -> store::flock::Flock<File>` at 80**. The spec said ":73" at 453de6989. It
    opens a second description (`OpenOptions` read and append) and calls `Flock::try_lock`.
  - `lock_is_free` 93
  - `outcomes` 102
- 17 tests, among them:
  - `an_opening_append_waits_for_a_held_lock_and_lands` 130
  - `an_append_held_past_the_bound_is_refused_naming_the_bound` 160
  - `a_zero_bound_refuses_at_once` 317
  - `eight_concurrent_appenders_lose_no_record` 335
  - `a_reader_never_waits_on_the_append_bound` 435
  - `a_run_whose_opening_met_a_held_lock_and_then_died_reads_interrupted` 490
  - `a_journal_replaced_while_an_append_waits_is_refused_by_name` 614
  - `a_holder_killed_while_holding_frees_the_lock_to_the_waiter` 651
  - `the_refusal_text_pins_the_bound_constant` 695 (source checks on audit.rs; keep it green)

**`crates/api/src/pullrun.rs`** (8,860 lines; call only)

- `pub(crate) fn restart_scan(journal) -> Restart` at **2042** (it calls `restart_scan_waiting`
  with `INDEX_WAITS` = 100 and `INDEX_WAIT` = 100 ms).
- `not_known(...)` 2664, word `"not_known"` 1340.
- `is_close` 1520 and `same_key` 1526. The spec's "near :1726" is 1526 at head.

**`crates/api/src/autopilot.rs`** (8,053 lines; keep unchanged)

- 3919 `crate::server::broker_run_by(&asked, site, ..., Driver::Autopilot).await`.
- 3943-3983: the record is `Refused` if `run.blocked` is set, `NotStarted` if `reached == 0`,
  otherwise `of_run`.
- 3985-3989: `journal.append` if blocked, else `run.ledger.close`.
- 3994 `run.ledger.rest`.
- 3996-4001: `journal_error = run.ledger.refused.clone().or_else(|| run.ledger.opened_refused.clone())`.
- So a run that AG-06b refuses at its opening must leave `BrokerRun` in a shape this unchanged
  code reads correctly. If the refusal comes back as `run.blocked`, the autopilot appends a
  `Refused` record and closes no run. Whether `opened_refused` is still set then is the builder's
  call, named in notes.

**`crates/store/src/flock.rs`** (1,147 lines; PREC-16's, never edit): `pub fn holder_note(path: &Path) -> String` at 390.

**`docs/`**:

- `docs/06-limits.md` AG-06 section at 20763-20864.
- `docs/05-decisions.md` D-6244 at 78517.
- `docs/04-invariants.md` AG-06 section at 8291-8312.
- No D-710x entry exists. No `L-AG06b` row exists.
- No test named in the spec exists yet (`a_run_whose_opening_is_refused_after_the_bound_...`,
  `a_contended_journal_append_never_stalls_the_http_surface`,
  `every_journal_call_on_the_pull_paths_goes_off_the_workers`,
  `the_measure_harness_closes_every_run_it_opens`). `journal_unavailable` appears nowhere in
  `crates/api/src` at head.

**IA-10c's branch (NOT merged; `fix/db-IA-10c` at 56634b002), what it changes in the owned region**

- In `broker_run_by` it replaces `let claimed_run = note_run_started(asked, targets.len());` with
  `let (landing, claimed_run) = match open_spot_landing(asked, site, targets.len()) { Ok(opened) => opened, Err((why, code)) => return BrokerRun::blocked(why, code) };`.
  That happens just before `let journal = site.journal();` and `BrokerRun::opened(...)`.
- It changes `Lanes::default()` to `Lanes::hooked(landing.as_ref())`.
- `fn open_spot_landing` is at 6147 on that branch.
- It also edits `pull_run` (the `progress.run_id` seat) and `pull_spot`, `pull_repair` and
  `fno_answer`.
- Its diff over feat/db-fix is 37 files, +4,210/-127 (three-dot, from its merge-base e0668a4bd).
- (Checker, 24 Oct, from `git show fix/db-IA-10c:...`.) `open_spot_landing` already runs its
  landing claim through `off_the_workers` (6160 on that branch), the same mechanism AG06B-2 asks
  for. It then calls `note_run_started(asked, instruments, run)` (now three arguments, fn at 4970
  on that branch) at 6175, so on IA-10c the `pull.run` "started" event and the sink's run key are
  both taken INSIDE `open_spot_landing`, before the journal opening. What it opens: a
  `crate::pullrun::LandingRun` (pullrun.rs 3596 on that branch), which holds the vendor's shared
  landing claim (`_claim: VendorClaim`, released when the `LandingRun` drops) and a hook whose
  `pull-spot`/`pull-repair` id `open_landing` (3615) allocated from the runs ledger (or the
  `/pull/run` leg's given id). The sink's run key is released only by `note_run_finished`.
- **IA-10c also edits the two files AG-06b must keep unchanged.** `crates/api/src/autopilot.rs`
  gains 2 lines (`run: 0,` in the autopilot's request at 3906, "THE BROKER RUN TAKES ITS OWN
  `pull-spot` ID (IA-10c, D-6260)"), and `crates/api/src/pullrun.rs` gains 199 lines (`run_id_json`,
  `slot_run_id`, `SinkRun`, `Answers`, `LandingRun`, `open_landing`, `keep_and_spool`, all from
  3519 on that branch). After the rebase, "autopilot.rs exactly as merged" and "pullrun.rs: call
  only" mean as merged WITH IA-10c: keep IA-10c's lines and add none of AG-06b's there.
- When AG-06b rebases onto it: the journal opening comes after IA-10c's landing claim. A run that
  AG-06b refuses at its opening must not leave IA-10c's landing claim or run-id seat dangling.
  Keep its seat, and name in notes how the refused run releases what IA-10c opened.

### 7.1 Existing tests and accounting the change meets (added by the independent check, 24 Oct)

Checked with `git show e851da1ca:<file>` in `W/wt/DBFIX`. None of this changes the package; it is
what the builder meets on the way, and none of it is in the spec text.

- **The sink's run key must be released on the refusal path too.** `note_run_started` claims the
  telemetry sink's run key (server.rs 4930-4978), and only `note_run_finished` (4993, release at
  5065) gives it back, at the end of `broker_run_by` (6317). A run refused at its opening that
  `return`s between 6154 and 6317 without `note_run_finished`, or its own `release_run`, leaves
  every later event of the process stamped with the refused run's id. On IA-10c the claim moves
  into `open_spot_landing` (section 7, IA-10c), so the same holds after the rebase.
- **The `pull.run` "started" event is written before the opening.** The spec says nothing is
  written "beyond the existing api.audit Error event and one pull.run refused event". At head
  `note_run_started` (the "started" event, 4962) runs at 6154, before `BrokerRun::opened` at 6158;
  on IA-10c it runs inside `open_spot_landing`, also before the opening. L-AG06-03 says the
  "started" event "is still written". Either open the journal before the run is started on the
  log, or count the "started" event and say so in notes; do not leave it unstated.
- **A new `pull.run` "refused" event is a new emit site, and the count is pinned.**
  `crates/api/src/emitted.rs` (not AG-06b's file) counts every `telemetry::emit(`,
  `telemetry::emit_if!(` and `telemetry::emit_for_run(` in the LIB target's source
  (`lib_emit_sites`, 1775) and asserts `lib_sites, 96` (2184-2189), and that
  `SITES_HERE + REACHED_IN_SERVER_TESTS + REACHED_IN_RECOVERY_TESTS + REACHED_IN_SPEED_TESTS +
  UNREACHABLE == lib_sites` (2191-2198; `ROWS` = 41 at 241, `REACHED_IN_SERVER_TESTS` = 32 at
  1958). Both asserts are in
  `the_three_sites_this_binary_cannot_reach_are_named_rather_than_forgotten` (1853); the row table
  is driven by `every_reachable_emit_site_puts_a_record_in_the_file` (1519). IA-10c does not
  change these figures. A new site fails the first test unless the same commit moves the figure
  and proves the site (a row, or the server-tests count with a test that reads the record back):
  list `emitted.rs` in `files_outside_ownership`. Emitting the refusal from an existing
  site is the other way; name the choice in notes.
- **Two source-shape tests pin text in the owned region's callers.**
  - `the_sweep_withholds_per_instrument_probes_the_store_and_says_the_horizon_once` (21369)
    needs the exact text `BrokerRun::opened(&journal, asked, targets.len(), withheld, driver)` in
    `broker_run_by`, after `BrokerRun::out_of_order(why)` (21388-21391).
  - `every_record_of_a_run_names_one_source_and_a_refusal_before_it_closes_none` (34266) needs
    `close_reaching_none(&mut run, &asked, now, journal, &first)` in `broker_answer`,
    `run.ledger.close(journal, &record)` in `close_reaching_none`,
    `ledger.close(journal, &record)` in `landed_answer`, `run.ledger.close(&journal, &record)?` in
    `recovery_spot`, and, in `autopilot.rs`, `run.ledger.close(&journal, &record).err()`
    (34306-34353).
  - Wrapping those calls as `off_the_workers(|| ...)` at the call sites changes the pinned text.
    Change a pin only with the reason in notes, never to weaken it. The `autopilot.rs` close
    cannot be wrapped there at all, because that file stays as merged. An observation, not a
    requirement: putting `off_the_workers` inside `RunLedger`'s own `open`, `close`, `flush` and
    `rest` (the owned region) would reach the autopilot's calls without touching `autopilot.rs`,
    and would leave every pinned call text as it is. `fno_walk`'s journal calls are plain
    `journal.append` (in `fno_opened` 11395, and through `recorded_fact` from
    `FnoPage::say_counted` 10468), not `RunLedger`, so they need their own routing.
- **The run's events reach the sink.**
  `the_run_events_and_the_request_event_reach_the_installed_sink` (async test, 34392) reads the
  `pull.run` "started" (34416) and "finished" (34431) records back off the installed sink. A
  refused-at-opening path should be proved the same way (`crate::emitted::sink`, `mark`,
  `landed`).
- **FM-162's far side (R4-11's specAppend, verbatim)**, for the note the spec asks for ("If your
  refusal also proves part of FM-162's text, say so in notes with the test name, so R4-11's
  builder can cite it"). Source: `__ARGS.specAppend["R4-11"]`, the paragraph that starts
  "FM-162".

```text
FM-162 (main session 20 Oct): this failure-matrix row is now yours. AG-06 (merged 5cec3e7cd / edc7870ad) built the bounded-wait half of it: every pull audit journal append waits a bounded time for its lock. The OPEN half is a run whose opening is refused after that bound and still runs unjournalled; AG-06 could not close it because RunLedger::open lives in the api server and api depends on cli (CLAUDE.md section 5), so the single ingest command you build must open its run record before any stage work, and refuse by name to start a stage whose run record could not be written. Close it with a failing-first test, or report it NOT DONE with the reason; never mark it fixed by the AG-06 half alone.
```

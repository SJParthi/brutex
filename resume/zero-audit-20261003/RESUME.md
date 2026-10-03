# Zero-findings audit loop — resume state, 2026-10-03 ~20:20 UTC (FINAL SAVE at 93%)

Saved because weekly usage reached 93% at 20:09 UTC. Work stopped after this save.

## UPDATE 20:20 UTC — read this first
- Main-thread batch **D-1769** (fixes CE-4, 5, 6, 7, 8, 10, 14, 15, 16, 17, 21, 27,
  28, 29, 30, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41; P3-01-01, P3-01-05, P3-02-02,
  P3-02-03, P3-02-04, P3-02-05; p3floor-1, p3floor-2; CE-22 was already fixed by
  D-1762). Invariants ZR-32..ZR-56. New module `crates/core/src/knob.rs`
  (shared empty-folder / HOME / boolean-switch refusals).
  - **COMMITTED AND PUSHED: final/all-fixes-zero @ 9c6284c2.** Tracker rows set
    to `branch 9c6284c2`. Verified: clippy -D warnings clean; full workspace
    tests --no-fail-fast green apart from two targets then fixed and re-run
    green (grid fingerprint re-taken for p3floor-2; libm_key lists live.rs).
    Not run: cargo-mutants, non-root tests.
- Web tests: `node --test web/tests/*.test.js` shows 20 failures that are
  ENVIRONMENTAL (`Cannot find package 'svelte'`; no node_modules in the cloud
  box). Same 20 fail with the batch stashed. Run `npm ci` in web/ to check them.
- Agents at stop: ledger-tails (branch zero/ledger-tails, D-1900..1919, prefix
  ZL-) and test-teeth (branch zero/test-teeth, D-1920..1939, prefix ZT-) were
  told to commit and push what they had. Review each branch, merge into
  final/all-fixes-zero keeping both sides of docs/04 + docs/05 conflicts, test.
- Next free numbers for the main thread: D-1770..D-1799, invariant ZR-57.
- Still open (state `found` in the tsv): CE-9, 12, 13, 18, 19, 20 (CE-20 needs a
  cli census change surfaced in api + web; CE-12/13 parquet pre-validation is
  large); P3-01-02 (routes silently ignore known fields), P3-01-03 (disconnect
  records `cancelled` while admission runs), P3-01-04 (masters refresh inside the
  request, page aborts at 90 s), P3-02-01, 06, 07; all P1-*; numeric items; slice
  findings; W3-runner5-0; W1-api5-4. Full evidence for each is in `findings/`.
- Not yet done on staging: merge origin/final/all-fixes (a21d031f, mastersrun
  conflict expected); cargo-mutants 26.2.0 on changed lines; non-root tests;
  message PR 74 CI thread (cse_01TPJRnnkg5yuNeRBHDyzaaP) then merge into #74;
  tell the 4 helper sessions the new staging head.


Saved because weekly usage reached 86% at 18:32 UTC (save at 93%, stop at 98%).

## The task (user's words, summarised)
Repeat adversarial audit rounds over the whole brutex workspace and fix EVERY
finding fully (no "documented only"; missing functionality and scenarios count)
until one whole round finds zero. Items only the user can do are named, not skipped.

## Branches
- Staging: `final/all-fixes-zero` (pushed). Head at save time: see `git log`; last
  known commits: 3f22aed8 (D-1760..1764), bfaf9611 + 5140aca3 (D-1765),
  77448192 (D-1766). Any agent branches merged after that are listed below.
- Target: `final/all-fixes` = PR #74, the only PR allowed. Before merging staging
  into it, message the PR 74 CI thread (session cse_01TPJRnnkg5yuNeRBHDyzaaP) so
  pushes are batched. Staging has NOT been merged into #74 yet.
- origin/final/all-fixes moved to a21d031f (D-1585 masters footer). Expect a
  conflict in api/src/mastersrun.rs footer/status_json and its test when merging.
- Not yet run on staging: cargo-mutants 26.2.0 on changed lines; non-root tests
  (`setpriv --reuid=65534 --regid=65534 --clear-groups`).

## Ranges (do not reuse)
- Decisions used: D-1760..D-1766 (main thread). Free for main thread: D-1767..D-1799.
- Agent ranges: D-1900..1919 ledger-tails (ZL-), D-1920..1939 test-teeth (ZT-),
  D-1940..1959 doc-truth (ZD-), D-1960..1969 pull-autopilot (ZA-),
  D-1970..1979 web-routes (ZW-), D-1980..1989 cli-edges (ZE-), D-1990..1999 numeric (ZN-).
  Invariant prefix ZR- (main thread) is at ZR-27.

## Tracker
`zero-findings.tsv.md` here (copy of /mnt/project-files/fix-board/status/zero-findings.tsv):
id, state (found / fixing / branch / merged), commit. All findings evidence is in
`findings/` (copy of /mnt/project-files/zero-rounds/).

## Fix work queued (briefs in `briefs/`; each agent uses COMMON-BRIEF.md)
1. pull-autopilot: CE-23 (HIGH), CE-24, CE-28, CE-29, CE-30, CE-14, CE-15+P1-19-02,
   CE-16, P1-19-01, P1-19-03, and one shared empty-path refusal plus one shared
   boolean-knob parser for CE-5, CE-6, CE-33, CE-36, CE-37, CE-38, CE-39
   (helper's resume: fix-queue resume/zero-rounds/crash-edge.md).
2. web-routes: CE-25, CE-26, CE-27, CE-32, P3-01-01..05, P3-02-01..07, P1-06-01..03.
3. cli-edges: CE-4, CE-6..10, CE-12, CE-13, CE-17, CE-18..22, CE-34, CE-35.
4. numeric (numeric-complexity.md): floor-vs-max class first (p2bool-1, p2inst-1,
   p2idx-1, run3-1, D-0743 pbo_ppm; div_ceil for MAX-gated only), D-0742 block >
   periods refusal, mediums pst-1 grk-1 run1-1 xcut-1, then the rest. User-decision
   items to NAME, not guess: pst-3, clib-1, clib-2, gaps-7 (survivorship).
5. torn ledgers follow-up: concurrency.md Pass 3 ledgers-1/2/3, locks-1/2/3;
   crash-edge CE-3, CE-11, CE-31.
6. CI gates and server security: P1-03-*, P1-04-*, P1-07-*, P1-08-*, P1-09-*, P1-20-01.
7. Round-1 slice findings still open: slices 00, 03, 08, 09, 10, 12, 17, 18, 19, 26.
8. Owned items: W3-runner5-0 (one frontier-rule authority, column digest v2; user
   approved), W1-api5-4 (/bars/window.json non-ts sort needs a precomputed index;
   currently O(n) per request, documented D-1446). W3-runner2-7 was already fixed (D-1183).
9. Then audit round 2: requeued slices 01 02 04 05 06 07 21 22 25 27 28 29 30, CI,
   web, docs-vs-code; repeat rounds until one finds zero.

## Helper sessions that re-audit when told the staging head
- tests/docs/security: session_01EdxH94ATfNqNDjU2V8Ctt4
- concurrency: session_01GHxZHAiyvEYbRszm7UuAgD
- crash/edge input: session_01LqVqUnY5Tn5VMB12SA7LRQ
- numbers/complexity: session_01QxTGhDS3kApsAREe3yVYF2

## Lessons
- `cargo test -p core` (package is `core`, lib is brutex_core).
- Never `pkill -f` a pattern contained in your own command line.
- `+` in a query is a space after `param` decoding; tests must send `%2B`.
- api::emitted counts every telemetry::emit site in the lib; a new emit needs a
  driven row (ROWS) and the lib_sites figure bumped.
- Master fixtures must carry the right-most vendor-id column (D-1761).

## Agent branch: zero/test-teeth (pushed, head 0ea46614, NOT merged into staging)
Fixes all 31 "tests that cannot fail" findings (P1-10-01..04, P1-11-01..03,
P1-12-01..05, P1-13-01..03, P1-14-01..07, P1-17-01..04). D-1920; invariants
ZT-01..ZT-06; P-05 corrected. 28 of 31 break-tested (break code, see test fail,
restore); P1-10-03, P1-11-03, P1-12-05 and the api P1-12 rows were not
break-tested. Full tests run only on core, costs, lake, engine, indicators,
pull, runner; for cli and api only touched tests. NOT run: clippy, CI gate
scripts, mutants. Merge notes:
- production changes: api `note_unstamped_lock` (emitted.rs counts 20/9 -> 21/8);
  pull `signing_key_for`; ci.yml gate 1d declares literal `iam`.
- Not fixed (production): `cli auto` exits 0 when it measured nothing — open
  finding; the test now uses 6 sessions.

## Agent branch: zero/ledger-tails (pushed, head 281cbb46, NOT merged into staging)
D-1900..D-1915, invariants ZL-01..ZL-21. Fixed + tested: original brief findings
incl. CE-31 (4cee129e), ledgers-1 (38280a94), CE-3 (78a216d5), ledgers-3
(bd22a9ad: six ledger writers cut a torn tail on open; readers still refuse),
locks-1 (9ba9110e), locks-2 (66c7632d), locks-3 (46f2c763), CE-11 (80b389c1).
PARTIAL: ledgers-2 (281cbb46, D-1915) — Base Evidence V2, Candidate Universe and
boolean evidence fixed; Lineage V4, Selection V6, Global Replay V4, Admission V4
still re-sync a failed barrier in place (open). Not run: clippy over cli/api,
full workspace tests, mutants, non-root. Agent reports pre-existing fmt drift
and clippy failures in boolean_candidate_persistence*.rs and
index_stop_vix_tests.rs on its base. ledgers-3's docs/06-limits entry not added.

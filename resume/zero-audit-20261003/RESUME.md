# Zero-findings audit loop — resume state, 2026-10-03 ~19:00 UTC

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
   CE-16, P1-19-01, P1-19-03, CE-5, CE-33.
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

# Per-finding status, all 91 (generated 2026-10-03 from scratchpad status.js and fix-board sweep.tsv)

Board states: found, fixing, branch, pushed, green. Also mirrored at /mnt/project-files/fix-board/status/sweep.tsv (id, state, commit) for the Fix Board; keep both current.

| id | severity | finding | fix status | board state | commit | note |
|---|---|---|---|---|---|---|
| gaps-6 | high | Stock splits look like massive winners | partial | found |  | Proved a split moves no intraday trade's P&L but changes ~79 prior-day conditions. Every stored stock report now names its largest overnight move (D-1540). Refusing needs a charter-sourced threshold: UNVERIFIED. |
| hunt-runner-1 | medium | Pessimistic P&L biased upward | fixed | pushed | a21d031 | Time exit outranks a target on the time-exit bar in the pessimistic reading (D-1541). |
| hunt-indicators-1 | medium | EMA and ATR report warm too early | fixed | pushed | a21d031 | EMA and ATR seeded from the mean of their first period (D-1542). |
| hunt-pull-1 | medium | F&O gap audit misses lost minutes after 3 Aug 2026 | fixed | pushed | a21d031 | Gap audit uses the venue's dated hours, 15:40 close included (D-1529). |
| hunt-pull-2 | medium | Tiny or negative prices stored as zero | fixed | pushed | a21d031 | Sub-paisa and negative prices refused (D-1530). |
| attackdata-1 | medium | Truncated bar file silently rebuilt empty | fixed | pushed | a21d031 | Sealed month refused when the file existed with a non-empty sidecar (D-1520). A month deleted whole is still recreated; stated limit. |
| hunt-store-1 | medium | Bar reader accepts overlay and Greeks files | fixed | pushed | a21d031 | Bar door accepts only the bar layout (D-1523). |
| hunt-cli-a-1 | medium | Run ledger can accept a duplicate run | fixed | pushed | a21d031 | Indexed prefix re-hashed on growth; rewrite refused (D-1560). |
| hunt-cli-a-2 | medium | One duplicate receipt makes every result set unreadable | fixed | pushed | a21d031 | Same prefix check on detail-sets.bin (D-1560). |
| hunt-api-1 | medium | Closing the browser tab cancels a hand pull half-way | fixed | pushed | a21d031 | Pulls run detached from the connection (D-1581). |
| hunt-api-2 | medium | Ctrl-C cannot stop the server during a sweep | fixed | branch | 700e644 | Shutdown now stops a running sweep at each month, candidate and timeframe boundary and names it CANCELLED (D-1551). |
| attacksweep-1 | medium | Slow-client guard bypassed with two bytes | fixed | pushed | a21d031 | Blank leading lines no longer stop the head deadline (D-1580). |
| hunt-api-3 | medium | Any open web page can wipe the server log | fixed | branch | 700e644 | Same-origin failure lines capped at 200 a minute too, each class with a named suppressed count (D-1552). |
| hunt-api-4 | medium | A retired F&O symbol can block every recovery | fixed | pushed | a21d031 | Recovery filters by plan before checking symbols (D-1584). |
| webcontract-1 | medium | Backtest page never shows condition names | fixed | pushed | a21d031 | /vocab.json now sends commit_digest (D-1586). |
| errpaths-1 | medium | AWS_PROFILE ignored for credentials | fixed | pushed | a21d031 | AWS_PROFILE honoured; half-set env identity refused (D-1534). |
| hunt-costs-1 | medium | STT priced at the entry day's rate | fixed | pushed | a21d031 | Each leg priced at its own date's regime (D-1535). |
| hunt-costs-3 | medium | Two cli processes corrupt the shared log | fixed | pushed | a21d031 | Cross-process lock; a second sink on one directory is refused (D-1537). |
| hunt-ci-1 | medium | A PR can weaken its own gates and merge itself | partial | found |  | CODEOWNERS plus an owner-approval check in auto-merge (D-1604). Fully closing it needs branch protection's code-owner review, a setting only the owner can turn on. |
| hunt-ci-2 | medium | main is never re-tested after a merge | fixed | pushed | a21d031 | Hourly check dispatches CI on main when its head has no run (D-1605). |
| testgaps-1 | medium | Four ✓ invariants cite tests that don't exist | fixed | pushed | a21d031 | Gate 10 checks two-segment and bare names; four rows cite real tests (D-1606). |
| testgaps-6 | medium | 11 invariants proven only by tests CI never runs | fixed | pushed | a21d031 | Gate 6d builds and runs the web/ Rust tests: 68 pass; 2 need operator data and are named. |
| gaps-1 | medium | About 16 cli modules unreachable from any command | documented | fixing | f1f25c3 | No retirement decision exists, so nothing deleted; modules recorded as unwired (D-1568). |
| gaps-3 | medium | Multiple-testing and walk-forward code is test-only | documented | fixing | f1f25c3 | Recorded as unwired with a test that pins the record (D-1547). |
| gaps-5 | medium | Cross-sectional winner table is in-sample only | open | found |  | Needs an out-of-sample split and correction across 210 instruments: a new feature, not a bug fix. |
| gaps-7 | medium | Survivorship bias in the equity universe | open | found |  | Needs point-in-time F&O membership data, which the charter does not source. |
| gaps-8 | medium | No equity charges and zero slippage | open | found |  | Needs a charter-sourced equity charge stack. |
| o1surface2-1 | medium | cli descend rebuilds a whole rung per step | documented | fixing | 78ce90f | Reusing the column is unsafe here (identity-bound); per-step bound now in limits (D-1567). |
| hunt-conc-1 | medium | Ledger row order follows thread timing (known) | partial | fixing | 78ce90f | sweep-all now files rows in walk order (D-1564); range-all and pool documented. |
| hunt-cli-a-3 | low | Two sealed rows with one identity accepted on open | fixed | pushed | a21d031 | Cold open refuses duplicate sealed identities (D-1561). |
| hunt-cli-b-1 | low | Re-running a finished search keeps adding checkpoints | fixed | pushed | a21d031 | Exhausted rerun publishes nothing (D-1562). |
| hunt-pull-3 | low | Exceptional-session warning repeated per bucket | fixed | pushed | a21d031 | Named once per day (D-1533). |
| hunt-indicators-2 | low | Opposite patterns fire on the same bar | fixed | pushed | a21d031 | Equal-body reversal lights neither pattern (D-1543). |
| hunt-indicators-3 | low | Homing pigeon and identical three crows are the wrong shape | fixed | pushed | a21d031 | Four patterns take classical shapes (D-1543). |
| hunt-runner-2 | low | White/SPA and Romano-Wolf disagree at p = 0.05 | fixed | pushed | a21d031 | p = 0.05 clears inclusively everywhere (D-1548). |
| hunt-runner-5 | low | SPA uses an i.i.d. standard error | documented | found |  | No sourced HAC bandwidth; stated (D-1549). |
| hunt-costs-2 | low | Run id can repeat after restart | fixed | pushed | a21d031 | Restart seeds above the last block's run ids (D-1536). |
| hunt-costs-4 | low | One clock jump stamps every later event in the future | fixed | pushed | a21d031 | Future time floor named and counted (D-1538). |
| hunt-costs-5 | low | Cost rates have no charter source | open | found |  | UNVERIFIED: the charter needs primary sources for cost rates. Not invented. |
| attacksweep-2 | low | Disk-full glues the next event to a fragment | fixed | pushed | a21d031 | Fragment closed before the next event (D-1539). |
| attacksweep-3 | low | Repeated POST form keys use the first value | fixed | pushed | a21d031 | Repeated form keys refused (D-1587). |
| hunt-store-2 | low | Torn first header makes an empty month unopenable | fixed | pushed | a21d031 | Torn genesis repaired when nothing is committed (D-1521). |
| hunt-store-3 | low | Writer recreates a missing store root | fixed | pushed | a21d031 | Missing store root refused (D-1522). |
| hunt-store-4 | low | New parent folders not synced | fixed | pushed | a21d031 | Created directories' parents synced (D-1522). |
| hunt-store-5 | low | Lake ignores timestamp unit and time zone | fixed | pushed | a21d031 | Lake refuses wrong unit or non-UTC timestamps (D-1528). |
| hunt-store-7 | low | 4 tests fail when run as root | fixed | pushed | a21d031 | Tests run where permission bits bind (D-1526). |
| webcontract-2 | low | Masters page puts vendor text into innerHTML | fixed | pushed | a21d031 | Vendor text escaped (D-1585). |
| webcontract-3 | low | Masters page hides a failed reload | fixed | pushed | a21d031 | Failed reload is said; footer corrected (D-1585). |
| errpaths-2 | low | A symlink loop reads as 'no receipt' | fixed | pushed | a21d031 | symlink_metadata; only NotFound is absent (D-1561). |
| errpaths-3 | low | An invalid stop ladder silently becomes empty | fixed | pushed | a21d031 | Invalid stop ladders refused (D-1545). |
| errpaths-4 | low | Swapped tolerance widths accepted | fixed | branch | 700e644 | Widths fields private; a swapped pair cannot be built outside the crate (compile_fail doctests, D-1553). |
| hunt-conc-2 | low | Catalog and later-period commands write from workers | documented | fixing | 78ce90f | Ordered phases need transaction re-plumbing; bound stated (D-1564). |
| hunt-conc-3 | low | Refusal text depends on HashMap order | fixed | pushed | a21d031 | Reconcile walks identities in sorted order (D-1565). |
| attackdata-2 | low | −0.0 Greek reported AlreadyPresent | fixed | pushed | a21d031 | Byte equality; greek domain enforced (D-1524). |
| attackdata-3 | low | Repeated JSON keys keep the last value silently | fixed | pushed | a21d031 | Repeated JSON keys refused (D-1531). |
| attackdata-4 | low | Over-precise price text can land one paisa off | documented | fixing | f1f25c3 | Needs serde_json arbitrary_precision; limit stated. |
| attackdata-5 | low | Fold nets negative volumes | fixed | pushed | a21d031 | Negative volume and over-wide buckets refused (D-1532). |
| attackdata-7 | low | Restated bar refused with the wrong reason | fixed | pushed | a21d031 | OverlapDisagrees names the conflict (D-1525). |
| attackdata-8 | low | Checksum flag can be cleared to skip verification | documented | fixing | f1f25c3 | Needs a new on-disk format version (§3 rule 8); limit stated (D-1528). |
| o1store2-1 | low | Timestamp lookup can take more reads than documented | fixed | pushed | a21d031 | Doc bound corrected (D-1527). |
| o1surface2-2 | low | Autopilot tick reads every vendor manifest twice | fixed | pushed | a21d031 | Tick reads only its own vendor once (D-1588). |
| o1surface2-3 | low | Pull landing does disk I/O on HTTP worker threads | fixed | pushed | a21d031 | Landing runs off the HTTP workers (D-1589). |
| o1eng2-1 | low | Sliding window rebuilds when exits move backward | documented | fixing | f1f25c3 | Θ(window) bound stated in limits; Newey-West effect UNVERIFIED. |
| hunt-ci-3 | low | 'cargo deny \|\| true' passes the step-runs check | fixed | pushed | a21d031 | step-runs refuses swallowed or skipped commands (D-1600). |
| hunt-ci-4 | low | Nothing guards ci-ok itself | fixed | pushed | a21d031 | Gate checks ci-ok's needs, always() and success-only (D-1601). |
| hunt-ci-5 | low | 3,630 lines of gate tools skip lint, coverage and mutation | fixed | pushed | a21d031 | Gate tools clippy-clean; Gate 6c lints them. |
| hunt-ci-6 | low | Gate 27 skips 179 invariant rows | fixed | pushed | a21d031 | Gate 27 reads 1,488 ids, up from 1,309. |
| hunt-ci-7 | low | Gate W4 runs without pipefail | fixed | pushed | a21d031 | pipefail and silent-zero refusal (D-1609). |
| hunt-ci-8 | low | Auto-merge says 'not enabled' while still armed | fixed | pushed | a21d031 | A stop disarms; PR picked by head sha (D-1604). |
| rustonly2-1 | low | Gate 1 counts an orphan lib.rs as compiled | fixed | pushed | a21d031 | Only members and built tools count as compiled. |
| rustonly2-2 | low | Gate 0 misses 8 inline-interpreter forms | fixed | pushed | a21d031 | Every inline-interpreter form refused. |
| rustonly2-3 | low | Gate 2 misses 3 build-script tricks | fixed | pushed | a21d031 | Indirect spawns from build scripts refused. |
| rustonly2-4 | low | Nothing refuses 'sh -c' from Rust code | fixed | pushed | a21d031 | Shell or interpreter spawned from crate code refused. |
| rustonly2-6 | low | Lock fingerprint misses version-only bumps | fixed | pushed | a21d031 | Fingerprint hashes versions. |
| rustonly2-7 | low | Gate 1g misses RUSTFLAGS linker settings | fixed | pushed | a21d031 | Gate 1g reads more config doors (D-1610). |
| testgaps-7 | low | 12 ignored tests never run in CI | documented | found |  | Each ignored test documented; they need operator data. |
| testgaps-11 | low | Limits doc names deleted code | fixed | pushed | a21d031 | Stale limits entries corrected. |
| gaps-11 | low | No handoff from discovery to qualification | open | found |  | Automated handoff from discovery to qualification is a new feature. |
| gaps-10 | low | V6 results have no API route or page | open | found |  | A V6 results route and page is a new feature. |
| hunt-ci-10 | info | 'Coverage 100%' check enforces 90/89 | fixed | pushed | a21d031 | Check renamed to its real 90% / 89% floors. |
| hunt-ci-12 | info | No permissions: block; actions pinned by tag | fixed | pushed | 0cab319 | All 23 workflow actions pinned to full commit SHAs, tag kept as a comment (D-1457, CI thread, eec9c64). |
| rustonly2-10 | info | core-foundation-sys compiles on macOS | documented | found |  | macOS-only native dependency; no change needed on Linux. |
| webcontract-5 | info | Static assets sent without cache headers | fixed | pushed | a21d031 | Content-hashed assets served immutable (D-1591). |
| hunt-api-6 | info | Browser-launch child never reaped | fixed | pushed | a21d031 | Browser-launch child reaped (D-1590). |
| hunt-cli-a-5 | info | One interrupted V6 write blocks that rung | open | fixing | 78ce90f | Not addressed in this pass. |
| hunt-conc-4 | info | Budget doc says bigger machines earn more | fixed | pushed | a21d031 | Doc corrected. |
| o1eng2-3 | info | Limits says 25 intents per minute; code allows 200 | fixed | pushed | a21d031 | Doc corrected. |
| o1store2-2 | info | Limits doc still lists heap buffers on the cold path | fixed | pushed | a21d031 | Doc corrected (D-1527). |
| hunt-runner-4 | info | Doc says 325 cells; test asserts 625 | fixed | pushed | a21d031 | Doc corrected (D-1547). |
| hunt-costs-6 | info | costs cites D-0039; the costs decision is D-0041 | fixed | pushed | a21d031 | Doc and D-number corrected (D-1535). |
| hunt-store-6 | info | Store doc says ragged tails are truncated | fixed | pushed | a21d031 | Doc corrected (D-1527). |

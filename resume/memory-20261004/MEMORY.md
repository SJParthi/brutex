# Brutex project
- Owner: Parthiban Subramanian (GitHub SJParthi)
- Repo: https://github.com/SJParthi/brutex
- What it is: Rust brute-force backtesting engine. Continuing work from the user's previous account.
- Resume notes live on branch fix-queue: resume/RESUME-20261003.md and resume/audit-20261003/RESUME.md Key facts: [[resume-20261003-pr74-state]], [[audit-20261003-state]], [[brutex-ci-and-merge-gotchas]].
- Final combined PR: #74 on branch final/all-fixes. No other new PRs (if #74 merged, exactly one new combined PR).
- GDFL engine build lives on the user's Mac: /Volumes/WD_BLACK/brutex/fresh-20260919/project, resume kit work-20260925/state/resume-kit/RESUME.md. NEVER push the GDFL reader branch while real GDFL data rows are in its history.

## User's Mac (from System Information screenshots, saved 2026-10-03)
- MacBook Pro Mac16,7, Apple M4 Pro: 14 CPU cores (10 performance + 4 efficiency), 20-core GPU (Metal 4), 48 GB LPDDR5.
- Internal SSD: 500 GB APFS, about 296 GiB free (Mac session measurement, 2026-10-03).
- External drive WD_BLACK (/Volumes/WD_BLACK): 8 TB APFS, about 7.43 TB used and only about 529-572 GB free (2026-10-03). Top level: brutex/, NSE (Stock+Index Tick) zip, NSE_Options_Tick/, NSE_Tick_2018-09-01_to_...-09-24/, Options/. This is the raw market data; never delete or move it without the user asking. Watch free space before large builds or data writes.
- 3 Thunderbolt 5/USB4 ports (up to 120 Gb/s). Usually on a 140W charger.
- Power: system sleep timer is 1 minute on both AC and battery, so long builds need caffeinate or similar to keep it awake.
- Size parallel builds and benchmarks to these limits; time p99 benchmarks on an idle machine.

## User rules (2026-10-03)
- Rust only, except the web frontend.
- O(1) wherever possible; explicitly name anything that can't be. Measure p50/p99/max too; never claim a guarantee.
- Work in parallel; attack extreme edge cases adversarially.
- Verify with real evidence, never guess.
- Never ask the user to tap, paste or approve anything; keep work running continuously.
- Stop at 98% weekly usage after saving resume state to GitHub. The Mac session reads usage (desktop get_usage) and reports it every ~30 min.
- Deliver results as easy comparison-table Artifacts.
- 12:51 UTC: user wants every finding fully fixed, nothing left "documented only" (including the 4 held audit items, now being fixed), and audit rounds with ~100 agents repeated until a round finds zero issues (thread "Audit rounds until zero findings").

## Resume facts (from fix-queue, read 2026-10-03)
- [[resume-20261003-pr74-state]]: #74 ready + squash auto-merge on, waits on ci-ok; #21-#73 folded and closed; lane1-b lost in restart.
- [[workspace-audit-20261003]]: 20-worker audit of #74 head 1087e54, 91 NEW findings (1 high, 28 medium), artifact + reports in /mnt/project-files/audit-20261003-workspace.
- [[audit-20261003-state]]: 55 lane1-b findings NOT fixed on #74; lane 3/audit-53/batch-1 unverified; one cli test failure to explain.
- [[brutex-ci-and-merge-gotchas]]: ci.yml size limit, root-only tests, docs ledger conflicts, gate 11 timing.
- Missing batch-1 C4 findings for 16 groups: fix-queue resume/lanes/c4-missing-findings.md (60 findings).

- [[brutex-fix-board-artifact]]: combined comparison board https://claude.ai/artifact/Air7S5kQkMHWGkqYws9dka; republish to same URL.

## Decision-number ranges claimed on final/all-fixes (2026-10-03)
- Attack audit: D-1480..D-1519, D-1620..D-1659, D-1800..D-1899; invariant prefixes AGA..AGD, AHA..AHD
- Codebase sweep: D-1520..D-1619, invariant prefixes AFA..AFE
- Lane 1-b redo: D-1660..D-1759 (stages on final/all-fixes-00bbns, merged in when verified)
- Zero-findings audit rounds: D-1760..D-1799 and D-1900..D-1999
- New work: start at D-2000 or above.
- [[zero-audit-loop]]: zero-findings audit loop thread, staging final/all-fixes-zero, D-1760..1799 + D-1900..1999
- [[zero-rounds-tests-docs-security]]: helper audit thread (tests/docs/security/CI), findings in /mnt/project-files/zero-rounds/tests-docs-security.md
- [[pr74-ci-thread-state]]: CI thread pushed 0cab319 at 15:02 UTC; D-1452..1460 used; gate 14/step-runs trap

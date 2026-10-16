# SC-5: Sort index version 2 inside the sealed file, for months of 1,024 bars or more

Prerequisites (must be merged into feat/db-fix first): SC-2, SC-3, R3-07, R3-08

## Specification

```text
Your specification is the compressed-store design /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression-20261012.md (12 Oct): read its section for SC-5 and everything that section cites (format, readers and writers, tests for every extreme case, limits with their measured p50/p99/max, decisions to record). The three measured proposals are under /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression/<blocks|tiers|derive>/PROPOSAL.md with their prototypes: reuse the prototype code only after reading it critically. Nothing in this package may delete or rewrite an existing bar file; retirement of old files is SC-9 and its switch ships off. RELAUNCH 13 Oct: you now wait for R3-07 and R3-08, which edit crates/api/src/bars_sort.rs before you (design section 9): give the sort index v2 change to bars_sort.rs on top of theirs. ACCEPTANCE (13 Oct, operator: 'even if it is compressed also always it should achieve near O(1) with p99 max'): read section 5.4 of the design, 'Acceptance at p50, p99 AND max, end to end': every /db route over sealed months on a shadow server, n >= 200, p50/p99/max beside the same months unsealed; flat at 1, 10 and 100 months; warm p99 within 2x unsealed and under 50 ms at first paint; every max over 50 ms named with its cause in docs/06-limits.md; the cold-read test re-run by the second agent.
```

## Work package (fix plan)

```json
{
 "name": "SC-5 Sort index version 2 inside the sealed file, for months of 1,024 bars or more",
 "goal": "Sort index version 2 inside the sealed file, for months of 1,024 bars or more. Your specification is the compressed-store design /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression-20261012.md (12 Oct): read its section for SC-5 and everything that section cites (format, readers and writers, tests for every extreme case, limits with their measured p50/p99/max, decisions to record). The three measured proposals are under /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression/<blocks|tiers|derive>/PROPOSAL.md with their prototypes: reuse the prototype code only after reading it critically. Nothing in this package may delete or rewrite an existing bar file; retirement of old files is SC-9 and its switch ships off. RELAUNCH 13 Oct: you now wait for R3-07 and R3-08, which edit crates/api/src/bars_sort.rs before you (design section 9): give the sort index v2 change to bars_sort.rs on top of theirs. ACCEPTANCE (13 Oct, operator: 'even if it is compressed also always it should achieve near O(1) with p99 max'): read section 5.4 of the design, 'Acceptance at p50, p99 AND max, end to end': every /db route over sealed months on a shadow server, n >= 200, p50/p99/max beside the same months unsealed; flat at 1, 10 and 100 months; warm p99 within 2x unsealed and under 50 ms at first paint; every max over 50 ms named with its cause in docs/06-limits.md; the cold-read test re-run by the second agent.",
 "files_owned": [
  "see the design section for SC-5"
 ],
 "finding_ids": [],
 "verification": "the tests, benches and limits the design names for SC-5",
 "decisions_needed": [],
 "depends_on": [
  "SC-2",
  "SC-3",
  "R3-07",
  "R3-08"
 ],
 "size": "M"
}
```

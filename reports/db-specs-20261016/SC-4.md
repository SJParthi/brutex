# SC-4: The write door: a sealed month refuses appends; repairs and re-offers

Prerequisites (must be merged into feat/db-fix first): SC-3

## Specification

```text
Your specification is the compressed-store design /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression-20261012.md (12 Oct): read its section for SC-4 and everything that section cites (format, readers and writers, tests for every extreme case, limits with their measured p50/p99/max, decisions to record). The three measured proposals are under /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression/<blocks|tiers|derive>/PROPOSAL.md with their prototypes: reuse the prototype code only after reading it critically. Nothing in this package may delete or rewrite an existing bar file; retirement of old files is SC-9 and its switch ships off. RELAUNCH 13 Oct: IA-16 owns the failure kinds (design section 9): if IA-16 has merged, register Sealed as one of its kinds in its admit table; if not, say in notes that IA-16 must add it. A repair or restate that meets a sealed month reaches it through the read door and is refused by name as Sealed, never written in place. The pull (WP-62) must treat Sealed the same way. ACCEPTANCE (13 Oct, operator: 'even if it is compressed also always it should achieve near O(1) with p99 max'): read section 5.4 of the design, 'Acceptance at p50, p99 AND max, end to end': every /db route over sealed months on a shadow server, n >= 200, p50/p99/max beside the same months unsealed; flat at 1, 10 and 100 months; warm p99 within 2x unsealed and under 50 ms at first paint; every max over 50 ms named with its cause in docs/06-limits.md; the cold-read test re-run by the second agent.
```

## Work package (fix plan)

```json
{
 "name": "SC-4 The write door: a sealed month refuses appends; repairs and re-offers",
 "goal": "The write door: a sealed month refuses appends; repairs and re-offers. Your specification is the compressed-store design /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression-20261012.md (12 Oct): read its section for SC-4 and everything that section cites (format, readers and writers, tests for every extreme case, limits with their measured p50/p99/max, decisions to record). The three measured proposals are under /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression/<blocks|tiers|derive>/PROPOSAL.md with their prototypes: reuse the prototype code only after reading it critically. Nothing in this package may delete or rewrite an existing bar file; retirement of old files is SC-9 and its switch ships off. RELAUNCH 13 Oct: IA-16 owns the failure kinds (design section 9): if IA-16 has merged, register Sealed as one of its kinds in its admit table; if not, say in notes that IA-16 must add it. A repair or restate that meets a sealed month reaches it through the read door and is refused by name as Sealed, never written in place. The pull (WP-62) must treat Sealed the same way. ACCEPTANCE (13 Oct, operator: 'even if it is compressed also always it should achieve near O(1) with p99 max'): read section 5.4 of the design, 'Acceptance at p50, p99 AND max, end to end': every /db route over sealed months on a shadow server, n >= 200, p50/p99/max beside the same months unsealed; flat at 1, 10 and 100 months; warm p99 within 2x unsealed and under 50 ms at first paint; every max over 50 ms named with its cause in docs/06-limits.md; the cold-read test re-run by the second agent.",
 "files_owned": [
  "see the design section for SC-4"
 ],
 "finding_ids": [],
 "verification": "the tests, benches and limits the design names for SC-4",
 "decisions_needed": [],
 "depends_on": [
  "SC-3"
 ],
 "size": "M"
}
```

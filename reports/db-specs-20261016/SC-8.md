# SC-8: Routes and pages: /seal.json, stop, resume, bytes by form on /disk.json, a Storage band on /ingest

Prerequisites (must be merged into feat/db-fix first): SC-7, R4-01, R4-04

## Specification

```text
Your specification is the compressed-store design /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression-20261012.md (12 Oct): read its section for SC-8 and everything that section cites (format, readers and writers, tests for every extreme case, limits with their measured p50/p99/max, decisions to record). The three measured proposals are under /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression/<blocks|tiers|derive>/PROPOSAL.md with their prototypes: reuse the prototype code only after reading it critically. Nothing in this package may delete or rewrite an existing bar file; retirement of old files is SC-9 and its switch ships off. ACCEPTANCE (13 Oct, operator: 'even if it is compressed also always it should achieve near O(1) with p99 max'): read section 5.4 of the design, 'Acceptance at p50, p99 AND max, end to end': every /db route over sealed months on a shadow server, n >= 200, p50/p99/max beside the same months unsealed; flat at 1, 10 and 100 months; warm p99 within 2x unsealed and under 50 ms at first paint; every max over 50 ms named with its cause in docs/06-limits.md; the cold-read test re-run by the second agent.
```

## Work package (fix plan)

```json
{
 "name": "SC-8 Routes and pages: /seal.json, stop, resume, bytes by form on /disk.json, a Storage band on /ingest",
 "goal": "Routes and pages: /seal.json, stop, resume, bytes by form on /disk.json, a Storage band on /ingest. Your specification is the compressed-store design /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression-20261012.md (12 Oct): read its section for SC-8 and everything that section cites (format, readers and writers, tests for every extreme case, limits with their measured p50/p99/max, decisions to record). The three measured proposals are under /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/design/store-compression/<blocks|tiers|derive>/PROPOSAL.md with their prototypes: reuse the prototype code only after reading it critically. Nothing in this package may delete or rewrite an existing bar file; retirement of old files is SC-9 and its switch ships off. ACCEPTANCE (13 Oct, operator: 'even if it is compressed also always it should achieve near O(1) with p99 max'): read section 5.4 of the design, 'Acceptance at p50, p99 AND max, end to end': every /db route over sealed months on a shadow server, n >= 200, p50/p99/max beside the same months unsealed; flat at 1, 10 and 100 months; warm p99 within 2x unsealed and under 50 ms at first paint; every max over 50 ms named with its cause in docs/06-limits.md; the cold-read test re-run by the second agent.",
 "files_owned": [
  "see the design section for SC-8"
 ],
 "finding_ids": [],
 "verification": "the tests, benches and limits the design names for SC-8",
 "decisions_needed": [],
 "depends_on": [
  "SC-7"
 ],
 "size": "M"
}
```

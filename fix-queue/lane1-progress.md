# Lane 1 progress (cloud)

Checkpoint for pause/resume. Updated after each item. Branch per item: `fix/cloud-<id>`. Decision numbers: D-0910..D-0919 (batch 1), D-0960..D-0999 (lane1-b).

## Batch 1 (lane1.md)

| Unit | Findings | State | PR |
|---|---|---|---|
| W3-store1-3 | W3-store1-3 | MERGED | https://github.com/SJParthi/brutex/pull/22 |
| AC-whp-cx-0 | AC-whp-cx-0 | Gate 18 mutant fixed (7be2243), CI rerunning | https://github.com/SJParthi/brutex/pull/24 |
| ET-bars-candles-store-0 | ET-bars-candles-store-0 | pushed, in review | https://github.com/SJParthi/brutex/pull/25 |
| W3-store1-0 | W3-store1-0, W3-store1-1, ET-bars-candles-store-12 | pushed, in review | https://github.com/SJParthi/brutex/pull/27 |
| ET-o1-proof-coverage-4 | ET-o1-proof-coverage-4, ET-bars-candles-store-9 | pushed, reviewed | https://github.com/SJParthi/brutex/pull/41 |
| ET-bars-candles-store-2 | ET-bars-candles-store-2, ET-bars-candles-store-3 | pushed | https://github.com/SJParthi/brutex/pull/45 |
| UC-7 | UC-7, UC-13, ET-rust-only-purity-5 | in progress | - |
| ET-o1-proof-coverage-3 | ET-o1-proof-coverage-3, ET-o1-proof-coverage-13 | in progress | - |

## Batch 2 (lane1-b.md, 64 findings)

State: 41 units planned in fix-queue/lane1-b-plan.md (U3 folded into U9's branch, U14 into U1's). All 39 branches being fixed now, 2 at a time, U21 first.

## Resume

After any pause: read this file, `git ls-remote origin 'refs/heads/fix/cloud-*'`, and open PRs; re-run any unit whose branch is missing or whose PR CI is red.

| lane1-b unit | Branch | State | PR |
|---|---|---|---|
| U21 | fix/cloud-W2-cli4-2 | pushed, in review | https://github.com/SJParthi/brutex/pull/26 |

## Batch 3 (audit-20261002 new-findings.md, queued after lane1-b)

Medium done: o1cli-1 PR #49 (D-0997), probeapi-3 cli half PR #50 (D-0998), probestore-3 PR #51 (D-0996, after #24). Next, one agent at a time: o1store-1, o1store-2, o1engine-22, o1engine-23, o1engine-24, o1engine-40, o1cli-2..6, probestore-4..7, probeapi-6. State: low items running. Source: /mnt/project-files/audit-20261002/new-findings.md (copied to fix-queue/lane1-c-findings.md).

## Usage mode (08:15 UTC)

Weekly usage at 45%: lane 1 now runs at most 2 agents (the step3 test-speed agent + one sequential workflow). Order: review pushed PRs, batch-1 rest, then lane1-b units U1, U2, U9(+U3), U4... Root-permission test fix paused with partial work in its worktree (resume later). Stop and checkpoint at 85%.

| extra | fix/cloud-step3-test-speed | pushed (55m -> 8m15s) | https://github.com/SJParthi/brutex/pull/40 |
| extra | fix/cloud-root-permission-tests | pushed (16 root-only failures fixed) | https://github.com/SJParthi/brutex/pull/43 |

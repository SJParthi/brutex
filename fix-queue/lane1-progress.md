# Lane 1 progress (cloud)

Checkpoint for pause/resume. Updated after each item. Branch per item: `fix/cloud-<id>`. Decision numbers: D-0910..D-0919 (batch 1), D-0960..D-0999 (lane1-b).

## Batch 1 (lane1.md)

| Unit | Findings | State | PR |
|---|---|---|---|
| W3-store1-3 | W3-store1-3 | in progress | - |
| AC-whp-cx-0 | AC-whp-cx-0 | in progress | - |
| ET-bars-candles-store-0 | ET-bars-candles-store-0 | in progress | - |
| W3-store1-0 | W3-store1-0, W3-store1-1, ET-bars-candles-store-12 | in progress | - |
| ET-o1-proof-coverage-4 | ET-o1-proof-coverage-4, ET-bars-candles-store-9 | in progress | - |
| ET-bars-candles-store-2 | ET-bars-candles-store-2, ET-bars-candles-store-3 | in progress | - |
| UC-7 | UC-7, UC-13, ET-rust-only-purity-5 | in progress | - |
| ET-o1-proof-coverage-3 | ET-o1-proof-coverage-3, ET-o1-proof-coverage-13 | in progress | - |

## Batch 2 (lane1-b.md, 64 findings)

State: planning units. Next after batch 1.

## Resume

After any pause: read this file, `git ls-remote origin 'refs/heads/fix/cloud-*'`, and open PRs; re-run any unit whose branch is missing or whose PR CI is red.

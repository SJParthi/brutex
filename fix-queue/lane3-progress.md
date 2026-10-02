# Cloud fix lane 3 progress (checkpoint; resume from here after any pause)

Decision numbers: D-0940..D-0959, then D-1200..D-1299. Branch per fix: fix/cloud-<id>.
Updated: 2026-10-02 17:20 UTC. Budget rule: max 2 agents; stop at 95-97% weekly usage after pushing everything and updating resume/RESUME-20261003.md.

## Set 1 (lane3.md)
| Fix | Findings | Branch | D | State | PR |
|---|---|---|---|---|---|
| VWAP sigma floor | ET-indicators-0, ET-indicators-12 | fix/cloud-et-indicators-0 | D-0940 | PR open, CI running | #28 |
| Zero-price daily anchor | ET-indicators-1, UC-3 | fix/cloud-et-indicators-1 | D-0941 | PR open, CI running | #30 |
| Refused-bar re-walk | W3-indicators1-0, W3-indicators1-1 | fix/cloud-w3-indicators1-0 | D-0942 | PR open, CI running | #34 |
| Overlay stub clamp | GAP12-5, GAP12-7, GAP4-47 | fix/cloud-gap12-5 | D-0943 | PR open, CI running | #29 |
| Gap candle by clock | ET-indicators-2, ET-indicators-11 | fix/cloud-et-indicators-2 | D-0944 | PR open, CI running | #33 |
| Dead crossing tests | AC-whp-tb-4 | fix/cloud-ac-whp-tb-4 | D-0945 | PR open, CI running | #31 |
| Muhurat doc | GAP12-10 | fix/cloud-gap12-10 | D-0946 | PR open, CI running | #36 |
| Stale lib.rs docs | ET-indicators-7, ET-indicators-10, UC-1 | fix/cloud-et-indicators-7 | D-0947 | PR open, CI running | #37 |

## Set 2 (lane3-b.md)
| Fix | Findings | Branch | D | State | PR |
|---|---|---|---|---|---|
| B1 credential halt | GAP2-36, GAP2-37, GAP2-38 | fix/cloud-gap2-36 | D-0948 | PR open, CI running | #38 |
| B2 autopilot | W1-api1-2, W1-api1-8, W1-api1-7, W1-api1-1 | fix/cloud-w1-api1-2 | D-0949 | PR open, CI running | #39 |
| B3 calendar_of | W1-api2-1, W1-api2-9, W1-api2-11, R9-api-law-0, W1-api2-0 | fix/cloud-w1-api2-1 | D-0950 | PR open, CI running | #44 |
| B4 json renderers | W1-api1-5, W1-api2-2, W1-api2-3, W1-api1-4, W1-api6-3, W1-api1-6 | fix/cloud-w1-api1-5 | D-0951 | PR open, CI running | #42 |
| B5 operation_audit | GAP14-57, W1-api3-5, W1-api3-0 | fix/cloud-gap14-57 | D-0952 | PR open, CI running | #46 |
| B6 server.rs | W1-api5-0..-11, UC-20, R9-api-cx-2, GAP14-63 | fix/cloud-w1-api5-0 | D-0953 | PR open, CI running | #47 |
| B7 sweeprun/audit/census | W1-api6-1, -4, -2, -5, W1-api1-9, R9-api-cx-1 | fix/cloud-w1-api6-1 | D-0954 | PR open, CI running | #56 |
| B8 pull ingest/fold | ET-bars-candles-store-1, -8, -4, W1-pull2-0, -3, -5, -6, R9-csr-cx-0, -1, GAP2-41, GAP12-12, W1-pull1-0 | fix/cloud-et-bars-candles-store-1 | D-0955 | PR open, CI running | #48 |
| B9 pull vendor docs | GAP12-8, GAP12-11, GAP2-43, W1-pull4-3, W1-pull4-2 | fix/cloud-gap12-8 | D-0956 | PR open, CI running | #55 |
| B10 docs-only | GAP5-50, UC-6, AC-whp-o1-1, AC-gates-o1-4, AC-whp-tb-3, -6, -7, ET-o1-proof-coverage-7, -8, -9, UC-17, ET-strategies-trades-ranking-costs-9, R9-csr-o1-0, ET-vocabulary-conditions-bits-3 | fix/cloud-gap5-50 | D-0957 | PR open, CI running | #32 |
| GAP17-33 | process finding about Mac landing branches, not repo code | — | — | no PR; returned to queue owner | |

## Set 3 (audit-20261002/new-findings.md), queued
| Fix | Findings | Branch | D | State | PR |
|---|---|---|---|---|---|
| C1 api hardening | probeapi-1, probeapi-2, probeapi-3 (api half) | fix/cloud-probeapi-1 | D-1200 | worker running | |
| C2 pull | o1api-44, probestore-1 | fix/cloud-o1api-44 | D-1201 | worker running | |
| C3 api low | probeapi-4, -5, -7, rustonly-4, o1api-3, -4, -21 | fix/cloud-probeapi-4 | D-1202 | next | |
| C4 pull low | o1api-33, -34, -36, -39, -54, probestore-2 | fix/cloud-o1api-33 | D-1203 | next | |
| C5 docs/tests | o1runner-9, o1runner-10, rustonly-3, audit-root | fix/cloud-o1runner-9 | D-1204 | next | |

Note 14:40: #22 landed on main; merged main into all 11 open lane-3 PRs (append-only ledger conflicts, resolver scratchpad/append_resolve.py). #38 fixed gate 1c (test literals).

17:05: #54 cli knob race fix (test-only, main), ported into #31. #39 and #47 fixed gate 11. #44 gate 11 under local diagnosis. B7 worker running; C1 (probeapi-1, D-1200) worker started. Local gate runners: scratchpad gate11.sh, gate12.sh.

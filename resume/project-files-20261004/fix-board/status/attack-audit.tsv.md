id	state	commit	note
GAP15-19	branch	88cbe235	
GAP15-17	branch	7ca675af	D-1644 Base Evidence V3; wip/audit-fixes-5b; checks running
AC-whp-tb-2	fixing		 | cli half dfd15112 D-1645 on wip/audit-fixes-5b
ET-strategies-trades-ranking-costs-7	fixing		 | cli half 50a61190 D-1646 on wip/audit-fixes-5b
gate8	branch	3afa02c3	
lookahead	fixing		
docs-web-01	branch	56d6d14f	WIP on wip/audit-fixes-7b, unvalidated
probeapi-1	branch	56d6d14f	WIP on wip/audit-fixes-7b, unvalidated
GAP17-33	branch	56d6d14f	WIP on wip/audit-fixes-7b, unvalidated
h-api-1	branch	56d6d14f	WIP on wip/audit-fixes-7b, unvalidated
h-api-2	branch	56d6d14f	WIP on wip/audit-fixes-7b, unvalidated
h-api-3	branch	56d6d14f	WIP on wip/audit-fixes-7b, unvalidated
h-cli-1	pushed	9d4a3e7b	
h-cli-2	pushed	18d8c2e0	
h-cli-3	pushed	b660db78	
h-eng-1	found		
h-eng-2	found		
ET-bars-candles-store-1 / -8	found		queued F8 re-check after 18:00 UTC; triage says inherent: no (this pass) — an incremental fold needs a per-rung resume point the store format does not record; adding one is a new derived-file version, and `re
rederive	found		queued F8 re-check after 18:00 UTC; triage says inherent: no (this pass) — an incremental fold needs a per-rung resume point the store format does not record; adding one is a new derived-file version, and `re
W1-pull2-3	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — the rerun IS how a derivation blocked by missing schedule evidence is retried; skipping it needs the same resume point
W1-pull2-0	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — re-reading and CRC-checking every entry under the lock is the D-0036 guarantee; a generation-keyed cache cannot see bit rot (same size, same mtim
W1-pull2-6	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — re-reading and CRC-checking every entry under the lock is the D-0036 guarantee; a generation-keyed cache cannot see bit rot (same size, same mtim
W1-api5-1	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — re-reading and CRC-checking every entry under the lock is the D-0036 guarantee; a generation-keyed cache cannot see bit rot (same size, same mtim
W1-api5-2	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — same D-0036 reason; a miss is caused by a manifest change
W1-pull2-5	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — the days come from the records themselves; no per-day index exists in the bar format
W1-pull1-0	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — per-day receipt revalidation is the D-0519 guarantee
W3-store1-0	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — inherent: an O(1) index would break the store format (records are not dense on the minute grid)
W3-store1-1	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — inherent: an O(1) index would break the store format (records are not dense on the minute grid)
ET-bars-candles-store-3	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — not a cost; store may not depend on pull (§5)
R9-csr-o1-0	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — the fstat is what detects records past the commit (D-0688)
W3-engine1-1	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — canonical order is required for idempotence and the prefix join; the log factor is bounded by the level ceiling
ET-o1-proof-coverage-2	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — inherent: Apriori must test k-2 subsets
W3-engine1-0	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — inherent: Apriori must test k-2 subsets
o1engine-20	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — a bounded top-k under a total order needs a heap; no production caller
W3-engine1-2	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — a bounded top-k under a total order needs a heap; no production caller
GAP16-26	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — not a cost; §7 allows statistics in f64
ET-strategies-trades-ranking-costs-9	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — text finding, already corrected
W3-runner2-2	found		queued F8 after 18:00 UTC: hoist
W3-runner4-1	found		queued F8 after 18:00 UTC: hoist
W2-cli8-6	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — each retry exists because inputs changed under it
o1cli-2	found		queued F8 after 18:00 UTC: hoist
o1cli-3	found		queued F8 after 18:00 UTC: hoist
o1cli-4	found		queued F8 after 18:00 UTC: hoist
o1cli-5	found		queued F8 after 18:00 UTC: hoist
o1cli-6	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — top-first selection is not equivalent (fallback past top rows)
AC-whp-o1-1	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — the ranker reads the retained Sweep; streaming would change the ranked result
cli-14 (W2-cli11-0/-1	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — re-authentication of the ledger IS the read's proof (D-0934)
W2-cli12-3/-4)	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — re-authentication of the ledger IS the read's proof (D-0934)
D-1631 (W2-cli1-2/-3)	found		queued F8 after 18:00 UTC: delete
D-1633 (W2-cli6-0)	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — the replay proves acknowledged children; a cache would be a new durable authority
D-1634 (W2-cli16-2)	found		queued F8 after 18:00 UTC: hold
D-1634 (W2-cli16-3)	found		queued F8 after 18:00 UTC: 
D-1636 (W2-cli15-2)	found		queued F8 after 18:00 UTC: hoist
D-1638 (W2-cli7-0)	found		queued F8 after 18:00 UTC: hoist
D-1639 (W2-cli10-2)	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — no production caller; the validated path is O(1)
D-1641 (W2-cli2-5)	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — the recheck is the page's proof
D-1642 (W2-cli14-1/2/3)	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — re-authentication of upstream
W1-api2-1	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — derivation reads what it derives from; cached after
W1-api2-2	found		queued F8 after 18:00 UTC: fix
W1-api2-3	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — any bounded cache can be thrashed
W1-api1-4	found		queued F8 after 18:00 UTC: fix
W1-api1-6	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — C fixed by artifact; the checks are the proof
W1-api3-0	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — append-only audit record; per-request cost O(1)
W1-api3-1	found		queued F8 after 18:00 UTC: fix
W1-api5-3	found		queued F8 after 18:00 UTC: 
W1-api5-5	found		queued F8 after 18:00 UTC: 
W1-api5-6	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — four free predicates, no index
W1-api5-7	found		queued F8 re-check after 18:00 UTC; triage says inherent: cost inherent (a scrub opens every file); blocking yes — move to run_store_read
W1-api6-0	found		queued F8 re-check after 18:00 UTC; triage says inherent: cost inherent (a scrub opens every file); blocking yes — move to run_store_read
W1-api5-8	found		queued F8 after 18:00 UTC: fix
W1-api5-9	found		queued F8 after 18:00 UTC: 
W1-api2-7	found		queued F8 after 18:00 UTC: 
W1-api5-11	found		queued F8 after 18:00 UTC: 
W1-api6-3	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — a cached refusal would outlive a repair that keeps the generation
o1api-4	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — bounded by the capped query length
o1api-33	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — O(body) inherent; tree only changes the constant
rustonly-4	found		queued F8 re-check after 18:00 UTC; triage says inherent: no — not a cost
W2-cli5-4	found		user approved 13:27 UTC (Fix them); queued F6 after 18:00 UTC: one frontier-rule authority, column digest v2
W3-runner2-8	found		user approved 13:27 UTC (Fix them); queued F6 after 18:00 UTC: one frontier-rule authority, column digest v2
W3-runner2-7	found		user approved 13:27 UTC (Fix them); queued F6 after 18:00 UTC: one frontier-rule authority, column digest v2
W3-runner5-0	found		user approved 13:27 UTC (Fix them); queued F6 after 18:00 UTC: one frontier-rule authority, column digest v2
h-cli-4	found		same seek(End)+write_all no-rollback shape seen by F9 in anchored_search_lineage_v2/v3, population_observations_v1, sweep_evidence; not yet audited

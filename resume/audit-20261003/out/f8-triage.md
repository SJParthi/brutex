# F8 triage: every DOCUMENTED cost, re-examined (worktree /home/claude/wt-fix8, base 331b05c6)

Rule applied: a cost stays documented only when it is inherent (the bytes the caller asked for, a
re-verification that IS the guarantee, a bound the store format fixes). Everything else is removed.
Skipped (owned by F5/F6/F7): GAP15-19, GAP15-17, W2-cli5-4, W3-runner2-8, W3-runner2-7,
W3-runner5-0, gate8, lookahead, docs-web-01, probeapi-1, GAP17-33.

| id | crate | current cost | removable? | plan |
|---|---|---|---|---|
| ET-bars-candles-store-1 / -8, rederive | pull | derive_all re-reads + refolds the whole month per batch; ~103,500 reads to fill a month session by session | no (this pass) — an incremental fold needs a per-rung resume point the store format does not record; adding one is a new derived-file version, and `reconcile_derived`'s full re-proof is the only check that finds a derived conflict | keep entry; say why it is inherent to the current format |
| W1-pull2-3 | pull | derive_all on a rerun that wrote nothing | no — the rerun IS how a derivation blocked by missing schedule evidence is retried; skipping it needs the same resume point | keep |
| W1-pull2-0, W1-pull2-6, W1-api5-1 | pull/api | read_census per window / per rolling answer, O(manifest + E_v) | no — re-reading and CRC-checking every entry under the lock is the D-0036 guarantee; a generation-keyed cache cannot see bit rot (same size, same mtime) | keep, reason stated |
| W1-api5-2 | api | census_now miss reads every manifest | no — same D-0036 reason; a miss is caused by a manifest change | keep |
| W1-pull2-5 | pull | committed_cash_days O(n_m) per month per request | no — the days come from the records themselves; no per-day index exists in the bar format | keep |
| W1-pull1-0 | pull | prepare_observed_with O(D x B) per body | no — per-day receipt revalidation is the D-0519 guarantee | keep |
| W3-store1-0, W3-store1-1 | store | timestamp lookup is a bisection | no — inherent: an O(1) index would break the store format (records are not dense on the minute grid) | keep |
| ET-bars-candles-store-3 | store | off-session bar admitted (not a cost) | no — not a cost; store may not depend on pull (§5) | keep |
| R9-csr-o1-0 | store | one fstat per tail-block re-verification | no — the fstat is what detects records past the commit (D-0688) | keep |
| W3-engine1-1 | engine | sort per level O(F log F) | no — canonical order is required for idempotence and the prefix join; the log factor is bounded by the level ceiling | keep |
| ET-o1-proof-coverage-2, W3-engine1-0 | engine | subset prune Theta(k) | no — inherent: Apriori must test k-2 subsets | keep |
| o1engine-20, W3-engine1-2 | engine | keep::Best admits in O(log cap) | no — a bounded top-k under a total order needs a heap; no production caller | keep |
| GAP16-26 | runner | Edge fields f64 | no — not a cost; §7 allows statistics in f64 | keep |
| ET-strategies-trades-ranking-costs-9 | runner | stale cost text (fixed) | no — text finding, already corrected | keep |
| W3-runner2-2, W3-runner4-1 | runner/cli | attest per call in two remaining callers | yes if the caller loops per candidate — check `execution_capability.rs` and `global_replay_v2.rs` | hoist |
| W2-cli8-6 | cli | up to 64 build attempts re-read minutes | no — each retry exists because inputs changed under it | keep |
| o1cli-2 | cli | rung loads span twice, may build column twice | yes — thread the loaded span into the kernel | hoist |
| o1cli-3 | cli | 24-32 minute-span reads per all-rungs command | yes in part — shares with o1cli-2/-4 | hoist |
| o1cli-4 | cli | kernel re-reads daily + minute contexts | yes — hand the build's contexts back | hoist |
| o1cli-5 | cli | four commands load a span for one number then again | yes — pass the loaded span to the handed-off work | hoist |
| o1cli-6 | cli | two full sorts in screen | no — top-first selection is not equivalent (fallback past top rows) | keep |
| AC-whp-o1-1 | cli | stored doors retain the Sweep | no — the ranker reads the retained Sweep; streaming would change the ranked result | keep |
| cli-14 (W2-cli11-0/-1, W2-cli12-3/-4) | cli | ledger rescans per append/row | no — re-authentication of the ledger IS the read's proof (D-0934) | keep |
| D-1631 (W2-cli1-2/-3) | cli | V2/V3 lineage rescan, no rollback | yes — never had a production caller (git -S), so no stored V2/V3 file can exist; delete the modules | delete |
| D-1633 (W2-cli6-0) | cli | per-launch replay of history | no — the replay proves acknowledged children; a cache would be a new durable authority | keep |
| D-1634 (W2-cli16-2) | cli | Trades::open per recorded run, Theta(N*H) | yes — open once per process call and reuse | hold |
| D-1634 (W2-cli16-3) | cli | size_sweeper per rung loads NIFTY span | check | |
| D-1636 (W2-cli15-2) | cli | StreamFactsV1::of per cell, O(C*E) | yes if the population's bars are one object — compute once per population | hoist |
| D-1638 (W2-cli7-0) | cli | population id per cell O(E) | yes — derive once per population | hoist |
| D-1639 (W2-cli10-2) | cli | public digest entry O(G) | no — no production caller; the validated path is O(1) | keep |
| D-1641 (W2-cli2-5) | cli | page rechecks journal twice | no — the recheck is the page's proof | keep |
| D-1642 (W2-cli14-1/2/3) | cli | selection reads replay sources | no — re-authentication of upstream | keep |
| D-1494, D-1498 | core/runner | not costs | no | keep |
| W1-api2-1 | api | calendar derivation O(records) | no — derivation reads what it derives from; cached after | keep |
| W1-api2-2 | api/cli | catalog read+hashed twice per page (doc says five) | yes — closing re-check by file generation, summary held | fix |
| W1-api2-3 | api | single-slot trade reader | no — any bounded cache can be thrashed | keep |
| W1-api1-4 | api | qualified campaign O(H) per GET | yes — single slot with require_current | fix |
| W1-api1-6 | api | O(C) currency syscalls per page | no — C fixed by artifact; the checks are the proof | keep |
| W1-api3-0 | api | journal grows one file per request | no — append-only audit record; per-request cost O(1) | keep |
| W1-api3-1 | api | rows_now miss on a runtime worker | yes — spawn_blocking | fix |
| W1-api5-3 | api | instruments_json O(U+E+T log T) | check | |
| W1-api5-5 | api | calendar_json sort per request | check | |
| W1-api5-6 | api | filtered census O(E) | no — four free predicates, no index | keep |
| W1-api5-7, W1-api6-0 | api | scrub inline on async worker | cost inherent (a scrub opens every file); blocking yes — move to run_store_read | fix |
| W1-api5-8 | api | window past last bar reads month | yes — a sealed file's verified tail proves the window empty | fix |
| W1-api5-9, W1-api2-7 | api | catalogue re-read per request | check | |
| W1-api5-11 | api | spot_targets O(U) | check | |
| W1-api6-3 | api | refusal walk per request | no — a cached refusal would outlive a repair that keeps the generation | keep |
| o1api-4 | api | param scans per field | no — bounded by the capped query length | keep |
| o1api-33 | pull | decode_body Value tree | no — O(body) inherent; tree only changes the constant | keep |
| rustonly-4 | api | xdg-open | no — not a cost | keep |

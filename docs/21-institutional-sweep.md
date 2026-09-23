# Institutional sweep: Selection V6 implementation checkpoint

The `ledger-v6` command now continues from Execution V4 into a separate
Selection V6 ledger for each canonical rung. The separate `ledger-v6-replay`
command retains all eight real selections and continues into Global Replay V4
over an explicitly supplied later civil month span. Both use actual selected
prefixes, including a valid empty selection. This is a source implementation
and local fixture-test checkpoint. It is not a deployed dashboard claim, a
completed production-market run, a profitability assurance, or completion of
the repository's coverage and mutation requirements.

## What the operator gets

| Question | Current behavior | Meaning and limit |
|---|---|---|
| Does the institutional command reach selection? | `ledger-v6 → Candidate/Pre-Admission → Statistics V3 → Admission V4 → Finalization V4 → Population V6 → Execution V4 → Selection V6` | A rung is counted complete only after its saved selection is reauthenticated and both prefixes are read. A refusal stops the route and remains visible. |
| Can both families produce no candidates? | Both terminal family envelopes survive; zero rows and zero execution parameter records are valid. | Empty means the authenticated source was empty, not a missing file or a made-up successful candidate. |
| Can only one family survive? | Each active family requires exactly two execution parameter records; the total is 0, 2, or 4. | An evaluated family requires at least two candidates. A singleton or a contradictory extinction label refuses. |
| Which candidates may win? | Only a candidate whose authenticated admission is `Admitted` **and** whose exact Execution V4 disposition is `Authorized`. | A high score cannot override rejected, unmeasured, refused, or execution-policy-refused evidence. |
| Are Top 10 and Top 25 separate searches? | One common ranking produces up to 25 rows; Top 10 is exactly its actual first ten. | No padding, duplicated winners, or invented rows when fewer qualify. |
| What identifies a winner? | Complete strategy, disposition and selected-exit digests; family; global and family source sequence; mask, direction, score, and exact ranking metrics. | The terminal report abbreviates digests for readability; the binary record retains all 32 bytes. |
| Can old Selection V5 data stand in? | No. The sole production constructor requires the opaque retained Execution V4 capability. | Different file, format version and hash domains. Structural bytes alone cannot construct the authority. |
| Are unknown ratios turned into zero? | The saved option flag distinguishes `None` from `Some(0)`. | Unknown admission evidence cannot become measured evidence through projection. |
| Does a separate chronological replay path exist? | `ledger-v6-replay` consumes the new `AllRungSelectionV6` capability and mints exact later stored OOS witnesses before Global Replay V4 scheduling. | Shared global occupancy spans instruments, directions and rungs. This is replay evidence, not a final portfolio admission or live execution assurance. Global Replay V3 and its V5-only types remain unchanged. |
| What happens to an entered trade that cannot be priced? | It retains the global position until its inclusive occupied-through timestamp; it produces an explicit unpriced admission, never a money row. | A later candidate cannot enter at that same final occupied minute. |
| Does an empty replay prove market-data coverage? | No. Zero selected streams produce a complete empty schedule without loading OOS bars or VIX. | The command names this limit explicitly; requested civil dates are not a data coverage attestation. |

## Entry point and required configuration

The existing command shape is unchanged:

```text
cli ledger-v6 VENDOR FROM_Y FROM_M TO_Y TO_M SUPPORT_PPM MAX_POINTS ROOT
cli ledger-v6-replay VENDOR FROM_Y FROM_M TO_Y TO_M SUPPORT_PPM MAX_POINTS ROOT OOS_FROM_Y OOS_FROM_M OOS_TO_Y OOS_TO_M
```

Stored, attested inputs for NIFTY and BANKNIFTY must already be available under
the configured store root. The command does not fetch missing vendor data.
The existing stored-input, calendar, provenance and exact-build-identity
checks still apply. The working-tree build used for local tests reports run
persistence disabled when its tree does not equal the index; generated test
fixtures do not establish that a production run passed that boundary.

The current V6 entry point also requires the three explicit checksum settings
in [the strict input contract](24-checksum-admission.md), after resolving the37
admission choices. It uses receipt-checked sizing and independent raw
signal/minute/daily load ceilings. Source/role receipts remain retained through
Candidate, Population, Selection and later OOS/replay publication.

A family whose very first frequent frontier is empty now has a separate opaque
stored family variant carrying its real Candidate/Base/V2 Pre-Admission proof.
It does not create a zero-row V1 Pre-Admission record or invent depth1. Depth0
means no nonempty frequent level after a completed measured search; cold,
all-refused and halted inputs cannot produce this authority. Both evaluated
and empty variants freshly reopen their supporting ledgers. Empty-family
sources remain retained even when no Candidate capability is needed by the
Population binder. Explicitly counted partial bar refusals remain represented
in the source census; zero candidates is not proof of complete market data.

The replay command requires its OOS starting month to be strictly later than
the requested training ending month. It inherits the existing three validated
signal/minute/daily load ceilings from `candidate_bounds`; it invents no OOS
period or admission policy. For each family with selected winners, the
retained Candidate/Pre-Admission source loads the actual later stored cohort
and Runner checks its exact instrument, feed, commit, evaluator, grid, selected
exit and causal training boundary. Missing, partial or corrupt OOS data refuses.
VIX months for globally admitted priceable entries and exits must be readable
through the existing reference loader. An absent exact minute in a valid
reference month is recorded as absent; an unreadable/missing month refuses.

The same `ledger_all::admission_policy` resolver supplies the existing 39
institutional gates. Two come from already stated policy: the `MAX_POINTS`
loss bound and the three-to-one minimum reward/risk rule. The remaining 37
can now come from the explicit runtime research file selected by
`BRUTEX_ADMISSION_POLICY_FILE`, or from `BRUTEX_ADMIT_<UPPERCASE_FIELD>` values.
The operator delegated the research choices on7September2026; the supplied
`config/intraday-research-v1.toml` answers all37 without manual entry.
`cli policy-check FILE MAX_POINTS` explains all39 values and their units before
any market computation. See [the guide](22-research-policy.md).
Missing values still produce the full worksheet and refuse; Selection V6
supplies none of them.
The existing resolver gives explicit knobs precedence for all 39 fields,
including those two stated values, and reports each applied value's source.
An invalid override refuses instead of falling back. A selected missing,
malformed, duplicate, unknown, oversized or overflowing profile refuses before
sizing. The profile's file digest and resolved risk amount appear in applied
gate provenance; canonical admission records continue to carry the actual
resolved policy values. The delegated profile is a documented research choice,
not a pre-existing approval file or institutional certification.

The V6 route resolves this complete policy immediately after vendor validation
and before loading any market spans for support sizing. Its replay command
uses the same route after validating the explicit later OOS span. Missing
policy therefore produces its worksheet before a missing-data error can hide
it, and creates no rung authority roots. Successful policy bytes, digests,
knob precedence and downstream run identities are unchanged by this ordering.
CLI process startup still independently validates the configured store root;
this is not a claim that a missing root can bypass that startup check.

None of the remaining 37 acceptance thresholds is inferred from machine
capacity or measured market outcomes. In particular, the search's per-rung
support count derived from `SUPPORT_PPM` and stored NIFTY row counts does not
supply the independent admission `min_support_hits` gate. The procedure's
1,000 bootstrap draws, seed 1 and block length 2 describe how it computes
evidence; they do not choose an acceptable minimum for that evidence.

| Required field group | Exact field suffixes, before conversion to uppercase |
|---|---|
| Sample and support | `min_support_hits`, `min_independent_sessions`, `min_trades` |
| Win, loss and return | `max_mae_paisa`, `min_win_rate_ppm`, `min_wilson_win_rate_ppm`, `min_return_drawdown_ppm`, `min_weakest_period_return_paisa`, `max_drawdown_paisa`, `max_losing_trade_rate_ppm`, `max_losing_trades`, `min_pessimistic_profit_paisa`, `min_winning_trades`, `min_average_win_paisa`, `max_average_loss_paisa`, `min_profit_factor_ppm`, `max_consecutive_losing_streak`, `min_consecutive_winning_streak` |
| Statistical and out-of-sample evidence | `max_pbo_ppm`, `max_fwer_p_value_ppm`, `max_spa_p_value_ppm`, `min_decided_folds`, `min_bootstrap_draws`, `min_bootstrap_strategies`, `min_bootstrap_periods`, `min_pbo_contributing_folds`, `max_pbo_unrankable_folds`, `min_profitable_oos_folds`, `min_oos_pessimistic_return_paisa`, `max_white_reality_p_value_ppm`, `require_white_reality_rejection`, `max_romano_wolf_p_value_ppm`, `require_romano_wolf_rejection` |
| Fill, gap and concentration | `max_ambiguous_fill_rate_ppm`, `max_gap_affected_rate_ppm`, `max_session_concentration_ppm`, `max_largest_trade_profit_share_ppm` |

Selection uses the existing `runner::topn` ranking algorithm and
`RankingPolicyV1::new(Weights::equal())`, matching the existing `ledger-all`
choice. Ranking weights are not new admission thresholds. The existing exit
grid policy and source evidence are unchanged by this selection stage.

## Durable contract

`selection_v6::commit_stored_selection_v6(root, bounds, execution, policy)` is
the only production commit door. `execution` is a
`CommittedStoredExecutionV4`, not a detached receipt or caller-authored row.
Its Selection V6 handoff freshly authenticates the retained Population V6
chain, reopens Execution V4, reproduces its canonical disposition records,
and checks exact joins before this module ranks them. The new module checks
the family/cardinality envelope, complete disposition matrix, direct metrics
against authenticated admission arithmetic, and complete ranking counters.

`ROOT/selection/<rung>/global-selection-v6.bin` is append-only. Each record is
16,384 bytes, little-endian, with this fixed layout:

| Bytes | Content |
|---|---|
| 0–15 | `BTX-SELV6-BLOCK` plus a zero byte |
| 16–19 | Version 6 as `u32` |
| 20–23 | Reserved zero bytes |
| 24–55 | Semantic selection identity |
| 56–879 | Exact upstream identities, ranking-policy digest, rung/horizon, parameter/percentile/count metadata, admission counts, 16-cell family/admission/execution matrix, and both family terminal envelopes |
| 880–959 | Complete considered/admitted/refused/unmeasured counts, actual Top-25/Top-10 counts, and ordered population-ranking digest |
| 960 onward | Actual winner records, 296 bytes each, at most 25, followed by zero padding |
| 16,352–16,383 | Completion seal |

The identity hashes the complete fixed payload except its own identity slot,
under `brutex-selection-v6-semantic-block`. The completion seal hashes the
payload including its identity under `brutex-selection-v6-completion-seal`.
The zero terminators in both hash domains are part of the format. The payload
is appended and synced before the completion seal is appended and synced.
The directory is synced before acknowledging either a new or reused record.
These are filesystem durability operations, not a claim about surviving every
storage-device or operating-system fault.

A writer takes an exclusive file lock. Reuse compares exact bytes and appends
nothing. A torn final prefix can be resumed only when every existing prefix
byte matches the requested record; resume appends only the missing suffix.
It never truncates or overwrites a foreign prefix. A reader takes a shared
lock, requires a complete record, checks all retained completion seals and
duplicate identities within the explicit limit, and compares the requested
record with the record freshly reproduced from its retained upstream source.
Read-path file generations are checked before and after the scan. Symlinks,
non-regular files, and Unix hard-link aliases refuse. A missing, corrupt,
different or incomplete record does not yield an empty successful selection.

The CLI physical ceiling is 64 GiB per Selection V6 file, at most 4,194,304
fixed records. Other constructor users must supply explicit nonzero bounds.
An allocation or ceiling refusal is visible; exceeding a bound does not trim
the candidate universe or alter an admission threshold.

## Global Replay V4 publication

`all_rung_selection_v6::AllRungSelectionV6` consumes eight real retained
Selection V6 capabilities. It validates the canonical 60/120/180/300/600/900/
1,800/3,600-second order, distinct source identities, and actual prefixes.
All eight sources are reauthenticated before the first OOS witness is minted,
again after all witnesses are ready, and around publication. The Population
V6 and Execution V4 forwarding methods exact-match requested strategy digests
to admitted, execution-authorized live dispositions and retain the stored OOS
cohort identity alongside Runner's opaque witness. Caller-written trade rows,
feed names or exit digests are not a substitute.

The scheduler is the shared `runner::portfolio::GlobalSinglePositionV1`.
Candidates are sorted chronologically; a minute holds at most 200 streams.
The existing priority/strategy tie-break is preserved. Every offered candidate
gets one decision, including occupied and simultaneous rejections. Pricing
refusals still occupy the global position. Only globally admitted priceable
decisions contribute pessimistic/optimistic money totals. Candidate quality
counts remain exact in the records; no additional portfolio admission policy
is silently invented at this stage.

Aligned Runner columns already place a completed condition at its delayed
fill coordinate. Consequently an authenticated replay candidate may have
`signal_bar == entry_bar` and equal timestamps. V4 accepts that existing
representation while rejecting reversed coordinates and retaining Runner's
OOS and alignment authority. A real stored-witness regression caught and
corrected an initially stricter check that wrongly refused this valid case.

New publications are stored at
`ROOT/global-replay-v4/<64-hex-publication-id>.bin`. They have fixed 1,024-byte
records: eight-byte magic (`BTXGRV4` followed by a zero byte), a `u64` kind, a `u64`
sequence, the kind's fixed payload and zero padding, then a 32-byte record
seal. All integers are little-endian; money is integer paisa.

| Kind | Saved fact |
|---|---|
| 0 | V4 header, fixed stride, eight-rung/25-per-rung shape and explicit requested OOS civil dates |
| 1 | Exactly eight complete Selection V6 source/ranking envelopes, including empty terminal families |
| 2 | One actual selected stream: strategy/disposition/exit identity, stored cohort and witness identities, OOS run/universe, full mask, ranking metrics, canonical feed and direction |
| 3 | Every exact pre-exclusivity candidate: source/entry/occupied-through coordinates, explicit pricing path, and complete price/quality evidence when available |
| 4 | Every chronological global decision, linked to the exact stream/candidate and carrying the occupancy/simultaneous/refusal outcome |
| 5 | Exact-or-absent entry/exit VIX stamps, only for globally admitted priceable decisions |
| 6 | Last completion: economic replay and publication identities, complete counts, occupancy counters and exact `i128` aggregate money |

The economic identity hashes kinds 0–4 under
`brutex-global-replay-v4-economic`; the publication identity additionally
includes kind 5 under `brutex-global-replay-v4-publication`. VIX changes
publication reference bytes without changing economic decisions or identity.
The whole-file digest includes the completion under its own
`brutex-global-replay-v4-complete-file` domain. All domains include a final
zero byte. The final 32-byte completion seal is appended only after every
preceding byte is synced, then the file and directory are synced. Exact reuse
and an exact torn-prefix resume do not overwrite history. Foreign prefixes,
changed lengths, wrong versions, invalid seals, non-regular files, symlinks
and Unix hard-link aliases refuse. A fresh audit checks every record and the
expected whole-file digest with filesystem generation checks around the read.

The current command bounds each publication to 64 GiB, expressed using the
record type's own 1,024-byte stride. Before replay it reserves the fixed header,
eight source envelopes, completion and actual selected-stream records. The
remaining record ceiling gives a conservative aggregate candidate ceiling of
half the remaining records, since every candidate requires both its observation
and its scheduler decision. Each retained witness deducts from that ceiling
before the next one is retained. One witness is still minted under the existing
stored-input ceilings before its actual candidate cardinality is known; VIX
records can also cause a later physical-cap refusal. No refusal publishes a
partial completed replay. Preparation and source loading may refuse
earlier under their explicit limits. This is a source implementation and local
fixture result; no new service deployment or browser presentation is claimed.

Before OOS loading, `global_replay_v4_lifecycle` durably starts a shared
`sweep_evidence` attempt with the additive `global-replay` operation
(discriminator 8). Its plan identity binds the eight authenticated Selection V6
identities/envelopes, explicit OOS dates, all three stored-input ceilings and
the physical publication bounds. This is labelled a plan identity, not an
OOS execution RunId: the exact OOS bytes have not been loaded yet.

The stored cohort then records its exact cohort/evaluator fold identity before
evaluating an input bar. The evaluator fingerprint comes from its canonical
token on an empty column, not a second encoding or a sampled input. The common
mint boundary calls a fallible publisher with the exact `ExecutionRunV1` RunId
immediately before trade replay. A publisher refusal prevents that computation.
Fold completion, per-strategy replay completion and whole-plan completion
remain separate persisted attempts. The per-strategy stream uses its own
`global-replay-stream` operation
(discriminator 9), so its Completed state cannot be confused with the
`global-replay` plan's Completed state by reading the operation field.
Only successful publication, source
reauthentication and fresh replay audit permit the whole-plan Completed
terminal. Ordinary errors finish the active stage and parent as Refused; a
failure to save that refusal is also returned. An abrupt process death can
leave a start without a terminal, which remains incomplete rather than being
inferred complete. Existing unobserved mint callers retain their original
contract. The durable attempt files live under the replay output root's
`results/sweep-evidence-v1` directory; structural telemetry additionally links full parent
and child identities and attempt tokens when the configured sink is available.

## What was tested and what remains

The first focused run passed 12 Selection V6 tests with no failures or ignored
tests. The log is `/private/tmp/brutex-selection-v6-tests-20260906.log`.
It covers a genuine retained Population V6 → Execution V4 → Selection V6
fixture, wrong replacement and deletion after authority creation, exact
reopen/reuse/append, duplicate/version/partial/foreign-prefix attacks, physical
ceiling overflow, links and non-regular files, all 16,384 single-byte corruption
positions, and exact empty/short/full ranking counts. The family projection
tests enumerate all four active/extinct combinations and the terminal/count
cross-product, including singleton and overflow refusals. The genuine full
upstream fixture in this new suite has **both families evaluated**; the mixed
and all-extinct cases here are bounded projection tests, not claimed full
market fixtures.

The focused Selection V6 rerun passed 12 tests after the directory-sync fix
(`/private/tmp/brutex-selection-v6-tests-final-20260906.log`). The ledger route
regressions passed seven tests (`/private/tmp/brutex-ledger-v6-tests-20260906.log`).
Global Replay V4 passed seven tests, including the nonempty real stored OOS
witness/scheduler test, cross-family/direction/rung conflicts, inclusive unpriced
occupancy, empty schedules, order-independent ties, duplicate/overwide-minute
refusals, VIX identity separation, every byte of one fixed record, fresh reuse,
sealed foreign replacements, and torn-prefix resumption
(`/private/tmp/brutex-global-replay-v4-tests-20260906.log`). The new all-rung
topology test enumerates reordered/duplicate/zero-ID slots. The current-source
Selection V6 plus all-rung rerun passed **13 tests** with no failures or ignored
tests (`/private/tmp/brutex-selection-v6-all-rung-tests-20260906.log`).
The current-source Global Replay V4 rerun passed **11 tests** with no failures
or ignored tests (`/private/tmp/brutex-global-replay-v4-complete-tests-20260906.log`).
The four added cases prove that plan identity changes with every source slot,
dates and input/output limits; durable parent/child starts precede work and a
retry retains the prior refusal; missing persistence and invalid stage order
cannot return success; and a real stored witness propagates a failing callback
before each calculation and records its actual execution run identity. The
current ledger regression rerun passed **seven tests** with no failures or
ignored tests (`/private/tmp/brutex-ledger-v6-final-tests-20260906.log`).
A full eight-rung production-market replay
has not been run by this task. It still requires the operator's actual policy,
stored training/OOS data, reference evidence and valid build identity.

The later fail-fast regressions
`ledger_v6::tests::missing_policy_refuses_before_ledger_v6_market_sizing` and
`ledger_v6::tests::missing_policy_refuses_before_ledger_v6_replay_market_sizing`
check the exact 37-of-39 refusal, every unresolved variable name, persisted
gate-refusal counts, no sizing events and no created authority roots. Their
focused execution log is
`/private/tmp/brutex-ledger-v6-policy-preflight-tests-20260906.log`: all nine
ledger V6 tests passed, with zero failures or ignored tests, in 0.02 seconds
after a 1-minute 4-second compile. The final combined verifier remains
authoritative for the complete source snapshot.

Strict CLI Clippy initially found local style diagnostics, now repaired;
other concurrent modules were still changing.
The final shared verification report remains the authority for current gate
results. No coverage percentage, surviving-mutant count, maximum load, real
market throughput, deployed UI, or production replay result is claimed here.

Authentication, ranking and file scans depend on input size. Ranking retains
candidate projections and the common selector's state; the on-disk winner
block has fixed capacity. A cold/read verification scans bounded retained
history and upstream files, and its duplicate index grows with that history.
Hash indexes do not provide adversarial worst-case constant time. Retained
history grows, and filesystem synchronization has variable latency. These
costs cannot honestly be described as universal O(1) time or O(1) total space.

The shared invariant and decision ledgers are updated by the coordinating
task; this checkpoint document does not replace their authority.

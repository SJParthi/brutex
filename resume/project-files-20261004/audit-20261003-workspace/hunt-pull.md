# hunt-pull: crates/pull/src at 1087e54

**Verdict.** I read the following line by line: fold/calendar/session/gaps/tenor/rolling, the CSV and JSON vendor decoders (column, object and positional shapes, timestamps, prices, counts), rate/governor, the masters fetch retry ladder, and the credential types (Secret, Credential, AwsIdentity, SsmError, config parser). The core semantics hold. Fold OHLCV is correct: open is the first row in file order, close the last, high the max, low the min, volume a checked sum, and OI the last non-null. The open anchor is right. Session edges are right too: the close is exclusive, 15:29 is the last 1-minute bar, and Window.to is inclusive and agrees with `scheduled_minutes`. Timestamp text is strictly shaped. Rupee snapping refuses sub-paisa values that are not zero. The retry loops are bounded and the rate math is sound. No secret reaches Debug or Display on the shipped types. I found **two silent-wrong-data defects**, both proven by a probe. The first is in the gap audit, which still uses the 375-minute session for F&O after the 3 Aug 2026 extension to 15:40, so a lost 15:30–15:39 tail is reported as "outside-window" with lost=0. The second is that the rolling (expired-options) decoder lacks the "a non-zero value that snaps to zero is refused" guard the intraday path has, so `0.004` and `-0.004` are stored as a clean price of 0. I also found one low-severity diagnostic-volume issue: exceptional sessions get one line per bucket. Rolling volume and timestamp zeroing are already DOCUMENTED (D-0952). Several prior findings are confirmed FIXED.

## Findings

| id | sev | crate | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| hunt-pull-1 | medium | pull (served by api) | crates/pull/src/gaps.rs `classify_with_subject` (the `DayKind::Open(session)` arm); caller crates/api/src/server.rs:3137 | The gap audit for NSE F&O series uses the calendar's fixed 375-minute `Session::full()` (09:15–15:29), not the venue's dated hours. From 2026-08-03, `NSE_DERIVATIVES_SESSIONS` (vendor.rs:1504-1510) closes at 15:40, which makes 385 minutes. A month where the vendor dropped 15:30–15:39 is reported as having no loss: those minutes are classed `OutsideWindow`, and `expected` undercounts by 10 a day. `fold::complete_minutes_for_venue` gets this right on the same data, so the two audits disagree. | Probe on 2026-08-04 (epoch day 20669), `NseDerivatives.hours_on` = (555, 940). **A**, all 385 minutes: `held=385 expected=375 lost=0`. **B**, 15:30–15:39 missing: `held=375 expected=375 lost=0 gaps_near_close=[Gap{from:930,to:1439,reason:OutsideWindow}]`. Fold on the same short day: `observed 375, scheduled 385 ... withheld`. Only cash gets venue hours (`cash_kind` → `fold::minute_session`), and server.rs:3133-3137 routes F&O to `classify_against`. | NEW |
| hunt-pull-2 | medium | pull | crates/pull/src/rolling.rs `fn paisa` (Rupees arm, `from_rupee_text_half_up(&cell.to_string())`) | The rolling decoder for Dhan expired F&O snaps any price under half a paisa to 0 with no refusal, and that includes negative values. http.rs `one_price` (≈1420-1445) refuses exactly this case ("a non-zero price that snaps to zero is refused"), with the reasoning that zero is a legal stored price, so a fabricated zero cannot be detected downstream. The rolling path has neither that guard nor the below-zero guard at the boundary. `-0.004` becomes 0, so it also skips the store's negative check. | Probe with `rolling::read` and the Dhan by-offset spec, `PriceScale::Rupees`: `0.004 -> ACCEPTED open=0 close=0`; `-0.004 -> ACCEPTED open=0 close=0`. | NEW |
| hunt-pull-3 | low | pull | crates/pull/src/fold.rs `complete_minutes_with_calendar`, the `if exceptional { diagnostics.push(...) }` arm | o1api-44 / D-1201 collapsed per-bucket lines into one per day for Unmeasured days only. Exceptional sessions (outage day, DR Saturdays, Muhurat) still push one identical "exceptional session ... withheld" line per bucket, and each line becomes a `pull.derive` warning and a clause of the rung refusal. | Probe on 2024-03-02 (day 19784, two windows, 105 minutes) at a 2-minute bucket: `complete=0 diagnostics=54`. Every line is the same sentence, differing only in the bucket start. | NEW (same class as o1api-44) |
| hunt-pull-4 | info | pull | crates/pull/src/rolling.rs `fn number`, used for `volume` and `timestamp` | A rolling volume of `null` or `"abc"` becomes 0, `12.7` becomes 12, and `-5` passes through to the store. A null or string timestamp becomes 0 (1970). | Probe: `vol null -> volume=0`, `vol 12.7 -> volume=12`, `vol "abc" -> 0`, `vol -5 -> -5`; `ts null -> ts_micros=0`. A ts of 0 is later refused loudly by `rolling_key`'s `day_of` at api server.rs:13763. A volume of -5 is refused loudly at the store. Null/garbage volumes and the truncation of 12.7 are silent. | DOCUMENTED: docs/05-decisions.md D-0952 ("Not changed. Volume and timestamp cells still go through `number`") |
| hunt-pull-5 | info | pull | crates/pull/src/http.rs `decode_positional` (`serde_json::Value::Null => 0` for volume) | On the Groww/Kite positional shape, a null volume becomes 0 with no counter. The CSV path counts the same substitution (`unreadable_volume`). | Code read. | DOCUMENTED: docs/05-decisions.md ≈25160 ("The positional arm skips a null price and zeroes a null volume") |

## Re-verification of prior pull rows

| prior id | at HEAD | status |
|---|---|---|
| probestore-1 (csv time accepts +/-) | csv.rs:403 `digits()` requires every byte ASCII-digit; `ist_seconds` and `day_of` use it | FIXED-since-prior |
| probestore-2 (csv date accepts +) | same `digits()` in `day_of` | FIXED-since-prior |
| o1api-34 (csv allocates per row, not pre-sized) | csv.rs `fields_of` returns `[&str; MAX_FIELDS]`; `decode_rows` uses `Vec::with_capacity(bound)` | FIXED-since-prior |
| o1api-39 (calendar cost note names one walk) | calendar.rs:478 test pins "five-element LENGTH_UNMEASURED … four-element IRREGULAR … at most nine comparisons" | FIXED-since-prior |
| o1api-44 (per-bucket diagnostics on unknown calendar) | fold.rs: `if calendar == Unmeasured { continue; }` plus `day_note` once per day | FIXED for Unmeasured; still open for exceptional days (hunt-pull-3) |
| o1api-54 (governor admit loop races) | http.rs:640-668 `wait_for_permit` uses `Governor::reserve` once under the lock and sleeps to the reserved instant; no loop | FIXED-since-prior |
| D-0xxx "vendor error under HTTP 200 raises allowance" | http.rs:2542 `weigh_refused_body(&body, status)` on the success path | FIXED (code read only) |

## Checked and clean (with the reason)

- Fold (fold.rs:152-300): the bucket uses `div_euclid` with an open anchor, which is 09:15 for intraday rungs and IST midnight for daily. Volume uses `checked_add`. Out-of-order input is refused. OI null is not allowed to overwrite.
- Session edges: `Window::verdict` (session.rs:1084-1145) drops `minute < open` and `minute >= close`, with close exclusive, so the 15:30 bar is dropped. calendar `LAST_MINUTE=15:29` is inclusive and `scheduled_minutes` uses `to+1`, which is consistent. IRREGULAR rows checked against the known events: 2021-02-24 outage 09:15–11:39 and 15:45–16:59; DR 2024-03-02 and 2024-05-18; Muhurat 2025-10-21 13:45–14:44.
- Timestamps: `local_seconds` (http.rs:3486) requires exactly 19 bytes and range-checks h/m/s. `stated_offset` bounds hours at 14 or less and minutes at 59 or less. Broker stamps must be on the grid (ingest.rs:1141).
- Numbers: `csv::paisa` refuses "", ".5", "--5", and a third decimal. `one_number` refuses `i64::MIN`, values past i64, and fractions. `1e5` as JSON is rendered "100000.0", which is accepted as a whole number and is correct.
- Retry and hang: the masters fetch is bounded by `ATTEMPTS_PER_URL` with one re-prime per URL (masters.rs:1430-1497). The body read is capped by `MAX_BODY_BYTES` per chunk. The governor reserve returns `None` on saturation and does not loop.
- Credentials (§8): `Secret`, `Credential` and `AwsIdentity` have hand-written redacting Debug impls. SSM refusal text quotes only closed fault tokens, never the body. The config has no default, refuses duplicate and unknown keys, and refuses a region other than ap-south-1. `Signable` derives Debug and holds `session_token: Option<&str>`, but I found no formatting call site, so I checked it and found no leak today. The rate day-ceiling being forgotten on restart is DOCUMENTED at docs/06-limits.md:7004.
- Path traversal: `path_safe` (http.rs:3439) allows unreserved bytes only and refuses `.` and `..` as whole segments.

## Hot-path table (pull loops touched)

| path | file:line | unit | cost | verdict | note |
|---|---|---|---|---|---|
| fold bucket | fold.rs:260-330 | one snapshot | arithmetic plus `last_mut` | O(1) | |
| complete_minutes per bucket | fold.rs:821-940 | one candidate bucket | minute_session plus at most 2 window intersections, cursor advance amortised over the bars | O(1) amortised | exceptional days emit one String per bucket (hunt-pull-3) |
| gaps classify | gaps.rs `classify_with_subject` | one day | walks 1440 minutes per day | bounded (constant 1440 per day) | per-day constant, not per bar |
| calendar kind_of | calendar.rs:392 | one day | at most 9 comparisons | O(1) | |
| governor reserve | rate.rs:900-930 | one request | 3 windows | O(1) | |

## Probe

`crates/pull/tests/zz_audit_hunt-pull_1.rs` was run with `cargo test -p pull --test zz_audit_hunt-pull_1 -- --nocapture`. Result: 3 passed, and the output is quoted verbatim above. The file has been deleted, and `git status --porcelain` lists no hunt-pull file.

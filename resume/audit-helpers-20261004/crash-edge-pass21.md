# Crash/edge pass 21 (ce21): HTTP protocol edges on the api server

Head: `1f4de71` (`/home/claude/wt/zero3`). IDs CE-99 upward. Read from source only. No cargo run; every repro below is "not run". Dependency behaviour was read from the locked sources: axum 0.8.9, http 1.5.0, hyper 1.11.0.

Scope: `admitted()` (server.rs:16191-16217) and its layers, in request order:
- `never_framed`
- `DefaultBodyLimit`
- `logs::note_request`
- `same_origin_writes_only`
- `within_request_bounds`
- `one_value_per_form_field`

Then `route_table` (server.rs:16505-16845, 63 routes, all `get` or `post`), the asset fallback (`assets::Assets::respond`), and the query and body readers (`param`, `params`, `percent_decode`, and the serde JSON bodies).

Not re-reported:
- P5-05 and P5-06: form-guard allocation, and the Content-Type/`{` bypass.
- P1-03-1: 100-continue rearms the head clock.
- P11-01 to P11-04 (Host and Origin security).
- The pass-14 body rows.
- The "absent or empty `feed` means Dhan" default (P1-02 context, pass-8 table).

## New findings

### CE-99 (low): an unrouted non-GET answers 405 with no `Allow` header, and calls a wrong path a wrong method
- **Where:** crates/api/src/assets.rs:810-816. This is reached from the router fallback at server.rs:16840-16844.
  ```rust
  if method != axum::http::Method::GET && method != axum::http::Method::HEAD {
      return answer(StatusCode::METHOD_NOT_ALLOWED, "text/plain; charset=utf-8",
          format!("{method} is not a method the front end answers\n").into_bytes());
  ```
- **Why it is wrong:**
  - Every POST, PUT, DELETE, OPTIONS or CONNECT to a path that has no route lands here once origin admission passes. That includes `/pull/spot/` (trailing slash), `//backtest/run` (double slash), `/backtest/runn` (typo) and `/backtest` (the front-end path). Each gets 405 "POST is not a method the front end answers".
  - The real fault is the path, and no resource exists there to have methods. So the honest answer is 404, which names the path, as `not_found` already does for GET.
  - RFC 9110 §15.5.6 says an origin server MUST send `Allow` with a 405. axum's own 405 for routed paths does: `append_allow_header`, method_routing.rs:847. This one sends none.
  - An operator or script that posts to `/pull/spot/` is told to change the method, which can never succeed (CLAUDE.md §4: name the reason).
- **Repro (not run):** send `curl -X POST -H 'Origin: http://127.0.0.1:8080' -H 'Host: 127.0.0.1:8080' http://127.0.0.1:8080/pull/spot/ -d 'vendor=dhan'`. Expected: `405`, the body "POST is not a method the front end answers", and no `allow:` header. The same request without the slash reaches `pull_spot`.
- **Minimal fix:** for a non-GET/HEAD in `respond`, answer 404 with "no route {method} {path}". If 405 is kept for the shell paths, add `Allow: GET, HEAD`.

### CE-100 (low): a trailing-slash, double-slash or case variant of a server route answers 200 with the front-end shell. `/health/` is always 200, whatever the health state
- **Where:**
  - crates/api/src/assets.rs:847-850 (`if looks_like_asset(&segments) { return self.not_found(raw_path); } Self::shell(root)`)
  - `segments` drops empty segments (assets.rs:290-292)
  - `health` is at server.rs:5075
  - axum 0.8 does no trailing-slash redirect
- **Why it is wrong:**
  - `/health` answers 503 when the universe read is degraded. Its doc says this is "because a monitor reads the status code and nothing else" (server.rs:5070-5074).
  - `/health/`, `//health` and `/Health` match no route. Their last segment has no `.`, so `respond` serves `index.html` with 200 `text/html`.
  - A monitor configured with any of those spellings reads "healthy" for ever. The same applies to `/dashboard/`, `/pull/`, `/store/`, `/logs/`, `/masters/`, `/instruments/`, `/bars/` and `/audit/page/`: a non-browser client gets 200 and an HTML shell instead of the server-rendered page or a refusal. A browser only sees the SPA's client-side 404.
  - `.json` variants are safe, because the `.` makes them a 404.
- **Repro (not run):** start `api serve` with a degraded master read. `curl -o /dev/null -w '%{http_code}' 127.0.0.1:8080/health` prints `503`. The same command against `/health/` prints `200`.
- **Minimal fix:** in the fallback, before `respond`, strip one trailing `/` and collapse `//`. If the normalised path is a registered server route, answer `308` to it, or at least 404. Alternatively, make the shell refuse any first segment that is a server route (`health`, `dashboard`, `pull`, `store`, `logs`, `masters`, `instruments`, `bars`, `audit`).

### CE-101 (low): a malformed or truncated request body is refused as "larger than N bytes"
- **Where:** crates/api/src/server.rs:16403-16416 (`one_value_per_form_field`)
  ```rust
  let Ok(bytes) = axum::body::to_bytes(body, bound).await else {
      return (StatusCode::PAYLOAD_TOO_LARGE, ..., format!("REFUSED — the request body is larger than the {bound} bytes this server reads. ..."))
  ```
- **Why it is wrong:**
  - `axum::body::to_bytes` (axum-0.8.9/src/body/mod.rs:48-54) is `Limited::new(body, limit).collect()`. It errors on the length limit and on any transport body error.
  - Hyper yields a body error for a bad chunk-size line in `Transfer-Encoding: chunked`, a chunked body cut off before its `0\r\n\r\n`, or a `Content-Length` body whose peer half-closes early.
  - Every one of those becomes `413` with a sentence claiming the body exceeded 8,192 bytes, or 27,347,836 on `/pull/run`, even for a 10-byte body. `note_request` logs it as a 413 too.
  - The refusal is loud, but it names the wrong reason (§4). The right distinction is already available: the error's source is a `LengthLimitError` only in the first case.
- **Repro (not run):** POST `/autopilot/control` with valid Host and Origin, `Transfer-Encoding: chunked` and the body `zz\r\nx\r\n0\r\n\r\n`. Expected: `413` "larger than the 8192 bytes".
- **Minimal fix:** match on `std::error::Error::source(&err)`:
  - `is::<http_body_util::LengthLimitError>()` → 413 as now
  - otherwise → 400 "the request body could not be read: {err}"

## Case table (all route families; outcome at 1f4de71)

| Case | Outcome | Class | Evidence |
|---|---|---|---|
| HEAD on any `get` route | The handler runs, and axum strips the body (route.rs:169). `/store.json` short-circuits HEAD with no census encoding and no Content-Length (server.rs:4935-4943, 5023-5025). A journaled HEAD is journaled as "HEAD …" and refused cross-site. | held | operation_audit.rs:113-120, server.rs:17053-17068 |
| HEAD on a `post` route | Treated as a read for admission, then axum 405 with `Allow: POST`. | held (clear) | method_routing.rs:847 |
| OPTIONS / PUT / DELETE / PATCH, no Origin | 403 "Origin says … <missing>" before routing. The wording blames origin, not method, but it is a clear refusal. | held | server.rs:16993-17045 |
| Same, with admitted Origin, routed path | axum 405 with `Allow`. | held | |
| Same, unrouted path | 405 with no Allow | **CE-99** | assets.rs:810-816 |
| CONNECT (authority-form, path `""`) / `OPTIONS *` | 403 at admission, then a fallback 405 with no Allow. No panic. | held / CE-99 | http uri/mod.rs:439-445 |
| Trailing slash `/x.json/` | Fallback, then 404 "no such asset". | held (clear) | assets.rs:847-848 |
| Trailing slash, extensionless `/health/`, `/pull/` | 200 SPA shell | **CE-100** | assets.rs:850 |
| Double slash `//x.json`, `/bars//window.json` | 404. `//health` gives 200 shell (CE-100). | held / CE-100 | assets.rs:290-292 |
| Percent-encoded path `/bars%2Ejson`, `/backtest%2Ejson` | No route match (matchit matches the raw path), so 404. The journal is not bypassed, because no handler runs. | held | assets.rs:212-226 |
| Repeated query key `?feed=a&feed=b`, `feed=a&feed` | 400 "names \"feed\" more than once" | clear refusal | server.rs:16316-16328, 16452-16466 |
| Encoded-key "repeat" `feed=a&fe%65d=b` | Not a repeat. `param` matches keys undecoded, so only `feed` is read. | held (consistent) | server.rs:800-809 |
| Repeated list field in a body (`member`, `leg`) | Allowed. `cash_identity` and the recovery fields are refused as repeats, then refused again by their readers. | held | server.rs:16334, ingest.rs:1003-1015 |
| Empty value | `feed=` = Dhan (known). `encoding=` = expanded. `contract=` = none. `page=`/`limit=` = default (view only). Hex identities are refused by the reader. `cash_identity=` is refused. | known default / held | ingest.rs:1568-1571 |
| `+` and `%XX` in values | Decoded once. A malformed `%` is kept literally. `M%26M` survives as `M&M`, and an unencoded `M&M` reads `M` and is refused as an absent file. | held | server.rs:847-871 |
| Non-UTF-8 escaped (`%FF`) | Lossy U+FFFD. Every data-bearing reader then refuses: `check_segment` (store/path.rs:1040-1045), `parse_vendor`, `hex_param`. The doc's "never silently a different query" is over-stated, but nothing accepts it. | held | server.rs:870 |
| Non-UTF-8 raw bytes in target | http 1.5 `from_utf8`, so hyper 400 before routing (no app headers or log) | clear refusal | http uri/path.rs:32-36 |
| Huge query | 414 past 8 KiB; hyper refuses past 65,534 | clear | server.rs:16297-16306 |
| Range header (assets, shell, JSON) | Ignored; 200 with the full body and no `Accept-Ranges` (permitted by RFC 9110) | held | assets.rs:835-846 |
| If-None-Match | Honoured only on `/store.json`, only on 200 (not 503), `*` ignored. Everything else has no validator, so it always gets 200. | held | server.rs:4904-4910, 5059-5065 |
| If-Modified-Since | Ignored everywhere. No route sends `Last-Modified`, so there is no heuristic browser freshness. | held | grep: no LAST_MODIFIED |
| Cache headers on JSON | None, except `/inspection.json` `no-store`. With no Last-Modified or Expires, fetch revalidates, so no stale JSON is shown on normal navigation. `_app/immutable/*` is a year (content-hashed). Back/forward history may show a stored page; that is browser behaviour, not a finding. | held | assets.rs:839-844, server.rs:16155 |
| Content-Encoding gzip on POST | Not decoded. Gzip bytes are never UTF-8 (0x8b), so the `String` extractor gives 400 "invalid UTF-8". Loud; the encoding is not named. | clear refusal | |
| Chunked, no length | Bounded by `to_bytes(body, form_read_bound)`. Malformed chunked gives 413 with the wrong reason. | held / **CE-101** | server.rs:16403 |
| TE + CL both, bad CL | hyper 400 | clear | |
| Expect: 100-continue | Origin check runs before any body poll, so a refused request never gets 100. The rearm is P1-03-1. | held / known | |
| Body on GET/HEAD | The form guard skips it, no GET handler extracts a body, and hyper discards it | held (ignored, RFC-permitted) | server.rs:16396-16401 |
| JSON body with BOM | serde "expected value at line 1 column 1", refused by the route's named refusal (`sweeprun.rs:711`, `booleanlaunch.rs:107`, `indexstoplaunch.rs:79`) | clear refusal | |
| Content-Type missing or wrong on POST | No handler checks it. JSON routes parse any type. Form routes parse any type, and a json-labelled form skips the dup guard (P5-06). | known (P5-06) | server.rs:16418-16422 |
| Panic route in any case above | None found. No `unwrap` or indexing on header, query or body bytes. `hex_escape` max is 255. | held | |

## Verification of earlier rows in this theme

| Row | Verdict | Evidence |
|---|---|---|
| probeapi-5 / D-1202 (repeated query key) | FIXED | server.rs:16316-16328 `repeated_query_key` |
| hunt-api-5 / D-1587 (repeated form key) | PARTIAL | server.rs:16425-16444; bypassed per P5-06 |
| P5-05 (`&`-sized HashSet) | NOT FIXED | server.rs:16342-16347 `with_capacity(body.bytes().filter(|b| *b == b'&').count()...)` |
| P5-06 (json sniff bypass) | NOT FIXED | server.rs:16418-16422 `contains("json") \|\| matches!(…first(), Some(b'{' \| b'['))` |
| P1-03-1 (100-continue rearms head clock) | NOT FIXED | server.rs:17463, :17476 `this.rearm()` |
| D-1202 target and header caps (o1api-3) | FIXED | server.rs:16234, 16251, 16293-16315 |

Tally: 2 FIXED, 1 PARTIAL, 3 NOT FIXED.

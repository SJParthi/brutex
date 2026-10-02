# probeapi — adversarial probing of `api` (HTTP) and `cli`

Commit bc53131. Binaries were built with `cargo build -p api -p cli` (debug), giving `target/debug/api` and `target/debug/cli`.
Environment: `HOME`, `BRUTEX_STORE` and `BRUTEX_MASTERS` all pointed at an empty tree under `scratchpad/probeapi/`, with `BRUTEX_NO_OPEN=1`. The server ran with `api serve 127.0.0.1:18777`. Autopilot stayed PAUSED, nothing touched the network, and no POST that pulls data was called.
The server was stopped with SIGINT and exited normally with code 3 because the universe was DEGRADED. `ps` afterwards shows no `api` or probe processes. `git status --porcelain` shows nothing of mine; the two `zz_audit_*` files belong to other workers.

## Findings

| id | sev | crate | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| probeapi-1 | medium | api | `crates/api/src/server.rs:16115-16127` (`serve` = bare `axum::serve`, no timer, so hyper has no `header_read_timeout`) | **No read timeout on a connection (slowloris).** A client can send part of a request and then go silent, and the server holds that connection open with no limit. 19,990 such sockets used up the whole fd limit (20,000), and a well-formed request then got no answer. | A single partial request (`GET /health HTTP/1.1\r\nHost: x\r\n`, then silence) was still open after 100 s (`timeout 100` → exit 124, no EOF). A holder opened 19,990 partial connections: the server's `/proc/PID/fd` count reached `20000` (limit `Max open files 20000`). A probe request `GET /vocab.json` sent from inside the holder got `WouldBlock ... bytes=0 in 10.2s`. A separate curl got in after 3.9 s. After 40 more seconds, "still open: 19990": the server closed none of them. axum's accept loop sleeps 1 s on EMFILE (`axum-0.8.9/src/serve/listener.rs:157`). | NEW (no match in queued_index/fq, 11-findings, 06-limits or 05-decisions for slowloris/read timeout/connection limit) |
| probeapi-2 | medium | api | `crates/api/src/server.rs:2396-2399` + `2979-2986` (`gaps_json` / month audit) | **`/gaps.json` answers 200 with a full "vendor-hole" gap report for an address it cannot use.** An empty exchange, `..` traversal segments, or no series parameters at all should be refused. Instead the store path's refusal is folded into `absent_file`, and the minutes are reported as lost vendor holes. `/bars.json` refuses the same input with 400. | `curl '/gaps.json?month=2026-01'` → `200 {"expected":7500,"held":0,"lost_minutes":7500,...,"absent_file":" 2026-01: path segment exchange is empty",...,"reason":"vendor-hole"...}`. `?exchange=..&segment=..&symbol=..&month=2026-01` → 200, `"absent_file":".. 2026-01: path segment exchange is a traversal and would escape"`, `lost_minutes 7500`. The same address on `/bars.json` → `400 {"error":" 2026-01: path segment exchange is empty"}`. Code: `Err(why) => (Vec::new(), Vec::new(), Some(why))` treats every `bars::open` error as an absent month. | NEW |
| probeapi-3 | medium | api, cli | `crates/api/src/main.rs:33`, `crates/cli/src/main.rs:66` (`std::env::args()`) | **Panic (exit 101) on any argument that is not valid UTF-8.** It is not a refusal, the exit code is outside the documented set, and `api`'s `note_exit` and `cli`'s lifecycle "finished" event never run. `cli` has already installed its log by that point (`main.rs` calls `install_log` before the args are read). | `cli $'\xff'` → `thread 'main' panicked at .../std/src/env.rs:876:51: called Result::unwrap() on an Err value: "\xFF"`, rc=101. `cli sweep-stored dhan $'NIF\xffTY' 1min 2026 1 1` → same panic, rc=101. `api $'\xff'` → same panic, rc=101. | NEW |
| probeapi-4 | low | api | `crates/api/src/server.rs:87` (`(Some("serve"), Some(addr)) => ...`) | **`api serve ADDR <anything...>` ignores everything after ADDR and starts serving.** The doc comment directly above (lines 75-80) says "An unrecognised argument is a refusal rather than a fallback to serving: a typo that silently starts a server is a typo nobody finds." | `api serve 127.0.0.1:18794 --typo-flag 0.0.0.0:80` → `brutex api listening on http://127.0.0.1:18794/`, takes the store lock and creates `telemetry/`. By contrast `api report extra` → `unknown argument "report"`, rc=2. | NEW |
| probeapi-5 | low | api | `crates/api/src/server.rs:692-700` (`param`, first match wins) | **A repeated query key is answered silently from its first value.** The second value, even an invalid one, is never looked at. Repeated *headers* are refused with 403 (`sole_visible_header`). Repeated *query keys* change the question with no refusal, which is the outcome `param`'s own doc says a query must never have ("must never silently become a different query"). | `/audit.json?feed=zerodha&feed=dhan` → answers `"feed":"zerodha"`. `/audit.json?feed=dhan&feed=bogus` → 200, `"feed":"dhan"` (an unknown feed is never refused). `/calendar.json?feed=dhan&feed=bogus` → 200. A raw request with two identical `Host:` headers → `HTTP/1.1 403 Forbidden`. | NEW |
| probeapi-6 | low | cli | `crates/cli/src/lib.rs:2056-2063` (`sweep` exit code) + test `lib.rs:20120` | **`cli sweep 1..5 N` exits 0 even though its own verdict says nothing was measured.** Every bar is in warm-up. `cli audit` with the same arguments exits 1. | `cli sweep 1 10` → `swept 0 0.0%`, `warming 375 100.0%`, `outcome NOTHING MEASURED no ladder was walked -- this is not extinction`, `trustworthy as a whole answer NO`, **rc=0**. `cli audit 1 10` → same verdict, `RESULT NOT RECORDED`, rc=1. `sweep 5 10` → swept 0 of 1875, rc=0. Test `a_valid_sweep_and_a_valid_auto_both_render_and_exit_zero` pins `sweep 1 1` → OK. SESSIONS accepts 1..=3650, while lib.rs:20406 says "One session never warms the five-session evaluator". | NEW (generated-data path only) |
| probeapi-7 | low | api | `crates/api/src/server.rs:16980` and the following banner `println!`s | **The server panics (exit 101) if stdout is closed while the banner prints**, after it has already bound the port, taken `serve.lock` and installed telemetry. | `api serve 127.0.0.1:18794 2>err.txt \| head -1` → rc=101, stderr `failed printing to stdout: Broken pipe (os error 32)`. `serve.lock` and `telemetry/` were left in the store. | NEW |

## Attacks that held

| attack | result |
|---|---|
| Duplicate `Content-Length` (0 and 5) | 400 |
| `Content-Length` + `Transfer-Encoding: chunked` + pipelined smuggled GET | one 200, connection closed, smuggled GET not served |
| Bad chunk size (`FFFF...`, `ZZ`) on a body-less POST | handler ran (200), connection closed, pipelined follower not served (hyper behaviour; noted only) |
| LF-only line endings, absolute-form target `http://evil.example/...`, 200 KB header | parsed and served normally; Host header still checked |
| HTTP/1.0 without Host; duplicate Host | 403 |
| OPTIONS / TRACE / DELETE | 403 with `allow: GET,HEAD`; HEAD → 200 with correct length |
| `%2e%2e/%2e%2e/etc/passwd`, `/assets/..%2f..%2fCargo.toml`, `/he%00alth` | 400 "a path segment is not an ordinary file name" |
| Raw `/../../etc/passwd` | normalised to SPA index (200 HTML); no file read |
| `..`, empty, `%00` exchange/segment/symbol on `/bars.json` | 400 |
| Months `2026-00`, `0000-01`, `0001-01`, year 99999, `from > to`, range > 240 months | 400 with reason; `/gaps.json` caps at 240 and sets `truncated:true` |
| u64/i64 overflow numbers | refused as "not a YYYY-MM month" or clamped by documented design (`/logs.json`) |
| 100 KB query value | 414 |
| Keep-alive pipelining of 3 GETs | 3 responses in order |
| Non-loopback, IPv4-mapped and bad-port serve addresses | refused, rc=2, no socket |
| Second `api` on the same store; same port | refused naming the holder pid; "Address already in use", rc=1 |
| Store root that is a file, missing, empty or relative (api and cli) | refused, nothing created |
| cli: no args, `--help`, unknown verb, wrong arity, 0/-1/u64-overflow/out-of-range SESSIONS, MIN_HITS 0 | rc=2 with a reason |
| cli `sweep 8 400` run twice; `verify dhan NIFTY` run twice | byte-identical |
| 4 concurrent `cli sweep` on one store | 1 ran, 3 refused loudly ("another sweep owns this store's execution lease"), rc=1 |
| `kill -9` mid-sweep, then rerun | lease recovered; rerun rc=0, output byte-identical to the clean run |

UNVERIFIED: stored-data cli verbs past argument parsing. Every `sweep-stored` stops at "this build carries no verified commit stamp", because the shared working tree carries other workers' untracked probe files. So month-13 handling and the store paths after that point were not reached.

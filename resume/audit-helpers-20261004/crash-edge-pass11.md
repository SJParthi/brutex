# Crash and edge-input audit, pass 11: text in, text out

Head: `1f4de71` (origin/final/all-fixes-zero, read in /home/claude/wt/zero3). Audit only. Nothing in any checkout was edited. No cargo was run: all three findings are low, and each is shown by reading the source.

**Verdict: 3 new findings at 1f4de71 (0 high, 0 medium, 3 low).** No injection or panic route was found. The output sinks are already hardened:

- api HTML goes through `render::escape`.
- Header notes go through `note_alphabet`.
- Hand-built JSON goes through `json_string` or `quote_for_json`.
- NDJSON goes through the telemetry encoder, which escapes everything below 0x20.
- Terminal words go through `stored::clipped` and `escape_debug`.
- The web pages use Svelte text binding, and `masters.js` uses `esc`, so the CE-25 fix holds.

All three findings are the neighbouring class: a text that is folded, decoded or escaped one way in one place and another way somewhere else.

One structural note: workspace `Cargo.toml:84` denies `clippy::indexing_slicing`, but that lint does **not** cover `str` slicing (`clippy::string_slice` is not enabled). So every `&s[a..b]` on a `str` in production is guarded by hand only. Each one is listed below with its guard.

---

### CE-67 (low): the folder census counts a collision only for byte-identical names, but ingest upper-cases them, so `reliance.csv` and `RELIANCE.csv` report `collisions: 0` and land in one store series

- **Where:**
  - crates/pull/src/folder.rs:390-399 (`census_of`)
  - crates/pull/src/archive.rs:308-327 (`instrument_name`)
  - crates/pull/src/ingest.rs:1761 and 2070-2072 (`identify`)
  - crates/core/src/symbol.rs:62-76 (`Symbol::new`)
- **Code:**
  - `instruments.sort_unstable(); let before = instruments.len(); instruments.dedup(); Ok(Census { collisions: before - instruments.len(), ...`
  - ingest: `} = identify(&member.instrument, exchange, segment)?;` → `let symbol = brutex_core::symbol::Symbol::new(instrument)`, which maps `b'a'..=b'z' => c - b'a' + b'A'`.
- **Why it is wrong:**
  - `Census::collisions` is documented (folder.rs:352-359) as "how many members named an instrument some other member had already named. **Reported rather than swallowed** ... a caller that sees only the deduplicated list cannot tell a clean folder from a colliding one."
  - The census compares the raw file-stem strings. Ingest keys each member by `Symbol::new`, which folds ASCII case. So two members whose stems differ only in case are two "instruments" in the census, with `collisions: 0`, but at ingest they are one key: `NSE/<seg>/RELIANCE/...`. Both members are appended into the same month files.
  - The archive's own doc (archive.rs:292-296) names the consequence: "A call and a put whose timestamp ranges do NOT overlap concatenate into one monotonic series and nothing objects." Overlapping ranges are caught only by the append guard, "by luck, not by design".
  - The `/pull` page (`web/src/routes/ingest/+page.svelte:7762`) shows a collision warning only when `c.collisions > 0`, so the operator sees a clean folder.
- **Reach:** a folder on a case-sensitive filesystem (Linux, or a case-sensitive APFS/ext volume) that holds two members whose names differ only in case. On default macOS APFS the two files cannot coexist. That limits the reach, so this is low.
- **Repro (not run):** make a folder with `RELIANCE.csv` (rows for 2024-01) and `reliance.csv` (rows for 2024-02), both in the declared layout. `GET /folder.json?...` answers `"instruments":["RELIANCE","reliance"],"collisions":0`. An ingest of the folder then writes both months under `NSE/CASH/RELIANCE`, and no failure names the merge.
- **Minimal fix:** in `census_of`, dedup on the identity ingest will use (ASCII upper-case, or better `Symbol::new(..)` with refused names kept apart), so `collisions` counts what ingest will merge. Alternatively, have ingest refuse a second member that resolves to an `Identity` already written in the same run, naming both paths.

### CE-68 (low): the rolling-option POST body is "JSON-escaped" but passes control characters raw, and the vendor id it carries admits them

- **Where:**
  - crates/pull/src/rolling.rs:487-503 (`push_pair`), called at :460 with `&ask.security_id`
  - crates/core/src/vendor.rs:294-317 (`VendorId::new`)
  - crates/api/src/server.rs:13967 and 13982
- **Code:**
  ```
  /// One `"key":"value"` pair, JSON-escaped.
  fn push_pair(out: &mut String, key: &str, value: &str, first: bool) {
      ... for ch in value.chars() { match ch {
              '"' => out.push_str("\\\""),
              '\\' => out.push_str("\\\\"),
              other => out.push(other),
  ```
- **Why it is wrong:**
  - RFC 8259 forbids unescaped U+0000..U+001F inside a string. `VendorId::new` trims ASCII whitespace at the ends only. It deliberately keeps everything inside: "this type carries bytes the vendor chose and hands them back unchanged". Its own test (vendor.rs:3495) keeps an interior U+00A0. An interior tab, `\x01` or NUL from a Dhan master row therefore survives into `Ask::security_id`, and `body()` emits it raw.
  - The request then goes out as invalid JSON. The vendor's answer, typically a 4xx with its own wording, is what the operator sees, not "our body was malformed". The cause is ours, and the refusal names the vendor.
  - Every other hand-built JSON in the workspace escapes the full `< 0x20` range: `render::json_string`, `pullrun::quote_for_json` and the telemetry encoder. The bars path uses `serde_json` (http.rs:3027-3032).
- **Repro (not run):** `VendorId::new("13\u{1}33")` is `Some`. `pull::rolling::body(&spec, &Ask { security_id: "13\u{1}33".into(), .. })` contains the raw byte 0x01 inside `"securityId":"..."`, and `serde_json::from_str::<serde_json::Value>(&body)` fails.
- **Minimal fix:** in `push_pair`, escape `c if (c as u32) < 0x20` as `\u{:04x}`, as `quote_for_json` does. Or build the body with `serde_json`, keeping the key order explicit. Optionally, refuse a rolling security id that is not ASCII digits before any request.

### CE-69 (low): the AWS identity reader treats a non-UTF-8 environment value as unset, so it silently signs as the credentials file's profile

- **Where:**
  - crates/pull/src/ssm.rs:244 (`from_env`), :308 (`discover`), :333-352 (`discover_from`), :453-455 (`non_blank`)
- **Code:** `Self::discover_from(|name| std::env::var(name).ok(), Self::from_shared_file)` and `let profile = non_blank(lookup("AWS_PROFILE")).unwrap_or_else(|| "default".to_owned());`
- **Why it is wrong:**
  - `std::env::var` returns `Err(NotUnicode)` for a set but non-UTF-8 value, and `.ok()` turns that into `None`, which is the same as unset. Two silent cases follow:
    - **(a)** `AWS_PROFILE` set to a non-UTF-8 name: the pull signs as `[default]`. That is a different identity from the one the operator chose. This is exactly the outcome D-1534 refused for a half-set pair: "silently signed as a DIFFERENT identity, with no mention of the variable they set."
    - **(b)** Both `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` set but non-UTF-8: the exported pair is skipped and `~/.aws/credentials` is used.
  - In the mixed case, one key non-UTF-8 and one valid, the refusal says "is unset or empty" about a variable that is set.
  - Every other env reader in the workspace refuses non-UTF-8 by name: `booleanlaunch.rs:456-462` and `indexstoplaunch_metadata.rs:179-185` ("{name} must be UTF-8"), `cli/index_stop_launch.rs:297-302`, and `render.rs:3142-3148` (mapped to U+FFFD and refused by `knob::switch`). This one degrades silently, which is the CLAUDE.md §4 shape.
- **Repro (not run):** `AWS_PROFILE=$'prod\xe9' brutex-api` with a `[default]` profile present. The SSM read is signed with `[default]`'s key, and nothing names `AWS_PROFILE`.
- **Minimal fix:** use `std::env::var_os(name)` and map `into_string()` failure to `SsmError::unreachable("{name} is set but is not UTF-8")`. The lookup closure would return `Result<Option<String>, SsmError>`, so a non-UTF-8 value refuses instead of falling through.

---

## Sites checked, and why each holds

**Byte-index slicing of `str` in production code.** These are not lint-covered (see above). Each was found by scanning every `x[..]` slice outside test modules, including code between an early `#[cfg(test)]` item and `mod tests`:

| Site | Why it holds |
|---|---|
| api/src/render.rs:851-864 `clamp` | `head` is built from `char_indices` and `+ c.len_utf8()`. `cut` comes from `rfind(", ")`, which is ASCII, so it is a boundary. |
| cli/src/boolean_campaign.rs:667-675 `bounded_reason` | It steps `end` down while `!is_char_boundary`. `REASON_BYTES - 32` is a constant, so there is no underflow. |
| cli/src/boolean_search_command.rs:513-522 `bounded_reason` | Same guard. `length` is `min(1024)`, and it stops at 0 because 0 is always a boundary. |
| pull/src/nseindex.rs:112-128 `numeric_runs` | Indices are positions of ASCII digits from `bytes()`, and an ASCII byte is always a boundary. |
| cli/src/lib.rs:18093-18095 `not_recorded_reason` | `find(NOT_RECORDED)` plus the needle's own length. |
| telemetry encode.rs `push_capped`, api/audit.rs:1471 `keep`, pull/http.rs:3594 | Each uses `get(..n)` (`None` and a fallback, not a panic) with a `char_indices` cut. |
| `String::truncate`, `split_off`, `drain`, `replace_range`, `str::split_at` | No production call on a `String`/`str`. Every hit is a `Vec` or an `OpenOptions::truncate`. |

**HTTP headers (CR/LF/NUL injection).**
- Every variable header goes through `note_header` → `note_alphabet` (server.rs:4420-4470). It maps everything outside 0x20..0x7E to `?` and caps the length by characters.
- ETag is quotes and hex, and its `from_str` failure arm just omits the header.
- `operation_audit.rs:221` is a number.
- The api has no `Content-Disposition`, `Location` or redirect anywhere.

**api HTML.**
- `render::escape` covers the five characters. `open_with` escapes the `<title>`, `receipt_page` escapes the scope, reason and facts, and the `/logs` form, table and footers escape every value (logs.rs:662-1058).
- Pager `href`s are built with `query_value`, which percent-encodes everything outside unreserved.
- The entity case is checked: `escape(view.vendor).to_uppercase()` (render.rs:2675) yields `&AMP;`/`&LT;`, which are valid HTML5 legacy references. No injection.

**JSON and NDJSON (log forging).**
- `render::json_string` escapes `"` `\` and below 0x20, plus `<`. `pullrun::quote_for_json` does the same.
- The telemetry encoder escapes control bytes and caps on a character boundary. A field holding `\n{"level":...}` cannot become a second line.
- `folder.rs:312-360` quotes every string through `json_string`.
- Rust `{:?}` never feeds a hand-built JSON value. Every `:?` in api goes into a `serde_json::json!` value or a test message.

**Terminal (ANSI and forged lines).**
- cli quotes outside words through `stored::clipped` (`escape_debug`, which escapes ESC, controls, format characters such as U+200B/U+202E, and separators). See batch.rs:507 and lib.rs:3223.
- The api startup and `/health` notes (server.rs:653-735) carry only these:
  - vendor names and counts
  - `&'static` skip reasons
  - `InstrumentError` Display, which is static text or numbers
  - `{code:?}` for an unrecognised listing class, which is Debug-escaped
  - `InstrumentKey`/ISIN, which are ASCII-validated
- Every `eprintln!` in pull prints numbers only.

**CSV export (`web/src/routes/db/+page.svelte:6911-6926`).**
- RFC 4180 quoting with CRLF records.
- Formula prefixes: every text cell is a store key. Symbols pass `check_segment`, which admits only `[A-Za-z0-9-_&]`, and an instrument key starts with its exchange. The other text cells are an enum code, an ISO date or an integer, so no cell can start with a vendor-chosen `=`/`+`/`@`. This holds as recorded by the earlier XSS pass.

**Web pages.**
- The only HTML sinks remain in `masters.js`, and every interpolated value goes through `esc`. `cell(m.file, ...)` takes a server constant.
- Svelte `{@html}` does not occur. Dynamic `href`s are validated (see the 05-web-xss pass).

**Outbound requests.**
- Bars path segments go through `path_safe` (http.rs:767-768).
- GET parameters go through `reqwest::query`, which encodes them. POST bodies go through `serde_json`.
- The only hand-built body is the rolling one (CE-68).

**Case folding and look-alike symbols.**
- `Symbol::new` is ASCII-only and upper-cases. `check_segment` refuses a lower-case store directory (`NotCanonicalCase`). The `/store` filter needle is folded once.
- `parse_fno_inner` (ingest.rs:1625) uses Unicode `to_uppercase` before `Symbol::new`. So `nıfty` (dotless i) and `ſbin` (long s) fold to `NIFTY` and `SBIN`. This canonicalises to the instrument typed and changes no key, so it is not a finding.
- `nseindex::collapse` keeps ASCII alphanumerics only, which strips zero-width and bidi characters. That is intended normalisation.
- The NSE constituents duplicate check (nse.rs:719-736) is case-sensitive. NSE publishes upper case. Not reported.

**Invalid UTF-8.**
- The master file goes through `read_to_string`, so a bad file is refused whole and by path.
- `percent_decode` is lossy, and its output is re-validated by `Symbol::new`, `Segment::parse` and the other parsers.
- Archive member names use `to_string_lossy`. A U+FFFD then fails `Symbol::new` loudly, by name, at `identify`.
- The env readers are covered under CE-69.

**BOM.**
- `api::master::Columns::locate` does not strip U+FEFF, and `str::trim` does not remove it. A BOM-led Dhan master would therefore be refused with `no column "SEM_EXM_EXCH_ID"`. That is loud, so it is not a §4 finding. It is noted for robustness: NSE (nse.rs:635/682) and cash_auction strip it.

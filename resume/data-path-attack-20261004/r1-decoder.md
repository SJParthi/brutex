# Attacker DECODER, round 1

Scope: core::{price, isin, symbol, instrument, vendor, universe}, pull::{fno, masters, resolve}.
There is NO Zerodha/NSE tradingsymbol-to-contract decoder in this repo. Zerodha's derivative rows
(NFO-FUT/NFO-OPT segments) are declined at the segment gate before any symbol is read, and the master
decoder takes expiry and strike from structured columns, never from the display symbol. The only
contract-NAME decoder is `pull::fno::read_contract` (Groww's hyphen-delimited `NSE-NIFTY-04Jan24-19200-CE`),
so the GDFL-style run-together ambiguity (ADANIENT18DEC195CE) cannot arise here: every piece is
hyphen-delimited.

## 1. Attacks

| attack | cases | failures on unmodified code | fixed? | evidence |
|---|---:|---:|---|---|
| price text, hand-picked ("0","-0","0.005","0.015","1e3","NaN","inf","12,345.50","+12.5", i64 edges, 20-digit, NBSP, fullwidth, 64/65/1000 bytes) | 43 | 0 | n/a | core/tests/attack_decoder.rs:119 |
| price text, exhaustive sign x 0..199 x 0-4 decimals vs floor(x+1/2) | 6,666,600 | 0 | n/a | :188 |
| price text, 10^6 random strings vs independent i128 reference (refusal reason included) | 1,000,000 | 0 | n/a | :218 |
| price text round trip of random paisa (plain, zero-padded, `+`) | ~3,000,000 | 0 | n/a | :273 |
| price float extremes (NaN, ±inf, -0.0, subnormal, MIN_POSITIVE, EPSILON, MAX, MIN, ties, sentinel) | 14 | 0 | n/a | :309 |
| price float 10^6 random bit patterns + in-range values: within 1/2 paisa or named refusal | 1,000,000 | 0 | n/a | :340 |
| ISIN real values, all 10 check digits, lowercase, padding, lengths 0..40 | ~250 | 0 | n/a | :434 |
| ISIN every single ASCII substitution of 5 real ISINs vs textbook Luhn | 7,680 | 0 | n/a | :472 |
| ISIN 10^6 random bodies vs reference | 1,000,000 | 0 | n/a | :493 |
| Symbol awkward names (M&M, M&MFIN, BAJAJ-AUTO, NAM-INDIA, 3MINDIA, NIFTYNXT50, J&KBANK) + look-alikes (Cyrillic, Greek, fullwidth, dotted/dotless I, ZWSP, NBSP, NUL) | 25 | 0 | n/a | :520 |
| Symbol 10^6 fuzz vs allowlist reference | 1,000,000 | 0 | n/a | :566 |
| Expiry exhaustive 1980..2110 x 0..13 x 0..32 (leap 2000/2020/2024/2100) | 61,446 | 0 | n/a | :602 |
| Contract::of non-positive strike | 10 | **10** (`2025-09-30-0-CE`, `2025-09-30--500-PE` rendered) | yes D-3150 | :685; fix instrument.rs:476-486 |
| Contract::parse non-canonical text (`FUT`, 24 x `A`, `2025-9-30-FUT`, `2025-02-30-FUT`, leading-zero strike, ...) | 20 | **many** (all accepted; `is_option()` true for garbage) | yes D-3151 | :713; fix instrument.rs:533-575 |
| Contract of/parse 10^6 kinds + 10^6 random texts vs reference grammar | 2,000,000 | yes (via the two above) | yes | :745 |
| vendor master: test marker in lower case (`031nsetest`) | 8 | **6** (kept as symbol `031NSETEST`) | yes D-3152 | :849; fix vendor.rs:933-944, 1484-1492 |
| vendor master: Zerodha index alias case (`Nifty 50` kept as `NIFTY50`, not `NIFTY`) | 6 | **3** | yes D-3153 | :867; fix vendor.rs:1633-1640 |
| vendor master: 1000-byte fields, look-alike index names, lowercase segment/type | 9 | 0 | n/a | :887 |
| vendor master: derivative strikes 2.5/77.5/1012.5/0.05/x.005 skip; 0,-0,-77.5,"",1e3,NaN,24,500,0.004 and 8 bad expiries error | 22 | 0 | n/a | :921 |
| vendor master: 10^6 random rows across 3 vendors, kept-row invariants | 1,000,000 | 0 | n/a | :965 |
| F&O universe: every member, case/space/-EQ variants, 1000-byte, look-alikes, 10^6 random probes, 208 shares, ISINs | ~1,000,900 | 0 | n/a | :1064 |
| read_contract documented shapes incl. BAJAJ-AUTO, M&M, M&MFIN 277.5, 3MINDIA, NIFTYNXT50, 2.5, 77.5, 1012.5, 0.05 | 11 | 0 | n/a | pull/tests/attack_decoder.rs:46 |
| read_contract zero strike | 3 | 3 (fixed by D-3150 before this test was written; demonstrated through the core test) | yes D-3150 | :146 |
| read_contract second spellings: `+4Jan24`, `Jan+4`, `019200`, `00.05` | 6 | **6** | yes D-3155, D-3157 | :163; fix fno.rs:1161-1201, 1127-1133 |
| read_contract underlying not a canonical symbol (1000 bytes, `a b`, lower case) | 1+ | **1** (1000-byte underlying read and copied out) | yes D-3156 | :180; fix fno.rs:1077-1089 |
| read_contract 33 malformed names (sides, 3 decimals, 1e4, NaN, 20 digits, wrong day/month/year, unicode digits) | 33 | 1 (the 1000-byte one above) | yes | :180 |
| read_contract 10^6 grammar-piece names + 10^6 arbitrary texts | 2,000,000 | yes (`BSE-BAJAJ-AUTO-+4Jan24-FUT` read) | yes | :256, :360 |
| masters::missing_columns BOM / quotes / case / CRLF / padding | 3 vendors x 7 | 0 | n/a | :400 |
| masters::nse_index_csv 10^5 random names/categories with `,` `\n` `\r` `"` NBSP | 100,000 | 0 | n/a | :423 |

resolve.rs: no decoder of names; its parsing is the snapshot wire. Not attacked beyond reading.

## 2. Draft decisions and invariants

**D-3150 — A contract segment has a positive strike.** `Contract::of` rendered `2025-09-30-0-CE` for a
zero strike and `2025-09-30--500-PE` for a negative one, and `fno::read_contract` filed `NSE-NIFTY-04Jan24-0-CE`
as such a contract. No listed option has a non-positive strike and the vendor-master path (`parse_strike`)
already refused one, so the two decoders disagreed. `render` now returns `None` for a strike <= 0, which every
caller already treats as a refusal (`store::path` as `ContractPathUnsupported`, `read_contract` as unreadable).

| DPD-01 | `Contract::of` returns `None` for any option strike <= 0 | dpd_a_non_positive_strike_has_no_contract_segment |

**D-3151 — `Contract::parse` accepts exactly what `Contract::of` renders.** It checked only the byte alphabet, so
`FUT`, 24 `A`s, `2025-9-30-FUT`, `2025-02-30-FUT` and `2025-09-30-0002465000-CE` were contracts and `is_option()`
answered true for the non-futures. A second spelling of one contract is a second store directory for one series.
`parse` now reads the text into the `Kind` it names, re-renders it, and accepts only a byte-identical rendering, so
the renderer is the one grammar. The existing in-crate test that asserted 24 `A`s parse encoded the defect and
now uses a real 24-byte contract.

| DPD-02 | `Contract::parse(t) == Some(c)` iff `Contract::of(k) == Some(c)` and `c.as_str() == t` for some `Kind` k | dpd_contract_parse_accepts_only_what_of_renders, dpd_contract_of_and_parse_are_inverses_under_fuzz |

**D-3152 — The exchange test-instrument marker is case-blind.** `decode_master_row` declined `NSETEST`/`BSETEST`
with a case-sensitive substring scan, then case-folded the kept symbol, so `031nsetest` was kept as `031NSETEST`.
The scan now ignores ASCII case. Both fields are already bounded at 64 bytes, so the cost stays constant.

| DPD-03 | No kept master row's symbol contains NSETEST or BSETEST in any case | dpd_a_test_instrument_is_declined_in_any_case |

**D-3153 — The index alias is looked up on the case-folded name.** Zerodha's `NIFTY 50` -> `NIFTY` alias was
matched case-sensitively, so `Nifty 50` became a second index identity `NIFTY50`. The collapsed name is now
upper-cased before the alias lookup; the symbol was folded one line later anyway, so nothing else changes.

| DPD-04 | A Zerodha index row decodes to the same symbol whatever the ASCII case of its name | dpd_the_index_alias_does_not_depend_on_case |

**D-3155 — A Groww expiry token is read byte by byte, digits only.** `expiry_token_agrees` parsed the day and year
with `str::parse`, which reads a leading `+`: `+4Jan24` and `Jan+4` matched. The token is now matched as a fixed
byte pattern, every digit place checked; this also removed an `.iter().position(`.

**D-3156 — A contract name's underlying must already be a canonical `Symbol`.** Only emptiness was checked, so a
1,000-byte or lower-case underlying read and was copied out whole for the chain's ask comparison to catch.

**D-3157 — One spelling per strike.** A leading zero (`019200`, `00.05`) is refused. The chain deduplicates by
NAME, so two spellings of one contract were two filings of one series and two bar requests.

| DPD-05 | `read_contract` never reads a non-positive strike | dpd_a_zero_strike_is_refused |
| DPD-06 | `read_contract` refuses a `+`, a leading-zero strike and a non-canonical underlying | dpd_a_second_spelling_of_the_same_contract_is_refused, dpd_malformed_names_are_refused_never_guessed |
| DPD-07 | Every name `read_contract` accepts yields a positive strike (for an option), a contract `Contract::parse` round-trips, the keyed expiry and a verbatim underlying | dpd_read_contract_fuzz_never_panics_and_reads_only_consistent_contracts |
| DPD-08 | Rupee text -> paisa equals floor(x + 1/2) exactly and never yields i64::MIN | dpd_price_text_exhaustive_small_domain_agrees_with_the_reference, dpd_price_text_fuzz_never_panics_and_agrees_with_the_reference |
| DPD-09 | `Isin::new` accepts exactly the ISO 6166 / Luhn-valid 12-byte strings | dpd_isin_every_single_ascii_substitution_agrees_with_the_reference |

## 3. Commands
- `CARGO_BUILD_JOBS=2 cargo test -p core --locked --test attack_decoder` on unmodified code: 16 passed, 6 failed. After the fixes: 22 passed.
- `cargo test -p core --locked`: all targets green (lib 168, attack_decoder 22, others).
- `cargo clippy -p core --all-targets --locked -- -D warnings`: clean.
- `cargo test -p pull --locked --test attack_decoder`: 9 passed (3 failed before the fno.rs fixes).
- `cargo test -p pull --locked --tests`: all green (lib 576 + 3 ignored, unit 156, census 20, derive 33, pipeline 36, ...).
- `cargo clippy -p pull --lib --test attack_decoder --locked -- -D warnings -A clippy::too_many_lines`: clean for my files. Plain `-D warnings` on pull fails on `http.rs:1247` and `ingest.rs:1759` (too_many_lines) and on `tests/attack_ingest.rs`, none of them mine.
- rustfmt --check clean on all five files I touched.

## 4. O(1) (debug build, shared box, per op)
| N | decode_master_row p50/p99/max | from_rupee_text_half_up p50/p99/max | FNO_INDEX.contains p50/p99/max |
|---|---|---|---|
| 1e3 | 361 / 771 / 38,489 ns | 108 / 292 / 9,425 ns | 75 / 350 / 766 ns |
| 1e4 | 386 / 728 / 273,297 ns | 108 / 261 / 146,617 ns | 74 / 211 / 18,330 ns |
| 1e5 | 370 / 687 / 170,393 ns | 107 / 232 / 239,060 ns | 75 / 185 / 131,400 ns |
| 1e6 | 228 / 557 / 6,201,721 ns | 63 / 162 / 149,661 ns | 47 / 117 / 119,430 ns |
p50 and p99 are flat (they fall slightly at 1e6 as caches warm); max is scheduler noise on a shared box.
`read_contract` allocates the vendor name and underlying (bounded now by the symbol check at 24 bytes for the
underlying; the vendor name itself is unbounded -- see 5).

## 5. Needs real vendor data (Mac GDFL / Zerodha thread)
1. GDFL pre-Feb-2019 option names (`ADANIENT18DEC195CE`: 18-Dec strike 195 vs Dec-2018...) -- no such decoder
   here. Settle from the GDFL file's own expiry/strike columns if it has them, or from NSE's bhavcopy for that
   date: a strike that never existed in the bhavcopy rules a reading out.
2. Groww discovery: does an NSE ask ever answer a `BSE-` (or empty-exchange) name? `read_contract` ignores the
   exchange token and `chain::month` compares only the underlying, so `BSE-NIFTY-...` would be filed as NSE NIFTY.
   Fix belongs in chain.rs (not owned): compare the exchange with the ask. Pinned by dpd_spellings_left_to_vendor_evidence_are_pinned.
3. Groww month-word case (`04JAN24` vs `04Jan24`) and fractional strike spelling (`77.5` vs `77.50`): both read
   to one contract today, so two spellings would be filed twice. Which one Groww writes is UNVERIFIED; a dedupe by
   CONTRACT (not by name) in chain.rs would make the question moot.
4. Zerodha master: no `listing_class` column, so `IDEA-BE`-style series-suffixed symbols in the NSE segment are
   kept as their own symbols with no unsuffixed key. How many F&O names appear suffixed in the real dump?
5. Zerodha/Dhan/Groww CSV headers: a UTF-8 BOM makes the first column "missing" (loud). Does any live file carry one?
6. Contract::parse is now strict. Any census written by an older build with a non-canonical contract text would
   now read back as "no contract". Censuses are written from `as_str`, so none should exist -- check a real store.
7. Doc/code contradiction, not mine to fix: `masters::nse_index_csv`'s doc says it refuses "an object whose values
   are not arrays of strings", but the code (and a test, masters.rs:2896) deliberately SKIPS such values.

## 6. Files modified
- crates/core/src/instrument.rs (Contract::render, Contract::parse, one in-crate test that encoded the lax parse)
- crates/core/src/vendor.rs (holds_ignoring_ascii_case, marker scan, alias case fold)
- crates/pull/src/fno.rs (read_contract underlying/strike checks, expiry_token_agrees rewrite)
- crates/core/tests/attack_decoder.rs (new)
- crates/pull/tests/attack_decoder.rs (new)
- `cargo test -p api --locked contract`: 20 passed, 0 failed (Contract::parse callers in api unaffected).

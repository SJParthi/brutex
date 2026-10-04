# Crash/edge pass 18 — instrument identity and mapping (ce18)

Checkout: /home/claude/wt/zero3 at 1f4de71 (read only). No cargo run; both findings are low and argued from source plus the repository's own fixtures.

## Verdict

2 new findings (0 high, 0 medium, 2 low): CE-91, CE-92.
Verification: 6 prior rows in this theme. 3 FIXED, 3 NOT FIXED.

## New findings

### CE-91 (low): the native-id reverse map treats a Dhan security id as unique across segments, so NIFTY (index id 13) and ABB (equity id 13) cancel each other and Dhan stops counting as a Zerodha cross-check witness for ABB

- **Where:** crates/api/src/merge.rs:303-334 (`NativeIds::build`). Consumers: crates/api/src/server.rs:7772 (Zerodha symbol policy) and :7809-7834 (`zerodha_isin_cross_check`). Recovery always uses the cross-checked policy (crates/api/src/recovery.rs:136, :147).
- **Code:**
  ```rust
  reverse
      .entry((source.vendor, listing.vendor_id))
      .and_modify(|key| { if *key != Some(listing.key) { *key = None; } })
      .or_insert(Some(listing.key));
  ...
  if let Some(id) = id && reverse.get(&(vendor, id)) == Some(&Some(key)) {
      by_key.insert((vendor, key), id);
  }
  ```
- **Why it is wrong:**
  - Dhan's `SECURITY_ID` is unique only within an exchange segment. Both Dhan segments that this code keeps, `I` (index) and `E` (equity), are filed under exchange `NSE` (core/src/vendor.rs:1191 `Self::Dhan => (&["I", "E", "D"], ...)`).
  - The repository's own data shows the clash. The Dhan NIFTY index id is `13` (api/src/coverage.rs:678, pull/tests/broker.rs:240 with `IDX_I`, pull/src/rolling.rs:968). A Dhan equity id is the NSE token: RELIANCE is `2885` in Dhan and in Kite's `exchange_token` (api/src/server.rs:20140). ABB's NSE token is `13` (pull/src/cash_auction.rs:784, `FinInstrmId` 13).
  - So `(Dhan, "13")` maps to two keys, the reverse slot becomes `None`, and `native_id(Dhan, NSE-NIFTY)` and `native_id(Dhan, NSE-ABB)` both return `None`. Any other share whose NSE token equals a Dhan index id is hit the same way. BANKNIFTY=25 against ADANIENT=25 is the likely second pair, but no fixture in the repository records it.
  - The pull itself is not mis-mapped. `spot_vendor_identity_for` reads `entry.ids`, and the request carries the segment, so there is no wrong-instrument fetch. What fails is the evidence. `zerodha_isin_cross_check` asks for an "unambiguous exact independent master witness", and Dhan is unambiguous for ABB, but it is discarded because an index shares the number.
- **Reach:** a cross-checked Zerodha pull or recovery of ABB (or another clashing share) is refused whenever Groww cannot witness it, for example when the Groww master is missing, ambiguous, or disagrees on the ISIN. The refusal message says no independent witness exists, which is false.
- **Repro (not run):** `merge(&[from(Vendor::Dhan, vec![spot_index("NIFTY","13"), equity("ABB","INE117A01022","13")]), from(Vendor::Zerodha, vec![equity("ABB",.., "3329")])])`, then `merged.native_id(Vendor::Dhan, abb.key)` gives `None`. Expected: `Some("13")`.
- **Minimal fix:** key `forward` and `reverse` by `(vendor, key.segment, id)`, so that a vendor id is required to be unique only inside the namespace the vendor issues it in. Add the NIFTY/ABB pair as a test.

### CE-92 (low): the `swept` target counts and pulls only what some loaded master lists, so a swept instrument no master names (an F&O share renamed by NSE after the transcription, or one a master refresh dropped) is not attempted, not counted as lacking, and not named

- **Where:**
  - crates/api/src/coverage.rs:426-470 (`from_master` walks `merged.by_key`).
  - crates/api/src/server.rs:7379-7428 (`spot_targets` walks `merged.by_key`).
  - crates/api/src/server.rs:7433-7457 (`spot_mapping_refusal`: `expected` comes from `asked.target.members()`).
  - crates/api/src/ingest.rs:331 (`Self::Swept | Self::Indices | Self::Everything => None`).
- **Code:** `let expected: Vec<&str> = if asked.members.is_empty() { asked.target.members().map_or_else(Vec::new, <[&str]>::to_vec) } else { ... }`. For `swept` this is empty, so a missing member can never become an issue.
- **Why it is wrong:**
  - The swept surface is a compile-time list: `InstrumentKey::SWEPT` plus `FNO_UNDERLYINGS` less `FNO_INDEX_UNDERLYINGS`, 210 names. It is not "whatever a vendor master calls an index".
  - It is nevertheless counted like `indices`. Its denominator is the merged masters. If no master carries a key, that key is in neither `matched` nor `lacks`, so the coverage line reads N of N.
  - A whole-target `swept` run builds the same list and then attempts N with a clean receipt. That silently narrows the swept pull, against CLAUDE.md §3 rule 2 and §4.
  - The `fno` target does not have this gap. It is join-backed (`members()` = `FNO_UNDERLYINGS`), so the same name is refused loudly. Ticking the member explicitly is also refused loudly. Only the target defined as the engine surface drops the name without a word.
  - The likely trigger is an NSE rename. The list holds post-rename names (ETERNAL, TMPV, LTF, GMRAIRPORT) transcribed from 2026-08 masters. After the next rename, both refreshed masters list only the new ticker, the old key disappears from `merged`, and the new ticker is not sweepable.
  - docs/06-limits.md:4309 still says "`swept` is 2 of 2 for both feeds", which has been stale since D-0506.
- **Repro (not run):** load a Dhan master with the ABB row deleted. `Coverage::of(Dhan, Swept)` gives matched 209, lacks 0, with ABB unnamed. `POST /pull/spot target=swept vendor=dhan` with no `member=` makes `spot_mapping_refusal` return `None`, `attempted` is 209, and the receipt has no ABB row. Only the boot note for the F&O tier (`constituents::Join::notes`) mentions it.
- **Minimal fix:** give `SpotTarget::Swept` a published roster. Either make `members()` return the 210 names, or have `spot_mapping_refusal` and `from_master` iterate `SWEPT ∪ shares` for `Swept` and push a key absent from `merged` as `lacks` with a "no loaded master lists this symbol (renamed or delisted?)" reason. Then correct limits §64.

## Edge cases checked

| Case | Path | Result |
|---|---|---|
| NSE rename, history split across names (REST) | `spot_vendor_identity_for` (ISIN from transcribed `nse_isin`), dated `DailyEligibility::eligibility` (exact symbol+ISIN) | Refuses loudly. A pre-rename day has no EQ mapping for the new symbol, so landing is refused by name (server.rs:7555-7558). No alias is invented. The reason names the missing mapping but does not say "rename". |
| NSE rename, master no longer lists the transcribed name | coverage/run for `swept` | **Silently dropped. CE-92.** The `fno` target and ticked members refuse it by name. |
| Rename in the stored store | `stored::swept_index`, `research::cash_symbols` | An old-symbol directory is refused as off-surface. The `research` inventory lists the new symbol's missing months. No symbol-change table exists anywhere, so the store does not merge the two names, by design. |
| Demerger / split / bonus | runner audit notes | Stated, not detected: "CORPORATE ACTIONS ARE UNCHECKED (D-0018, D-0694)" on every equity report. TMPV kept TATAMOTORS's ISIN, so the identity holds. |
| Delisting / suspension | `independently_inactive` (FORCEMOT only), dated eligibility explicit 0/absent | Absent evidence is `UNVERIFIED`, never "active". Only the one cited interval is certified. |
| Joined or left F&O during history | `FNO_UNDERLYINGS` (compile-time snapshot), `membership_snapshot_digest_v1` | Not time-aware, and that is stated (limits §11, research_family.rs:3-4, the mapping refusal text "Current membership is a snapshot"). The family decode refuses a stale snapshot digest. `parse_fno` refuses an underlying that has since left F&O, by name. |
| Two vendors, different symbols for one share | `merge` (ISIN-asserted suffix strip only), `single_vendor_members` | Kept as two keys and reported under UNCHECKED IDENTITY. No silent merge. |
| Two vendors, different tokens for one instrument | `Entry::ids` per vendor discriminant | Held. Each vendor's own id is used, and a missing id is refused ("lists no id") rather than borrowed. |
| One vendor, one key, conflicting rows (listed twice) | `Loaded::keep`, `merge_assertion` | Held. The key becomes ambiguous (`None`), the lookup refuses, and `duplicate_keys` is logged. Identical duplicates collapse. |
| One vendor id reused for two keys | `NativeIds` reverse map | Refused. Over-refuses for Dhan cross-segment ids: **CE-91**. The main pull path is unaffected because the request carries the segment. |
| Token reuse across expiry (contracts) | `fno::read_contract`, `ingest::matching` | Contracts are keyed by decoded expiry, strike and side, and the token is never persisted as identity. Not swept. |
| Kite token uniqueness | `master_columns` Zerodha `instrument_token` | Exchange-scoped, unique. Held. |
| `M&M`, `GVT&D`, `L&TFH` in URLs | `render::query_value`, `server::param`/`percent_decode`, `http.rs` `.query(&pairs)`, recovery `encode` | Held. Values are percent-encoded on every server-built link and outbound query. |
| `BAJAJ-AUTO`, `NAM-INDIA` | `fno::read_contract` (both ends), `InstrumentKey` Display, cli `strip_prefix("NSE-")`, web `sweptSymbolOf` | Held. `lake::contract::ContractName::parse` still splits on every `-`, but it has no production caller and the measured lake holds only index contracts. |
| Case | `Symbol::new` upper-cases; `ContractName` refuses lower case; `DailyEligibility` exact; `ResearchFamilyV1::decode` re-encodes | Held, except CE-67 and CE-76 (below, not re-reported). |
| Whitespace | `Symbol::new` refuses a space; index rows only collapse spaces; `VendorId::new` trims ASCII; `board_of` trims | Held. A space in an equity or derivative ticker is a row error, counted and named. |
| Master missing a column, short row, or 4 MiB field | `Columns::locate`, short-row error, `over_wide` | Refused by name. |
| Refreshed master lacks a swept instrument | `masters::validated_body` | Lands (only shape and row decode are checked). The loss then surfaces per pull by name, and silently for the `swept` target (CE-92). |
| F&O index names as cash | `is_sweepable`, `swept_index`, research filter | Refused (D-0682). |
| List integrity | `both_lists_are_sorted_and_unique...`, ISIN alignment tests, 208 shares all in `NIFTY_TOTAL_MARKET` (checked: `comm` leaves only the 5 indices) | Held. |

## Verification of earlier rows in this theme

| ID | Status | Evidence at 1f4de71 |
|---|---|---|
| CE-67 | NOT FIXED | pull/src/folder.rs:388-398 still sorts and dedups the raw `member.instrument` strings. Case variants are not counted as collisions. |
| CE-76 | NOT FIXED | cli/src/lib.rs:3748-3751 passes the argv `underlying` into `Recording`, and :17847 `results::field(into.underlying)` still records the typed word. |
| Z1-slice12-F2 | NOT FIXED | api/src/census.rs:1078-1090 `swept_series` still hard-codes BANKNIFTY/NIFTY, and :797-801 still says "exactly these two". |
| CE-1 | FIXED | api/src/master.rs:269-288 `widest()` folds `self.vendor_id`. |
| P1-02-04 | FIXED | api/src/server.rs:33473-33496: an absent or unknown `feed` is a 400 naming the vendors, with no Groww default. |
| P3-02-02 | FIXED | web/src/routes/backtest/+page.svelte:3337, :4783 use `sweptSymbolOf`, and no `split("-")` remains in web/build nodes. |

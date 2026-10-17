STATUS: IN PROGRESS

# R4-17 — Zerodha series identity: listing dates, landing refusals (day rung too), history ends at delisting, store scan, token guard

Branch `cloud/db-R4-17-20261017`, cut from `origin/feat/db-fix` 3911d1b9.
Prerequisites: none (R4-17.md); confirmed against
`git log origin/feat/db-fix --merges --format=%s`, so nothing is missing.

## Done (turns 1-2)

- Read `CLAUDE.md`, the README, `CONTEXT.md` and `R4-17.md`; mapped the owned files
  (`fetch.rs` `RowCause`, `identity_screen::screen`, `series_identity`, `cli series-identity`)
  and their neighbours (`ingest.rs:1245` caller, `cash_session_cache::read_local_lifecycle`,
  `cash_auction`'s 1980-epoch decode, `brutex_core::universe::nse_isin`).
- **NSE Masters Data v1.6 fetched and read** (HTTP 200, 817,499 bytes, 39 pages, sha256
  `a1e45b6de312ebf05298fac8a1c4fbe337b8339b7eb110ac75705925bd05487e`): §2.3 p7 epoch
  1980-01-01, §3.1.1 p9 Listing/Expulsion/Re-Admission Date LONG. Charter row appended to
  `docs/00-charter.md`. The listing refusal can therefore be ON (it is not UNVERIFIED).
  What the PDF does not say (the MII CSV's abbreviated column names) is labelled there.

- **`pull::listing`** (`listing.rs`, `listing_tests.rs`, one `pub mod` line in `lib.rs`): span per (NSE, symbol, ISIN) from the
  newest receipted master, a newer master that does not check passed over BY NAME, epoch 0 and every other unstated
  listing UNKNOWN (refuses nothing), inferred-from-vendor-span label for names in no master, removal day ends history
  unless readmitted, O(1) `span`, `for_store` remembered only once a master loads. 9 tests pass.
- Merged `origin/feat/db-fix` 62396df0 (AF-07: every cited decision number must head a ledger entry).

## Left

1. (done above)
2. `fetch.rs`: `RowCause::{NonPositivePrice, BeforeListing, InSessionJump}`; the history-ends result.
3. `identity_screen.rs`: the day-rung arm (non-positive price, before listing; never by range) and the
   minute arm (non-positive, before listing, in-session jump).
4. `series_identity.rs`: `JUMP_BPS` + jump rule; remove the `state/fix` path at :84.
5. `cli series-identity`: jumps per month, cause per suspect minute, quarantine plan under `<store>/audit/`.
6. `crates/pull/tests/token_identity.rs` (CON-13 source-shape test + self-test).
7. `crates/pull/benches/ratio/identity.rs` (listing probe p50/p99/max at 1x/10x).
8. docs/04, docs/05 (listing rule, day-rung amendment, history ends, each cause), docs/06.
9. Gates.

## Open question carried to the Mac

`JUMP_BPS` must be MEASURED over every stored Zerodha CASH minute before it is trusted (R4-17 item 2).
There is no store in the cloud. The plan: the cli scan prints the per-share in-session jump distribution
(p99, p999, max) so the Mac's read-only run IS the measurement; the constant carries a provisional value
labelled UNVERIFIED until that run is recorded beside it and in docs/06.

## Gate results

Turn 2: `cargo test -p pull --lib --locked listing` 9/9 new tests pass. Full gates not run yet.

## NEEDS THE MAC

To be completed when the code lands; the read-only runs will be
`cli series-identity ALL --segment CASH --feed zerodha` and `cli series-identity ALL --segment INDEX --feed zerodha`.

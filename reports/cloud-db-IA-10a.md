STATUS: HELD

# IA-10a (cloud): the api's journal readers name member keys and holes, the day band's run from the event (D-4896), the receipt's census and horizon

Branch `cloud/db-IA-10a-20261024`, cut from `origin/feat/db-fix` at `e851da1ca`.
Scope in force: `reports/db-specs-20261024/IA-10a.md` section 3.2 (the read side only, D-6900).
Nothing of the dropped half is built: no api child mode, no `/gdfl/*` route, no alerts route,
no restart scan, no alert put.

## Prerequisites

All merged on `origin/feat/db-fix` (checked 24 Oct 2026): IA-2, IA-16, IA-3, IA-11r.

## Done so far (WIP, not gated)

- Item 0: `pull::gdfl_cm::member_share` is `pub` (one line and its doc).
- One accessor beside `hole_is_repairable`: `pull::gdfl_import::hole_reasons()`, the five
  store-hole reasons by pull's own list (outside ownership, named below).
- Item 1: `crates/api/src/import_journal.rs`: a row keeps where a member key's `+` is
  (`member_at`), `DayRow::base`, `DayRow::member`, `DayRow::hole` (`{reason, repairable}`),
  `member_key`, `member_symbol` (share members by `member_share`, index members behind
  `NSE-INDEX-`); a parse-time start index of the `run` lines (`runs_by_start`) with
  `Parsed::started_of`, `holds_run`, `run_running_at` (D-4896).
- Item 2: `crates/api/src/imports.rs`: rows carry `base_filter`, `member`, `member_symbol`,
  `hole`; each logged refusal carries its record's `run`; counts put a hole under `holes` by
  reason and `holes_unrepairable`, never under `counts.incomplete`; `outcome=incomplete` lists
  the plain ones; `Owners::names` reads a member key as naming its member and closing its base's
  kind-day; `placed_of` places a refusal by its own run (D-4896), else by time; `RunsAsked::Every`
  replaced by `RunsAsked::Unasked`.
- Items 3 and 6: `crates/api/src/dayflags.rs`: `Names::names` reads a member key; the band no
  longer decodes every `run` line: a refusal is placed through `imports::placed_of` and the
  parse's indexes; superseded refusals say `run_from`; the record says `placed`.
- `cargo check -p api --tests --locked`: clean.

## Left

Items 4 (open-day scan), 5 (provenance), 7a/7b (receipt), the four named tests and the extreme
cases, docs/04, docs/05 (D-4896), docs/06 (L-IA-39), every gate, measurements.

## Gates

Not run yet (WIP).

## Files outside ownership

- `crates/pull/src/gdfl_import.rs`: one `pub const fn hole_reasons()` beside
  `hole_is_repairable` (spec section 11.2 names this route).

## D-numbers

D-4896 (this part's). Others, if needed: `D-TBD-IA-10a-1`, ...

## NEEDS THE MAC

To be written at the end (spec section 13).

## Log

### 2026-10-24 09:34 IST

- Waited for the spec and GO (about 50 minutes of polling), then read README, CONTEXT, IA-10a.
- Edited the five files above; `cargo check -p api --tests --locked` clean.
- HOLD found on `origin/cloud/db-base-20261024` at this save point: committed, pushed, waiting.
- NEXT: when GO returns and HOLD is gone, item 4 (`feeds_route.rs` open-day scan), then item 5
  (`provenance.rs`), then item 7, then tests.

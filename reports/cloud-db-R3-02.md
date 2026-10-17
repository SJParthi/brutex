STATUS: IN PROGRESS

# R3-02 — /db controls: date, time and text boxes, time lists, and the words on Universe, Strike and Instrument (cloud build report)

Branch `cloud/db-R3-02-20261017`, from `origin/feat/db-fix` 50093fa3.

## Prerequisites

`Merge R3-01: /db address and history: every choice is in the link, links open, Back and Forward work`
(00c07ee0) is merged into `origin/feat/db-fix`.

## Done (every change is in a file R3-02 owns)

Files changed: `web/src/lib/db/notes-controls.js`, `web/src/lib/db/selection.svelte.js`,
`web/src/lib/DayField.svelte`, `web/src/lib/Picker.svelte`, `web/src/routes/db/DbControls.svelte`;
new tests `web/tests/db-controls.test.js`, `web/tests/dayfield.test.js`. `DbCalendar.svelte` and
`dates.js` needed no change (the year calendar already reads its whole year's `/dayflags.json`;
the "calendar" of DB-025's goal is the day fields' popup). No Rust, no `Cargo.toml`/`Cargo.lock`,
no `web/build` change.

| Finding / goal | What changed | Test (fails before, passes after) |
|---|---|---|
| DB-074 (goal 1): the time lists offer every minute the days in view traded | `sessionRows` adds the windows of every special session in view (Muhurat hours, DR Saturdays, 24 Feb 2021's 15:45–16:59) from the `special` list of the `/calendar.json` answer the page already reads; a window of special days only lists no regular minute (`regularDaysIn`, prefix sums built once per calendar, O(1) per question). From time, To time and Go to all read it. | `the_time_lists_offer_every_traded_minute_of_the_days_in_view` |
| DB-045 (goal 2): order words, 1969/2099 as typed, 6,000 characters, a To box takes a day | `DayField` takes the held span as `min`/`max` and the other box's day as `after`/`before` (they were folded into one bound, so a To before From was refused in the held span's words). Refusals are `dayFieldWhy`: the held span in `dayBoundWhy`'s words naming the typed day, the order in `windowWhy`'s words (the sentence a backwards link gets), the shape via `checkDay`. One parser with the link reader (`parseDay`). A paste over 32 characters is cut and refused in one sentence. An emptied box takes `empty` (the held span's end), so the To box always holds a day. | `a_backwards_window_names_the_order`, `six_thousand_characters_are_refused_in_one_sentence`, `a To box always takes a day, and Escape closes a calendar opened with the mouse`, `the edges of the day words…` |
| Escape closes a calendar opened with the mouse | the field's root hears Escape (focus is on the opener after a click) and the panel takes focus as it opens | dayfield test 3 + Chromium harness |
| DB-045 (goal 3): the calendar strikes exactly the days its sentence says | footer is `calendarNote` (held span, plus an order clause only when it strikes more days); a struck cell's title is `dayStruckWhy` | `a_backwards_window_names_the_order` |
| DB-025 (goal 3): a refused or incomplete day is marked in any month shown | the open grid tells the page each month it shows (`onshow`); `showMonth` reads that month's `/dayflags.json` once unless the day band already covers it and keeps it (at most 48 months, keyed by feed, instrument, month and census read); an unmarked month says why under the grid | `the_calendar_marks_refused_days_outside_the_window` |
| DB-041 (goal 4): no "this feed's master" on a feed with no master; name what a tier leaves out | `instrumentListWhy` (INSTRUMENT label), `everythingWhy` (Everything row), a no-master `tierRefusal`, "another feed's instrument list"; the Universe note names the stored names a chosen tier leaves out (`tierLeavesOut`, up to 8 then a count) | `no_master_wording_on_a_feed_without_a_master` |
| DB-G63 (goal 4): "short of the fullest series" right for shares and every index | per rung, against the fullest series of the SAME segment at (month, rung); a share at 1s is trade-driven (D-3336 §2) and never "short" (`spotShortfalls`, `shortfallText`) | `the shortfall is per rung, against the same segment…`, `the edges of the shortfall fold…` |
| DB-020 (goal 4): the Strike banner never says "no row can match" over a full grid | each stranded rung carries `strandedEffect`: "no row can match" only when no row matches; a partial strike pick says the other strikes still match; a stale rung over a full grid says it narrows nothing on screen | `a stranded rung says "no row can match" only when no row does` |
| D-3338 (goal 4): Timeframe and Segment lists say months held and hidden outside the window | `hiddenMonths` per segment and per rung, "· N hidden" on the row and the sentence on its title; a rung held only outside the window is listed and refused | `the Timeframe and Segment lists say the months held outside the window…` |
| Goal 5: a note cut to two lines carries the whole sentence as hover text | `use:wholeTitle` (the node's own text, kept current) on every clamped `.note` that had no title | `a note cut to two lines carries the whole sentence as hover text` |
| Probes: 6,000 letters in the From time / Go to list search boxes | `Picker`'s search box keeps 128 characters and says so in one sentence; "Nothing matches" quotes at most 40 characters | `six_thousand_characters_are_refused_in_one_sentence` + harness |
| Measured target (list open) | off-screen list rows skip layout and paint (`content-visibility: auto`) | harness timing below |

**Fail-before evidence.** On the base tree both new files fail at import (the functions do not
exist). With this branch's `notes-controls.js` copied onto the base tree so the imports resolve,
all 10 tests the two files held at that point failed against the base's components and state
(each test also checks the wiring that calls the function). On the branch they pass.

**Two existing tests pin source shapes, in files R3-02 does not own, and both still pass:**
`calendar-building.test.js` evaluates the calendar-read callback with a fixed scope (so the
callback stores the raw `special` list and `specialsOf` reads it outside), and `db-address.test.js`
SC-CLOSE-18 pins `return sessionGrid(rungSecs, held, { venue, lo: fromDay || heldDays.lo, hi: toDay || heldDays.hi });`
(kept, inside `rungGrid`).

## Chromium harness (this container, headless Chromium 1194 via Playwright 1.56.1)

A scratch page mounts the real `Picker.svelte` (376 rows: Any + 375 minutes) and the real
`DayField.svelte` as /db drives them (To date, held 01–28 Mar 2019, From 15 Mar 2019). Measured on
this branch:

- Escape after opening the calendar with the mouse: open 1 → after Escape 0.
- Footer: "01 Mar 2019 – 28 Mar 2019 can be picked. This selection holds … Days before the From
  date, 15 Mar 2019, are struck too: a window ends on or after the day it starts."; 14 Mar's title
  "Before the From date, 15 Mar 2019: …".
- Typed `14 Mar 2019`: "From 15 Mar 2019 is after To 14 Mar 2019, so the window holds no day —
  swap them or move one. Not applied: the field still holds 15 Mar 2019. Escape restores it."
- Typed `01 Jan 1969`: "To date 01 Jan 1969: Outside 01 Mar 2019 to 28 Mar 2019 — This
  selection holds … Not applied: …".
- 6,000 characters filled into the day box: box holds 32; "To date is 6,000 characters long, and
  /db reads at most 32 — cut, not applied: the field still holds 15 Mar 2019."
- Emptied To then blur: the box shows `28 Mar 2019`; one change, to `2019-03-28`.
- 6,000 characters in the list's search box: box holds 128; "This search is 6,000 characters long,
  and /db reads at most 128 — cut to its first 128."; `Nothing matches "ZZZZ…"… (128 characters)`.
- Network requests while doing all of the above, including opening the list 220 times: 0.
- Opening the 375-minute list, click to the frame after it is drawn: see Gate results.

## Left

- Workspace `cargo test` (running), the final Picker timing on an idle machine, the third mutation
  run on the final `notes-controls.js`.
- Not R3-02's file: the pager's "Go to" text box (`DbBarsGrid.svelte`, `parseGoto`) — the
  6,000-character probe there belongs to its owner.

## Gate results (so far)

- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean (exit 0).
- `cargo test --workspace --locked`: running.
- `cargo deny check`: base and branch fail on exactly the same five advisories
  (RUSTSEC-2026-0344, -0346, -0347, -0348, -0349); `bans ok, licenses ok, sources ok`.
- Web suite `node --test tests/*.test.js`: base 1393 pass / 0 fail / 5 cancelled; branch 1403 pass /
  0 fail / 5 cancelled (before the last edge tests were added) — the same five timer tests
  (`ask.test.js` ×3, `census-worker.test.js` ×2) are cancelled on both ("Promise resolution is still
  pending but the event loop has already resolved").
- `svelte-check`: 0 errors and 0 warnings in the touched files; 6 errors elsewhere
  (`ingest-months.js`, `routes/ingest/+page.svelte` ×2, `tests/ingest-months.test.js`,
  `tests/settings-ceilings.test.js` ×2), none in a file R3-02 touches.
- Coverage (node's `--experimental-test-coverage`, the two new test files only):
  `notes-controls.js` 100% lines (914/914 at the first full measurement), 100% branches, 100%
  functions. The Svelte components and the runes module `selection.svelte.js` cannot be measured
  by any tool installed here (node runs them only as compiled `data:` modules): NOT MEASURED; their
  wiring is pinned by source checks and the components' behaviour by server renders and the
  Chromium harness.
- Mutation (Stryker 9 `@stryker-mutator/core`, installed in the scratch dir only, command runner
  over the two new test files, `notes-controls.js`): run 1 592 killed / 2 timeout / 181 survived
  (95 of them in the module's pre-existing vocabulary constants, the rest boundaries and redundant
  conditions); run 2 710 killed / 1 timeout / 5 survived; run 3 on the final file: running.
  `selection.svelte.js` and the components: NOT MEASURED (no mutation tool here runs Svelte).

## NEEDS THE MAC

To be completed with exact commands.

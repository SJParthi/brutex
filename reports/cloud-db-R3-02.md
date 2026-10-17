STATUS: IN PROGRESS

# R3-02 — /db controls: date, time and text boxes, time lists, and the words on Universe, Strike and Instrument (cloud build report)

Branch `cloud/db-R3-02-20261017`, from `origin/feat/db-fix` 50093fa3.

## Prerequisites

`Merge R3-01: /db address and history: every choice is in the link, links open, Back and Forward work`
(00c07ee0) is merged into `origin/feat/db-fix`.

## Done (every change is in a file R3-02 owns)

| Finding / goal | What changed | Test (fails before, passes after) |
|---|---|---|
| DB-074 (goal 1): time lists offer every minute the days in view traded | `sessionRows` adds the windows of every special session in view (Muhurat hours, DR Saturdays, 24 Feb 2021's 15:45–16:59) from the `special` list of the `/calendar.json` answer the page already reads; a window of special days only lists no regular minute (`regularDaysIn`, O(1) prefix sums per calendar). From time, To time and Go to all read it. | `the_time_lists_offer_every_traded_minute_of_the_days_in_view` |
| DB-045 (goal 2): order words, 1969/2099, 6,000 characters, a To box takes a day | `DayField` takes the held span as `min`/`max` and the other box's day as `after`/`before`; refusals are `dayFieldWhy`: held span in `dayBoundWhy`'s words naming the typed day, order in `windowWhy`'s words, shape via `checkDay`; one parser (`parseDay`); pastes over 32 characters are cut and refused in one sentence; an emptied box takes `empty` (the held span's end). | `a_backwards_window_names_the_order`, `six_thousand_characters_are_refused_in_one_sentence`, `a To box always takes a day, and Escape closes…` |
| Escape closes a calendar opened with the mouse | the field root hears Escape (focus is on the opener after a click), and the panel takes focus as it opens | dayfield test 3 (source) + Chromium harness (below) |
| DB-045 (goal 3): the calendar strikes exactly the days its sentence says | footer is `calendarNote` (held span, plus an order clause only when it strikes more); a struck cell's title is `dayStruckWhy` | `a_backwards_window_names_the_order` (calendarNote/dayStruckWhy) |
| DB-025 (goal 3): refused/incomplete days marked in any month shown | the open grid tells the page each month it shows (`onshow`); `showMonth` reads that month's `/dayflags.json` once unless the day band already covers it, keeps it (≤48 months, keyed by feed, instrument, month, census read); an unmarked month says why under the grid | `the_calendar_marks_refused_days_outside_the_window` |
| DB-041 (goal 4): no "this feed's master" on a feed with no master; name what a tier leaves out | `instrumentListWhy`, `everythingWhy`, no-master `tierRefusal`, "another feed's instrument list"; the Universe note names the stored names a tier leaves out (`tierLeavesOut`) | `no_master_wording_on_a_feed_without_a_master` |
| DB-G63 (goal 4): "short of the fullest series" right for shares and every index | per rung, against the fullest of the same segment at (month, rung); a share at 1s is trade-driven and never "short" (`spotShortfalls`, `shortfallText`) | `the shortfall is per rung, against the same segment…` |
| DB-020 (goal 4): the Strike banner never says "no row can match" over a full grid | each stranded rung carries `strandedEffect`: "no row can match" only when no row matches; a partial strike pick says the other strikes still match | `a stranded rung says "no row can match" only when no row does` |
| D-3338 (goal 4): Timeframe and Segment lists say months held and hidden outside the window | `hiddenMonths` per segment and per rung; a rung held only outside the window is listed and refused | `the Timeframe and Segment lists say the months held outside the window…` |
| Goal 5: a note cut to two lines carries the whole sentence as hover text | `use:wholeTitle` on every clamped `.note` without a title | `a note cut to two lines carries the whole sentence as hover text` |
| Probes: 6,000 letters in the From time / Go to list search boxes | `Picker`'s search box keeps 128 characters and says so in one sentence; "Nothing matches" quotes at most 40 characters | `six_thousand_characters_are_refused_in_one_sentence` + harness |

Fail-before evidence: with `notes-controls.js` from this branch copied onto the base tree (so the
imports resolve), all 10 tests in the two new files fail against the base's components and state;
on the branch all 10 pass. Without that copy both files fail on the base at import.

Two existing tests in files R3-02 does not own pin source shapes that the fix had to keep:
`calendar-building.test.js` evaluates the calendar-read callback with a fixed scope (so it stores
the raw `special` list and `specialsOf` reads it outside the callback), and `db-address.test.js`
SC-CLOSE-18 pins the `return sessionGrid(rungSecs, held, { venue, lo: …, hi: … })` line (kept, in
`rungGrid`). Both pass.

## Left

- Picker open time for the 375-minute list measured above 50 ms under load; re-measure on an idle
  machine and trim the open cost if still over.
- Coverage and mutation runs on `notes-controls.js`; workspace `cargo test`.
- Not R3-02's file: the pager's "Go to" text box (`DbBarsGrid.svelte`, `parseGoto`) — the 6,000
  character probe there belongs to its owner.

## Gate results (so far)

- `cargo fmt --check`: clean.
- `cargo deny check`: base and branch fail on exactly the same five advisories
  (RUSTSEC-2026-0344, -0346, -0347, -0348, -0349); `bans ok, licenses ok, sources ok`.
- Web suite `node --test tests/*.test.js`: base 1393 pass / 0 fail / 5 cancelled; branch 1403 pass /
  0 fail / 5 cancelled — the same five timer tests (`ask.test.js` ×3, `census-worker.test.js` ×2)
  are cancelled on both ("Promise resolution is still pending but the event loop has already
  resolved").
- `svelte-check --threshold error`: 6 errors, all in files R3-02 does not touch and all present on
  the base (`ingest-months.js`, `routes/ingest/+page.svelte`, two tests); none in the touched files.
- clippy, workspace tests: running.

## NEEDS THE MAC

To be completed with exact commands.

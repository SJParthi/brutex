STATUS: DONE

# R3-02 — /db controls: date, time and text boxes, time lists, and the words on Universe, Strike and Instrument (cloud build report)

Branch `cloud/db-R3-02-20261017`, from `origin/feat/db-fix` 50093fa3 (no merge of a newer
`feat/db-fix` was needed: its head is still 50093fa3 at the time of this report).

`cargo test --workspace` is NOT green in this container. It has 27 failures, and none comes from
this branch: 25 are permission tests that cannot hold when the tests run as root (this container
runs as uid 0), and all 25 pass when re-run as `nobody`; 1 is an inode-reuse flake (it passes 2
runs in 3 as `nobody`); 1 (`cited_commits`) fails identically on the base tree. This branch
changes no `.rs`, `Cargo.toml` or `Cargo.lock`, so the Rust test binaries are the base's. Details
and the re-run command are under Gate results and NEEDS THE MAC. Every other gate is green, and
`cargo deny` fails on exactly the base's five advisories.

## Prerequisites

`Merge R3-01: /db address and history: every choice is in the link, links open, Back and Forward work`
(00c07ee0) is merged into `origin/feat/db-fix`.

## Done (every change is in a file R3-02 owns)

Files changed: `web/src/lib/db/notes-controls.js`, `web/src/lib/db/selection.svelte.js`,
`web/src/lib/DayField.svelte`, `web/src/lib/Picker.svelte`, `web/src/routes/db/DbControls.svelte`;
new tests `web/tests/db-controls.test.js` (12 tests) and `web/tests/dayfield.test.js` (4 tests).
`DbCalendar.svelte` and `dates.js` needed no change: the year calendar already reads its whole
year's `/dayflags.json`, and the "calendar" of DB-025's goal is the day fields' popup. No Rust, no
`Cargo.toml`/`Cargo.lock`, no `web/build`, no `docs/` change.

| Finding / goal | What changed | Test (fails before, passes after) |
|---|---|---|
| DB-074 (goal 1): the time lists offer every minute the days in view traded | `sessionRows` adds the windows of every special session in view (Muhurat hours, DR Saturdays, 24 Feb 2021's 15:45–16:59) from the `special` list of the `/calendar.json` answer the page already reads; a window of special days only lists no regular minute (`regularDaysIn`: prefix sums built once per calendar, O(1) per question). From time, To time and Go to all read it. | `the_time_lists_offer_every_traded_minute_of_the_days_in_view`, `the edges of the special minutes and the list rows` |
| DB-045 (goal 2): order words, 1969/2099 as typed, 6,000 characters, a To box takes a day | `DayField` takes the held span as `min`/`max` and the other box's day as `after`/`before`. They used to be folded into one bound, so a To before From was refused in the held span's words. Refusals come from `dayFieldWhy`: the held span in `dayBoundWhy`'s words, naming the typed day; the order in `windowWhy`'s words (the sentence a backwards link gets); the shape via `checkDay`. The box reads with the same parser as the link reader (`parseDay`). A paste over 32 characters is cut and refused in one sentence. An emptied box takes `empty` (the held span's end), so the To box always holds a day. | `a_backwards_window_names_the_order`, `six_thousand_characters_are_refused_in_one_sentence`, `a To box always takes a day, and Escape closes a calendar opened with the mouse`, `the edges of the day words…` |
| Escape closes a calendar opened with the mouse | the field's root hears Escape (after a click, focus is on the opener) and the panel takes focus as it opens | dayfield test 3 + Chromium harness |
| DB-045 (goal 3): the calendar strikes exactly the days its sentence says | the footer is `calendarNote` (the held span, plus an order clause only when it strikes more days); a struck cell's title is `dayStruckWhy` | `a_backwards_window_names_the_order` |
| DB-025 (goal 3): a refused or incomplete day is marked in any month shown | the open grid tells the page each month it shows (`onshow`). `showMonth` reads that month's `/dayflags.json` once, unless the day band already covers it, and keeps it (at most 48 months, keyed by feed, instrument, month and census read). A month left unmarked says why under the grid. | `the_calendar_marks_refused_days_outside_the_window` |
| DB-041 (goal 4): no "this feed's master" on a feed with no master; name what a tier leaves out | `instrumentListWhy` (INSTRUMENT label), `everythingWhy` (Everything row), a no-master `tierRefusal`, and "another feed's instrument list". The Universe note names the stored names a chosen tier leaves out (`tierLeavesOut`: up to 8, then a count). | `no_master_wording_on_a_feed_without_a_master` |
| DB-G63 (goal 4): "short of the fullest series" is right for shares and every index | the shortfall is per rung, against the fullest series of the SAME segment at (month, rung). A share at 1s is trade-driven (D-3336 §2) and is never called "short" (`spotShortfalls`, `shortfallText`). | `the shortfall is per rung, against the same segment…`, `the edges of the shortfall fold…` |
| DB-020 (goal 4): the Strike banner never says "no row can match" over a full grid | each stranded rung carries `strandedEffect`: "no row can match" only when no row matches. A partial strike pick says the other strikes still match; a stale rung over a full grid says it narrows nothing on screen. | `a stranded rung says "no row can match" only when no row does` |
| D-3338 (goal 4): the Timeframe and Segment lists say which months are held and hidden outside the window | `hiddenMonths` per segment and per rung: "· N hidden" on the row, with the sentence on its title. A rung held only outside the window is listed and refused. | `the Timeframe and Segment lists say the months held outside the window…` |
| Goal 5: a note cut to two lines carries the whole sentence as hover text | `use:wholeTitle` (the node's own text, kept current) on every clamped `.note` that had no title | `a note cut to two lines carries the whole sentence as hover text` |
| Probes: 6,000 letters in the From time / Go to list search boxes | `Picker`'s search box keeps 128 characters and says so in one sentence; "Nothing matches" quotes at most 40 characters | `six_thousand_characters_are_refused_in_one_sentence` + harness |
| Measured target: opening the 375-minute list | list rows off the screen skip layout and paint (`content-visibility: auto`) | harness timing below |
| (module whole) | the pre-existing vocabulary in `notes-controls.js` is pinned, so the touched module has no surviving mutant | `the fixed vocabulary of the strips is exactly this`, `the census key and the membership tokens…` |

**Fail-before evidence.** On the base tree both new files fail at import, because the functions do
not exist there. With this branch's `notes-controls.js` copied onto the base tree so the imports
resolve, all 10 tests the two files held at that point failed against the base's components and
state, because each test also checks the wiring that calls the function. On the branch all 16 pass.

**Two existing tests pin source shapes in files R3-02 does not own, and both still pass.**
`calendar-building.test.js` evaluates the calendar-read callback with a fixed scope, so the
callback stores the raw `special` list and `specialsOf` reads it outside. `db-address.test.js`
SC-CLOSE-18 pins `return sessionGrid(rungSecs, held, { venue, lo: fromDay || heldDays.lo, hi: toDay || heldDays.hi });`,
which is kept inside `rungGrid`.

**Not R3-02's file:** the pager's "Go to" text box (`DbBarsGrid.svelte`, `parseGoto`). The
6,000-character probe of that box belongs to its owner.

## Chromium harness (this container: headless Chromium 1194 via Playwright 1.56.1, 4 vCPU)

A scratch page, outside the repository, mounts the real `Picker.svelte` (376 rows: Any plus 375
minutes) and the real `DayField.svelte`, driven as /db drives them (To date, held 01–28 Mar 2019,
From 15 Mar 2019). Measured on this branch:

- Escape after opening the calendar with the mouse: calendar open 1, then 0 after Escape.
- Footer: "01 Mar 2019 – 28 Mar 2019 can be picked. This selection holds … Days before the From
  date, 15 Mar 2019, are struck too: a window ends on or after the day it starts."; 14 Mar's title
  is "Before the From date, 15 Mar 2019: …".
- Typed `14 Mar 2019`: "From 15 Mar 2019 is after To 14 Mar 2019, so the window holds no day —
  swap them or move one. Not applied: the field still holds 15 Mar 2019. Escape restores it."
- Typed `01 Jan 1969`: "To date 01 Jan 1969: Outside 01 Mar 2019 to 28 Mar 2019 — This
  selection holds … Not applied: …".
- 6,000 characters filled into the day box: the box holds 32; "To date is 6,000 characters long,
  and /db reads at most 32 — cut, not applied: the field still holds 15 Mar 2019."
- Emptied To, then blur: the box shows `28 Mar 2019`; one change, to `2019-03-28`.
- 6,000 characters in the list's search box: the box holds 128; "This search is 6,000 characters
  long, and /db reads at most 128 — cut to its first 128."; `Nothing matches "ZZZZ…"… (128 characters)`.
- Network requests during all of the above, including 220 openings of the list: 0.
- **Opening the 375-minute list**, timed from the click to the frame after the 376 rows are drawn,
  20 warm-up openings then n = 200, with long tasks (> 50 ms) counted by `PerformanceObserver`:

  | build | load avg | p50 | p99 | max | long tasks |
  |---|---|---|---|---|---|
  | branch | 2.66 | 18.2 ms | 29.2 ms | 29.9 ms | 0 |
  | branch | 2.47 | 17.4 ms | 27.3 ms | 37.9 ms | 0 |
  | branch | 2.36 | 17.1 ms | 28.3 ms | 30.4 ms | 0 |
  | base | 2.44 | 32.2 ms | 54.1 ms | 60.2 ms | 11 (longest 86 ms) |
  | base | 2.52 | 29.0 ms | 46.7 ms | 54.0 ms | 3 (longest 67 ms) |
  | base | 2.30 | 29.8 ms | 52.4 ms | 87.3 ms | 5 (longest 87 ms) |

  Earlier rounds on the branch at load 2.0–2.3 measured p99 39.7–40.6 ms, max 44.9–46.9 ms, with
  1–3 long tasks of 59–62 ms somewhere in the 220-opening loop; those rounds did not attribute the
  long tasks to openings. The load came from one Rust test running beside the measurement. On this
  container the target (no pause over 50 ms, n = 200) held in the three attributed rounds and not in
  every earlier round, so it is measured here, NOT PROVED on the Mac; it is listed below.

## Gate results

- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean (exit 0).
- `cargo test --workspace --locked -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store`
  (the known slow cli test skipped, per CONTEXT): 9,009 passed, 27 failed, 47 ignored. The 27:
  - 25 permission tests (api 12, cli 3, pull 3, store 3, telemetry 4) assert that a mode-000 or
    read-only path refuses, which never holds for root ("mode 000 does not stop this process from
    searching …: run the tests as a user the mode bits bind"). Re-run as `nobody` on the same
    binaries: api 12/12, cli 3/3, pull 3/3, telemetry 4/4 and store 3/3 pass.
  - `store` `staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported`
    asserts a new file gets a new inode number. It fails as root; as `nobody` it passed in 2 runs
    of 3. When the filesystem reuses the freed inode, the test fails.
  - `store` `cited_commits::every_commit_the_store_records_cite_is_on_main_or_says_why_not`:
    `docs/06-limits.md` cites feat/db-fix commits (023d6ab8, 079ad4af, 4756cce0, …) that are not on
    `origin/main`. It fails identically on the base tree (built separately here).
  None of the 27 involves a file this branch changes.
- `cargo deny check`: base and branch both fail on exactly the same five advisories
  (RUSTSEC-2026-0344, -0346, -0347, -0348, -0349); `bans ok, licenses ok, sources ok`.
- Web suite `node --test tests/*.test.js` (Node 22.22.0): base 1,393 pass / 0 fail / 5 cancelled;
  branch 1,409 pass / 0 fail / 5 cancelled. The same five timer tests (`ask.test.js` ×3,
  `census-worker.test.js` ×2) are cancelled on both, also when run alone ("Promise resolution is
  still pending but the event loop has already resolved").
- `svelte-check`: 0 errors and 0 warnings in the touched files. There are 6 errors elsewhere
  (`ingest-months.js`, `routes/ingest/+page.svelte` ×2, `tests/ingest-months.test.js`,
  `tests/settings-ceilings.test.js` ×2), none in a file R3-02 touches.
- Coverage (node's `--experimental-test-coverage`, the two new test files alone):
  `notes-controls.js` 100% lines, 100% branches, 100% functions. `selection.svelte.js` (a runes
  module) and the three components cannot be measured by any tool installed here, because node
  runs them only as compiled `data:` modules: NOT MEASURED. Their wiring is pinned by source
  checks, and the components' behaviour by server renders of the real `DayField` and by the
  Chromium harness.
- Mutation (Stryker 9, `@stryker-mutator/core` installed in a scratch directory only, command runner
  over the two new test files, mutating `notes-controls.js`):
  - run 1: 592 killed / 2 timeout / 181 survived (95 in the module's pre-existing vocabulary
    constants; the rest boundaries and redundant conditions);
  - run 2: 710 / 1 / 5;
  - run 3, on the final file: 705 killed, 1 timeout (an infinite-loop mutant, detected), 0 survived
    — score 100%.

  `selection.svelte.js` and the components: NOT MEASURED, since no mutation tool here runs Svelte.

## NEEDS THE MAC

Run from the merged tree, on a FRESH shadow server started with the Mac's own `state/fix/shadow.sh`
and `serve.sh` (never :8080, never the real store). `<P>` below is that shadow's port.

1. **Rust tests as the operator's user** (not root), where the 25 permission tests can hold:
   `cargo test --workspace --locked -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store`
   — expected: only `cited_commits` (red on the base too) and, possibly, the inode-reuse staging test
   fail.
2. **Front-end build** (the served page is the committed `web/build`; this branch does not touch it):
   `cd web && npm ci && npm run build` and commit `web/build`, as R3-05 repair 5 did.
3. **Web suite on the Mac**: `cd web && node --test tests/*.test.js` (expected 1,409 pass, 0 fail;
   the five timer tests may pass there).
4. **Time lists on special-session days** (DB-074), each URL opened on the shadow; open the From time
   list and read its rows:
   - `http://127.0.0.1:<P>/db?feed=gdfl&sym=NIFTY&seg=spot&tf=1s&from=2024-11-01&to=2024-11-01&view=bars`
     → 18:00 to 18:59 only, each "special session · 01 Nov 2024"; no 09:15.
   - `…&from=2023-11-12&to=2023-11-12…` → 18:15 to 19:14.
   - `…&from=2021-02-24&to=2021-02-24…` → 09:15 to 11:39 and 15:45 to 16:59, no 12:00.
   - `…&from=2024-10-28&to=2024-11-01…` → 09:15 to 15:29, then 18:00 to 18:59.
   - The To time and Go to lists hold the same minutes. The browser's network panel shows no request
     when a list opens.
5. **Calendar flags outside the window** (DB-025): open
   `http://127.0.0.1:<P>/db?feed=gdfl&sym=NIFTY&seg=spot&tf=1s&from=2022-03-08&to=2022-03-08&view=bars`,
   open the From date calendar (March 2022): exactly one `/dayflags.json?…from=2022-03-01&to=2022-03-31`
   request, and 07 Mar 2022 is marked, its title naming the refusal. Page Back to February 2022: one
   more request, for that month. Close and reopen on March: no new request.
6. **Day boxes** on `http://127.0.0.1:<P>/db?feed=zerodha&sym=NIFTY&seg=spot&tf=1min&from=2026-09-01&to=2026-09-30&view=bars`:
   - type `01 Aug 2026` into To date → "From 01 Sep 2026 is after To 01 Aug 2026, so the window
     holds no day — swap them or move one." (if 01 Aug 2026 is inside the held span; otherwise the
     held-span sentence naming 01 Aug 2026);
   - paste 6,000 characters into From date and into To date → one sentence each, and the box holds
     32 characters;
   - empty To date and leave the box → it shows the last day held;
   - open the From date calendar with the mouse and press Escape → it closes.
7. **6,000 characters in the list search boxes**: open the From time list and the Go to list, paste
   6,000 letters into each → the box holds 128 characters and says "This search is 6,000 characters
   long, and /db reads at most 128 — cut to its first 128."
8. **No-master wording** (DB-041) on `http://127.0.0.1:<P>/db?feed=gdfl&universe=fno`: the INSTRUMENT
   label's tooltip and the Everything row never say "this feed's master"; the Universe note says
   "F&O underlyings leaves out N stored name(s): …" with real names (expected to include option-only
   or renamed underlyings such as ALBK or DHFL, NOT PROVED here).
9. **Shortfall wording** (DB-G63): hover RELIANCE and NIFTY in the GDFL and Zerodha Instrument lists →
   per-rung clauses "1min: N bar(s) short of the fullest CASH/INDEX series of each month", and
   "1s: trade-driven …" for a share at 1s.
10. **Strike banner** (DB-020): on `http://127.0.0.1:<P>/db?feed=gdfl&sym=NIFTY&seg=options&tf=1s&exp=2019-03-28&view=bars`,
    pick two strikes, then choose another expiry whose chain lacks one of them → the banner reads
    "… so it matches nothing; the table shows the 1 other value(s) still chosen on this rung.", never
    "no row can match" over rows.
11. **Hidden months** (D-3338): `http://127.0.0.1:<P>/db?feed=zerodha&sym=NIFTY&seg=spot&tf=1day` → the
    Timeframe row "1day N held · M hidden", with M the count of 2015–2017 months; the Segment row
    likewise.
12. **Clipped notes** at a 1600 × 1000 window: hover every two-line note under the strip → the whole
    sentence shows as hover text.
13. **List-open timing on the real page** (the measured target): on the URL of check 4, open and close
    the From time list n = 200 times (after 20 warm-up openings), timing each from the click to the
    frame after its rows are drawn, and count long tasks with
    `PerformanceObserver({type: 'longtask'})`. Expected: no long task over 50 ms; report p50, p99 and
    max. The harness that measured the table above clicked `.pbtn`, waited until
    `.pmenu label` numbered 376, awaited one `requestAnimationFrame`, then clicked again to close.

## Left in this container (not deleted, per the rules)

Scratch only, outside the repository: the base worktree and harness, Stryker runs and logs under
this session's scratchpad; `/tmp/r302-nobody` (scratch directory for the `nobody` re-runs).
`/dev/null` was briefly replaced by a regular file by a mistaken command in this session; it was
restored as the character device (1, 3), and the stray file was moved into the scratchpad, not
deleted.

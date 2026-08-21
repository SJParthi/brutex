<script>
  /**
   * INGEST — ask for a window, watch it happen, read every outcome.
   *
   * THE THREE THINGS THIS PAGE EXISTS TO FIX
   *
   * 1. A pull takes up to 32 minutes and used to show NOTHING until it
   *    finished. A working pull and a hung one looked identical, so the only
   *    way to tell them apart was to wait half an hour and find out.
   *
   * 2. A run that refused 407 of 785 instruments showed ONE reason. The
   *    receipt renders `done.failures.iter().take(5)` — five, and the run's
   *    own `Members failed` count is beside it, so the page can SEE that it
   *    was truncated and say so instead of implying the list is complete.
   *
   * 3. The bar length was not askable. `api::ingest::parse_spot` has read a
   *    `granularity` field all along; the server's own form never emitted one,
   *    so every request meant `1min` by omission and nobody could see that
   *    from the page.
   *
   * WHAT IS MEASURED AND WHAT IS ESTIMATED, AND THE PAGE NEVER BLURS THEM
   *
   * There is no pull-status endpoint. `crates/api/src/server.rs`'s router is
   * `/`, `/instruments`, `/instruments.json`, `/feeds.json`, `/bars.json`,
   * `/store.json`, `/typeahead.js`, `/pull`, `/pull/spot`, `/pull/fno`,
   * `/audit`, `/store`, `/health` — and nothing there answers "how far along
   * is the run". So this page does NOT invent a progress bar out of a timer.
   *
   * It polls `/store.json`, which the server rebuilds per request — its own
   * comment says "FRESH, NOT THE STARTUP SNAPSHOT" — and reports the growth it
   * measures. Instrument-months that gained rows are a FACT. The share of the
   * request they represent is an ESTIMATE, because the denominator is
   * arithmetic on what was asked for rather than a number the server states,
   * and the page labels it as one. A remaining time appears only once a rate
   * has actually been observed, and it is labelled an extrapolation.
   *
   * `CLAUDE.md` §3 rule 6: never claim a measurement not taken, and label
   * extrapolations as extrapolations.
   */
  import { feeds } from '$lib/feeds.svelte.js';
  import Picker from '$lib/Picker.svelte';
  import { catalogue } from '$lib/index.svelte.js';
  import { MON, dayLabel, monthLabel, stampLabel } from '$lib/dates.js';
  // `untrack`, because one effect on this page must react to a FEED CHANGE and
  // to nothing else — see the rung-drop effect for why an effect that reacts to
  // its own write is a hazard rather than a nicety.
  import { onMount, untrack } from 'svelte';
  // `survey` IS ALREADY IN MEMORY AND COSTS NOTHING TO READ.
  //
  // `$lib/feeds.svelte.js` awaits `surveyStores(feeds.all)` while loading the
  // feed list, because picking a sensible default feed means knowing which
  // stores hold anything. So every page — this one included — boots with one
  // `FeedHolding` per feed already folded and stamped with the feed list it
  // answered for. Importing it here adds a READER, not a fetch, and the call
  // below is a no-op whenever that answer is already held.
  import {
    store,
    survey,
    surveyStores,
    syncStore,
    refreshStore,
    watchStore,
    foldMonths
  } from '$lib/store.svelte.js';
  import { notAReceipt, RECEIPT_HEADER } from '$lib/receipt.js';
  // IMPORTED AS `request`, AND THE ALIAS IS THE WHOLE POINT.
  //
  // `readFolder` declares its own `const ask` for probing folder segments. A
  // module-scope `import { ask }` is shadowed by it, so `await ask(url)` inside
  // that helper called THE HELPER, not the wrapper -- an unbounded recursion
  // that builds clean, type-checks clean, and is invisible to every test in
  // this tree because none of them drive `readFolder`. It shipped in 74ad3a9
  // and was removed by accident when 6eee183 rewrote the call site back to a
  // bare `fetch`.
  //
  // The name is the fix. `request` cannot be shadowed by anything here, and
  // `web/tests/timeout.test.js` now refuses a bare `fetch` anywhere in
  // `web/src` so the ceiling cannot be reverted silently a second time.
  import { ask as request } from '$lib/ask.js';
  import { exact } from '$lib/money.js';
  // FINDING A NAME IN 750 OF THEM. One `Map.get` per keystroke against an
  // index over distinct symbols; see `$lib/find.js` for why an infix index
  // rather than the prefix one the typeahead uses.
  import { build as buildFind, probe as probeFind, symbolCount } from '$lib/find.js';
  // A STORE KEY CARRIES ITS SEGMENT, AND THE CENSUS HAS TO READ IT.
  import { parseKey, segmentOf } from '$lib/instrument.js';
  // THE SELECTION LIVES IN THE ADDRESS BAR. See `$lib/urlstate.js` for why,
  // and for the measurement of what used to reset on every reload.
  import { encode as encodeSel, decode as decodeSel, same as sameSel } from '$lib/urlstate.js';

  // ─────────────────────── WHAT AN ANSWER LOOKS LIKE ───────────────────────
  //
  // THE SHAPES ARE WRITTEN DOWN ONCE, AND EVERY ONE OF THEM WAS READ OFF A
  // LIVE RESPONSE OR OFF THE RUST THAT WRITES IT — never guessed from the
  // reader. `/feeds.json`, `/folder.json?feed=truedata&segment=INDEX`,
  // `/instruments.json?feed=dhan` and `/ingest/status.json` were each fetched
  // from the running binary on 15 Aug 2026, and the two shapes that came back
  // empty on a paused autopilot (`in_flight`, `waiting_on`) are transcribed
  // from the `write!` that emits them in `crates/api/src/ingest.rs` rather than
  // from what this file happens to read. §3 rule 1.
  //
  // WHY THEY EXIST AT ALL is the argument `$lib/feeds.svelte.js` already makes
  // for its own `Feed`: an unannotated `$state(null)` infers `never` and an
  // unannotated `$state([])` infers `never[]`, so every read of a field off one
  // is an error reported at the READER — dozens of them, none where the fix is.
  // One annotation at the declaration clears the lot, and it also records where
  // the shape came from, which a checker cannot.

  /**
   * ONE RUNG OF ONE FEED'S HISTORY CLAIM, as `/feeds.json` sends it.
   *
   * `crates/api/src/server.rs` builds the array from
   * `pull::vendor::Descriptor::history` — one row per rung the feed either
   * serves or has a recorded claim for. `contested` is the DISPLACED claim and
   * travels with the binding one; three of this repository's four recorded rows
   * have two sources that disagree, which is why it is a field and not a
   * footnote. `pairFloor` below reads exactly these names.
   *
   * IMPORTED NOW, NOT RESTATED. This shape was spelled out here in full, which
   * put a second copy of one server contract in the tree — the exact drift the
   * note below already argued against for `Feed`'s own fields, and the argument
   * holds identically for this one. It lives in `$lib/feeds.svelte.js` beside
   * the `Feed` it hangs off, and moves when that moves.
   *
   * @typedef {import('$lib/feeds.svelte.js').HistoryRow} HistoryRow
   */

  /**
   * A `/feeds.json` ROW AS THIS PAGE READS IT — and it is now simply `Feed`.
   *
   * This was `Feed & { history?: HistoryRow[] }`, an INTERSECTION adding the one
   * field the shared typedef did not spell. The reasoning recorded here was that
   * widening the shared shape "would edit a file three other routes import for a
   * fact only this one asks about". That was true of the EDIT and false of the
   * FIELD: `history` is on every row the server sends, so the shared typedef was
   * not describing `/feeds.json` — it was describing the subset one page had
   * happened to need first, and calling it the contract.
   *
   * `fno` was missing for the same reason and did not stay quiet about it: two
   * `svelte-check` errors, both reported at this page rather than at the
   * typedef, for a field every descriptor row carries.
   *
   * OPTIONAL ARITY IS PRESERVED, and still matters for the reason given below:
   * `rungServed` reads an absent array as "this server states nothing" rather
   * than "nothing is served" — the running-binary-predates-the-field case. The
   * shared typedef spells it `history?:` so that distinction survives the move.
   *
   * @typedef {import('$lib/feeds.svelte.js').Feed} FeedRow
   */

  /**
   * `GET /folder.json?feed=…` — the walk of an archive feed's directory.
   *
   * Read from the live answer for `truedata&segment=INDEX`: a `path`, a `reach`
   * of `{state, earliest, latest, files, rows}`, 8,632 `instruments`, a
   * `collisions` count and 123 `rejected` rows each carrying the decoder's own
   * words. A REFUSAL comes back through the same fetch carrying `refused` and
   * nothing else — which is why every field but that one is optional. The page
   * tells the two apart by `r.ok`, never by probing for a key.
   *
   * THE THREE STATES ARE THE WHOLE SET AND THE TWO ENDS ARE DISCRIMINATED BY
   * THEM. `crates/api/src/folder.rs` maps `pull::folder::Reach` to exactly
   * `empty`, `blank` and `days`, and its own comment states the rule this union
   * encodes: the two ends are `null` on the arms that have none and NEVER
   * absent, because a missing key and a null key read alike in a browser and
   * mean different things here.
   *
   * @typedef {{
   *   feed?: string,
   *   kind?: string,
   *   verb?: string,
   *   path?: string,
   *   reach?: {
   *     state: 'days', earliest: string, latest: string,
   *     files: number, rows: number
   *   } | {
   *     state: 'empty' | 'blank', earliest: null, latest: null,
   *     files: number, rows: number
   *   },
   *   instruments?: string[],
   *   collisions?: number,
   *   rejected?: { path: string, why: string }[],
   *   refused?: string
   * }} FolderBody
   */

  /**
   * ONE ROW OF `/instruments.json?feed=…`, read from the live answer for Dhan.
   *
   * `universes` IS OPTIONAL AND THE OPTIONALITY IS THE POINT. D-0089/D-0090
   * added the full array beside the frozen `universe` string, and
   * `membershipTokens` reads the array when a row carries one and splits the
   * string when it does not. A required field here would erase the very
   * distinction `carriesArray` exists to measure.
   *
   * @typedef {{
   *   symbol: string,
   *   key: string,
   *   kind: string,
   *   exchange: string,
   *   segment: string,
   *   universe: string,
   *   universes?: string[],
   *   bars: number,
   *   href: string
   * }} CatalogueRow
   */

  /**
   * `/ingest/status.json` — the cell in flight, transcribed from the `write!`
   * in `crates/api/src/ingest.rs` that emits it. The live answer was `null`
   * because the autopilot is paused, so the field names come from the Rust and
   * not from a body this page happened to see.
   *
   * @typedef {{ instrument: string, month: string, index: number, of: number }} PilotFlight
   */

  /**
   * ONE FEED'S RUNG OF THE SWEEP LADDER — `waiting_on[]` of the same answer,
   * from the same `write!`. The live array was empty for the same reason.
   *
   * This page reads three of the thirteen — `feed`, `vendor` and `halted` — and
   * all thirteen are named anyway, because a typedef that lists only what today
   * happens to be read is one that has to be edited before the next field can
   * be looked at.
   *
   * @typedef {{
   *   feed: string, vendor: string, month: string, window: string,
   *   behind: number, done: boolean, attempts: number, attempts_max: number,
   *   months_done: number, months_total: number, stalled_months: number,
   *   last_reason: string, halted: string
   * }} PilotFeed
   */

  /**
   * WHAT `foldMonths` HANDS BACK, AND IT IS THAT MODULE'S OWN TYPE.
   *
   * AN ALIAS, NOT A COPY. `$lib/store.svelte.js` declares `WindowFold` beside
   * the function that returns it — the fold's arithmetic plus the two fields
   * that module stamps on — and restating those seven names here would be a
   * second definition of one shape, which is the drift this repository is
   * written against. The local name exists only so the sites below read as
   * prose; the fields are the store's and move when it moves.
   *
   * `at` IS NULLABLE THERE AND STAYS NULLABLE HERE: it is `null` until a read
   * succeeds and `Date.now()` from that moment on. Both folds in this file are
   * taken only after `store.state === 'ready'`, and the one line that needs a
   * number out of it narrows in the open rather than defaulting.
   *
   * @typedef {import('$lib/store.svelte.js').WindowFold} StoreFold
   */

  /**
   * A UNIVERSE ROW — a DISPLAY SET, and `target` is what it can spell on the
   * wire. `null` there is the whole refusal; see `UNIVERSES`.
   *
   * @typedef {{
   *   id: string, label: string, field: string | null, token: string | null,
   *   target: string | null, note?: string
   * }} Universe
   */

  /**
   * A RUNG OF THE CONTROL — three names for one bar length. See `RUNGS` for why
   * `dir`, `label` and `phrase` are three fields and never one.
   *
   * @typedef {{
   *   dir: string, label: string, phrase: string, stored: boolean,
   *   per: number | null
   * }} Rung
   */

  /**
   * A SEGMENT ROW. `served` is a statement about the ROUTE, not about the
   * vendor; `short` is the visible refusal and `null` where there is none.
   *
   * @typedef {{
   *   key: string, label: string, note: string, served: boolean,
   *   short: string | null, why: string
   * }} Segment
   */

  /**
   * THE SEVEN VERDICTS, AS A TYPE. It is the key of `VERDICT`, the member of
   * `VORDER`, the field a census row and a census month each carry, and the
   * value the pill filter holds — one name for all five, so a verdict added to
   * the table and forgotten in the order is a build failure rather than a row
   * that silently sorts last.
   *
   * @typedef {'fail' | 'never' | 'short' | 'retry' | 'unknown' | 'beyond' | 'ok'} Verdict
   */

  /**
   * ONE SERIES IN THE WINDOW — instrument × segment × rung, with its month
   * files resolved. Built by `censusRows`, read by the table, the sort, the
   * pills and the row-level pull.
   *
   * @typedef {{
   *   id: string, key: string, mkey: string, sym: string, kind: string,
   *   seg: string, segLabel: string, tf: string, tfLabel: string,
   *   per: number | null,
   *   months: {
   *     month: string, sessions: number, exp: number | null,
   *     got: number | null, k: Verdict
   *   }[],
   *   got: number, exp: number, unproved: number, missing: number,
   *   state: Verdict
   * }} CensusRow
   */

  /**
   * ONE INSTRUMENT'S OUTCOME FROM ONE RUN. `group` is a `GROUP` label — prose,
   * ranked by `GROUP_ORDER` and never by the alphabet.
   *
   * @typedef {{
   *   key: string, symbol: string, kind: string, gained: number,
   *   before: number, group: string, reason: string, tone: string
   * }} OutcomeRow
   */

  /**
   * THE SERVER'S OWN ANSWER TO A PULL, read out of the returned document.
   *
   * ONE SHAPE FOR BOTH DOORS. `readReceipt` builds it from the parsed receipt,
   * and `$lib/receipt.js`'s `refusal()` builds the identical shape for a
   * document that is not a receipt at all — same fields, `good: false`, and the
   * reason spelled out. A second shape for the refusal is how a caller learns
   * to check which one it got.
   *
   * @typedef {{
   *   ok: boolean, status: number, verdict: string, good: boolean,
   *   scope: string, reason: string, facts: { k: string, v: string }[],
   *   raw: string
   * }} Receipt
   */

  // ---------------------------------------------------------------- constants

  /**
   * THE LADDER'S ORDER, AND THE ORDER ONLY.
   *
   * `pull::vendor::Granularity::ALL` in its declared sequence, which ASCENDS
   * with coarseness. That is not an accident anybody may rely on quietly:
   * `crates/pull/src/vendor.rs` pins it with a `const` block that destructures
   * all eleven variants and asserts every discriminant, so a rung inserted in
   * the middle of the ladder is a BUILD FAILURE there before it is a wrong
   * answer here.
   *
   * IT CARRIES NOTHING BUT NAMES — no label, no store flag, no session count.
   * Those are facts about a rung that can go stale against the Rust, and this
   * list exists for one thing: making "is this rung finer than that one" ONE
   * integer comparison rather than a walk, the same comparison
   * `Granularity::is_finer_than` makes on the discriminant.
   *
   * IT IS ALL ELEVEN WHILE `RUNGS` BELOW IS THREE, AND THE SPLIT IS LOAD
   * BEARING. `/feeds.json` states a feed's floor with any rung name the enum
   * can spell, and a floor this file cannot PLACE is a floor it cannot compare
   * against: `finerThan` answers `false` for a name missing from the map, so a
   * rung BELOW an unplaceable floor would be drawn live and offered. The order
   * is therefore total over every name the wire can send, and what the control
   * OFFERS is a separate decision, made below and for separate reasons.
   */
  const LADDER = [
    'tick',
    '1s',
    '5s',
    '1min',
    '3min',
    '5min',
    '15min',
    '30min',
    '60min',
    '1day',
    '1week'
  ];
  /** O(1) per lookup, built once at module scope. It does not scan. */
  const RUNG_RANK = new Map(LADDER.map((dir, i) => [dir, i]));
  /**
   * Strictly finer — `a` sits nearer the event end of the ladder than `b`.
   *
   * `false` when either name is not on the ladder, which is the only honest
   * answer: an unplaceable rung is not finer and is not coarser, and returning
   * either would be an ordering claim about a name this page cannot place.
   */
  /** @param {string} a @param {string} b */
  function finerThan(a, b) {
    const x = RUNG_RANK.get(a);
    const y = RUNG_RANK.get(b);
    return x !== undefined && y !== undefined && x < y;
  }

  /**
   * WHAT THE CONTROL OFFERS: ONE MINUTE, ONE DAY, AND THE ARCHIVES' SECOND.
   *
   * THE OWNER'S RULE, 14 Aug 2026: *"under timeframes just show one minute and
   * one day alone, and one and only for TrueData or GDFL alone ticks should be
   * displayed along with this. Even dynamic timeframes adding is not needed —
   * because after pulling one day or one min or ticks for TrueData or GDFL,
   * always we will do the internal calculations."*
   *
   * Eleven rows stood here. Three are drawn now, and the other eight were never
   * rungs anybody could pull:
   *
   *   NO FEED IN THIS BUILD DECLARES ONE OF THEM. `Descriptor::granularities`
   *   is Dhan `{1day}`, Groww `{1min, 1day}`, `TrueData` `{1s, 1min}`, GDFL
   *   `{1s}` — and the union of all four is EXACTLY the three rows below. Every
   *   other rung was drawn live and annotated "not fetched by this build", on
   *   every feed, forever. `crates/api`'s own no-JS form has said so all along:
   *   `render::granularity_select` offers a rung only where some feed declares
   *   it AND the store can file it, which is `1min` and `1day`. This control was
   *   the only surface in the product drawing eleven.
   *
   *   AND COARSER IS COMPUTED, NOT BOUGHT. Five minutes is a fold of five
   *   one-minute bars this repository already holds. Asking a vendor for it
   *   spends a request, a quota and a window on arithmetic `crates/pull`
   *   performs for nothing — which is the owner's second sentence, and it is
   *   why "add a timeframe" is not a control that is missing.
   *
   * THREE NAMES FOR ONE RUNG, AND EACH ONE ANSWERS A DIFFERENT QUESTION.
   *
   *   `dir`     THE WIRE, THE PATH AND THE PARSER. `Granularity::dir()` emits
   *             it, `api::ingest::parse_granularity` matches it, and the store
   *             writes a directory of that name. `CLAUDE.md` §3 rule 8 makes it
   *             append-only history: it is never reworded, and the detail column
   *             prints it beside every row so the operator can always see what
   *             his tick actually sends.
   *   `label`   THE WORD ON THE BUTTON, and it is the OPERATOR'S vocabulary.
   *   `phrase`  THE RUNG INSIDE A SENTENCE, and it is the MEASURED vocabulary.
   *
   * THE OWNER'S RULE, 14 Aug 2026: *"anyhow inside the CSV files the downloaded
   * data will be marked as ticks, right — so let us just keep it as ticks.
   * Meanwhile internally let us make it as seconds and minutes."* That is
   * exactly this split, and it is the split `crates/pull` already makes:
   * `Granularity::label` is "prose and may be reworded freely" while
   * `Granularity::dir` "may not". So the second rung is labelled **Ticks**,
   * because that is what the vendor marks the files he bought and what he calls
   * them — and nothing about the request, the path or the store moves an inch:
   * `dir` is still `1s`, the fold is still one-second buckets, and what lands is
   * still minutes.
   *
   * `phrase` EXISTS SO THE BUTTON WORD NEVER GETS READ AS A MEASUREMENT. Four
   * sentences on this page name a rung mid-clause — "it serves ___ and
   * coarser", "the store has no directory for ___", "___ is this feed's finest
   * rung", "one record at ___ is a ___". Dropping the button word into those
   * writes *"one record at Ticks is a conflated snapshot"*, which reads as this
   * page calling a snapshot a tick — the one claim `/feeds.json`'s `finest` was
   * built to stop, and the thing `server.rs`'s own
   * `feeds_json_carries_the_granularity_floor_and_never_calls_a_snapshot_a_tick`
   * asserts on the other side. The button says what he bought; the sentence says
   * what is in it. Both are true, and neither is allowed to answer for the other.
   *
   * WHY `tick` IS NOT ONE OF THE THREE, AND WHY `1s` CARRIES THE WORD INSTEAD.
   * `Granularity::is_requestable` is `false` for `Tick` and true for every other
   * rung, for every feed (D-0118) — so a `tick` row is a row that could never be
   * sent, and drawing one is the control-that-hides-a-failure `CLAUDE.md` §4
   * bans. What the operator BUYS and calls a tick is the archive:
   * `NSE_<seg>_TICK_<date>.zip` and `GFDLNFO_TICK_<date>.zip` are the file names
   * in `pull::vendor`'s own `ArchiveName` consts. Those files were MEASURED at
   * one row per whole second with no sub-second field — 22,426 rows across a
   * 22,500-second session, up to four rows sharing a second with no tiebreaker
   * (`docs/08-vendor-samples.md`). That is the `1s` rung, and it is a conflated
   * snapshot rather than a print stream. So `1s` is the row his word lands on,
   * and the row states what one record at it actually is rather than repeating
   * the file name back at him.
   *
   * "FOR TRUEDATA OR GDFL ALONE" IS NOT SPELLED HERE, AND MUST NOT BE. It falls
   * out of `/feeds.json`: `floorVerdict` marks `1s` permanently refused for any
   * feed whose stated floor is a minute, and `rungRows` drops a permanently
   * refused row (D-0137). Dhan and Groww bottom out at one minute, the two
   * archives bottom out at one second, so the row appears on exactly those two
   * — read from the server on every load, with no vendor named in this file. A
   * fifth feed that reaches a second gets the row the day its descriptor says
   * so; a hardcoded pair would not, and would be the second copy of a vendor
   * fact that D-0126, D-0131 and D-0138 each deleted from this page.
   *
   * `stored` is `Granularity::store_timeframe().is_some()`. It is FALSE on `1s`,
   * and that is the honest state of a tick pull today: the request parses, the
   * feed declares the rung, and `pull::ingest::Plan::timeframe` then refuses it
   * BY NAME at the write boundary because `crates/store` ships no `1s`
   * directory. The row says so before it is ticked. The fold that makes those
   * same archive seconds storable already exists — `pull::fold`, one-second
   * buckets, first/max/min/last in file order — and it runs when these two feeds
   * are asked for `1min`.
   *
   * `per` is BARS PER NSE SESSION, and `null` where this page cannot state one
   * from a verified fact. 09:15 to 15:30 is 375 minutes, so a full minute
   * session is 375 bars and a full day session is 1. `1s` is `null`: a session
   * is 22,500 seconds and the archive held 22,426 rows, so a second is not a
   * bar that either exists or is missing and no yardstick divides it —
   * `CLAUDE.md` §3 rule 6.
   */
  /** @type {Rung[]} */
  const RUNGS = [
    { dir: '1s', label: 'Ticks', phrase: 'one second', stored: false, per: null },
    { dir: '1min', label: '1 minute', phrase: 'one minute', stored: true, per: 375 },
    { dir: '1day', label: '1 day', phrase: 'one day', stored: true, per: 1 }
  ];

  // ───────────────────── HOW FINE EACH FEED CAN EVER ANSWER ─────────────────
  //
  // A SECOND FLOOR, AND IT IS NOT THE FIRST ONE. `pairFloor` further down
  // answers HOW FAR BACK, off the wire's `history` rows. This one answers HOW
  // FINE, and the two refusals share nothing but the word:
  //
  //   below the HISTORY floor    another DAY is refused. The rung is not — the
  //                              vendor serves it, just not that far back
  //   below THIS floor           nothing makes the rung exist. Not a pull, not
  //                              an entitlement, not a purchase, not a code
  //                              change. The vendor has never published it
  //   not fetched by this build  `Descriptor::granularities` does not carry it
  //                              and `crates/api`'s `served()` refuses the POST
  //                              BY NAME before it reads a credential. The
  //                              vendor is willing; a code change fixes it
  //   no store directory         `Granularity::store_timeframe` has nowhere to
  //                              file it, so the WRITE boundary refuses the
  //                              bar. A store-format version fixes it
  //
  // FOUR REFUSALS, AND EXACTLY ONE OF THEM IS PERMANENT. They were rendered
  // identically — `no directory`, on every rung the store cannot file, on a
  // Groww `tick` row no vendor on earth serves, and on a TrueData `1s` row a
  // vendor serves today. One annotation standing for four different amounts of
  // hope is the fallback `CLAUDE.md` §4 bans: the operator reads "nothing saved
  // here yet", ticks the box, and waits for a pull that can never arrive.
  //
  // ═════ THE TABLE THAT STOOD HERE IS GONE, AND THE FACT IS ON THE WIRE ═════
  //
  // Four rows used to sit here transcribing the four `GranularityFloor` consts
  // in `crates/pull/src/vendor.rs`, word for word, because `/feeds.json`
  // emitted no granularity floor and there was nothing to read. TWO COPIES OF
  // ONE VENDOR FACT IS THE DEFECT AND NOT THE WORKAROUND: they can disagree,
  // and the browser's copy is the one that would be wrong — it is versioned
  // with this file, and nothing rebuilds it when a const is reworded.
  //
  // `/feeds.json` now carries `finest` on every feed row:
  //
  //   rung         the finest rung the vendor can EVER serve, spelled with the
  //                same directory name `RUNGS` above is spelled with
  //   kind         `tick` | `snapshot` | `bar` — `pull::vendor::FinestKind`
  //   label        that kind's own sentence, so this file spells none of them
  //   tick_stream  whether this is a real print stream. FALSE on every row
  //   conflated    whether records between two of the feed's own slots were
  //                discarded before the file was written. TRUE for the two
  //                archives, and it is what makes one second of an archive a
  //                different object from one second of anything else
  //   because      why nothing finer exists, IN THE VENDOR'S OWN TERMS
  //   source       where those words were read — §3 rule 1
  //
  // THE TWO BOOLEANS ARE THE POINT OF THE FIELD. A page handed `1s` and
  // nothing else is free to write "tick" beside it, and the two feeds that
  // bottom out at one second serve no prints at all: what sits on that second
  // is a conflated snapshot of the best bid, the best ask and the best last
  // price, and every print between two of them was discarded before the file
  // was written. They are ASKED rather than inferred from the word, so nothing
  // here matches on a string to decide what may be printed next to a number.
  //
  // A FEED WHOSE ROW CARRIES NO `finest` CLAIMS NOTHING, AND THE PAGE SAYS
  // EXACTLY THAT. It does not fall back to a table — there is no table left to
  // fall back to, and that is the whole change: a hardcoded fallback that looks
  // authoritative is the `CLAUDE.md` §4 shape, and the operator cannot tell a
  // transcription from a reading. The honest response to a missing field is
  // that THE RUNNING BINARY PREDATES IT, which names the fix — restart on a
  // newer build — instead of quietly answering out of a copy of what the
  // vendors said the last time this file was edited.
  //
  // O(feeds) ONCE PER FEED-LIST CHANGE, then O(1) per lookup. `floorVerdict`
  // is called once per offered rung per render of the control and must not
  // scan.
  const finestByWire = $derived.by(() => {
    const m = new Map();
    for (const f of feeds.all) {
      const x = f?.finest;
      // EVERY PART OR NOTHING. A half-emitted floor is not a floor: a `rung`
      // with no `kind` would let this page state a number and stay silent
      // about what one record at it is, which is the exact silence the kind
      // exists to break.
      if (f?.wire && x && typeof x.rung === 'string' && typeof x.kind === 'string') m.set(f.wire, x);
    }
    return m;
  });

  /**
   * THE FEED'S FLOOR IN ONE SENTENCE, COMPOSED FROM FACTS THE SERVER SENT.
   *
   * Not a transcription and not prose about a vendor: every noun in it is a
   * field off `/feeds.json` — the feed's own `display`, `kind_label`, which is
   * `pull::vendor::SourceKind`'s own words, and the rung's phrase off the
   * ladder. This file supplies the grammar and nothing else, so a vendor whose
   * floor moves moves this sentence with no edit here.
   *
   * `phrase`, NOT `label`. This is a SENTENCE about what the vendor publishes,
   * and the button word is the operator's — "it serves ticks and coarser" is a
   * claim about the vendor's records that `/feeds.json` contradicts on the very
   * next line. See `RUNGS` for the three-name split and why it exists.
   *
   * `''` when the row carries no floor, and the caller states the absence
   * rather than printing an empty sentence.
   */
  /** @param {string} wire */
  function shortFloor(wire) {
    const floor = finestByWire.get(wire);
    if (!floor) return '';
    const feed = feeds.all.find((f) => f.wire === wire);
    const phrase = RUNGS.find((r) => r.dir === floor.rung)?.phrase ?? floor.rung;
    const kind = feed?.kind_label ? ` — ${feed.kind_label}` : '';
    return `${feed?.display ?? wire}${kind}. It serves ${phrase} and coarser.`;
  }

  /**
   * WHAT THIS FEED'S GRANULARITY FLOOR SAYS ABOUT ONE RUNG.
   *
   * The browser spelling of `pull::vendor::Descriptor::granularity_verdict`:
   *
   *   refused   finer than THIS feed's finest. Permanent.
   *   finest    this rung IS the feed's finest, and the floor says what one
   *             record at it is. The one rung where that is answerable at all.
   *   coarser   above the floor, so the floor has no opinion — and inventing
   *             one would be §3 rule 1. It says NOTHING about whether this
   *             build fetches the rung; that is a different question with its
   *             own answer below, and the two are deliberately not merged.
   *   unstated  THIS SERVER SENT NO FLOOR FOR THIS FEED. Rust has a descriptor
   *             for every feed it can name, so this is not a vendor with
   *             nothing to say — it is a running binary that predates the
   *             field, or a row that carries only half of one. Nothing is
   *             refused on the strength of it and the page says why.
   *
   * One map read and one integer comparison. It does not walk the ladder.
   *
   * A `never` arm stood above `refused`, answering for `tick` without consulting
   * the feed. `RUNGS` no longer offers that rung and nothing else on this page
   * can name one, so the arm was unreachable — and an unreachable branch in the
   * function that decides whether a control is dead is a claim nobody can check.
   * The fact it carried is not lost: `Granularity::is_requestable` still refuses
   * a tick for every feed in `crates/pull`, `/feeds.json` still states it from
   * the other side with `tick_stream: false` on every row, and the `finest` arm
   * below is where an operator now reads it — on the two archive feeds, on the
   * rung their tick files actually hold.
   */
  /** @param {string} wire @param {string} dir */
  function floorVerdict(wire, dir) {
    const floor = finestByWire.get(wire);
    if (!floor) return { state: 'unstated', permanent: false, floor: null };
    if (finerThan(dir, floor.rung)) return { state: 'refused', permanent: true, floor };
    if (dir === floor.rung) return { state: 'finest', permanent: false, floor };
    return { state: 'coarser', permanent: false, floor };
  }

  /** How many rows one page of the census draws. */
  const PAGE_SIZE = 25;

  /**
   * THREE-WAY, ALWAYS. A comparator that answers 1 for "equal" is not a
   * comparator: it makes the sort order depend on the input order, so the same
   * rows in a different sequence rank differently and a stable second key never
   * gets to break the tie it exists for.
   */
  /** @param {unknown} a @param {unknown} b */
  function cmpStr(a, b) {
    const x = String(a ?? '');
    const y = String(b ?? '');
    return x < y ? -1 : x > y ? 1 : 0;
  }
  /** @param {unknown} a @param {unknown} b */
  function cmpNum(a, b) {
    const x = Number(a ?? 0);
    const y = Number(b ?? 0);
    return x < y ? -1 : x > y ? 1 : 0;
  }

  /**
   * THE EIGHT UNIVERSES, IN NSE'S OWN ORDER, AND WHAT EACH CAN PUT ON THE WIRE.
   *
   * A universe is a DISPLAY SET. The request is not: `api::ingest::SpotRequest`
   * carries exactly one field that names a set — `target` — and
   * `api::ingest::SpotTarget` spells exactly three values: `swept`, `indices`,
   * `equities`. So a universe can be ASKED FOR here only where one of those
   * three names the same set.
   *
   * Where none does, the row is DRAWN AND REFUSED, in place, in order, with the
   * reason on it. Dropping it would hide the gap — a list of four reads as if
   * the other four were not universes at all rather than universes this route
   * cannot spell. Inventing a target for it would build a request the server
   * cannot parse, which is the fallback that hides a failure.
   *
   * `token` is the `/instruments.json` universe bit a row is COUNTED from:
   * `row.universe` is a `+`-joined string of `index`, `fno` and `ntm`, and
   * `'*'` is every row rather than a fourth bit. `field` is the token a row
   * would need out of the `universes` ARRAY that `crates/api/src/server.rs`
   * emits (D-0089/D-0090) — a field the running binary may predate, which is
   * measured below and never assumed.
   *
   * `target: null` is the whole refusal: it disables the row and it is why.
   */
  /** @type {Universe[]} */
  const UNIVERSES = [
    // THE FOUR TIERS ARE REQUESTABLE. They carried `target: null` because
    // nothing turned (feed, universe) into a request: `SpotTarget::names`
    // decides membership from a universe bit and never consults the feed, so
    // the API could count a tier globally and not describe one PER FEED. The
    // join now exists — `crates/api/src/constituents.rs`, keyed on
    // (exchange, ISIN), never on symbol — and it resolves 50/50, 100/100,
    // 200/200 and 500/500 for BOTH masters on disk, with nothing lacking,
    // ambiguous or malformed. The slugs are the ones `SpotTarget` has parsed
    // since D-0105. A tier this feed cannot fill still refuses, but now it
    // refuses with a MEASURED count from /universes.json?feed= rather than by
    // being disabled at birth.
    { id: 'n50', label: 'NIFTY 50', field: 'n50', token: null, target: 'n50' },
    { id: 'n100', label: 'NIFTY 100', field: 'n100', token: null, target: 'n100' },
    { id: 'n200', label: 'NIFTY 200', field: 'n200', token: null, target: 'n200' },
    { id: 'n500', label: 'NIFTY 500', field: 'n500', token: null, target: 'n500' },
    { id: 'ntm', label: 'NIFTY Total Market', field: null, token: 'ntm', target: 'equities' },
    // REQUESTABLE SINCE D-0136, and it is the row whose join was built first
    // and reached last. `constituents::Tier::FnoUnderlyings` has always
    // existed, declares `published() == 213`, and had no `SpotTarget` pointing
    // at it — so this row said `no target` while the machinery that answers it
    // sat complete one crate away. `SpotTarget::Fno` now maps to that tier, so
    // the count beside this row is a MEASURED join per feed, not a roster
    // length.
    { id: 'fno', label: 'F&O Underlyings', field: null, token: 'fno', target: 'fno' },
    // NOT AN NSE CONSTITUENT FILE, AND THE LABEL MUST NOT IMPLY ONE.
    //
    // The four NIFTY tiers and the Total Market each come from a published NSE
    // CSV, transcribed with its URL and fetch date in `core::universe`, and are
    // joined to a vendor's master on (exchange, ISIN) by
    // `api::constituents` under a partition that must sum.
    //
    // This row is different in kind. `api::ingest::SpotTarget::Indices`
    // resolves as `universe.contains(Universe::INDEX)` — "whatever the vendor
    // master lists as an index series on this build" — and its `members()`
    // returns None ON PURPOSE, because NSE publishes no file naming that set
    // and hardcoding one would be invention (§3 rule 1) that went stale the day
    // a vendor added a series.
    //
    // The consequence the operator has to know: THIS SET IS PER FEED AND THE
    // FEEDS MAY LEGITIMATELY DIFFER. There is no canonical list to verify
    // against, so "Groww's indices" and "Dhan's indices" are two answers to two
    // questions, not one answer measured twice. `note` says so on the control.
    {
      id: 'index',
      label: 'NSE Indices',
      field: null,
      token: 'index',
      target: 'indices',
      note: 'this feed’s own index series — NSE publishes no list of them, so the set is the vendor’s and two feeds may differ'
    },
    // REQUESTABLE SINCE D-0136, and the ONE row whose slug this repository
    // chose rather than borrowed.
    //
    // `token: '*'` is a BROWSER SENTINEL and has never been a wire word —
    // `namedByUniverse` short-circuits on it and `masterCount` returns every
    // row. So there is no `/instruments.json` token to lift, which is the one
    // place D-0105's "the slugs are the wire's own words" rule has no answer.
    // `all` is invented, and D-0136 records that it is.
    //
    // The set is `catalog::tracked` — the union the pull path already filters
    // by — so the count on this row and the number of instruments a run
    // attempts are the same number by construction, which is the property
    // `SpotTarget::names` exists to hold.
    { id: 'all', label: 'Everything', field: null, token: '*', target: 'all' }
  ];

  /**
   * THE SWEPT PAIR, AND IT IS NOT ONE OF THE EIGHT.
   *
   * `CLAUDE.md` §1 names `NSE-NIFTY` and `NSE-BANKNIFTY` as the entire engine
   * surface. That is not an NSE index family — it is this repository's own
   * sweep surface, so it does not belong in a list of NSE universes and it is
   * not spelled like one.
   *
   * It is also the ONLY set a broker path can serve: `pull::vendor::HttpSpec`
   * carries no request-parameter map, so a target naming a wide set is refused
   * rather than fetching one series under a name nobody asked for (the caution
   * below states it). Dropping it to make room for the eight would leave this
   * form unable to build a legal broker request at all, so it is kept, beside
   * them, spelled as what it is.
   *
   * A concrete object, never `UNIVERSES.at(-1)`: it is the fallback for an
   * unrecognised id, and a fallback that can be `undefined` needs a guard at
   * every reader.
   */
  /** @type {Universe} */
  const SWEPT = {
    id: 'swept',
    label: 'NSE-NIFTY + NSE-BANKNIFTY',
    field: null,
    token: null,
    target: 'swept'
  };
  const UNIVERSE_BY_ID = new Map([...UNIVERSES, SWEPT].map((u) => [u.id, u]));

  /**
   * The three segments the store keys on, and WHICH ROUTE FILLS EACH.
   *
   * A segment is a row, not a branch — the same shape as `RUNGS` and `UNIVERSES`
   * above. `served` is the only thing that decides whether this form can offer
   * it, and it is a statement about the ROUTE rather than about the vendor:
   * `/pull/spot` fills a whole target in one request, `/pull/fno` fills one
   * settled contract. Offering a control that produces a request nothing can
   * send is the fallback that hides a failure.
   *
   * ══════ WHY THIS LIST LOOKED LIKE ONE ITEM, AND WHY IT NO LONGER DOES ══════
   *
   * The two refused rows were FILTERED OUT of the menu and counted in a clause
   * beside it. A one-item dropdown reads as "this axis does not exist"; the
   * operator asked why expired futures and options were missing, which is
   * exactly the question a hidden row cannot answer. Every row is drawn now —
   * dead, in place, carrying its own refusal — the same shape the universe menu
   * has always used, and the shape `$lib/Picker.svelte` documents its
   * `disabled`/`why` fields for.
   *
   * THE REFUSAL IS THREE MEASURED FACTS, not one. All three were read out of
   * this repository's own Rust, not assumed:
   *
   *   1. NO EXPIRED CONTRACT IS IN ANY MASTER. `core::vendor::decode_master_row`
   *      declines every `FUT`, `CE` and `PE` row with `Skip::LiveContract`
   *      ("live derivative contract"), and its comment states the measurement:
   *      both vendors PURGE ON EXPIRY, and the earliest expiry in either master
   *      is three days from now — so every derivative row a master carries is
   *      live by definition, and the master is not where history comes from.
   *      Declined rows never enter `api::merge::Merged::by_key`, so
   *      `/instruments.json` has no FNO row to list and this page's catalogue
   *      cannot show one. The dropdown is empty of them because the DATA is.
   *   2. NOTHING IN THIS BUILD FETCHES ONE. `POST /pull/fno` is registered
   *      (`api::server`) and parses in full, and then answers `503` with
   *      `Outcome::NotStarted` — "expired F&O has no local-archive path and no
   *      HTTP transport in this build". A well-formed request is refused, not
   *      served.
   *   3. THE REQUEST SHAPES DO NOT MEET. `api::ingest::FnoRequest` carries ONE
   *      underlying, ONE series and ONE settled expiry; a segment tick here
   *      names a whole target. There is no route that takes the two together.
   *
   * `short` is the VISIBLE sentence on the row; `why` is the long form on its
   * `title`. Both, because a refusal only a hover reveals is one most readers
   * never meet.
   */
  /** @type {Segment[]} */
  const SEGMENTS = [
    {
      key: 'spot',
      label: 'Spot',
      note: 'continuous series — no expiry',
      served: true,
      short: null,
      why: 'POST /pull/spot — the request this form builds.'
    },
    {
      key: 'futures',
      label: 'Expired futures',
      note: 'contracts that have already settled · discovered per month',
      served: true,
      short: null,
      why: 'POST /pull/fno — expiries first, then the contracts each one held, then their candles. The route walks a MONTH: the window you choose names it, and every settled expiry inside it is pulled. A window running to today is refused, because a contract that has not settled is not history.'
    },
    {
      key: 'options',
      label: 'Expired options',
      note: 'every strike each settled expiry held',
      served: true,
      short: null,
      why: 'POST /pull/fno with series=opt — the same walk as expired futures, and it runs only after them. Dhan reaches ATM±10 strikes on an index and ATM±3 on a stock; Groww serves every listed strike by name. One unreadable contract name is reported and counted, never skipped.'
    }
  ];

  /** `api::ingest::MAX_WINDOW_DAYS`. Refused by the parser before anything runs. */
  const MAX_WINDOW_DAYS = 3653;

  /** The earliest year the server's own picker offers, read from `GET /pull`. */
  const FLOOR_YEAR = 2015;

  /**
   * The month names spelled in full, and they exist to READ a typed month, not
   * to write one.
   *
   * The THREE-LETTER table is not here — it is `MON` in `$lib/dates.js`, which
   * this file imports and the grid renders. A local copy would be a third
   * spelling of the same twelve strings, and the one that drifts is always the
   * copy nobody remembers exists. Nothing below ever renders one of these: they
   * are the accept-set `parseDay` matches an operator's typing against, so
   * `02 September 2024` is a day this page understands and `02 Sepxyz 2024` is
   * not — a three-letter prefix test alone would take the second for the first.
   */
  const MONTHS = [
    'January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December'
  ];

  /** Prefix depth of the outcome search index — the same bound `$lib/index` uses. */
  const MAX_PREFIX = 4;

  // ------------------------------------------------------------- date helpers
  //
  // EVERY DATE IS AN ISO STRING AND EVERY SUM IS IN UTC. A local-time `Date`
  // shifts under DST and this operator's own machine renders `dd/mm/yyyy`,
  // which is the exact ambiguity `crates/api/src/calendar.rs` was written to
  // remove: `01/07/2025` is 1 July here and 7 January in half the world.

  const DAY_MS = 86_400_000;
  const IST_OFFSET_MS = 19_800_000; // +05:30, and India has no daylight saving.

  /** The IST date of a moment, as `YYYY-MM-DD`. */
  function istDay(ms = Date.now()) {
    return new Date(ms + IST_OFFSET_MS).toISOString().slice(0, 10);
  }
  /** @param {number | string} n @param {number} [w] */
  function pad(n, w = 2) {
    return String(n).padStart(w, '0');
  }
  /** @param {number} y @param {number} m @param {number} d */
  function iso(y, m, d) {
    return `${pad(y, 4)}-${pad(m)}-${pad(d)}`;
  }
  /** @param {string} isoDay */
  function parts(isoDay) {
    const [y, m, d] = isoDay.split('-').map(Number);
    return { y, m, d };
  }
  /** Days since the epoch, so two dates can be compared and subtracted. */
  /** @param {string} isoDay */
  function dayNum(isoDay) {
    const { y, m, d } = parts(isoDay);
    return Math.round(Date.UTC(y, m - 1, d) / DAY_MS);
  }
  /** @param {string} isoDay @param {number} n */
  function addDays(isoDay, n) {
    return new Date((dayNum(isoDay) + n) * DAY_MS).toISOString().slice(0, 10);
  }
  /** @param {number} y @param {number} m */
  function daysInMonth(y, m) {
    return new Date(Date.UTC(y, m, 0)).getUTCDate();
  }
  /** @param {string} s */
  function isValidIso(s) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(s)) return false;
    const { y, m, d } = parts(s);
    return m >= 1 && m <= 12 && d >= 1 && d <= daysInMonth(y, m);
  }
  /** Every `YYYY-MM` the window touches, inclusive at both ends. */
  /** @param {string} a @param {string} b @returns {string[]} */
  function monthsBetween(a, b) {
    if (!isValidIso(a) || !isValidIso(b)) return [];
    const from = parts(a);
    const to = parts(b);
    const out = [];
    let y = from.y;
    let m = from.m;
    // Bounded by the window cap: 3,653 days is at most 122 months.
    while (y < to.y || (y === to.y && m <= to.m)) {
      out.push(`${pad(y, 4)}-${pad(m)}`);
      m += 1;
      if (m > 12) {
        m = 1;
        y += 1;
      }
    }
    return out;
  }
  // ------------------------------------------------- the NSE session calendar
  //
  // WHICH DAYS HOLD BARS AT ALL. It answers two questions and only two: where
  // the CEILING is (§ below), and whether a day in the picker is drawn faint
  // because nothing was ever traded on it. IT REFUSES NOTHING — see the floor
  // block below: the only bound that may take a day away from the operator is
  // the FEED's, because that is the one his own choice can move.
  //
  // THE HOLIDAY TABLE IS THE ONE THING HERE THAT CANNOT BE COMPUTED, and no
  // route on this server states it: `crates/api/src/server.rs`'s router carries
  // no calendar endpoint, and `crates/api/src/calendar.rs` renders a day picker
  // that knows about years and month lengths, not about trading days. So the
  // table is checked in HERE, dated, and it MUST MOVE SERVER-SIDE — the store
  // knows which days it holds bars for and the browser is guessing beside it.
  //
  // WHAT HAPPENS WHEN IT RUNS OUT is stated rather than hidden. Past
  // `HOL_THRU` a weekday is treated as a session, which will call a holiday a
  // session — and every sentence built from it says so. The alternative was
  // freezing the ceiling on the table's last day, which is a page whose "newest
  // day" stops moving and never explains why. `CLAUDE.md` §4: degrade loudly
  // and name the reason.

  /** The first day the table below is complete for. Sep 2024 carried no NSE trading holiday. */
  const HOL_FROM = '2024-09-01';
  /** The last day it covers. After this, weekday-only — and every reader is told. */
  const HOL_THRU = '2026-12-31';
  const HOLIDAYS = new Set([
    '2024-10-02', '2024-11-01', '2024-11-15', '2024-12-25',
    '2025-02-26', '2025-03-14', '2025-03-31', '2025-04-10', '2025-04-14', '2025-04-18',
    '2025-05-01', '2025-08-15', '2025-08-27', '2025-10-02', '2025-10-21', '2025-10-22',
    '2025-11-05', '2025-12-25',
    '2026-01-26', '2026-03-04', '2026-03-19', '2026-04-01', '2026-04-03', '2026-05-01',
    '2026-08-15', '2026-10-02', '2026-11-09', '2026-12-25'
  ]);

  /** How many holidays that table names. Counted from it, never written twice. */
  const HOLIDAY_COUNT = HOLIDAYS.size;

  const DAYNAME = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
  /** 0 = Sunday. Built at UTC midnight like every other date here, so no zone moves it. */
  /** @param {string} isoDay */
  function weekdayOf(isoDay) {
    const { y, m, d } = parts(isoDay);
    return new Date(Date.UTC(y, m - 1, d)).getUTCDay();
  }
  /** Whether the NSE traded that day, as far as this page can honestly say. */
  /** @param {string} isoDay */
  function isSession(isoDay) {
    const w = weekdayOf(isoDay);
    if (w === 0 || w === 6) return false;
    return !HOLIDAYS.has(isoDay);
  }
  /** Why a day holds no bars, or `null` when it is a session. */
  /** @param {string} isoDay */
  function noSessionWhy(isoDay) {
    const w = weekdayOf(isoDay);
    if (w === 0 || w === 6) return `${DAYNAME[w]}, no session`;
    if (HOLIDAYS.has(isoDay)) return 'NSE holiday, no session';
    return null;
  }
  /** Whether the holiday table actually covers a day, or is only guessing weekends. */
  /** @param {string} isoDay */
  function holidaysKnownFor(isoDay) {
    return isoDay >= HOL_FROM && isoDay <= HOL_THRU;
  }

  /** NSE equity close, 15:30 IST. The minute a session stops being partial. */
  const CLOSE_MIN = 15 * 60 + 30;
  /** Minutes past IST midnight, from the same instant `istDay` reads. */
  function istMinute(ms = Date.now()) {
    const d = new Date(ms + IST_OFFSET_MS);
    return d.getUTCHours() * 60 + d.getUTCMinutes();
  }

  // ------------------------------------------- how far back each feed answers
  //
  // THE FLOOR IS A PROPERTY OF THE FEED AND OF THE TIMEFRAME, NOT OF THIS PAGE.
  // How far back you may ask depends on WHO you ask and on WHAT you ask them
  // for: Dhan serves daily bars back to a scrip's inception and minute bars for
  // a rolling five years; Groww serves daily "full history" and one-minute bars
  // for the last three months. A floor keyed on the vendor alone is wrong for
  // one of that vendor's own two rungs, so the key is (feed, timeframe).
  //
  // ════════════ THE LAST SECOND COPY IS GONE, AND THIS IS WHAT IT WAS ═══════
  //
  // `GET /feeds.json` emits `wire`, `display`, `kind`, `kind_label`, `verb`,
  // `ready`, `why`, `finest` and `history` (`crates/api/src/server.rs`), and
  // `history` is exactly this fact: one row per rung carrying the binding
  // claim, the contested one, the RESOLVED oldest day and the source of each,
  // read off `pull::vendor::Descriptor`, which has carried a per-rung history
  // floor since D-0110.
  //
  // Until now this file also held its own copy — `FLOOR_OPERATOR` and
  // `FLOOR_DOC`, two hardcoded tables — and its own comment admitted they were
  // a second copy and that wiring them to the emitted fact "is not done here".
  // They are deleted. `pairFloor` below is a lookup into `feeds.all`.
  //
  // THE DRIFT WAS NOT HYPOTHETICAL, WHICH IS WHY THIS IS A FIX AND NOT TIDYING:
  //
  //   * `FLOOR_OPERATOR` carried a `zerodha` row for a feed no descriptor in
  //     this build names, and NEITHER table had a row for `truedata` or `gdfl`,
  //     which the wire answers for.
  //   * D-0131 reversed Groww's one-minute rung from a rolling three months to
  //     the operator's fixed 2020-01-01. The wire moved. These tables did not.
  //     The page was refusing January 2020 through May 2026 on a rung the
  //     server says answers for it.
  //
  // AND THE CLIENT-SIDE CLOCK WENT WITH THEM. A rolling floor was recomputed
  // here with `Date.UTC(y - years, ...)` and compared against an IST day — two
  // clocks in one comparison, off by a day for part of every day on any machine
  // west of IST. `oldest` arrives resolved, so there is nothing left to be
  // wrong: a server-named day is compared against a server-named day.
  //
  // FOUR SHAPES, AND THE WIRE'S OWN WORD FOR EACH:
  //   rolling — N years/months back from today. It moves every single day, so a
  //             window answerable yesterday can be refused today: a fact about
  //             the vendor, not a bug here. The SERVER resolves it.
  //   fixed   — a real calendar date that does not move.
  //   none    — the vendor states there is no floor. NOT the same as unknown.
  //   unknown — nothing states anything, and the page then claims NOTHING.
  /**
   * The oldest day (feed, timeframe) answers for, READ OFF THE WIRE.
   *
   * `/feeds.json` emits `history` as one row per rung, each carrying the
   * binding claim, the contested one, the resolved oldest day and the source of
   * each — `crates/api/src/server.rs`, built from
   * `pull::vendor::Descriptor::history`. This function is now a lookup into
   * that, and it holds no vendor fact of its own.
   *
   * # THE DAY IS THE SERVER'S, AND THAT DELETES A WHOLE CLASS OF BUG
   *
   * `oldest` arrives already resolved. The browser used to recompute a rolling
   * floor itself with `Date.UTC(y - years, ...)` and compare the result against
   * an IST day — two clocks in one comparison, wrong by a day for part of every
   * day on any machine west of IST. There is nothing left here to be wrong: the
   * page compares a day the server named against a day the server named.
   *
   * # `at` VERSUS `known`, AND WHY A CALLER MAY NEVER COLLAPSE THEM
   *
   * `at` is null both when the vendor states it holds everything and when
   * nobody has stated anything. `known` is what tells those apart, and it comes
   * from the wire's own word: `kind: "unknown"` is nobody having said anything,
   * `kind: "none"` is the vendor stating there is no floor. Emitting null for
   * both and letting the reader guess is exactly the fallback §4 bans.
   */
  /**
   * @param {string} wire
   * @param {string} rungDir
   * @returns {{ at: string | null, known: boolean, rolling: boolean, src: string[] }}
   */
  function pairFloor(wire, rungDir) {
    // `FeedRow`, NOT `Feed`, AND THE CAST IS THE WHOLE DIFFERENCE. `history` is
    // on the wire and not in the shared typedef; see `FeedRow` for why it is
    // added here by intersection rather than by widening that module.
    const feed = /** @type {FeedRow | undefined} */ (feeds.all.find((f) => f.wire === wire));
    const row = feed?.history?.find((h) => h.rung === rungDir);
    // NO ROW IS NOT NO FLOOR. A rung the wire says nothing about is `unknown`,
    // and `known: false` is what the caller reads to say so out loud.
    if (!row) return { at: null, known: false, rolling: false, src: [] };

    const src = [];
    if (row.source) {
      src.push(`${row.source}${row.oldest ? ` → ${dayLabel(row.oldest)}` : ' → no floor'}`);
    }
    // THE DISPLACED CLAIM TRAVELS WITH THE BINDING ONE. Deleting it would leave
    // a number no reader could argue with, and this repository has two sources
    // that disagree on three of its four recorded rows.
    if (row.contested?.source) {
      src.push(
        `contested — ${row.contested.source}${
          row.contested.oldest ? ` → ${dayLabel(row.contested.oldest)}` : ' → no floor'
        }`
      );
    }
    if (row.binds_because) src.push(row.binds_because);

    return {
      at: row.oldest ?? null,
      known: row.kind !== 'unknown',
      // A ROLLING FLOOR MOVES EVERY DAY, and the sentence says so. The word is
      // the wire's, not a shape this file infers from the numbers.
      rolling: row.kind === 'rolling',
      src
    };
  }

  // ------------------------------------------------------------ month helpers
  //
  // THE MONTH IS NOT THE WINDOW ANY MORE — IT IS THE STORE'S FILE. `/store.json`
  // answers one row per instrument-MONTH and the store address ends in
  // `/2024-11`, so a day window still has to be able to say which month files it
  // touches and the calendar still pages by month.
  //
  // THE SAME RULE AS THE DAYS ABOVE: `YYYY-MM` is the KEY form. It is what the
  // state holds, what every comparison reads and what sorts correctly as a
  // plain string. `monthLabel` from `$lib/dates.js` is called at the render
  // site and nowhere else — see that file's header for why a reformatted key
  // is an identity change wearing a presentation change's clothes.

  /** Months since year 0, so two months can be compared and subtracted. */
  /** @param {string} ym */
  function monthNum(ym) {
    const [y, m] = ym.split('-').map(Number);
    return y * 12 + (m - 1);
  }
  /** @param {number} num */
  function monthFromNum(num) {
    return `${pad(Math.floor(num / 12), 4)}-${pad((num % 12) + 1)}`;
  }
  /** @param {string} ym @param {number} n */
  function addMonths(ym, n) {
    return monthFromNum(monthNum(ym) + n);
  }

  /**
   * ═══════════ A TYPED DAY -> THE ISO KEY, OR A STATED REFUSAL ═══════════
   *
   * DISPLAY -> WIRE, and one of exactly two crossings between the two spellings
   * of a day on this page (`dayLabel` is the other, and it only goes the other
   * way). Everything that counts, compares, keys a Map or goes on the wire
   * reads `YYYY-MM-DD`; `02 Sep 2024` exists in the two text fields and
   * nowhere else. If a display string ever reached a comparison the page would
   * look perfect and narrow nothing, which is the silent failure this split
   * exists to make impossible.
   *
   * LIBERAL ABOUT SEPARATORS AND CASE, STRICT ABOUT THE MONTH: it has to be a
   * NAME. `02/09/2024` is refused deliberately and by name — it reads as 2
   * September to one reader and 9 February to another, which is the exact
   * ambiguity `crates/api/src/calendar.rs` was written to remove, and a parser
   * that picked one of them would be inventing an intent this page cannot know.
   * Both readings are quoted back when both exist, because the refusal is an
   * explanation of why the page will not choose, not a scolding.
   *
   * The ISO wire form is accepted too: it is the value the field's own state
   * carries, and pasting it back must not be an error.
   *
   * `{ iso }` on success, `{ iso: '' }` for a field cleared on purpose — which
   * is "not set", not a failure — and `{ err }` otherwise. Never a guess.
   */
  /** @param {string} raw @returns {{ iso?: string, err?: string }} */
  function parseDay(raw) {
    const s = String(raw ?? '').trim().replace(/\s+/g, ' ');
    if (!s) return { iso: '' };
    const wire = s.match(/^(\d{4})-(\d{1,2})-(\d{1,2})$/);
    if (wire) {
      const [y, m, d] = [Number(wire[1]), Number(wire[2]), Number(wire[3])];
      const built = m >= 1 && m <= 12 && d >= 1 && d <= daysInMonth(y, m) ? iso(y, m, d) : null;
      return built ? { iso: built } : { err: `${s} is not a date — that month is shorter than that.` };
    }
    const t = s.split(/[\s.,/-]+/).filter(Boolean);
    if (t.length !== 3) return { err: `“${s}” is not a date. Write it as 02 Sep 2024.` };
    const ai = t.findIndex((x) => /^[A-Za-z]{3,9}$/.test(x));
    if (ai < 0) {
      const a = Number(t[0]);
      const b = Number(t[1]);
      const y = t[2];
      const four = /^\d{4}$/.test(y);
      const reads = [];
      if (four && a >= 1 && a <= 31 && b >= 1 && b <= 12) reads.push(`${pad(a)} ${MON[b - 1]} ${y}`);
      if (four && b >= 1 && b <= 31 && a >= 1 && a <= 12) reads.push(`${pad(b)} ${MON[a - 1]} ${y}`);
      return {
        err:
          reads.length === 2
            ? `${s} is ambiguous — ${reads[0]} or ${reads[1]}? Write the month as a name.`
            : 'Write the month as a name — 02 Sep 2024 — or the wire form, 2024-09-02.'
      };
    }
    // The FULL names are the accept-set, never a three-letter prefix alone: a
    // prefix test takes `Sepxyz` for September, which is a month the operator
    // did not type. `Sept` is accepted on the way IN and never emitted on the
    // way out — `$lib/dates.js` fixes the abbreviation at three letters.
    const want = t[ai].toUpperCase();
    const mo = MONTHS.findIndex((full) => full.toUpperCase().startsWith(want));
    if (mo < 0) return { err: `“${t[ai]}” is not a month. Use Jan … Dec.` };
    const rest = t.filter((_, i) => i !== ai);
    const yi = rest.findIndex((x) => /^\d{4}$/.test(x));
    if (yi < 0) return { err: `Write the year in full: 02 ${MON[mo]} 2024.` };
    const dt = rest[1 - yi];
    if (!/^\d{1,2}$/.test(dt)) return { err: `“${dt}” is not a day of the month.` };
    const y = Number(rest[yi]);
    const d = Number(dt);
    return d >= 1 && d <= daysInMonth(y, mo + 1)
      ? { iso: iso(y, mo + 1, d) }
      : {
          err: `${dt} ${MON[mo]} ${y} is not a date — ${MON[mo]} ${y} has ${daysInMonth(y, mo + 1)} days.`
        };
  }

  /** The last day of a month file — the calendar's `End` key, and nothing else. */
  /** @param {string} ym */
  function monthLastDay(ym) {
    const [y, m] = ym.split('-').map(Number);
    return iso(y, m, daysInMonth(y, m));
  }
  /** @param {number} ms */
  function clockOf(ms) {
    const s = Math.max(0, Math.round(ms / 1000));
    const h = Math.floor(s / 3600);
    const rest = s % 3600;
    const body = `${pad(Math.floor(rest / 60))}:${pad(rest % 60)}`;
    return h > 0 ? `${h}:${body}` : body;
  }
  /**
   * A COUNT ON THE WAY TO A HUMAN, GROUPED THE INDIAN WAY: 8,78,28,617.
   *
   * THE LOCALE IS NAMED, NEVER INHERITED. `toLocaleString()` with no argument
   * takes the host's, so one count read `8,78,28,617` on this machine,
   * `87,828,617` on a US one and `87.828.617` on a German one — three spellings
   * of a single number, which makes a screenshot unreproducible and a bug
   * report unfalsifiable. `en-IN` is what the audit and autopilot pages already
   * pin, and it is the grouping this operator's own readers count in.
   *
   * It changes the SEPARATORS and nothing else: the default fraction bound is
   * three digits either way, and `NaN` and `∞` still render as themselves.
   *
   * Built ONCE, at module scope. `n` is called per row of an outcome list built
   * for 750 instruments today, and a formatter constructed per call would load
   * locale data on every one of them.
   */
  // THE FIFTH AND LAST SPELLING OF `en-IN`, AND IT LEAKED "NaN".
  //
  // `IN.format(Number(x ?? 0))` renders the literal string "NaN" for anything
  // that is not a number and not nullish -- a string, an object, an already-NaN
  // -- at all 189 call sites on this page. `/db` shipped the same leak and it
  // was closed in 2ef43bb; this is the third page to carry it.
  //
  // `exact` is the shared formatter: one `en-IN`, and an em dash for every
  // shape of no-number. The `?? 0` is KEPT rather than removed, deliberately --
  // turning nullish into zero is this page's existing intent at 189 sites and
  // changing it is a display decision, not a bug fix. It is worth revisiting
  // separately: CLAUDE.md §7's "zero means zero" argues an absent count should
  // read as the dash rather than as a counted nothing.
  const n = (/** @type {unknown} */ x) => exact(Number(x ?? 0));

  // ------------------------------------------------------------------- feed

  const active = $derived(feeds.all.find((f) => f.wire === feeds.active));

  // -------------------------------------------------------------- source kind
  //
  // THE OPERATOR'S RULE, 12 AUG 2026, AS A FIELD ON THE ROW — never a list of
  // vendor names in this file:
  //
  //   "for truedata and gdfl alone, one and only, we will pull the data
  //    entirely from csv files from the precise folder … except these two alone
  //    only, for all other vendors or brokers feeds it should be always REST."
  //
  // `pull::vendor::SourceKind` is that rule as a type, and `/feeds.json`
  // carries it per row as `kind`, `kind_label` and `verb`. All three are
  // `const fn` on the Rust side, so none costs the server a filesystem call and
  // all three can sit on a per-render route.
  //
  // FIVE THINGS TURN ON IT, and each is a real difference rather than a label:
  //
  //   | | REST | folder |
  //   |---|---|---|
  //   | credential | required | none, and §8 must not run |
  //   | quota | a budget | none — no request is made |
  //   | reach | a history floor | THE FILES PRESENT |
  //   | finest rung | one minute | one SECOND |
  //   | the verb | pull | READ |
  //
  // ═════ `transport` IS GONE, AND THE GUESS THAT READ IT WENT WITH IT ═════
  //
  // `/feeds.json` used to emit the SAME two-valued split twice: `transport`
  // (`broker` / `archive`), matched off `Transport`'s arms in the handler, and
  // `kind` (`rest` / `folder`), read from `SourceKind`. This file then wrote
  // `active?.kind ?? (active?.transport === 'archive' ? 'folder' : 'rest')` —
  // a hardcoded reconstruction of the fact, standing behind the fact, looking
  // exactly as authoritative as the real thing. It is the §4 shape and it is
  // the same one the granularity table above was: nothing on screen said which
  // of the two answered, so a server that stopped sending `kind` would have
  // gone on rendering a confident page built from a guess.
  //
  // ONE FIELD, NO FALLBACK. `null` when this server states nothing, and
  // `sourceKindUnstated` below turns that into a sentence rather than into a
  // default. Both flags are false in that state ON PURPOSE — the page must not
  // draw the broker form OR the folder form for a feed whose kind it does not
  // know, and `!isBroker` is no longer a spelling of "folder".
  const sourceKind = $derived(
    active?.kind === 'rest' || active?.kind === 'folder' ? active.kind : null
  );
  const isFolderFeed = $derived(sourceKind === 'folder');
  const isBroker = $derived(sourceKind === 'rest');
  /**
   * THE SERVER SENT A FEED AND DID NOT SAY WHAT KIND OF SOURCE IT IS.
   *
   * Not a shrug and not a default: `crates/pull` has a transport for every feed
   * it can name, so this is a running binary that predates the field. It is
   * stated where the form would otherwise have been drawn, because the form's
   * SHAPE is what the answer decides — a credential and a rate budget, or a
   * path on this disk.
   */
  const sourceKindUnstated = $derived(Boolean(active) && sourceKind === null);
  /**
   * The sentence that stands where the guess used to. Written once, because it
   * is said in three places and three wordings of one refusal is how a reader
   * learns to skip all of them.
   */
  const SOURCE_KIND_UNSTATED =
    'This server did not say whether this feed is a REST API or a folder of files. That is a MISSING FIELD and not a vendor with nothing to say: /feeds.json emits kind, kind_label and verb on every row from pull::vendor::SourceKind, which crates/pull derives from the transport every feed declares. This page holds no table to answer from — a guess reconstructed from another field would look exactly as authoritative as the fact, and the two decide different forms: a credential and a rate budget, or a path on this disk. Restart the server on a build that emits it.';
  /**
   * ARCHIVE FEEDS STAY OUT OF SIGHT UNTIL THEIR DATA IS BOUGHT, and the gate is
   * the server's own `ready` flag rather than a list of vendor names here. The
   * day TrueData has a store prefix its row flips and the folder form appears
   * with no edit to this file — the same reason the feed picker is built from
   * `/feeds.json`.
   */
  const archiveHidden = $derived(isFolderFeed && active?.ready === false);
  /**
   * THE HONEST VERB, and it is the server's word rather than this file's.
   *
   * A folder is not pulled, fetched or requested: nothing is asked of anybody,
   * no quota moves, nothing can rate-limit it and there is no remote party to
   * be unavailable — the bytes are already on the disk. Calling it a pull put
   * every failure in the wrong diagnostic frame, and the first questions asked
   * were always about tokens, entitlements and outages when the answer was a
   * path.
   *
   * AND THE FALLBACK IS NEUTRAL, not `pull`. A server that states no verb is
   * one this page cannot ask the question of, and defaulting to `pull` would
   * print the network word over a folder read — the exact wrong diagnostic
   * frame this field exists to stop. `ingest` claims neither side.
   */
  const verb = $derived(active?.verb ?? 'ingest');
  /** The verb with its first letter raised, for the start of a sentence. */
  const Verb = $derived(verb.charAt(0).toUpperCase() + verb.slice(1));

  // ------------------------------------------------ how far a folder reaches
  //
  // READ OFF THE DISK, AND THERE IS NOTHING ELSE IT COULD BE.
  //
  // A REST vendor publishes how far back it answers and `/feeds.json` carries
  // that claim with its source. NOBODY PUBLISHES ANYTHING about a directory on
  // this machine, so the reach of a folder feed is whatever files are in it —
  // which means it has to be READ, and reading it means walking the folder.
  //
  // That walk is why this is its own route. `/feeds.json` renders on every page
  // load and an O(files) probe there would be the cost `/store` exists to
  // avoid, so `GET /folder.json?feed=<wire>` answers it on demand and this page
  // asks once, when a folder feed is actually chosen. See `crates/api/src/folder.rs`.
  //
  // THREE ANSWERS AND A HALT, AND THE PAGE MUST NOT COLLAPSE THEM:
  //   empty — the folder is there and holds nothing. AN ANSWER. The operator
  //           has not bought that month yet.
  //   blank — files were delivered and they carry no rows. Different: they were
  //           bought, and they are blank.
  //   days  — the real span, both ends inclusive.
  //   halt  — missing, unreadable, or holding something that will not decode.
  //           NOT an empty reach. It names the PATH, because the path is the
  //           one thing an operator needs next and the one thing a run that
  //           produced no bars never used to say. `CLAUDE.md` §4 bans the
  //           version of this that reads as "no data yet".
  /**
   * @type {{
   *   wire: string | null,
   *   state: 'idle' | 'reading' | 'read' | 'halted',
   *   body: FolderBody | null,
   *   why: string | null,
   *   segment?: string | null
   * }}
   */
  let folderReach = $state({ wire: null, state: 'idle', body: null, why: null });

  $effect(() => {
    const wire = feeds.active;
    if (!wire || !isFolderFeed) return;
    // `untrack` AROUND THE READ OF `reach`, for the reason the rung effect
    // below gives: this effect WRITES `reach`, so reading it as a dependency
    // would make every one of its own writes schedule another run. The only
    // thing that may trigger this is a FEED CHANGE.
    if (untrack(() => folderReach.wire === wire && folderReach.state !== 'idle')) return;
    let live = true;
    folderReach = { wire, state: 'reading', body: null, why: null };
    // THE SEGMENT IS DISCOVERED, NOT ASSUMED — AND ASSUMING IT WAS A BUG.
    //
    // `/folder.json` reads a folder with ONE column layout and refuses to pick
    // when a feed declares more than one: "a reach read with the wrong shape is
    // a precise and wrong answer". TrueData had a single layout until its plain
    // F&O row was measured; it now has two, so the unnamed ask became
    // ambiguous and started refusing.
    //
    // The first fix here hardcoded `&segment=INDEX`, and that was WRONG in the
    // direction that matters. GDFL has measured exactly ONE layout and it is
    // FNO — this vendor publishes no index archive at all — so a page that
    // always names INDEX turned GDFL's real refusal, which is about the shape
    // of a file on disk, into "no column layout was ever measured for Global
    // Datafeeds INDEX". A true sentence about a question nobody asked, standing
    // where the actual fault used to be printed. That is the §4 shape arriving
    // by the back door: the operator is sent to fix a layout when the finding
    // is a file with nine fields where ten were declared.
    //
    // SO THE SEGMENT IS FOUND BY ASKING. The unnamed ask is tried first, which
    // is right for every feed with one layout and is exactly what this page did
    // before. ONLY the ambiguity refusal — the server's own words, matched on
    // the sentence it emits for that case — is retried, once per segment this
    // build can name, and each answer is kept beside its segment. Every other
    // refusal is kept whole and shown, because it is the finding.
    //
    // Nothing here decides which segments a vendor has. It offers the two the
    // route can parse and keeps whichever answer; a feed that has neither says
    // so in the server's words, and a third segment is a row on this list.
    readFolder(wire)
      .then(({ ok, data, segment }) => {
        if (!live) return;
        // A HALT IS KEPT AS A HALT. `ok` is false for every refusal the server
        // makes, and the body carries `path` on the ones that have one — so
        // the page can name the folder rather than saying "nothing found".
        folderReach = ok
          ? { wire, state: 'read', body: data, why: null, segment }
          : { wire, state: 'halted', body: data, why: data?.refused ?? 'refused', segment };
      })
      .catch((why) => {
        if (!live) return;
        // LOUD, NOT SILENT. A reach that quietly fails to load must not render
        // as an empty folder — that is exactly the confusion this route exists
        // to end, arriving by a different door.
        folderReach = { wire, state: 'halted', body: null, why: String(why), segment: null };
      });
    return () => {
      live = false;
    };
  });

  /**
   * The sentence `/folder.json` refuses an AMBIGUOUS segment with, and no other.
   *
   * `crates/api/src/folder.rs` emits it verbatim when a feed declares more than
   * one measured layout and the caller named none. Matched on the two words
   * that are structural rather than decorative — a reworded refusal stops
   * matching and this page falls back to showing it whole, which is the safe
   * direction: an unretried ambiguity is a visible halt, and a retry triggered
   * by the WRONG refusal would ask a segment question of a folder whose fault
   * is a file.
   */
  const AMBIGUOUS = /measured column layouts/i;

  /**
   * THE SEGMENTS THIS BUILD CAN NAME, in the order they are tried.
   *
   * `brutex_core::instrument::Segment` parses these three and `/folder.json`
   * refuses anything else with all three listed. This page holds the words and
   * not the knowledge of which vendor has which — that is a fact about a
   * descriptor, and asking is how it is found out.
   */
  const FOLDER_SEGMENTS = ['INDEX', 'FNO', 'CASH'];

  /**
   * Read the folder, discovering the segment only when the server says one is
   * needed.
   *
   * ONE REQUEST for every feed that declares one layout — which is what this
   * page did before it started guessing, and is why GDFL is answered by its own
   * refusal again rather than by a sentence about an index archive it has never
   * published. At most one more per segment for a feed that declares several,
   * and only ever after the server has said in its own words that a segment is
   * required.
   *
   * The FIRST segment that answers wins and its name is carried out, so the
   * census can say which archive it counted instead of implying it counted the
   * folder whole. When none answers, the AMBIGUITY refusal is what is returned
   * — never the last segment's refusal, which would name whichever this list
   * happened to end on and read as a finding about that segment.
   */
  /** @param {string} wire */
  async function readFolder(wire) {
    /** @param {string | null} segment */
    const ask = async (segment) => {
      const at = segment ? `&segment=${segment}` : '';
      const r = await request(`/folder.json?feed=${encodeURIComponent(wire)}${at}`);
      return { ok: r.ok, data: await r.json(), segment };
    };
    const first = await ask(null);
    if (first.ok || !AMBIGUOUS.test(String(first.data?.refused ?? ''))) return first;
    for (const segment of FOLDER_SEGMENTS) {
      const next = await ask(segment);
      if (next.ok) return next;
    }
    return first;
  }

  /**
   * THE FOLDER'S ANSWERABLE RANGE, IN WORDS, and every one of them is read.
   *
   * `null` for a REST feed — it has a history floor instead, and the two must
   * never be rendered by the same sentence.
   */
  const folderReachSentence = $derived.by(() => {
    if (!isFolderFeed || !active) return null;
    const path = folderReach.body?.path;
    const at = path ? ` · ${path}` : '';
    if (folderReach.state === 'reading') return `Reading ${active.display}'s folder…`;
    if (folderReach.state === 'halted') {
      return `${active.display} · the folder could not be read${at} — ${folderReach.why}`;
    }
    const r = folderReach.body?.reach;
    if (!r) return null;
    if (r.state === 'days') {
      return `${active.display} answers for ${dayLabel(r.earliest)} – ${dayLabel(r.latest)} — read from the ${n(r.files)} file(s) present, ${n(r.rows)} row(s)${at}`;
    }
    if (r.state === 'blank') {
      return `${active.display} · ${n(r.files)} file(s) are there and not one carries a row${at}`;
    }
    return `${active.display} · the folder is there and holds nothing${at}`;
  });

  /**
   * THE ARCHIVE CENSUS — the folder described as a set, not as a sentence.
   *
   * # Why this exists beside `folderReachSentence` rather than instead of it
   *
   * `/folder.json` answers six facts — `path`, `state`, `earliest`, `latest`,
   * `files`, `rows` — and every one of them was being spent on ONE line at the
   * foot of a calendar popover. That line is the right thing in the place it
   * sits; it is the wrong thing to be the only place the six exist, because
   * they are the closest thing an archive feed HAS to a universe.
   *
   * # An archive has no universe to pick, and this is what it has instead
   *
   * A broker is asked for a SET and answers it: `SpotTarget` names one, the
   * master resolves it, and `/universes.json?feed=` counts what the feed
   * reaches. An archive is asked for nothing. Its instruments are the FILES the
   * operator bought, so the only true answer to "what is in this feed" is a
   * walk — which is what this is. `crates/api/src/constituents.rs` states the
   * other half from the join's side: a vendor with no master `lacks` every
   * name, which is a fact about a master that does not exist rather than about
   * a folder that does.
   *
   * # `null` FOR A REST FEED, ALWAYS
   *
   * Not an empty census — `null`. A broker has a history floor and a master,
   * and rendering "0 files" for one would be this page inventing a folder for a
   * vendor that has none. The two must never share a renderer.
   *
   * # WHAT IS NOT HERE, AND IT IS NAMED RATHER THAN OMITTED
   *
   * THE INSTRUMENT LIST. `pull::archive::Member` already carries `instrument`,
   * taken from the file name with its extensions removed, and
   * `pull::folder::reach_of` counts the members and discards their names. So
   * the walk HAS them and the wire does not carry them. That is a server field
   * that does not exist yet, not a fact this page may reconstruct: the file
   * names are the only identity an archive has, and a browser guessing them
   * from a count would be inventing the one thing it cannot check.
   *
   * O(1) — six field reads off a body fetched once per feed change.
   */
  const archiveCensus = $derived.by(() => {
    if (!isFolderFeed || !active) return null;
    // THE TWO SHORT SHAPES ARE TYPED AS LITERALS, AND THAT IS WHAT MAKES THE
    // MARKUP'S GUARD REAL RATHER THAN DECORATIVE.
    //
    // This derivation returns FOUR different things: `null`, a reading marker, a
    // halt, and the full census. Only the last carries `instruments`,
    // `collisions` and `rejected`. Without a literal on `state` the checker
    // infers `string` here, the union stops being discriminated, and
    // `{#if c.state === 'reading'}` narrows nothing — so a later read of
    // `c.instruments` type-checks against a shape that does not have it.
    //
    // It did not merely type-check. Measured on the running binary: selecting an
    // archive feed threw `TypeError: Cannot read properties of undefined
    // (reading 'length')` from the instruments panel, because `c.instruments` is
    // `undefined` in these two shapes, `undefined === null` is false, and the
    // next branch reached straight for `.length`. Nine `svelte-check` errors
    // were pointing at exactly that line and were read as noise.
    if (folderReach.state === 'reading') {
      return /** @type {{ state: 'reading' }} */ ({ state: 'reading' });
    }
    if (folderReach.state === 'halted') {
      return /** @type {{ state: 'halted', path: string | null, why: string }} */ ({
        state: 'halted',
        path: folderReach.body?.path ?? null,
        why: folderReach.why
      });
    }
    const r = folderReach.body?.reach;
    if (!r || typeof r.state !== 'string') return null;
    // THE NAMES, AND `null` RATHER THAN `[]` WHEN THE SERVER SENT NONE.
    //
    // An empty array is a folder that names nothing — an ANSWER, and the one
    // `Reach::Empty` exists for. A missing key is a RUNNING BINARY THAT
    // PREDATES THE FIELD, which names a different fix: restart on a newer
    // build. Collapsing the two would print "0 instruments" at a server that
    // was never asked, which is the §4 shape and the same discipline
    // `rungServed` and `finestByWire` already read their fields under.
    const named = folderReach.body?.instruments;
    return {
      state: r.state,
      path: folderReach.body?.path ?? null,
      earliest: r.earliest ?? null,
      latest: r.latest ?? null,
      files: Number(r.files ?? 0),
      rows: Number(r.rows ?? 0),
      instruments: Array.isArray(named) ? named : null,
      collisions: Number(folderReach.body?.collisions ?? 0),
      // FILES OF ANOTHER PRODUCT, which the walk now reports instead of
      // halting on. `[]` is a clean folder; a server that predates the field
      // sends nothing and this reads `[]` too — and that is the one place the
      // two may be merged, because an absent list and an empty one both mean
      // "nothing to show here" for a FINDING. A missing INSTRUMENT list is
      // different and stays `null`: there, absent means the page cannot answer
      // and empty means the folder names nothing.
      rejected: Array.isArray(folderReach.body?.rejected) ? folderReach.body.rejected : [],
      // WHICH ARCHIVE THIS COUNTED. `null` for a feed that answered without a
      // segment being named — one measured layout, so the folder IS the answer
      // — and the segment's own word when one had to be discovered.
      segment: folderReach.segment ?? null
    };
  });

  // ---------------------------------------------------------------- the form

  /**
   * EVERY ONE OF THESE IS INTENT AND SURVIVES A FEED CHANGE.
   *
   * Nothing below resets them when the feed in the first control moves, and that is
   * deliberate: unticking a feed and re-ticking it must RESTORE the choice, not
   * destroy it. What a feed change does move is what those choices can REACH —
   * counted separately, further down, and never read off these.
   */
  /**
   * THE UNIVERSE IS THE CHOICE; THE TARGET IS WHAT IT SPELLS ON THE WIRE.
   *
   * Two names for one decision would be two things to keep in step, and the
   * one that drifts is always the copy nobody remembers exists. `universe` is
   * the only state; `target` is derived from it and is never assignable, so a
   * request cannot carry a target the rung above it did not choose.
   *
   * The default is the swept pair because it is the one set every transport
   * can serve — a broker included. A default further down the list would leave
   * the form's first legal state illegal on the feed it opens with.
   */
  // THE DEFAULT IS A SET THIS MENU ACTUALLY OFFERS.
  //
  // It was `'swept'`, and 1c06dab removed the sweep pair from the menu at the
  // operator's instruction — so the control kept DISPLAYING
  // "NSE-NIFTY + NSE-BANKNIFTY" as its value while the list no longer contained
  // it. Removing an option without moving the state off it leaves exactly that:
  // a button showing a choice nobody can make.
  //
  // `n50` is the first row of the menu and it resolves to a real target
  // (`target=n50`), so the page opens on something a request can name.
  let universe = $state('n50');
  /**
   * Is the universe drawer open? Sets no request can name are folded behind one
   * line rather than listed dead above the ones that work — see the drawer in
   * the markup. Closed by default; nothing is deleted and every refusal is one
   * click away.
   */
  let uniTuck = $state(false);
  /**
   * The feeds this build cannot pull, and whether their drawer must be open.
   *
   * Same trap as the universe drawer: `feeds.active` can BE one of these — the
   * default picks the feed holding the most bars, but an operator can select a
   * not-ready feed and the page has blank states for exactly that. A closed
   * drawer holding the current selection would draw a menu with no tick, so the
   * open state is derived and forced whenever the selection is inside.
   */
  const notReadyFeeds = $derived(feeds.all.filter((f) => f.ready !== true));
  let feedTuck = $state(false);
  // Same promotion rule as the universe drawer, for the same reason: forcing it
  // open whenever the selection was inside meant it was open whenever an
  // operator had selected a not-ready feed, which is exactly when the menu most
  // needs to be short. The selected feed is rendered above the drawer instead.
  const feedOpen = $derived(feedTuck);
  const tuckedFeeds = $derived(notReadyFeeds.filter((f) => f.wire !== feeds.active));
  const activeNotReady = $derived(notReadyFeeds.find((f) => f.wire === feeds.active) ?? null);
  // THE SELECTED ROW IS PROMOTED OUT OF THE DRAWER, NOT AUTO-OPENED INTO VIEW.
  //
  // The first draft forced the drawer open whenever the selection lived inside
  // it, so that a menu could never draw without a visible tick. That reasoning
  // was right and the result was useless: `universe` DEFAULTS to `'swept'`, so
  // the selection was always inside, so the drawer was always open, so the menu
  // looked exactly as it had before and the operator correctly said nothing had
  // changed.
  //
  // Promoting instead satisfies both: the chosen row is always visible with its
  // tick, and the drawer holds only what is NOT chosen. Pick a NIFTY family and
  // the sweep pair folds away; pick the sweep pair and it rides up beside them.
  const sweptIsChosen = $derived(universe === SWEPT.id);
  // THE THREE THAT READ `refusedUniverses` NOW SIT BELOW IT. They stood here,
  // above its declaration, and worked only because a `$derived` body does not
  // run until something reads it — the drawer's own markup, long after the
  // module has finished evaluating. That is true and it is an argument nobody
  // reading this line can check, and the checker calls it what it is: a
  // block-scoped binding used before its declaration. Moving the readers below
  // the thing they read costs nothing and makes the order visible.
  let segSet = $state(new Set(['spot']));
  /**
   * THE TIMEFRAME IS A SET, NOT A VALUE.
   *
   * It was one rung and one only, which made the ask arithmetic multiply by a
   * constant 1 and made the census able to describe a single row of the store.
   * An operator who wants the minute bars AND the daily bars for a window was
   * asking twice and comparing by memory.
   *
   * WHAT THE WIRE STILL CARRIES IS ONE RUNG. `api::ingest::SpotRequest` has one
   * `granularity` field, so N ticked rungs are N requests, sent in the ladder's
   * own order, and the wire fold prints every one of them. That is a fact about
   * the route and it is stated rather than hidden behind a control that takes
   * more ticks than it can send.
   */
  let rungSet = $state(new Set(['1min']));

  /**
   * WHICH FEEDS THIS RUN ASKS — a set, because the operator asked for several.
   *
   * # Why this is not `feeds.active`
   *
   * `feeds.active` is the page's SCOPE: every count, every census and every
   * reach number on this page is measured for one vendor, and /db and
   * `+layout.svelte` read the same value. Widening it to a set would change
   * what those pages mean. This is the ASK, which is a different question, and
   * keeping them apart is what lets the strip go on counting one feed while the
   * run fans out over several.
   *
   * EMPTY MEANS "the scope feed", so the control needs no seeding and cannot
   * drift out of step with the page when the operator changes scope.
   */
  let pullFeeds = $state(new Set());
  const feedsChosen = $derived(
    pullFeeds.size > 0 ? [...pullFeeds] : feeds.active ? [feeds.active] : []
  );
  /**
   * WHETHER THE OPERATOR HAS TOUCHED THE TIMEFRAME HIMSELF.
   *
   * `false` until he does, and it is what separates HIS choice from THE
   * DEFAULT — two things this control had no way to tell apart, and the
   * difference is the whole of the bug below.
   *
   * # What went wrong, measured
   *
   * The default is `1min`. Dhan's descriptor declares `Day1` and nothing else,
   * because its `bars_path` is pinned to the DAILY endpoint. So opening the
   * page on Dhan and pressing Start sent a minute request that
   * `crates/api`'s `served()` refused by name — and the audit journal caught
   * exactly that at 21:11 on 14 Aug 2026:
   *
   * > `360ONE: Dhan does not serve 1min bars. Nothing was sent.`
   *
   * The refusal is correct and it is the gate working. What was wrong is that
   * the form's OWN default guaranteed it: a first-time operator on this feed
   * could not press the only button on the page without being refused, and
   * nothing he did caused it.
   *
   * # Why this is not the drop effect below, and must not be folded into it
   *
   * That effect drops a rung the vendor can NEVER serve, and it deliberately
   * keeps one this build merely does not fetch — "those refusals are somebody's
   * work item, and taking the tick away would be this page deciding the work
   * will never be done". That reasoning is about a tick the OPERATOR made.
   *
   * A default is not a decision he made. Moving it costs him nothing and tells
   * him nothing false. Moving HIS tick would do both. So the two are separated
   * by this flag rather than by making the drop rule cleverer.
   */
  let rungTouched = $state(false);
  /** The ticked rungs, in the ladder's order. Never the tick order. */
  const rungsChosen = $derived(RUNGS.filter((r) => rungSet.has(r.dir)));
  /**
   * THE FIRST TICKED RUNG, for the things that can only be about one: the
   * header chip and the feed floor's own sentence. Every COUNT reads
   * `rungsChosen`, never this.
   */
  const rung = $derived(rungsChosen[0]?.dir ?? '');

  /**
   * WHETHER THIS BUILD FETCHES A RUNG AT ALL — READ FROM THE SERVER, NOT
   * TRANSCRIBED.
   *
   * `GET /feeds.json` emits a `history` array per feed with one row per rung
   * the feed either serves or has a history claim recorded for, and `served`
   * on that row is `Descriptor::granularities` — the SAME bit
   * `crates/api`'s `served()` gates the POST on. That gate refuses an
   * undeclared rung by name, before it reads a credential, because this feed's
   * bars path is pinned to one rung and a request for another comes back with
   * a DIFFERENT bar length filed under the one that was asked for.
   *
   * A rung ABSENT FROM THE ARRAY is a rung the server declined to emit, which
   * its own comment defines as "neither served nor recorded" — so absent from
   * the array is `false`, not unknown.
   *
   * `null` WHERE THE ARRAY ITSELF IS ABSENT, and that is a real state rather
   * than a defensive shrug: the running binary may predate the field. A missing
   * answer is reported as missing and nothing is refused on the strength of it
   * — the same discipline `membershipTokens` reads the `universes` array under.
   */
  const rungServed = $derived.by(() => {
    // THE SAME CAST `pairFloor` MAKES, AND FOR THE SAME REASON — see `FeedRow`.
    const rows = /** @type {FeedRow | undefined} */ (active)?.history;
    if (!Array.isArray(rows)) return null;
    const m = new Map();
    for (const h of rows) if (h && typeof h.rung === 'string') m.set(h.rung, h.served === true);
    return m;
  });
  /** `true`, `false`, or `null` when this server states nothing about it. */
  /** @param {string} dir */
  function feedFetches(dir) {
    if (!rungServed) return null;
    return rungServed.get(dir) ?? false;
  }

  /**
   * ONE ROW PER RUNG, CARRYING EVERY REFUSAL THAT APPLIES TO IT AND NO OTHERS.
   *
   * THREE INDEPENDENT QUESTIONS, ASKED SEPARATELY AND NEVER COLLAPSED INTO ONE
   * WORD:
   *
   *   can the VENDOR ever serve it   `/feeds.json` `finest` — and this is the
   *                                  only one that disables a row
   *   does this BUILD fetch it       `/feeds.json` `history[].served`
   *   can the STORE file it          `RUNGS.stored`
   *
   * ONLY THE FIRST DISABLES, and the rule is worth stating because the other
   * two are tempting: a row is drawn dead only where nothing anybody could ever
   * do makes it exist. Every other refusal is drawn LIVE and stated, because
   * each of them names work a person could do — record an endpoint in
   * `pull::vendor`, widen `store::path::Timeframe` — and a control that hides
   * them hides the work along with the refusal. It is also why the store's
   * annotation stays on rows the vendor refuses: a rung can be refused twice,
   * and saying only the louder half is still a half.
   *
   * O(1) per row, three rows. No lookup here scans.
   *
   * TWO OF THE THREE ARE DRAWN ON EVERY FEED and the third is drawn on the two
   * whose floor reaches it — see `RUNGS`. The vendor question is the only one
   * that removes a row, so "which feeds show the second rung" is answered by
   * `/feeds.json` on every load and by nothing written in this file.
   */
  const rungRows = $derived.by(() => {
    const wire = feeds.active ?? '';
    const name = feedName(wire);
    return RUNGS.map((r) => {
      const v = floorVerdict(wire, r.dir);
      const fetches = feedFetches(r.dir);
      const says = [];
      const long = [];

      // ── THE VISIBLE LINE IS THE VENDOR'S, AND ONLY THE VENDOR'S ──
      //
      // It carries the fact that had no representation anywhere before this,
      // and it carries nothing else. An earlier turn of this put all three
      // refusals on every row and the result was "the store has no directory
      // for it" repeated down nine of eleven rows — which is where a reader
      // stops reading, and the one sentence that mattered was on the row they
      // stopped at. Eleven is three now and the argument is unchanged: one row
      // says one thing. The other two refusals are compact in the detail column,
      // whole in the hover, and in full sentences under the form for the rungs
      // actually TICKED, which is where they are actionable.
      if (v.state === 'refused') {
        says.push(
          `${shortFloor(wire)} Nothing makes this rung exist — not a pull, not an entitlement, not a purchase, not a code change.`
        );
        long.push(`${v.floor.because} — ${v.floor.source}`);
      } else if (v.state === 'finest') {
        // THE BOOLEAN, NOT THE WORD. `conflated` is
        // `pull::vendor::FinestKind::is_conflated` answered on the server, so
        // this page never decides from a string whether the sentence beside a
        // number may say "tick".
        //
        // THIS IS THE ROW THE OPERATOR CALLS THE TICK ROW, and it is the only
        // place on this control where the word can be answered honestly. It is
        // drawn on exactly the feeds whose stated floor reaches a second — the
        // two archives — and its sentence is what the archive was MEASURED to
        // hold, not what its file name says.
        says.push(
          v.floor.conflated === true
            ? `The finest ${name} serves, and it is what its tick archive actually holds: one record is a CONFLATED SNAPSHOT — the best bid, the best ask and the best last price as of that second. SECOND-KEYED: two rows can share a second, and that is expected input. Never a print.`
            : `The finest ${name} serves, and one record is a ${v.floor.label ?? v.floor.kind}.`
        );
        long.push(`${v.floor.label ?? v.floor.kind}. ${v.floor.because} — ${v.floor.source}`);
      } else if (v.state === 'unstated' && wire) {
        long.push(
          `This server sent no granularity floor for ${name}, so nothing here refuses any rung for it. That is a MISSING FIELD and not a vendor with nothing to say: crates/pull carries a floor for every feed it can name, and /feeds.json emits it as finest on every row — a row without one is a running binary that predates the field, or one that sent only half of it. This page holds no copy of the table to answer from, deliberately. Restart the server on a build that emits it.`
        );
      }

      // ── THE OTHER TWO REFUSALS: COMPACT IN THE COLUMN, WHOLE IN THE HOVER ──
      //
      // The detail column states the STRONGEST true thing about the row and
      // stops. These two are the refusals a PERSON can close, so they never
      // remove a row — they annotate one. `no store directory` on the second
      // rung is the whole of why a tick pull cannot land today, and it is the
      // sentence that must not be softened: the feed answers, the request
      // parses, and the bar has nowhere to go.
      if (fetches === false) {
        long.push(
          `${name} does not declare this rung: crates/api's served() refuses the POST for a rung pull::vendor::Descriptor::granularities does not carry — by name, and before it reads a credential — because this feed's bars path is pinned to one rung and a request for another would be answered with a DIFFERENT bar length and filed under the one you asked for. Fixable, and not from here: record the endpoint in pull::vendor.`
        );
      }
      if (!r.stored) {
        long.push(
          'The store has no directory for it, and the refusal is at the WRITE boundary rather than here: store::path::Timeframe::KNOWN ships 1min, 3min, 5min, 15min, 30min, 60min and 1day, and nothing below a minute, so pull::vendor::Granularity::store_timeframe answers None and pull::ingest::Plan::timeframe refuses the bar BY NAME rather than filing it under a rung it is not. That is a store-format limit and not a vendor one — the feed answers, and nothing here can file what it answers with. The fold that makes these same one-second records storable already exists (pull::fold, one-second buckets); it runs when this feed is asked for 1 minute.'
        );
      }

      const status = v.permanent
        ? 'never served'
        : fetches === false && !r.stored
          ? 'not fetched · no store dir'
          : fetches === false
            ? 'not fetched by this build'
            : !r.stored
              ? 'no store directory'
              : r.per === null
                ? 'no session count'
                : `${n(r.per)} bar(s) per session`;

      return {
        key: r.dir,
        name: r.label,
        detail: `${r.dir} · ${status}`,
        disabled: v.permanent,
        why: says.join(' ') || undefined,
        title: long.join(' ') || undefined,
        state: v.state,
        // NOT `disabled` — see Picker's `selectAll`. This rung can be ticked
        // and a person may mean to; what it must not be is ticked FOR him by a
        // bulk action, because `served()` refuses it by name and the run comes
        // back with a refusal nobody chose.
        skipBulk: fetches === false,
        fetches,
        stored: r.stored,
        label: r.label
      };
    })
      // A RUNG THIS FEED CAN NEVER SERVE IS NOT DRAWN AT ALL.
      //
      // This list used to keep them, struck through, behind a
      // "N rows this feed cannot serve" drawer, and the reasoning above this
      // function argued for it: hiding a refusal hides the work that would
      // close it. That argument holds for the rungs a PERSON could make exist
      // — one this build does not fetch yet, one the store has no directory
      // for — and those are still drawn, live, with their sentence.
      //
      // It does not hold for `permanent`. `v.permanent` is `/feeds.json`'s
      // granularity floor: the vendor does not publish this rung and no pull,
      // entitlement, purchase or code change makes it exist. There is no work
      // behind that row. On Groww it drew tick, 1 second and 5 seconds dead on
      // every load — three of eleven rows, permanently, saying nothing that
      // changes with anything the operator can do.
      //
      // Removed at the operator's instruction, 14 Aug 2026, stated twice. The
      // fact is not lost: the floor is still on `/feeds.json`, the feed's own
      // caption still names its finest rung, and a feed that DOES serve one
      // second — TrueData, GDFL — still shows it, because `permanent` is
      // answered per feed and not per rung. That is the whole of why this is a
      // filter here and not a deletion from the ladder. D-0137.
      //
      // ═══ AND THIS LINE IS NOW THE WHOLE OF "TICKS FOR TRUEDATA OR GDFL
      // ALONE" ═══
      //
      // The operator's rule of 14 Aug 2026 asks for the second rung on exactly
      // two feeds. Nothing in this file names them. `RUNGS` offers `1s`
      // unconditionally; `floorVerdict` marks it `refused` and `permanent` for
      // any feed whose `/feeds.json` floor is a minute; this filter drops it.
      // Dhan and Groww bottom out at a minute and the two archives bottom out
      // at a second, so the row is drawn for those two and for nobody else —
      // decided by the server on every load. A pair of feed names written into
      // a condition here would answer the same today and be a second copy of a
      // vendor fact tomorrow, which is what D-0126, D-0131 and D-0138 each
      // deleted from this page.
      .filter((row) => !row.disabled);
  });

  /**
   * THE SELECTED FEED'S FLOOR, RESOLVED ONCE.
   *
   * `null` when THIS SERVER sent no floor for the active feed, which the
   * control states rather than papers over — and it is the only reason it can
   * be null now, because the page holds no table of its own to fall back to.
   * `label` is the rung's DISPLAY name — the prose beside it is for a reader,
   * and `1min` is the key the store and the wire use, never the words a
   * sentence is built from. `short` is composed from the row's own fields; see
   * `shortFloor`.
   */
  const feedFinest = $derived.by(() => {
    const wire = feeds.active ?? '';
    const f = finestByWire.get(wire);
    if (!f) return null;
    // `rungPhrase`, NOT `label` AND NOT THE WIRE'S `label`. Three names collide
    // on this one line and each would be wrong in its own way:
    //
    //   the wire's `label`  is the KIND's sentence — "conflated snapshot — best
    //                       bid, best ask, best last price" — and it is the
    //                       right half of "is a ___". Spelling the rung into
    //                       this key would have printed "one record at 1 second
    //                       is a 1 second".
    //   the rung's `label`  is the BUTTON word, so this would read "one record
    //                       at Ticks is a conflated snapshot" — this page
    //                       calling a snapshot a tick, in the one sentence
    //                       written to say it is not one.
    //   the rung's `phrase` is the measured name, and it is the only one that
    //                       belongs in a clause: "one record at one second".
    return {
      ...f,
      short: shortFloor(wire),
      rungPhrase: RUNGS.find((r) => r.dir === f.rung)?.phrase ?? f.rung
    };
  });

  /** What the rows add up to, counted once and read by every line that states it. */
  const rungTally = $derived.by(() => {
    let dead = 0;
    let unfetched = 0;
    let unstored = 0;
    for (const row of rungRows) {
      if (row.disabled) dead += 1;
      if (row.fetches === false) unfetched += 1;
      if (!row.stored) unstored += 1;
    }
    // `dead` IS NOW ALWAYS ZERO and the field stays, because the two callers
    // read it to decide whether to say anything at all — and a feed whose floor
    // this server did not send has `permanent === false` on every row, so the
    // count is the honest way to ask "were any dropped" rather than an
    // assumption that none were.
    return { dead, unfetched, unstored, live: rungRows.length };
  });

  /**
   * A RUNG THE NEW FEED CAN NEVER SERVE IS DROPPED, AND THE DROP IS THE LOUDEST
   * THING ON THE CONTROL.
   *
   * THE OWNER'S RULE: "as per the chosen broker it should display the entire
   * below sections." Moving the feed re-gates every rung, and a tick that
   * survives the move is a tick the new feed has no word for.
   *
   * BOTH QUIET ANSWERS ARE WRONG, AND THEY ARE WRONG IN OPPOSITE DIRECTIONS.
   * Keeping a ticked `1s` while the feed moves from TrueData to Groww builds a
   * request Groww's wire cannot spell — the interval field has nothing to put
   * in it. Clearing it without a word is the silent rewrite §4 bans, and leaves
   * an operator looking at a control that lost a tick he made and never said
   * so. So it is dropped AND named, and the notice OUTLIVES the change that
   * caused it: it stands until the operator touches the timeframe himself or
   * moves the feed again.
   *
   * ONLY PERMANENT REFUSALS ARE DROPPED. A rung this build does not fetch, or
   * one the store cannot file, stays ticked — those refusals are somebody's
   * work item, and taking the tick away would be this page deciding the work
   * will never be done.
   *
   * `untrack` around the read of `rungSet`, so the only thing that can trigger
   * this is a FEED CHANGE. Without it the effect would re-run on its own write,
   * and an effect that reacts to itself is one bad predicate away from a loop.
   */
  /**
   * AN UNTOUCHED DEFAULT FOLLOWS THE FEED; A TICK HE MADE DOES NOT.
   *
   * Runs before the drop effect below and answers a different question. That
   * one asks *can this vendor ever serve the rung the operator chose*; this one
   * asks *does the rung nobody chose have any chance of working here*.
   *
   * `feedFetches` is `/feeds.json`'s `history[].served`, which is the SAME bit
   * `crates/api`'s `served()` gates the POST on — so "this will be refused" is
   * read from the server rather than guessed, and a feed whose row carries no
   * such flag (`null`) is left alone: an unknown is not a reason to move a
   * default, and nothing here refuses on the strength of a missing field.
   *
   * The FIRST rung the feed declares, in the ladder's own order, so a feed that
   * serves both a minute and a day opens on the minute — the finer of the two,
   * and the one the engine sweeps.
   *
   * `untrack`, for the reason the drop effect gives: this effect WRITES
   * `rungSet`, and reading it as a dependency would make every one of its own
   * writes schedule another run.
   */
  $effect(() => {
    const wire = feeds.active ?? '';
    void wire;
    untrack(() => {
      if (rungTouched || !rungServed) return;
      const chosen = RUNGS.filter((r) => rungSet.has(r.dir));
      // ONLY WHERE EVERY TICKED RUNG WOULD BE REFUSED. One rung out of two
      // that this build does not fetch is a partial run and a stated one, not
      // a form that cannot be submitted.
      if (chosen.length > 0 && chosen.some((r) => feedFetches(r.dir) !== false)) return;
      const usable = RUNGS.find((r) => feedFetches(r.dir) === true);
      if (usable && !rungSet.has(usable.dir)) {
        rungSet = new Set([usable.dir]);
      }
    });
  });

  /**
   * WHAT THE LAST FEED CHANGE TOOK, or `null` when it took nothing. See the
   * effect below: it is retired by the next change rather than left standing.
   *
   * @type {{
   *   feed: string,
   *   kept: number,
   *   rows: { dir: string, label: string, why: string }[]
   * } | null}
   */
  let droppedRungs = $state(null);
  $effect(() => {
    const wire = feeds.active ?? '';
    untrack(() => {
      /** @type {Rung[]} */
      const gone = [];
      /** @type {Rung[]} */
      const kept = [];
      for (const r of RUNGS) {
        if (!rungSet.has(r.dir)) continue;
        (floorVerdict(wire, r.dir).permanent ? gone : kept).push(r);
      }
      if (gone.length === 0) {
        // A LATER FEED CHANGE THAT TAKES NOTHING RETIRES THE LAST ONE'S NOTICE.
        // The message is about what THIS change took; left standing under a
        // third feed it names a drop that is two decisions old, which is a
        // stale claim on screen rather than a record.
        droppedRungs = null;
        return;
      }
      rungSet = new Set(kept.map((r) => r.dir));
      droppedRungs = {
        feed: feedName(wire),
        kept: kept.length,
        // ONE REASON, BECAUSE THERE IS ONE WAY TO BE DROPPED. A `never` arm
        // stood beside this one and answered for `tick`; that rung is no longer
        // offered, so the only rung a feed change can take is the second — and
        // it is taken for exactly one reason, which is that the feed it was
        // ticked for reaches below a minute and this one does not.
        rows: gone.map((r) => ({
          dir: r.dir,
          label: r.label,
          why: `${shortFloor(wire)} It was ticked for a feed that reaches below a minute.`
        }))
      };
    });
  });
  /**
   * THE WINDOW IS TWO DAYS, AND THE ISO `YYYY-MM-DD` IS THE KEY FORM.
   *
   * DAYS, BECAUSE THAT IS WHAT THE VENDOR ANSWERS AND WHAT THE WIRE CARRIES:
   * `api::ingest::parse_spot` reads `from` and `to` as `YYYY-MM-DD` and
   * `MAX_WINDOW_DAYS` caps their difference. A month picker stood here and was
   * wrong — it named a unit of the STORE (the month file) as if it were the
   * unit of the REQUEST, and it silently widened `02 Sep` into all of
   * September.
   *
   * THE MONTH HAS NOT VANISHED, because the store still writes one file per
   * month: `/store.json` answers one row per instrument-MONTH and the store
   * address still ends in `/2024-11`. What the day window decides is how much
   * of each of those files is being asked for — `windowMonths` is the set it
   * touches, and it is counted, never assumed.
   *
   * `*Text` is what is IN the field — the characters the operator typed, kept
   * verbatim so a refusal can quote them. `*Day` is the parsed value, and it is
   * empty whenever the text does not parse: an unreadable string must not leave
   * a stale day behind it and quietly pull a window nobody asked for.
   */
  let fromDay = $state('');
  let toDay = $state('');
  let fromText = $state('');
  let toText = $state('');

  /**
   * THE RISK-FREE RATE, AS A DECIMAL, AND EMPTY BY DEFAULT.
   *
   * Empty is a real answer and the ordinary one: `crates/api/src/ingest.rs`
   * treats an absent `rate` as "compute no greek", and the receipt then says so
   * in words rather than rendering the same as a run that priced everything.
   *
   * It is NOT defaulted to a plausible number, and that is the whole point.
   * `docs/00-charter.md` records no rate, so `CLAUDE.md` §3 rule 1 forbids this
   * build claiming one; `pull::pricing::Rate` cannot even be constructed
   * without naming where its value came from. A box the operator fills is the
   * only honest source there is today, and what they type travels into every
   * priced row so the result stays reproducible.
   *
   * A value outside ±1.0 is refused by the server, by name — `9.46` where
   * `0.0655` was meant prices every option in the run wrongly, comes out finite
   * everywhere, and errors nowhere downstream.
   */
  /**
   * TYPED ONCE, NOT ONCE PER RUN.
   *
   * The rate genuinely cannot be derived and genuinely cannot be skipped — see
   * `pull::pricing::the_rate_materially_moves_a_weekly_which_is_why_it_must_be_supplied`,
   * which MEASURES it: on a seven-day ATM weekly, implied volatility runs
   * 0.1776 at r=0% down to 0.1557 at r=12%, a spread of **0.0219**, about a 13%
   * relative swing. A first version of that test asserted the opposite on the
   * reasoning that `exp(-r·T)` over seven days is one part in eight hundred;
   * the reasoning was wrong, because the rate moves the FORWARD and not just
   * the discount, and 0.23% of forward against a 2.1% expected move is not
   * small.
   *
   * What the same measurement also shows is that it does not have to be
   * PRECISE: across the plausible 5.5%–8% band the spread is only **0.0046**.
   * So the operator has to supply one and does not have to agonise over it.
   *
   * The friction worth removing is therefore re-typing, not the field. It
   * persists here the way the theme does in `+layout.svelte`, and a storage
   * failure is not an error worth a banner — the box simply starts empty and
   * everything else still works.
   */
  const RATE_KEY = 'brutex.riskFreeRate';

  function readRate() {
    try {
      const v = localStorage.getItem(RATE_KEY);
      /* VALIDATED ON THE WAY OUT, not trusted because it came from us. The
         server refuses anything outside ±1.0 by name, and a stored value that
         would be refused is worse than none: it looks like a setting that
         works. */
      if (v && Number.isFinite(Number(v)) && Math.abs(Number(v)) <= 1) return v;
      return '';
    } catch {
      return '';
    }
  }

  let rateText = $state(readRate());

  $effect(() => {
    const v = rateText.trim();
    try {
      if (v) localStorage.setItem(RATE_KEY, v);
      else localStorage.removeItem(RATE_KEY);
    } catch {
      /* Private browsing, or storage disabled. The rate still travels with
         this run; it just will not be there for the next one. */
    }
  });
  /** The refusal a typed string earned, per field. Cleared the moment one takes. */
  /** @type {{ from: string | null, to: string | null }} */
  let typedErr = $state({ from: null, to: null });
  let folder = $state('');
  let showProblems = $state(false);

  /**
   * WHICH MENU IN THE STRIP IS OPEN — one value for the whole page, so "two are
   * open at once" is not a state this component can reach.
   *
   * The same rule `$lib/Picker.svelte` holds for itself, and for the same
   * reason: a per-menu `open` flag plus an outside-click close never shuts the
   * first menu, because a click on the SECOND menu's button is not outside the
   * first one's `.dd` — it is inside that one. Picker's own module-level id
   * closes its menus when anything else on the page is pressed, and the
   * pointer-capture handler in `onMount` closes this one for the same press,
   * so the two never overlap in either direction.
   *
   * `'feed' | 'uni' | 'ins' | null`.
   *
   * `'feed'` IS BACK, AND IT IS THE ONLY ONE OF ITS KIND. This page carried its
   * own feed dropdown, then lost it because the top bar carried one too, and
   * two controls for one value teach the reader they are independent when they
   * are not. The rule was right and the remedy was aimed at the wrong control:
   * the feed is the FIRST RUNG of this page's cascade — universe, instruments,
   * segments, timeframe and window are all its answer — so the control belongs
   * beside the four it governs. `+layout.svelte` now omits its picker on the
   * routes that own one (FEED_OWNED there), which keeps the count at exactly
   * one on every route rather than at one page-wide.
   */
  /** @type {'feed' | 'uni' | 'ins' | null} */
  let drop = $state(null);

  /**
   * The chosen universe, and the wire value it spells. `SWEPT` is the concrete
   * floor rather than `UNIVERSES.at(-1)`: a fallback that can be `undefined`
   * needs a guard at every reader, and a missed guard on the row that decides
   * what the request asks for is a request with no target at all.
   */
  const universeSpec = $derived(UNIVERSE_BY_ID.get(universe) ?? SWEPT);
  const target = $derived(universeSpec.target ?? '');

  let nowMs = $state(Date.now());
  const istToday = $derived(istDay(nowMs));
  const istMin = $derived(istMinute(nowMs));

  /**
   * ═════════════ THE CEILING: THE NEWEST SESSION THAT HAS CLOSED ═════════════
   *
   * COMPUTED FROM THE IST CLOCK ON EVERY TICK, NEVER A LITERAL. A date written
   * down here is correct on the day it is typed and one day staler every day
   * afterwards, and nothing on the page can see that it has gone wrong —
   * `nowMs` is re-read twice a second, so this moves on its own at midnight
   * IST and again at 15:30.
   *
   * WHY THE CLOSE MATTERS, and why "yesterday" is not an off-by-one: while a
   * session is still running no feed has a final bar for it. Asking for it
   * yields a PARTIAL day, and an append-only store cannot correct a partial day
   * later. So the newest askable day is the newest session that has actually
   * finished — which is today after 15:30 IST, and the previous session before
   * it. `ceilReason` says which of those it is, in words, wherever the ceiling
   * bites.
   *
   * The walk backwards is bounded by construction: at most three non-sessions
   * can sit in a row here — a long weekend — and the loop is capped anyway.
   */
  const maxDay = $derived.by(() => {
    let d = istToday;
    if (isSession(d) && istMin >= CLOSE_MIN) return d;
    for (let i = 0; i < 10; i += 1) {
      d = addDays(d, -1);
      if (isSession(d)) return d;
    }
    return d;
  });
  /**
   * WHY THE CEILING IS WHERE IT IS. One sentence, one definition, read by the
   * calendar footer, the ceiling day's own tooltip and the refusal for any day
   * after it — three places that must not be able to say different things.
   */
  const ceilReason = $derived.by(() => {
    const today = istToday;
    const guessing = !holidaysKnownFor(today)
      ? ` This page's NSE holiday table covers ${dayLabel(HOL_FROM)} – ${dayLabel(HOL_THRU)} and today is outside it, so a weekday here is ASSUMED to be a session — an NSE holiday would be offered and would come back empty.`
      : '';
    if (isSession(today) && istMin < CLOSE_MIN) {
      return `Today, ${dayLabel(today)}, is a trading session and it has not closed yet — NSE closes 15:30 IST and it is ${pad(Math.floor(istMin / 60))}:${pad(istMin % 60)} — so no feed has a final bar for it. The newest day that can be asked for is ${dayLabel(maxDay)}, the last session that closed.${guessing}`;
    }
    if (isSession(today)) {
      return `Today, ${dayLabel(today)}, is a trading session and it closed at 15:30 IST, so today IS the newest day that can be asked for.${guessing}`;
    }
    return `${dayLabel(today)} — ${noSessionWhy(today)} — so the newest day that can be asked for is ${dayLabel(maxDay)}, the last NSE session before it.${guessing}`;
  });

  /**
   * THE FLOOR OF THIS PAGE'S OWN CALENDAR, and it is the ONE thing here that is
   * allowed to be a constant: `GET /pull`'s own picker offers 2015 onward
   * (`calendar::YEARS_OFFERED`), so nothing older can be asked for through this
   * server at all.
   *
   * IT IS NOT A DATA FLOOR. An earlier revision refused every day before the
   * page's own session table began, which was this page's THINNESS wearing a
   * rule's clothes: a day nobody had loaded a holiday list for is still a day a
   * vendor answers for. The only floor that may take a day away from the
   * operator is the FEED's, below, because that is the one his own choice moves.
   */
  const minDay = $derived(iso(FLOOR_YEAR, 1, 1));

  /**
   * THE SELECTED FEED'S OWN FLOOR, at the timeframe selected, resolved against
   * today so a rolling one moves with the clock. `null` when the feed states
   * none and when nothing has been read for it — `known` tells those apart.
   */
  /**
   * THE STRICTEST FLOOR ACROSS EVERY TICKED RUNG, because the window is one
   * window and it is asked for at each of them. Taking the first rung's floor
   * would offer a day that the second rung's feed refuses, and the refusal
   * would arrive from the vendor rather than from the control.
   */
  /**
   * THE RUNG IS OPTIONAL BECAUSE ONE ARM CANNOT NAME ONE. With nothing ticked
   * there is no rung the floor belongs to, and inventing one would put a bar
   * length in a refusal the operator never chose. `floorSentence` falls back to
   * the header's own rung when it is absent, which is the only other rung on
   * screen.
   *
   * @type {{ at: string | null, known: boolean, rolling: boolean, src: string[], rung?: Rung }}
   */
  const feedFloor = $derived.by(() => {
    const wire = feeds.active ?? '';
    if (rungsChosen.length === 0) return pairFloor(wire, rung);
    // SEEDED FROM THE FIRST RUNG RATHER THAN FROM null.
    //
    // The old shape started at `null` and relied on `!worst` being true on the
    // first pass to fill it. That is true — the early return above guarantees at
    // least one iteration — but it is true by an argument a reader has to
    // reconstruct, and the checker cannot reconstruct it at all: it flagged
    // `worst.f` and `worst.r` on the return as possibly null. Seeding removes
    // the null from the type instead of asserting it away, so the guarantee is
    // in the code rather than in a comment.
    let worst = { r: rungsChosen[0], f: pairFloor(wire, rungsChosen[0].dir) };
    for (const r of rungsChosen.slice(1)) {
      const f = pairFloor(wire, r.dir);
      if (f.at && (!worst.f.at || f.at > worst.f.at)) worst = { r, f };
    }
    return { ...worst.f, rung: worst.r };
  });

  /**
   * THE VISIBLE WINDOW IS FEED-INDEPENDENT. THE WIRE IS NOT.
   *
   * # Why the field must not show one feed's floor
   *
   * It briefly did — `feedFloor.at`, so Dhan opened on its rolling five years.
   * That is only correct while exactly ONE feed is ticked. Tick two and there is
   * no single honest date to put in one box: Dhan answers from five years back
   * and Groww from 01 Jan 2020, and a field showing either is wrong about the
   * other. Measured on the running page — the strip read `2 feeds · 2 request(s)`
   * over a From of 18 Aug 2021, which is Dhan's floor presented as the window
   * for both.
   *
   * So the two fields state the OPERATOR'S INTENT and nothing else: the whole
   * span the server's picker offers, 01 Jan 2015 to the newest askable day.
   * `minDay` and `maxDay`, neither of which knows or cares which feeds are
   * ticked.
   *
   * # Where the per-feed dates belong
   *
   * On the wire, per body, and only there — see `floorFor` and `wireBodies`.
   * Each request already goes out as its own POST, so each one can carry the
   * date ITS vendor can answer for. The floors are declared, not invented:
   * `pull::vendor` holds `Rolling { years: 5 }` for Dhan,
   * `Fixed { 2020, 1, 1 }` for Groww and `Rolling { years: 10 }` for Zerodha,
   * each with the operator's own observation recorded as its source.
   *
   * # Seeded once, not pinned
   *
   * `maxDay` moves at midnight and at the 15:30 close; re-running this would
   * overwrite a window the operator had chosen. `windowTouched` is
   * `rungTouched`'s argument applied here — "A default is not a decision he
   * made. Moving it costs him nothing and tells him nothing false. Moving HIS
   * tick would do both."
   */
  let windowTouched = $state(false);

  $effect(() => {
    if (windowTouched || !isValidIso(maxDay) || !isValidIso(minDay)) return;
    windowTouched = true;
    setDay('from', minDay);
    setDay('to', maxDay);
  });
  /**
   * The refusal, naming the FEED, the TIMEFRAME and the day it starts at.
   *
   * `label` HERE, NOT `phrase`, AND IT IS THE ONE SENTENCE THAT GOES THAT WAY.
   * The `·` makes this a breadcrumb rather than a clause — FEED · TIMEFRAME —
   * and a refusal must name the control the operator TOUCHED, with the word
   * that is printed on it. Telling him "one second is out of reach" when he
   * ticked a box marked Ticks sends him looking for a control he does not have.
   * Every other rung-in-a-sentence takes `phrase`; see `RUNGS`.
   */
  const floorSentence = $derived.by(() => {
    if (!feedFloor.at) return null;
    const r = feedFloor.rung ?? RUNGS.find((x) => x.dir === rung);
    const others =
      rungsChosen.length > 1
        ? ` It is the strictest of the ${n(rungsChosen.length)} ticked timeframes, and the window is asked for at every one of them.`
        : '';
    return `${feedName(feeds.active)} · ${r?.label ?? rung} answers from ${dayLabel(feedFloor.at)}${feedFloor.rolling ? ' — a rolling floor, so it moves every day' : ''}.${others}`;
  });

  const minMonth = $derived(minDay.slice(0, 7));
  const maxMonth = $derived(maxDay.slice(0, 7));

  /**
   * THE WINDOW ON THE WIRE, AND THERE IS NO CROSSING LEFT TO MAKE. The two
   * fields hold the two days the request carries — no widening, no clamping and
   * no unit change between what was picked and what is sent.
   */
  const from = $derived(isValidIso(fromDay) ? fromDay : '');
  const to = $derived(isValidIso(toDay) ? toDay : '');

  const windowDays = $derived(
    isValidIso(from) && isValidIso(to) ? dayNum(to) - dayNum(from) + 1 : 0
  );
  /**
   * A BACKWARDS WINDOW HAS NO SPAN, and the counters say so instead of printing
   * `-3 days` and `2 instrument-months` as if the request meant something. The
   * refusal below names the fault; the tiles must not quietly contradict it.
   */
  const windowOk = $derived(windowDays > 0);

  /**
   * WHETHER THE WINDOW REACHES OUTSIDE THE HOLIDAY TABLE THIS PAGE CARRIES.
   *
   * `false` while no window is picked — an absent window is not outside
   * anything, and saying it is would put a caveat on a number nobody asked for.
   *
   * It decides whether the census footer's fold OPENS ITSELF. Outside the
   * table's range every "expected" count can be one session too high, so that
   * paragraph stops being a definition and becomes a caveat on a number that is
   * on screen — and a limit that applies right now is not something to make an
   * operator hunt for. Inside it, the same paragraph is a definition and folds
   * away with the rest.
   */
  const outsideHolidayTable = $derived(
    windowOk && (!holidaysKnownFor(from) || !holidaysKnownFor(to))
  );

  /**
   * THE WINDOW ASKED FOR, AGAINST THE DAYS THE FOLDER ACTUALLY HOLDS.
   *
   * `null` unless there is something to say — a REST feed, an unread folder, a
   * folder with no span, or a window sitting wholly inside the measured one all
   * answer nothing.
   *
   * # Why this is a caution and not a refusal
   *
   * The calendar deliberately does NOT strike days outside the folder's span,
   * and its own comment says why: those days are askable and come back empty
   * rather than refused, so striking them would claim a refusal nobody made.
   * That reasoning is right and it is not the whole job. A read of a folder for
   * days that hold no file was always going to land nothing, and `cautions`
   * exists precisely so an operator sees a known blocker BEFORE the clock
   * starts rather than on the receipt.
   *
   * So the day stays pickable and the outcome stops being a surprise.
   *
   * # Why a folder may be compared this way when a vendor may not
   *
   * A REST feed's history floor is a CLAIM — the vendor's own words about how
   * far back it will answer, which this repository does not get to check. A
   * folder's span is a MEASUREMENT taken by walking it. Comparing a window
   * against a measurement is arithmetic; comparing it against a claim is what
   * `pairFloor` already does, separately, with the source printed beside it.
   *
   * Two shapes, because they are two different mistakes:
   *   past      the whole window is outside the span — nothing can land at all
   *   partial   one end is outside — the run is shorter than it looks
   *
   * Placed here rather than beside `archiveCensus` because it reads `from`,
   * `to` and `windowOk`, all declared above this line. A `$derived` is lazy and
   * would have resolved either way; standing above its own inputs is a thing
   * the next reader has to verify, and it does not have to be.
   */
  const windowVsFolder = $derived.by(() => {
    const c = archiveCensus;
    if (!c || !from || !to || !windowOk) return null;
    // AN EMPTY OR BLANK FOLDER IS ITS OWN SENTENCE AND NOT THIS ONE. There is
    // no span to compare a window against, and "outside 0 days" reads as a
    // range rather than as an absence.
    if (c.state !== 'days' || !c.earliest || !c.latest) return null;
    if (to < c.earliest || from > c.latest) return { shape: 'past', ...c };
    if (from < c.earliest || to > c.latest) return { shape: 'partial', ...c };
    return null;
  });
  const windowMonths = $derived(windowOk ? monthsBetween(from, to) : []);

  /**
   * The exact bytes of ONE request. Shown, not described.
   *
   * `granularity` and the two days are parameters rather than reads of the
   * form, because the same encoder builds the row-level pull below: a button on
   * a census row sends this shape with a narrower window and the row's own
   * rung, and there is exactly one place in this file that knows what a request
   * looks like.
   */
  /** @param {string} dir @param {string} a @param {string} b */
  function wireBodyFor(dir, a, b, vendor = null) {
    const p = new URLSearchParams();
    p.set('target', target);
    /* THE VENDOR IS AN ARGUMENT NOW, NOT A READ OF THE PAGE'S SCOPE. With one
       feed the two are the same value; with several, reading `feeds.active`
       here would have sent N identical requests for the scope feed and
       reported them as N different ones. */
    p.set('vendor', vendor ?? feeds.active ?? '');
    p.set('from', a);
    p.set('to', b);
    p.set('granularity', dir);

    // THE TICKED INSTRUMENTS, WHICH THIS FORM USED TO DRAW AND NEVER SEND.
    //
    // The picker above says `1 of 213 ticked` and the line under the form says
    // `ASKED 1 instrument(s)`. Neither reached the wire: the body carried the
    // TARGET alone, so the server expanded it to the whole set. Measured — an
    // operator ticked NIFTY and the store came back holding GLENMARK, IOC,
    // ASIANPAINT, BHEL and two hundred more, in 430 files. The count on the
    // button, the count on the receipt and the run were three answers to one
    // question.
    //
    // REPEATED `member=`, one per tick, which `api::server::params` reads whole
    // — `param` takes the first match and would silently turn a selection into
    // its first element.
    //
    // ALL TICKED SENDS NOTHING, deliberately. Naming every member and naming
    // none ask for the same set, and the empty form is the one the autopilot
    // already sends and the one every stored bookmark carries. It also keeps a
    // 750-name selection from putting 750 fields on a query string to say
    // "everything".
    if (ticked.length > 0 && ticked.length < insPool.length) {
      for (const m of ticked) p.append('member', m.symbol ?? m.key);
    }
    if (isFolderFeed) p.set('folder', folder);
    return p.toString();
  }
  /** One body per ticked rung, in the ladder's order — what Start would send. */
  /* ONE REQUEST PER (FEED x RUNG), which is what `runPull` has always walked:
     it takes an ARRAY of bodies and POSTs each in turn, and until now that
     array was one entry per ticked timeframe. Adding the feed to the product
     is the whole of the fan-out -- nothing about the loop, the receipts or the
     abort changes, because they were already written for N. */
  /**
   * THE DAY THIS ONE REQUEST MAY START AT, for this feed at this rung.
   *
   * Three bounds, and the latest of them wins:
   *
   *   1. the window the operator asked for — never widened, only narrowed;
   *   2. the feed's own declared floor, `pairFloor`, which for Dhan is a
   *      rolling five years, for Groww the fixed 01 Jan 2020 and for Zerodha a
   *      rolling ten — each traceable to the operator's own observation in
   *      `pull::vendor`, not invented here;
   *   3. `MAX_WINDOW_DAYS` behind the To date, because a feed with no declared
   *      floor at all (both archives are `Unbounded`) would otherwise inherit
   *      the full 01 Jan 2015 span, which is about 4,250 days and the parser
   *      refuses at 3,653 before any vendor is contacted.
   *
   * A FEED THAT STATES NO FLOOR IS NOT A FEED WITH NO LIMIT. Bound 3 is what
   * keeps the honest answer "as much as one request may carry" from becoming
   * the dishonest one "refused, and the page let you press it".
   *
   * THIS NARROWS THE ASK AND IT IS NOT SILENT. Every body it produces is
   * printed verbatim in the wire fold below the form, so the dates that
   * actually go out are on screen beside the dates that were asked for — which
   * is the whole reason the fold exists.
   *
   * @param {string} wire @param {string} rungDir
   */
  function floorFor(wire, rungDir) {
    const declared = pairFloor(wire, rungDir).at;
    const cap = isValidIso(to) ? addDays(to, -(MAX_WINDOW_DAYS - 1)) : from;
    let start = from;
    if (declared && declared > start) start = declared;
    if (cap > start) start = cap;
    return start;
  }

  /**
   * THE EXPIRED-F&O REQUESTS, one per feed per settled series per underlying.
   *
   * A DIFFERENT ROUTE AND A DIFFERENT SHAPE, which is why they are built here
   * rather than folded into `wireBodies`. `/pull/spot` fills a whole target in
   * one request; `/pull/fno` names ONE underlying, because discovery is keyed
   * on the underlying's own expiry list and there is no call that takes a set.
   *
   * NO `granularity` AND NO `expiry`. The route walks the month the window
   * names and pulls every settled expiry inside it — the shape an operator
   * actually asks for, and the shape the vendor publishes. An expiry field
   * would narrow it to one, which is the old one-at-a-time form.
   *
   * These run AFTER every spot body, never beside them: `pull::fold`'s ladder
   * puts spot before futures and futures before options, and a derivative
   * request that arrives first is refused for want of the spot month.
   */
  /**
   * WHICH FEEDS CAN ANSWER AN EXPIRED-F&O REQUEST AT ALL.
   *
   * `/feeds.json` states it as `fno`, one of four words, because there are
   * four cases and they are not a yes/no:
   *
   *   by_name           — a discovery walk over contract names. This is the
   *                       only shape `POST /pull/fno` builds today.
   *   by_strike_offset  — no names to discover: the request names ATM±n and an
   *                       expiry CODE. A different builder, not yet written.
   *   none              — the vendor serves no derivative history.
   *   local_folder      — an archive. Its expired contracts are files on disk,
   *                       not an answer to a GET, and never will be.
   *
   * MEASURED, WHICH IS WHY THIS EXISTS. On 2026-08-19 this page fired an
   * expired-F&O request at every ticked feed. The ones aimed at Dhan came back
   * 502 — correctly, since Dhan publishes no name to discover — but only after
   * paying for a credential read and a socket each. A page cannot decline what
   * it cannot see, and until `/feeds.json` said this, it could not see it.
   */
  /**
   * WHICH SEGMENTS EACH TICKED FEED CAN ACTUALLY BE ASKED FOR.
   *
   * THIS USED TO BE `fno === 'by_name'` AND DROPPED DHAN ENTIRELY.
   *
   * Dhan reports `by_strike_offset` — it addresses expired options by ATM±n
   * rather than by contract name — and that one-word filter meant **no F&O
   * request was ever built for it**. Measured 2026-08-20: zero Dhan contracts
   * on disk, zero `rollingoption` requests in a 1,253-event journal. The path
   * was not failing; it was never being asked.
   *
   * The declined-reason beside it said the cause was that Dhan's *"own request
   * builder is not written yet"*. That was FALSE — `fno_roll` and `roll_one`
   * are about a thousand lines, and `fno_walk` already routes `by_offset`
   * straight to them. The filter was excluding a feed on the strength of a
   * stale sentence.
   *
   * OPTIONS ONLY FOR AN OFFSET FEED, and that part is real: the endpoint is
   * `v2/charts/rollingoption`, and `fno_roll` refuses `Series::Futures` by
   * name. Sending it a futures request would earn a refusal this page can
   * predict, so it does not send one.
   */
  const fnoSegmentsFor = (/** @type {string} */ wire) => {
    const how = feeds.all.find((f) => f.wire === wire)?.fno;
    if (how === 'by_name') return ['futures', 'options'];
    if (how === 'by_strike_offset') return ['options'];
    return [];
  };

  const fnoCapable = $derived(
    feedsChosen.filter((v) => fnoSegmentsFor(v).length > 0)
  );

  /**
   * THE FEEDS THAT WERE TICKED AND CANNOT SERVE, with the reason each one
   * cannot. Rendered rather than dropped: silently sending fewer requests than
   * the operator ticked is the §4 fallback that hides a failure, and a run that
   * quietly skipped a broker is one an operator would read as a broker that
   * quietly failed.
   */
  const fnoDeclined = $derived(
    feedsChosen
      .map((v) => ({ wire: v, how: feeds.all.find((f) => f.wire === v)?.fno }))
      /* NOW ONLY THE FEEDS THAT GENUINELY CANNOT BE ASKED. `by_strike_offset`
         used to land here with a reason that was not true — see `fnoCapable`.
         It is served now, so it is no longer declined. */
      .filter((x) => x.how !== 'by_name' && x.how !== 'by_strike_offset')
      .map((x) => ({
        feed: feedName(x.wire),
        why:
          x.how === 'local_folder'
            ? 'is a folder of files, not an endpoint — there is no expiry list to ask it for.'
            : 'states no expired-derivative history.'
      }))
  );

  /**
   * THE FEEDS SERVED FOR SOME SEGMENTS AND NOT ALL, with which and why.
   *
   * Neither declined nor fully served, and the gap between those two is where a
   * silent drop lives. Dhan answers expired OPTIONS by strike offset and has no
   * expired-futures endpoint at all — so a run that ticks both segments sends it
   * one and not the other, and this line is what stops that being invisible.
   */
  const fnoPartial = $derived(
    fnoCapable
      .map((v) => ({ wire: v, segs: fnoSegmentsFor(v) }))
      .filter((x) => x.segs.length > 0 && x.segs.length < 2)
      .map((x) => ({
        feed: feedName(x.wire),
        why: `serves expired ${x.segs.join(' and ')} only — its endpoint is v2/charts/rollingoption, which addresses options by ATM±n and publishes no expired-futures series.`
      }))
  );

  const fnoBodies = $derived(
    fnoCapable.flatMap((v) =>
      segmentsReached
        /* PER FEED, NOT GLOBALLY. A name-discovery feed answers both segments;
           an offset feed answers options only. Filtering once for all vendors
           would either send Dhan a futures request it refuses by name, or drop
           futures from Groww to match Dhan — and both are wrong. */
        .filter((seg) => fnoSegmentsFor(v).includes(seg.key))
        .flatMap((seg) =>
          ticked.map((m) => {
            const p = new URLSearchParams();
            p.set('underlying', m.symbol ?? m.key);
            p.set('series', seg.key === 'futures' ? 'fut' : 'opt');
            p.set('vendor', v);
            p.set('from', floorFor(v, '1day'));
            p.set('to', to);
            /* SENT ONLY WHEN TYPED. An empty `rate` and an absent one mean the
               same thing to the server — compute no greek — but sending an
               empty string would make the receipt's "no rate was supplied"
               line arrive for a field the operator can see is present, which
               reads as a bug rather than as the answer it is. */
            if (rateText.trim()) p.set('rate', rateText.trim());
            return {
              dir: seg.key,
              vendor: v,
              route: '/pull/fno',
              label: `${feedName(v)} · ${seg.label} · ${m.symbol ?? m.key}`,
              from: floorFor(v, '1day'),
              body: p.toString()
            };
          })
        )
    )
  );

  /**
   * EVERY LEG ONE PRESS SENDS — spot and expired-derivative together.
   *
   * # The disagreement this removes
   *
   * `runPull` built `[...wireBodies, ...fnoBodies]` inline while the feed
   * picker counted `wireBodies` ALONE. Measured on the running page with three
   * segments ticked: the strip read **`3 feeds · 6 request(s)`** and the press
   * submitted **nine**. The label, the counter and the run were three answers to
   * one question — the shape this product removes everywhere else, and the §4
   * failure of a surface that reports something other than what happened.
   *
   * Declared once here so there is nothing left to keep in step: the counter and
   * the submission read the same expression, and a segment added later joins
   * both or neither.
   *
   * Spot first, then derivatives — `pull::fold`'s ladder is the server's to
   * enforce and it re-sorts each vendor's legs itself, so this order is what is
   * OFFERED, never what is sent.
   */

  const wireBodies = $derived(
    feedsChosen.flatMap((v) =>
      rungsChosen.map((r) => ({
        dir: r.dir,
        route: '/pull/spot',
        /* THE FEED THIS BODY BELONGS TO. `runPull` groups on it: the vendor is
           already inside `body` as a form field, and digging it back out of a
           URLSearchParams to decide scheduling would be reading the wire to
           learn something the caller knew. */
        vendor: v,
        label: feedsChosen.length > 1 ? `${feedName(v)} · ${r.label}` : r.label,
        /* THE FROM IS THIS FEED'S, NOT THE FORM'S. See `floorFor`: the two
           date boxes carry the operator's intent, and each body carries the day
           the vendor it names can actually answer from. */
        from: floorFor(v, r.dir),
        body: wireBodyFor(r.dir, floorFor(v, r.dir), to, v)
      }))
    )
  );

  /**
   * EVERY LEG ONE PRESS SENDS — spot and expired-derivative together.
   *
   * # The disagreement this removes
   *
   * `runPull` built `[...wireBodies, ...fnoBodies]` inline while the feed picker
   * counted `wireBodies` ALONE. Measured on the running page with three segments
   * ticked: the strip read **`3 feeds · 6 request(s)`** and the press submitted
   * **nine**. The label, the counter and the run were three answers to one
   * question — the shape this product removes everywhere else, and the §4
   * failure of a surface reporting something other than what happens.
   *
   * Declared once so there is nothing left to keep in step: the counter and the
   * submission read the same expression, and a segment added later joins both or
   * neither.
   *
   * Spot first, then derivatives — but that is the order they are OFFERED, never
   * the order they are sent. `pull::fold`'s ladder is the server's to enforce and
   * `pullrun::by_feed` re-sorts each vendor's legs onto it.
   */
  /**
   * ONE LEG OF A RUN — the shape `runPull` takes and `chain` sends.
   *
   * WRITTEN DOWN BECAUSE THE INLINE VERSION WENT STALE. `runPull`'s `@param`
   * spelled four keys — `dir`, `label`, `body`, `vendor?` — while the two
   * builders emit six. The two it missed were `route`, which decides whether a
   * leg goes to `/pull/spot` or `/pull/fno`, and `from`. Because `route` is
   * read exactly once, behind a `?? '/pull/spot'` fallback, the gap surfaced as
   * a type error at the READER and looked like an optional field with a
   * sensible default rather than a required one the annotation had dropped.
   *
   * `vendor` AND `route` ARE OPTIONAL, AND THAT IS NOT A HEDGE. `pullRow` — the
   * per-row button on the census — deliberately sends neither: `wireBodyFor`
   * defaults the vendor to `feeds.active`, and a census row is a spot row, so
   * the `?? '/pull/spot'` fallback is its answer rather than its accident. Both
   * fallbacks are named at their own call sites. Typing these as required would
   * have made this file describe two of its three callers.
   *
   * `route: string` and not a two-literal union: an object literal in a `.map`
   * widens `'/pull/fno'` to `string` where it is built, so the union would
   * reject the very values it describes. The two routes are pinned by
   * `web/tests/proxy.test.js`, which diffs every URL this tree fetches against
   * `vite.config.js`'s list — a real check where this would be a decorative one.
   *
   * @typedef {{
   *   dir: string,
   *   label: string,
   *   body: string,
   *   vendor?: string,
   *   route?: string,
   *   from?: string
   * }} Leg
   */

  const allBodies = $derived([...wireBodies, ...fnoBodies]);

  /**
   * EVERY FEED WHOSE ASK WAS MOVED, AND THE DAY IT MOVED TO.
   *
   * `floorFor` narrows and never widens — it is a `max` of three bounds — so a
   * window the operator picks INSIDE every ticked feed's reach goes out exactly
   * as typed. One week, one day, one month: nothing here fires and nothing is
   * changed. This list is empty in that case, and the note it feeds does not
   * draw at all.
   *
   * It fills only when the From asked for is EARLIER than what a feed can
   * answer, which is the one case where the request that goes out is not the
   * request on screen. §4 does not allow that to be silent.
   */
  const narrowings = $derived(
    wireBodies
      .filter((b) => b.from !== from)
      .map((b) => ({
        feed: feedName(b.vendor),
        rung: RUNGS.find((r) => r.dir === b.dir)?.label ?? b.dir,
        at: b.from
      }))
  );

  /* THE HOLDINGS ARE RE-ASKED WHEN THE FEED LIST CHANGES, AND NEVER OTHERWISE.
     `$lib/feeds.svelte.js` already folded this at boot to pick a default feed,
     and `surveyStores` is a no-op whenever the answer for that list and
     generation is already held — so the common case costs nothing and this
     exists for the uncommon one: a feed appearing in `/feeds.json` after the
     page loaded, which would otherwise leave its lane reading "nothing held"
     against a store nobody had looked at. The same shape and the same reasoning
     as `/db`'s own call. */
  $effect(() => {
    if (feeds.all.length > 0) surveyStores(feeds.all);
  });

  /**
   * THE WINDOW YOU ASKED FOR, AND WHAT EACH LEG CAN ACTUALLY ANSWER OF IT.
   *
   * `narrowings` above states this in prose and only when it is bad news. The
   * sentence is accurate and it is also the hardest kind of fact to act on: it
   * names a date, and what the reader needs is a PROPORTION. "Dhan · 1 minute
   * from 21 Aug 2021" against a window opening 01 Jan 2015 is six and a half
   * years of nothing, and the sentence gives no sense of that at all.
   *
   * The coverage column on the census proves the same thing after the fact —
   * measured on the running page, the first two of four blocks read `out of
   * reach`. This draws it BEFORE the press, next to the two date fields that
   * decide it, which is where the decision is actually made.
   *
   * A LANE PER LEG, because that is the unit that goes on the wire. `wireBodies`
   * is feed x rung and each entry already carries the `from` the server will be
   * sent — `floorFor` decided it — so this reads the request rather than
   * recomputing it. Two views of one number cannot disagree if only one of them
   * does arithmetic.
   *
   * GOOD NEWS DRAWS TOO. A feed reaching the whole window gets a full lane and
   * says so, where `narrowings` stays silent. Silence reads as "not checked"
   * just as easily as "nothing wrong", and one of those is false.
   *
   * `Date.parse` on three ISO days: all three are UTC midnight, so the ratio is
   * exact and no zone enters it. The window is the denominator, never the
   * calendar.
   */
  const reachLanes = $derived.by(() => {
    if (!windowOk || !from || !to) return [];
    const opens = Date.parse(from);
    const span = Date.parse(to) - opens;
    if (!(span > 0)) return [];
    return wireBodies.map((leg) => {
      const lost = Math.max(0, Math.min(span, Date.parse(leg.from) - opens));
      const deadPct = (lost / span) * 100;
      // WHAT THIS FEED'S STORE ACTUALLY HOLDS, beside what it can answer for.
      // Reach is a CLAIM the descriptor makes; holding is a MEASUREMENT off
      // disk, and the two failure modes look nothing alike: a feed that reaches
      // the whole window and holds nothing has never been asked, while one that
      // cannot be read at all is broken and no pull will fix it. `survey` is
      // already folded per feed — see the import — so this is a Map lookup.
      const hold = survey.byFeed.get(leg.vendor) ?? null;
      return {
        key: `${leg.vendor}·${leg.dir}`,
        wire: leg.vendor,
        feed: feedName(leg.vendor),
        rung: RUNGS.find((r) => r.dir === leg.dir)?.label ?? leg.dir,
        at: leg.from,
        deadPct,
        livePct: 100 - deadPct,
        whole: lost === 0,
        bars: hold?.bars ?? 0,
        broken: hold?.error ?? null,
        held: hold !== null
      };
    });
  });

  // -------------------------------------------------------------- validation
  //
  // EACH ONE NAMES THE FIELD, THE VALUE AND THE RULE. "Invalid date" is a
  // refusal that tells the operator nothing; every message below carries the
  // number that failed and the number it failed against.

  const problems = $derived.by(() => {
    // THE BLOCKER NAMES THE CONTROL THAT IS ACTUALLY UNANSWERED.
    //
    // Below the cut the fields are not on the page at all, so every message
    // this function would otherwise produce points at a control the reader
    // cannot see: "From has no date" is true, useless, and sends them looking
    // for a date box that the segment above it retracted. One refusal, naming
    // the one rung that has to be answered next.
    if (gate) return [{ field: gate.id, why: gate.why }];
    const out = [];
    // THE TWO "not a real day" MESSAGES QUOTE THE RAW STRING ON PURPOSE. Every
    // other date below is rendered for a human, because a human reads it; these
    // two are naming the exact value that failed to parse, and a formatter
    // handed an unparseable value would either echo it anyway or replace it
    // with a dash. The reader has to see the characters that are actually in
    // the field, not a tidied version of them.
    // THE PAIR IS A TUPLE, NOT TWO STRINGS AND A THIRD. `field` keys `typedErr`
    // and is handed to `openCal`, both of which take the two names and no other
    // — so the literal type is carried through the loop rather than widened to
    // `string` and asserted back at each use.
    for (const [field, text, day] of /** @type {Array<['from' | 'to', string, string]>} */ ([
      ['from', fromText, fromDay],
      ['to', toText, toDay]
    ])) {
      const Name = field === 'from' ? 'From' : 'To';
      // A STRING THE PARSER ALREADY REFUSED IS QUOTED BACK IN ITS OWN WORDS.
      // `parseDay` names which of six ways it failed — an ambiguous 02/09/2024,
      // a month that is not a month, a 31st of a 30-day month — and repeating
      // "not a date" over the top of that would replace a specific answer with
      // a vague one.
      if (typedErr[field]) {
        out.push({ field, why: `${Name} reads "${text}". ${typedErr[field]}` });
      } else if (!text.trim() && !day) {
        out.push({
          field,
          why: `${Name} has no date. Type one — ${dayLabel(maxDay)} — or open its calendar with the ▦ button.`
        });
      } else if (!isValidIso(day)) {
        out.push({
          field,
          why: `${Name} reads "${text}", which is not a date this page can read. Write it as "${dayLabel(maxDay)}" or as "${maxDay}".`
        });
      }
    }
    if (isValidIso(fromDay) && isValidIso(toDay) && dayNum(toDay) < dayNum(fromDay)) {
      out.push({
        field: 'to',
        why: `To is ${dayLabel(toDay)}, which is ${n(dayNum(fromDay) - dayNum(toDay))} day(s) BEFORE From (${dayLabel(fromDay)}). A window runs forward — the two are the wrong way round.`,
        // THE FIX IS THE ONLY ONE THIS PAGE CAN BE SURE OF. Two valid days in
        // the wrong order have exactly one reading; every other refusal here
        // has to guess which end the operator meant, so only the ones with a
        // single answer carry a button.
        fix: {
          label: 'Swap them',
          run: () => {
            const [a, b] = [fromDay, toDay];
            setDay('from', b);
            setDay('to', a);
          }
        }
      });
    }
    for (const [field, value] of [['from', fromDay], ['to', toDay]]) {
      if (!isValidIso(value)) continue;
      const Name = field === 'from' ? 'From' : 'To';
      if (dayNum(value) > dayNum(maxDay)) {
        out.push({
          field,
          why: `${Name} is ${dayLabel(value)}, after ${dayLabel(maxDay)}. ${ceilReason}`,
          fix: {
            label: `Use ${dayLabel(maxDay)}`,
            run: () => setDay(/** @type {'from' | 'to'} */ (field), maxDay)
          }
        });
      }
      // THE FEED'S FLOOR IS THE ONLY THING THAT MAY REFUSE AN OLD DAY, and the
      // refusal NAMES IT: which feed, at which timeframe, and from when. "Out
      // of range" tells an operator nothing he can act on; this tells him to
      // move the date, the feed or the rung, and which one would be enough.
      // THE FEED'S FLOOR NO LONGER REFUSES THE WINDOW, at the operator's
      // instruction: one window is asked of every ticked feed, and each feed
      // reaches back a different distance, so a floor that blocks the FORM
      // makes the earliest-reaching feed unusable because a later-reaching one
      // is also ticked. The floor is a property of the ASK, so it is applied
      // where the ask is made.
      //
      // IT IS NOT SILENT, and that is the half CLAUDE.md §4 cares about. The
      // clause under the day window still states each ticked feed's floor by
      // name, with the date and the citation, so an operator asking for 2015
      // from a feed that answers from 2021 is told what he will get before he
      // presses. What changed is that it INFORMS instead of BLOCKING.
      if (dayNum(value) < dayNum(minDay)) {
        out.push({
          field,
          why: `${Name} is ${dayLabel(value)}, earlier than ${dayLabel(minDay)} — the floor the server's own picker offers (crates/api/src/calendar.rs).`,
          fix: {
            label: `Use ${dayLabel(minDay)}`,
            run: () => setDay(/** @type {'from' | 'to'} */ (field), minDay)
          }
        });
      }
    }
    /* THE CAP IS NO LONGER A REFUSAL, BECAUSE IT IS NO LONGER REACHABLE.
       This used to refuse the WINDOW when it exceeded `MAX_WINDOW_DAYS`, and
       that was right while the two date boxes were the thing that went on the
       wire. They are not: `floorFor` clamps every body to the latest of the
       operator's From, the feed's declared floor, and the cap itself — so
       `to - from` on any body sent is at most the cap BY CONSTRUCTION, and a
       refusal here would refuse a request this page will never send.
       Measured: with the fields on 01 Jan 2015 – 18 Aug 2026 the page blocked
       every Pull button over a 4,248-day window, while the body it would have
       posted for Dhan was 1,827 days and perfectly legal.
       The narrowing is NOT silent — see `narrowings` and the note under the day
       window, which names every feed whose ask was moved and the day it moved
       to. §4 wants the change stated, not the request refused. */
    if (isFolderFeed && !folder.trim()) {
      out.push({ field: 'folder', why: 'Folder is empty. An archive run reads CSV files from a directory you name.' });
    }
    return out;
  });

  const problemFor = $derived.by(() => {
    const m = new Map();
    for (const p of problems) if (!m.has(p.field)) m.set(p.field, p.why);
    return m;
  });

  /**
   * A DAY THAT WAS LEGAL WHEN IT WAS CHOSEN AND IS NOT ANY MORE.
   *
   * The floor belongs to the feed and the timeframe, and both are controls
   * ABOVE this one: ticking Groww and 1 minute puts a three-month floor in
   * front of a `from` date chosen half an hour ago, and a rolling floor does
   * the same thing overnight with nobody touching anything. The ceiling moves
   * on its own too, at midnight and at 15:30.
   *
   * NOTHING IS SNAPPED. Silently rewriting a window the operator chose is the
   * fallback §4 bans — the date stands, and this is what makes the refusal
   * VISIBLE without waiting for a submit. Field-level messages wait for one
   * because telling someone their date is missing before they opened the
   * calendar is nagging; a bound moving underneath a date they already set is
   * not nagging, it is news.
   */
  const staleWindow = $derived(
    (isValidIso(fromDay) && dayBlock(fromDay) !== null) ||
      (isValidIso(toDay) && dayBlock(toDay) !== null)
  );

  /**
   * Not refusals — cautions. The SERVER has the last word on every one of
   * these, and the request is still sent. They are here because an operator
   * who is about to spend thirty-two minutes should see the known blocker
   * before the clock starts, not on the receipt.
   */
  const cautions = $derived.by(() => {
    const out = [];
    // EVERY TICKED RUNG, NOT THE FIRST ONE. This read `rungsChosen[0]` while
    // the control below it takes N ticks, so a run with `1min` and `1day`
    // ticked described the minute rung and said nothing at all about the other
    // request it was about to send.
    //
    // THREE SENTENCES, BECAUSE THERE ARE THREE REFUSALS AND THEY ARE FIXED BY
    // THREE DIFFERENT THINGS. See `finestByWire` above for why one word
    // standing for all of them is the failure §4 names.
    const unstored = rungsChosen.filter((x) => !x.stored);
    if (unstored.length > 0) {
      const many = unstored.length > 1;
      out.push(
        // `phrase`, NOT `label` — see `RUNGS`. "The store has no directory for
        // ticks" is a sentence about the STORE, and the store has never heard
        // the word: what it has no directory for is one second.
        //
        // AND THE RUNG LIST IS THE STORE'S OWN, NOT TWO OF SEVEN. This read
        // "answers for 1 minute and 1 day only", which D-0132 made false —
        // `Timeframe::KNOWN` has held seven entries since D-0054 and
        // `store_timeframe` answers for five of them. The true fact for the one
        // rung that can reach this line is that nothing below a minute is filed.
        `The store has no directory for ${unstored.map((x) => x.phrase).join(', ')}. store::path::Timeframe::KNOWN ships 1min, 3min, 5min, 15min, 30min, 60min and 1day, and nothing below a minute, so ${many ? 'these rungs are' : 'this rung is'} refused at the WRITE boundary by pull::ingest::Plan::timeframe — expect a refusal rather than bars. That is a store-format limit and not a vendor one: the feed answers, and nothing here can file what it answers with. The fold that makes these same one-second records storable already exists (pull::fold, one-second buckets); it runs when this feed is asked for 1 minute.`
      );
    }
    const unfetched = rungsChosen.filter((x) => feedFetches(x.dir) === false);
    if (unfetched.length > 0 && active) {
      const many = unfetched.length > 1;
      out.push(
        // `phrase` again: what a VENDOR declares is a rung, never a button word.
        `${active.display} does not declare ${unfetched.map((x) => x.phrase).join(', ')}. crates/api's served() refuses a POST for a rung pull::vendor::Descriptor::granularities does not carry — by name, and before it reads a credential — because this feed's bars path is pinned to one rung and a request for another would come back with a DIFFERENT bar length filed under the one you asked for. ${many ? 'Those requests' : 'That request'} will be refused whole. Fixable, and not from here: record the endpoint in pull::vendor.`
      );
    }
    // ══ THE WINDOW AGAINST THE FILES THAT ARE ACTUALLY THERE ══
    //
    // A folder feed has no history floor to refuse an old day with — TrueData
    // and GDFL both declare `history: &[]`, because no vendor page states a
    // floor for a directory on this machine — so nothing above this line stops
    // a window that names days the operator never bought. The calendar says as
    // much and leaves the day pickable, correctly: an unbought day comes back
    // EMPTY, which is not a refusal and must not be drawn as one.
    //
    // What was missing is that it said so only in a popover, after the fact.
    // This is the same fact where a run is about to be started. It is a
    // MEASUREMENT, not a vendor claim: the span was read by walking the folder,
    // so comparing a window against it is arithmetic rather than an opinion.
    // AN EMPTY TICK LIST IS A CAUTION, not a banner of its own. It belongs
    // where the other standing facts about this run are, behind one count —
    // not stacked above the button as a second alert. It draws only when
    // nothing at all is ticked; a narrowed selection reaches the wire intact
    // and has nothing to warn about.
    if (askGap) {
      out.push(askGap);
    }
    if (windowVsFolder && active) {
      const w = windowVsFolder;
      const span = `${dayLabel(w.earliest)} – ${dayLabel(w.latest)}`;
      const at = w.path ? ` The folder is ${w.path}.` : '';
      out.push(
        w.shape === 'past'
          ? `Not one day of this window is in ${active.display}'s folder, which holds ${span} across ${n(w.files)} file(s). The read will not be refused — an archive answers with whatever files are present, and for these days that is none — so expect a clean run that lands ZERO bars. Move the window inside that span, or buy the months first.${at}`
          : `Part of this window is outside ${active.display}'s folder. It holds ${span} across ${n(w.files)} file(s), and the days either side of that hold no file at all: they come back empty rather than refused, so the run will be shorter than the window says and the census will read short without anything having failed.${at}`
      );
    }
    // WHAT A RECORD AT THE FINEST RUNG ACTUALLY IS, said where an operator is
    // about to ask for it. A one-second archive row is a CONFLATED SNAPSHOT and
    // the word "tick" never applies to it — the invoice says otherwise and the
    // invoice is not evidence.
    const snap = rungsChosen.filter((x) => {
      const v = floorVerdict(feeds.active ?? '', x.dir);
      return v.state === 'finest' && v.floor?.conflated === true;
    });
    const snapFloor = finestByWire.get(feeds.active ?? '');
    if (snap.length > 0 && active && snapFloor) {
      out.push(
        // THE VENDOR'S HALF IS THE VENDOR'S WORDS, AND THIS FILE HOLDS NONE OF
        // THEM. This sentence used to carry the measurement itself — "22,426
        // rows across 22,500 seconds", "three of them at TrueData, four at
        // GDFL" — which is a THIRD copy of the fact and the worst-placed one:
        // it was written for whichever archive was active and printed one
        // vendor's numbers under the other's name. `because` and `source` come
        // off the wire and say it per feed.
        //
        // What stays local is the FOLD, and it is not a vendor fact: it is what
        // this repository's reader does with a shared second — SK-08 in
        // `docs/04-invariants.md`. Nobody published it, so nothing on
        // `/feeds.json` could carry it.
        // `phrase`, AND THIS IS THE SENTENCE THE SPLIT WAS BUILT FOR. The
        // control above says TICKS, because that is what the operator bought and
        // what the vendor marked the files. This says what is inside them, and
        // it must not borrow the button word to do it — "ticks is TrueData's
        // finest rung and one record there is a conflated snapshot. It is not a
        // tick" is a sentence that argues with itself.
        `${snap.map((x) => x.phrase).join(', ')} is ${active.display}'s finest rung and one record there is a ${snapFloor.label ?? 'conflated snapshot'}. The archive is MARKED as ticks — the file name says so and the invoice says so — and it does not hold them; it is not a print stream and must never be read as one. ${snapFloor.because} — ${snapFloor.source} THE TIMESTAMP IS KEYED TO THE SECOND, so two rows sharing one second is EXPECTED INPUT and not corruption. They are folded into one record at that second: the first in file order is its open, the extremes are its high and low, the volumes sum, and the LAST in file order is its close — last wins for the price, and nothing is dropped or refused. File order is the only order there is, so nothing is sorted: a sort would invent a tiebreaker and quietly change which price became the open.`
      );
    }
    // THE FORM'S SHAPE IS UNDECIDED, AND THAT IS SAID BEFORE THE CLOCK STARTS.
    // Everything below this line — whether a credential is read, whether a
    // quota is charged, whether `folder` means anything — turns on a field this
    // server did not send. Named here as well as at the control, because this
    // is the list an operator reads last.
    if (sourceKindUnstated) {
      out.push(SOURCE_KIND_UNSTATED);
    }
    // THE ONE-INSTRUMENT CAUTION IS GONE, AND IT WAS A FALSE STATEMENT ABOUT
    // THIS REPOSITORY'S OWN CODE, RENDERED IN A YELLOW BOX.
    //
    // It read: "The broker path in this build addresses ONE instrument.
    // pull::vendor::HttpSpec carries no request-parameter map, so a target
    // naming a set has been refused..." Both clauses were untrue:
    // `HttpSpec::params` exists and both broker descriptors populate it, and
    // `api::server::broker_run` builds the chosen target's instrument list and
    // calls `broker_window` once per member. The guard it described was removed
    // in D-0136 for exactly that reason.
    //
    // Nothing replaces it. A caution has to name a thing that will happen, and
    // this one named a refusal that no longer exists. The counts beside the
    // universe control already say how many instruments a run will attempt, and
    // they are measured rather than asserted.
    // AND ONLY WHILE A RUN IS ACTUALLY GOING. This fired on an idle form,
    // where "live progress below" describes nothing that is happening and the
    // operator has not asked for anything yet. A caution about how a reading is
    // taken is worth saying at the moment the reading is being taken.
    if (rung !== '1min' && phase !== 'idle') {
      out.push(
        `Live progress below is measured from /store.json, which stamps every row "1m" regardless of rung. Growth shown while this run is going may belong to another rung — it cannot be separated from the wire.`
      );
    }
    // A FEED NOTHING HAS BEEN READ FOR CLAIMS NOTHING, AND THAT IS SAID OUT
    // LOUD — ONCE, AND ONLY WHERE IT CHANGES WHAT HAPPENS. An unstated floor is
    // not an unlimited one: this page refuses no day for such a feed, so an old
    // window WILL be sent and the vendor is the one that will refuse it.
    // Silence here would read as "reaches everywhere", which is the one thing
    // the page must not imply. Feeds whose floor IS stated say so at the day
    // they refuse and nowhere else — see the window rung.
    //
    // AND IT IS A REST SENTENCE. Every clause of it — a depth the VENDOR
    // states, an absence of evidence, "the vendor will answer for how far back
    // it actually goes" — is a fact about an endpoint somebody else operates.
    // ALL OF IT IS FALSE FOR A FOLDER FEED: there is no vendor to answer, no
    // claim to be absent, and the reach is not unknown at all. It is the files
    // present, it was read, and the arm below says what was found.
    if (
      active &&
      !isFolderFeed &&
      !feedFloor.known &&
      windowOk &&
      from < addDays(istToday, -365)
    ) {
      out.push(
        `No history depth is stated for ${active.display} at ${RUNGS.find((r) => r.dir === rung)?.phrase ?? rung}, so nothing here refuses an old day for it and this window reaches back past a year. That is an absence of evidence, not a reach: the vendor will answer for how far back it actually goes. A floor belongs on pull::vendor::Descriptor beside granularities, not in a browser.`
      );
    }
    // A FOLDER FEED'S REACH IS THE FILES, AND THE WINDOW IS CHECKED AGAINST
    // WHAT WAS READ — never against a floor, because there is none.
    //
    // The three answers stay three. An empty folder is not "no data yet" and
    // not a failure: it is a folder that is there and holds nothing, and the
    // thing to do about it is buy the month. A folder that could not be read is
    // a HALT and names the path, because the thing to do about that is fix the
    // path — a completely different hour of an operator's life.
    if (active && isFolderFeed && folderReach.state === 'halted') {
      out.push(
        `${active.display} is a folder feed, so its answerable range is the files present — and this folder could not be read. ${folderReach.body?.path ? `The path is ${folderReach.body.path}. ` : ''}${folderReach.why} Nothing here can say what it reaches until that is resolved, and this is a halt rather than an empty range: the two look identical in a bar count and mean opposite things.`
      );
    }
    if (active && isFolderFeed && folderReach.state === 'read') {
      const r = folderReach.body?.reach;
      const where = folderReach.body?.path ? ` The folder is ${folderReach.body.path}.` : '';
      if (r?.state === 'empty') {
        out.push(
          `${active.display}'s folder is there and holds no file this feed reads, so it answers for NO day at all.${where} That is a read answer and not a missing one — nothing is wrong with the path, and the files for the window you want have not been bought and put there yet. There is no history floor to consult: for a folder feed the reach IS the files.`
        );
      } else if (r?.state === 'blank') {
        out.push(
          `${n(r.files)} file(s) are in ${active.display}'s folder and not one of them carries a row, so it answers for no day.${where} They were delivered and they are blank — which is a different thing to do about than an empty folder, and the reason the two are not one sentence here.`
        );
      } else if (r?.state === 'days' && windowOk && (from < r.earliest || to > r.latest)) {
        out.push(
          `${active.display} answers for ${dayLabel(r.earliest)} – ${dayLabel(r.latest)}, read from the ${n(r.files)} file(s) present, and this window reaches outside that. The part outside will come back with nothing — not because a vendor refused it, but because those files are not in the folder.${where} Buy and drop the months you want; there is no request that can widen this.`
        );
      }
    }
    return out;
  });

  // ------------------------------------------------------------ the universe
  //
  // THE FEED IS THE ROOT OF THIS PAGE and the form says so in its first line:
  // everything below is the selected feed's answer. `/instruments.json` is
  // asked per feed and answers per feed — the two brokers do not list the same
  // instruments — so a count under a feed's name is only a count if the list it
  // was taken from is that feed's.

  /**
   * Whether the catalogue on hand answers for the feed on screen.
   *
   * `$lib/index.svelte.js` stamps every answer with the feed it was asked for.
   * Between a feed change and its reload, and forever after a reload that
   * failed, the rows belong to a DIFFERENT feed — and printing their count
   * beneath this feed's header is exactly the stale value `CLAUDE.md` forbids.
   */
  const catalogueIsThisFeed = $derived(catalogue.ready && catalogue.feed === feeds.active);

  /**
   * THE CATALOGUE'S ROWS, GIVEN THEIR SHAPE ONCE.
   *
   * `$lib/index.svelte.js` declares `catalogue` as `$state({ rows: [], … })`,
   * which infers `never[]` — so every read of `row.universes`, `row.key` or
   * `row.symbol` on this page is an error reported at the READER. That module
   * belongs to another surface and the shape is `/instruments.json`'s, not
   * this page's, so it is named here as `CatalogueRow` (read off the live
   * answer for Dhan) and applied at ONE place rather than at each of the six
   * that walk the list.
   *
   * `?? []` is kept verbatim from the sites it replaces: `rows` is always an
   * array today, and the coalesce is what those sites said, so it stays said.
   */
  const catRows = $derived(/** @type {CatalogueRow[]} */ (catalogue.rows ?? []));

  /** A feed's own name for itself, out of `/feeds.json`, never a wire string. */
  /** @param {string | null | undefined} wire */
  function feedName(wire) {
    return feeds.all.find((f) => f.wire === wire)?.display ?? wire ?? 'no feed';
  }

  /**
   * THE TOKENS ONE CATALOGUE ROW CARRIES, OUT OF THE FIELD THAT CARRIES THEM.
   *
   * `/instruments.json` sends TWO membership fields and they are not the same
   * fact. `crates/api/src/server.rs:813` emits both on every row:
   *
   *   · `universe`  — a `+`-joined STRING, deliberately frozen at the first
   *                   `LEGACY_UNIVERSE_TOKENS` = 3 entries of `UNIVERSE_TOKENS`
   *                   (`index`, `fno`, `ntm`). It is the COMPATIBILITY field,
   *                   never widened so an old reader keeps working, and it will
   *                   therefore never name a NIFTY tier however good the data.
   *   · `universes` — the FULL ARRAY, `["index","fno","ntm","n500","n200",
   *                   "n100","n50"]` in that declared order. D-0089 / D-0090.
   *
   * So the ARRAY is read when the row has one and the legacy STRING is split
   * when it does not. That is not a fallback that hides a failure: the string
   * is a complete, correct answer for the three tokens it names, and the four
   * it cannot name are refused BY NAME below rather than guessed at. Nothing
   * here ever synthesises a tier.
   *
   * A FUNCTION DECLARATION so it hoists above `namedByUniverse`, which is the
   * only other place a row's membership is read. One definition, because two
   * answers to "is this name in this set" is how the second one goes stale.
   *
   * The same pair of fields is read the same way on `/db`. This page asks the
   * question about a REQUEST and that page asks it about the DISK, so the two
   * cannot share a module without one of them importing the other's state, but
   * the field precedence is deliberately identical.
   */
  /** @param {CatalogueRow} row @returns {string[]} */
  function membershipTokens(row) {
    return Array.isArray(row?.universes)
      ? row.universes.map(String)
      : String(row?.universe ?? '')
          .split('+')
          .filter(Boolean);
  }

  /** The token a universe row is COUNTED from, whichever field names it. */
  /** @param {Universe | null | undefined} u @returns {string | null} */
  function membershipToken(u) {
    return u?.field ?? u?.token ?? null;
  }

  /**
   * The four tokens the LEGACY `universe` string is structurally unable to
   * name, whatever the data holds. `LEGACY_UNIVERSE_TOKENS = 3` in
   * `crates/api/src/server.rs` cuts `UNIVERSE_TOKENS` after `index`, `fno` and
   * `ntm`, so a row without a `universes` array cannot report these four even
   * when the instrument is in all of them.
   */
  const LEGACY_BLIND = new Set(['n50', 'n100', 'n200', 'n500']);

  /**
   * HOW MANY NAMES THIS FEED'S MASTER PUTS BEHIND EACH TOKEN — counted from the
   * rows on hand, never asserted about a binary this page cannot see.
   *
   * This replaced a one-row probe that asked only whether `rows[0].universes`
   * was an array. That answered "does this build emit the field" and was then
   * read as if it had answered "can this tier be pulled", which are different
   * questions with different fixes. A build can emit the field and a feed can
   * still list nothing behind a tier, and an operator told to "rebuild the API"
   * for an empty set is sent to fix the wrong system.
   *
   * EMPTY WHEN THE CATALOGUE IS NOT THIS FEED'S, and every reader treats that
   * as NOT MEASURED rather than as absent — `reachWhy` is the sentence for that
   * state. A tier is called empty only when a master that DOES answer for this
   * feed has been read and does not carry the token.
   *
   * ONE PASS over a list bounded at ~800 rows, recomputed only when the
   * catalogue changes, never per row and never inside a render loop.
   */
  const tokenCount = $derived.by(() => {
    const m = new Map();
    if (!catalogueIsThisFeed) return m;
    for (const row of catRows) {
      for (const t of membershipTokens(row)) m.set(t, (m.get(t) ?? 0) + 1);
    }
    return m;
  });

  /**
   * Whether the master on hand carries the ARRAY field at all — one probe, not
   * an assumption about a version.
   *
   * This is the fact that tells a MISSING FIELD from an EMPTY SET, and the two
   * are different findings with different fixes.
   */
  const carriesArray = $derived.by(() => {
    if (!catalogueIsThisFeed) return false;
    for (const r of catRows) if (Array.isArray(r?.universes)) return true;
    return false;
  });

  /**
   * How many names this feed's master puts in a universe, or `null` when that
   * cannot be measured yet. `null` is NOT zero and no caller may treat it as
   * zero: one means "nobody counted" and the other is a finding.
   */
  /** @param {Universe} u @returns {number | null} */
  function masterCount(u) {
    if (!catalogueIsThisFeed) return null;
    const rows = catRows;
    if (u.token === '*') return rows.length;
    if (u.id === 'swept') return rows.filter((r) => namedByUniverse(r, u)).length;
    const t = membershipToken(u);
    return t === null ? null : (tokenCount.get(t) ?? 0);
  }

  /**
   * Whether one catalogue row is named by a universe.
   *
   * ONE definition, read by the member list and by the not-on-this-feed list
   * alike. Two copies would be two answers to "is this name in this universe",
   * and the second one is always the one that goes stale.
   *
   * MEMBERSHIP IS READ THROUGH `membershipTokens` AND NOWHERE ELSE, so which
   * field answers is decided in exactly one place: the `universes` ARRAY when
   * the row carries it, the frozen `universe` STRING when it does not.
   *
   * The consequence worth stating: a ranked tier answers FALSE against an old
   * binary — the legacy string cannot spell `n50`, so nothing here can — and
   * the row that would have used the answer is refused BY NAME rather than
   * quietly returning an empty set. A tier is never synthesised: NIFTY Total
   * Market arrives in alphabetical order, and slicing its first fifty names
   * would print 360ONE as a NIFTY 50 constituent and call it a fact.
   */
  /** @param {CatalogueRow} row @param {Universe | null | undefined} u */
  function namedByUniverse(row, u) {
    if (!u) return false;
    if (u.id === 'swept') {
      // `CLAUDE.md` §1 names these two and no others.
      return row.key === 'NSE-NIFTY' || row.key === 'NSE-BANKNIFTY';
    }
    if (u.token === '*') return true;
    const want = membershipToken(u);
    return want !== null && membershipTokens(row).includes(want);
  }

  /** Which instruments the chosen universe names, out of THIS feed's catalogue. */
  const members = $derived.by(() => {
    if (!catalogueIsThisFeed) return [];
    return catRows.filter((r) => namedByUniverse(r, universeSpec));
  });

  /**
   * WHY A UNIVERSE CANNOT BE ASKED FOR, or `null` when it can. Every branch
   * names the Rust item that would have to change, because "not supported" is
   * a sentence that tells an operator nothing about who can fix it.
   *
   * THREE GAPS, MEASURED IN THE ORDER AN OPERATOR CAN ACT ON THEM.
   *
   *   1 THE SET IS EMPTY HERE. A master that answers for this feed was read and
   *     puts nothing behind the token. No wire can fix that and no rebuild can:
   *     a pull would ask a vendor for nothing. Stated FIRST, and it disables a
   *     row that the wire could otherwise spell — offering a target whose
   *     selection is empty is the fallback `CLAUDE.md` §4 bans.
   *   2 THE FIELD IS ABSENT. The rows carry no `universes` array, so a ranked
   *     tier cannot be counted at all. That is a SERVER BUILD, and the sentence
   *     says which file already has the code.
   *   3 THE WIRE CANNOT SPELL IT. `api::ingest::SpotTarget` has three variants.
   *     This is the gap the four NIFTY tiers are actually stuck behind on a
   *     current binary, and it is the one no browser change can close.
   *
   * A MEASURED ZERO IS A FINDING; `null` IS NOT. `masterCount` returns `null`
   * while no master answers for this feed, and that state falls through to the
   * wire sentence rather than accusing a feed of listing nothing — the ladder
   * above already refuses everything for the uncounted reason.
   */
  /** @param {Universe} u @returns {string | null} */
  function universeRefusal(u) {
    const t = membershipToken(u);
    const have = masterCount(u);
    const rows = catRows.length;
    const feed = feedName(feeds.active);

    // ---- 1 and 2: is this set answerable from THIS feed's master at all?
    if (t !== null && t !== '*' && have === 0) {
      return !carriesArray && LEGACY_BLIND.has(t)
        ? `${u.label} cannot be expressed by /instruments.json?feed=${feeds.active ?? ''} on the API this page is talking to. Its rows answer with the legacy "universe" string only, and crates/api/src/server.rs freezes that string at three tokens — index, fno and ntm — through LEGACY_UNIVERSE_TOKENS. The full "universes" array, which names n500, n200, n100 and n50, is emitted beside it by that same file (D-0089/D-0090): the source has this tier and the running binary predates it. Rebuild and restart the API. What will not happen is a tier synthesised here.`
        : `0 in this feed's master. ${feed} lists no instrument in ${u.label}, counted over all ${n(rows)} row(s) /instruments.json?feed=${feeds.active ?? ''} returned for it. The field that would name this set is present on those rows and not one carries it — an EMPTY SET, not a missing endpoint. Nothing needs rebuilding and there is nothing here to ask a vendor for.`;
    }

    // ---- 3: the set is answerable. Can a request name it?
    if (u.target !== null) return null;
    // THE SENTENCE NO LONGER COUNTS THE ENUM, AND THAT IS THE FIX.
    //
    // It read "api::ingest::SpotTarget spells swept, indices and equities
    // only" — a hand-maintained count that had been wrong since D-0105 took
    // the enum to seven, and wrong again at nine. This page cannot see the
    // enum, so any number it states is a number that goes stale silently.
    // `u.target === null` is the WHOLE condition, and it is the only thing
    // said here.
    const wire =
      'No target names this set, so a pull for it cannot be put on the wire. The set is counted here from the universes array this server sends; what is missing is a request that can ask for it.';
    if (t === null || !LEGACY_BLIND.has(t)) return wire;
    return `${wire} MEMBERSHIP IS NOT THE GAP: ${n(have ?? 0)} name(s) in ${u.label} are counted here for ${feed}, off the universes array this server does send (D-0089/D-0090). This row is waiting on a fourth api::ingest::SpotTarget variant, which is a docs/05-decisions.md entry and a crates/api change — nothing in the browser closes it.`;
  }

  /**
   * THE SETS NO REQUEST CAN NAME, with the reason each one carries.
   *
   * Derived rather than an `{@const}` in the markup, which Svelte 5 permits
   * only as the immediate child of a block — a `<div>` is not one, and the
   * build says so rather than rendering something half-right.
   *
   * It is REACTIVE and must stay so: `universeRefusal` reads the catalogue and
   * the master counts, so a set that is unreachable before `/instruments.json`
   * answers becomes reachable after it, and the drawer has to empty itself
   * when that happens.
   */
  const refusedUniverses = $derived(
    UNIVERSES.map((u) => ({ u, why: universeRefusal(u) })).filter((x) => x.why !== null)
  );

  /* THE DRAWER'S OWN THREE, moved down from beside `sweptIsChosen` so they sit
     below `refusedUniverses` rather than above it. Nothing about them changed;
     see the note at the old site for why the old order was legal and still
     wrong to read. */
  const uniTucked = $derived(refusedUniverses.map((x) => x.u));
  const uniOpen = $derived(uniTuck);
  const uniTuckLabel = $derived(
    uniTucked.length === 1
      ? `1 set ${sweptIsChosen ? 'no request can name' : 'more'}`
      : `${n(uniTucked.length)} set(s) not offered here`
  );

  /**
   * REACH, not intent. `target` is a choice and it survives everything; this is
   * how many names that choice can actually be asked for right now, and it is
   * the only figure any count on this page is allowed to be built from.
   */
  const reach = $derived(members.length);
  const reachKnown = $derived(catalogueIsThisFeed);
  /** Why nothing is counted, when nothing is counted. Never blank. */
  const reachWhy = $derived.by(() => {
    if (reachKnown) return null;
    if (catalogue.error) {
      return `/instruments.json could not be read, so no name is counted here: ${catalogue.error}`;
    }
    if (!catalogue.ready) return 'reading /instruments.json…';
    return `the instrument list on hand answers for ${feedName(catalogue.feed)}, not for ${feedName(feeds.active)} — nothing on this page is counted from another feed's master`;
  });

  /**
   * THE REFUSED ROWS, SAID ON THE PAGE RATHER THAN ONLY ON HOVER.
   *
   * `universeRefusal` above is the complete answer and it is a paragraph, which
   * is the right length for a tooltip and the wrong place for the only question
   * this rung actually gets asked: *why is NIFTY 50 grey?* A greyed button
   * whose reason is reachable only by pointing at it has not answered that, and
   * a keyboard reader never reaches it at all.
   *
   * TWO CAUSES, NEVER MERGED, because they have different owners and different
   * fixes:
   *
   *   · `unspellable` — the set IS counted here, off the `universes` ARRAY this
   *     server sends. Nothing is missing from the wire and no rebuild changes
   *     it: `api::ingest::SpotTarget` has three variants and none names this
   *     set. That is a `docs/05-decisions.md` entry and a `crates/api` change.
   *   · `empty` — a master that answers for THIS feed was read and puts nothing
   *     behind the token. No target and no rebuild helps, because there is
   *     nothing to ask a vendor for. `CLAUDE.md` §4: name the reason out loud,
   *     never offer an empty selection quietly.
   *
   * `null` WHILE NOTHING IS COUNTED. `masterCount` answers `null` then, and a
   * row with no count is not a row with a count of zero — printing it under
   * "0 in this feed's master" would invent a finding out of a loading state.
   * The counted line above already carries `reachWhy` for that state, and a
   * second copy of that sentence here is exactly the drift this file has paid
   * for before.
   *
   * EIGHT ROWS, ONE PASS, recomputed only when the catalogue or the feed
   * changes — `masterCount` reads the `tokenCount` map and never scans.
   */
  const refusalSummary = $derived.by(() => {
    if (!reachKnown) return null;
    const empty = [];
    const unspellable = [];
    for (const u of UNIVERSES) {
      if (universeRefusal(u) === null) continue;
      const have = masterCount(u);
      if (have === null) continue;
      if (have === 0) empty.push(u.label);
      else unspellable.push(`${u.label} (${n(have)})`);
    }
    if (empty.length === 0 && unspellable.length === 0) return null;
    return {
      empty: empty.join(', '),
      unspellable: unspellable.join(', '),
      anyEmpty: empty.length > 0,
      anyUnspellable: unspellable.length > 0
    };
  });

  /**
   * WHAT THE UNIVERSE CONTROL IS, AND WHY FOUR OF ITS ROWS ARE GREY, ON THE
   * CONTROL ITSELF.
   *
   * This was two paragraphs standing open under the rung. They are not
   * dropped: `universeRefusal(u)` is on each refused ROW in full, and the two
   * CAUSES — counted-but-unspellable, and empty-in-this-feed's-master — are
   * summarised here, on the thing they are about. They have different owners
   * and different fixes and are never merged: one is a
   * `docs/05-decisions.md` entry plus a `crates/api` change, the other is a
   * master with nothing behind the token and nothing to ask a vendor for.
   */
  const universeTitle = $derived.by(() => {
    const head = reachKnown
      ? `The set this feed is asked for. ${universeSpec.label} — ${n(reach)} name(s) reachable on ${feedName(feeds.active)}, pulls with target=${target}.`
      : `The set this feed is asked for. ${universeSpec.label} — not counted: ${reachWhy}`;
    if (!refusalSummary) return head;
    const un = refusalSummary.anyUnspellable
      ? ` Counted here, not requestable: ${refusalSummary.unspellable} — membership is measured off the universes array this server sends, and what is missing is a target that can ask for it. A rebuild does not close it: a docs/05-decisions.md entry and a crates/api change do.`
      : '';
    const em = refusalSummary.anyEmpty
      ? ` 0 in this feed's master: ${refusalSummary.empty} — ${feedName(feeds.active)} lists no instrument in these, counted over all ${n(catRows.length)} row(s) /instruments.json?feed=${feeds.active ?? ''} returned. An empty set, not a missing endpoint.`
      : '';
    return head + un + em;
  });

  /** The census spells an instrument `EXCHANGE-SEGMENT-SYMBOL`. */
  /** @param {CatalogueRow} row */
  function censusKey(row) {
    return `${row.exchange}-${row.segment}-${row.symbol}`;
  }

  // ------------------------------------------------- what this feed is MISSING
  //
  // A NAME THIS FEED CANNOT SERVE IS SHOWN AND MARKED, NEVER DROPPED. Hiding it
  // removes the only evidence the gap exists: `/instruments.json?feed=X` returns
  // the names X's master gave an id to, so a name X does not carry is simply
  // absent and the list looks complete.
  //
  // THE COMPLETE MEMBERSHIP IS NOT ON ANY ENDPOINT THIS BUILD SERVES. There is
  // no `GET /universe.json`, and `/instruments.json` has no all-feeds mode — it
  // reads `feed=` and defaults to one. So the widest membership the browser can
  // MEASURE is the union of every feed's master, one request each, and the page
  // says that rather than claiming an index's true constituent list.
  //
  // It is measured on request rather than on load because each of those
  // requests rebuilds the census server-side. A page that quietly fires one per
  // feed every time it opens is paying that cost for a panel nobody asked for.

  /**
   * `byKey` IS EVERY FEED'S MASTER MERGED, one entry per key, each carrying the
   * displays of the feeds that listed it. `feeds` is the only field this page
   * adds to a `/instruments.json` row.
   *
   * @type {{
   *   at: number,
   *   key: string,
   *   byKey: Map<string, CatalogueRow & { feeds: string[] }>,
   *   error: string | null,
   *   busy: boolean
   * }}
   */
  let roster = $state({ at: 0, key: '', byKey: new Map(), error: null, busy: false });

  /** Which feeds a roster reading would be ABOUT. A change makes it stale. */
  const rosterKey = $derived((feeds.all ?? []).map((f) => f.wire).join('|'));
  const rosterFresh = $derived(roster.at > 0 && roster.key === rosterKey);

  /**
   * WHY THE MEASUREMENT CANNOT BE TAKEN RIGHT NOW, or `null` when it can.
   *
   * Three separate conditions used to grey this one button and the label said
   * none of them, so a press that did nothing was indistinguishable from a
   * button that was simply decorative. The ladder above states its blocker by
   * name; a disabled control owes the reader the same sentence.
   *
   * Each branch names the condition AND what would settle it, because "not
   * counted yet" is only half an answer — the other half is which request has
   * to come back before the press is worth making.
   */
  const rosterBlock = $derived.by(() => {
    if (roster.busy) {
      return `a reading is already in flight — ${n((feeds.all ?? []).length)} request(s) out, one per feed. A second press would duplicate every one of them, and each rebuilds the census server-side.`;
    }
    if ((feeds.all ?? []).length === 0) {
      return feeds.error
        ? `/feeds.json could not be read, so there is no list of masters to take a union of: ${feeds.error}`
        : '/feeds.json named no feed at all. The union is one request per feed, and with none listed there is nothing to request and nothing to compare this feed against.';
    }
    if (!reachKnown) {
      // A difference needs BOTH sides. The union is the far side; this feed's
      // own names are the near one, and without them a subtraction would be
      // taken against nothing and print a total that looks like a measurement.
      return `${feedName(feeds.active)}'s own names are not counted yet, and this measurement is a difference that needs both sides — ${reachWhy} — so it settles when /instruments.json?feed=${feeds.active ?? ''} answers for the feed named in the first control of the strip. Until it does, a union measured here would be subtracted from nothing and the total would read like a measurement.`;
    }
    return null;
  });

  async function measureRoster() {
    if (roster.busy) return;
    const list = feeds.all ?? [];
    const key = rosterKey;
    roster = { ...roster, busy: true, error: null };
    try {
      const answers = await Promise.all(
        list.map((f) =>
          request(`/instruments.json?feed=${encodeURIComponent(f.wire)}`)
            .then((r) => {
              if (!r.ok) throw new Error(`${f.display}: /instruments.json answered HTTP ${r.status}`);
              return r.json();
            })
            .then((rows) => ({ feed: f, rows }))
        )
      );
      const byKey = new Map();
      for (const a of answers) {
        for (const row of a.rows) {
          let held = byKey.get(row.key);
          if (!held) byKey.set(row.key, (held = { ...row, feeds: [] }));
          held.feeds.push(a.feed.display);
        }
      }
      roster = { at: Date.now(), key, byKey, error: null, busy: false };
    } catch (why) {
      // NAMED, NOT SWALLOWED. A measurement that fails silently leaves a dash
      // that reads like "none missing", which is the opposite of the truth.
      roster = { ...roster, busy: false, error: String(why) };
    }
  }

  /**
   * Names in this universe that some feed lists and THIS one does not. `null`
   * until it has been measured — the difference between "none missing" and
   * "nobody counted" is the whole point, and one dash cannot say both.
   */
  const missingHere = $derived.by(() => {
    if (!rosterFresh || !reachKnown) return null;
    const here = new Set(members.map((m) => m.key));
    const out = [];
    for (const [key, row] of roster.byKey) {
      if (here.has(key)) continue;
      if (!namedByUniverse(row, universeSpec)) continue;
      out.push(row);
    }
    return out.sort((a, b) => a.symbol.localeCompare(b.symbol));
  });

  /** How wide the universe is across every feed — reach plus what reach misses. */
  const universeWidth = $derived(missingHere ? reach + missingHere.length : null);

  // ══════════════════════════ WHICH NAMES ARE TICKED ══════════════════════
  //
  // A COUNT IS NOT A CHOICE, and this rung used to be a count. It rendered two
  // summary lines — how many this feed reaches, how many another feed lists —
  // and nothing on it could be ticked. Its own label conceded it: "read, not
  // chosen".
  //
  // WHAT THE WIRE CARRIES AND WHAT THE PAGE MEASURES ARE THE SAME FACT, and
  // that is newer than most of the prose around it. `SpotRequest` carries
  // `members: HashSet<Symbol>`; `wireBodyFor` appends one `member=` per tick
  // for a strict subset and NOTHING when every name is ticked, because naming
  // all and naming none are the same ask and 750 query fields to say
  // "everything" is waste. A tick therefore decides both what this page counts
  // — the ask arithmetic, every row of the census, the outcome list — and what
  // the server actually walks.
  //
  // A NAME THIS FEED CANNOT SERVE IS DRAWN, REFUSED AND TOLD WHY — never
  // dropped. Dropping it teaches the reader the name does not exist, which is
  // the opposite of the finding.

  /**
   * WHAT IS **UN**TICKED, not what is ticked — and the inversion is the whole
   * design of this control.
   *
   * A NEW POOL OPENS FULLY TICKED, because that is what a pull actually asks
   * for: `target=` resolves to the whole set on the server. Holding the ticks
   * instead would need a seeding step, that step would need a "has the pool
   * changed" guard, and the first render before the guard ran would print an
   * ask of zero beside a request for everything. Holding the EXCLUSIONS has no
   * such moment: the default is the truth with no work done, a name added to
   * the master later is ticked like its neighbours, and an exclusion is keyed
   * by instrument so it survives a universe change and comes back with it.
   *
   * Keys are `row.key`, the master's own key, everywhere — in this Set, in the
   * `{#each}` key and in every comparison. The symbol is display text.
   */
  let insOff = $state(new Set());
  /** What is in the search box. Text, and it never becomes a selection. */
  let insQ = $state('');

  /** The names this feed carries, sorted. One order, and it is the symbol's. */
  const insPool = $derived(
    [...members].sort((a, b) => cmpStr(a.symbol, b.symbol) || cmpStr(a.key, b.key))
  );

  /** The ticked rows themselves, in pool order. Every count reads this. */
  const ticked = $derived(insPool.filter((m) => !insOff.has(m.key)));
  const insCount = $derived(ticked.length);

  /**
   * EVERY ROW THE MENU DRAWS — what this feed carries, and what it does not.
   *
   * The second group only exists once the union has been measured (it is one
   * request per feed and it is a press, not a page load). Until then the menu
   * says so rather than presenting this feed's list as the whole universe.
   */
  const insRows = $derived.by(() => {
    const rows = insPool.map((m) => ({
      key: m.key,
      sym: m.symbol,
      detail: m.kind ? String(m.kind).toLowerCase() : 'in this master',
      // `null` IS THE TICKABLE ROW AND A SENTENCE IS THE REFUSAL — the second
      // loop below pushes one. Typed here so the two arms of one list are one
      // shape rather than two the reader has to reconcile.
      off: /** @type {string | null} */ (null)
    }));
    for (const r of missingHere ?? []) {
      rows.push({
        key: r.key,
        sym: r.symbol,
        detail: `only on ${r.feeds.join(', ')}`,
        off:
          `${feedName(feeds.active)}'s master has no id for ${r.symbol}, so there is nothing to ask this feed for. ` +
          `/instruments.json?feed=${feeds.active ?? ''} did not return it; ${r.feeds.join(', ')} did. ` +
          `It is drawn rather than dropped because a name deleted from this list is a gap with no evidence left.`
      });
    }
    rows.sort((a, b) => cmpStr(a.sym, b.sym) || cmpStr(a.key, b.key));
    return rows;
  });

  /** What the filter currently shows. One definition, read by the list AND the
      bulk actions, so the button can never act on a set the eye cannot see. */
  const insShown = $derived.by(() => {
    const needle = insQ.trim().toUpperCase();
    if (!needle) return insRows;
    return insRows.filter(
      (r) =>
        r.key.toUpperCase().includes(needle) ||
        r.sym.toUpperCase().includes(needle) ||
        r.detail.toUpperCase().includes(needle)
    );
  });
  /** Of those, the ones a tick would mean something for. */
  const insSelectable = $derived(insShown.filter((r) => r.off === null));

  /** The control's own face: how many of how many, and it agrees with the ask. */
  const insSummary = $derived.by(() => {
    if (!reachKnown) return '—';
    if (insPool.length === 0) return 'Nothing in this universe';
    if (insCount === 0) return `None of ${n(insPool.length)} ticked`;
    if (insCount === insPool.length) return `All ${n(insPool.length)} ticked`;
    return `${n(insCount)} of ${n(insPool.length)} ticked`;
  });

  /**
   * WHY THE LIST IS EMPTY, WHEN IT IS EMPTY. Never a blank panel: §4 says an
   * empty list states its own reason, and the reason is always a control the
   * reader can reach.
   */
  const insEmptyWhy = $derived.by(() => {
    if (!reachKnown) return reachWhy;
    if (insPool.length === 0) {
      return `${feedName(feeds.active)} lists no instrument in ${universeSpec.label}. Choose another universe above, or a feed whose master carries these names — nothing here is synthesised.`;
    }
    if (insShown.length === 0) {
      return `Nothing in ${universeSpec.label} matches “${insQ.trim()}”. Clear the box to see all ${n(insRows.length)} row(s).`;
    }
    return null;
  });

  /** Sets are replaced, never mutated: an in-place add is not a state change. */
  /** @param {string} key */
  function toggleIns(key) {
    const next = new Set(insOff);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    insOff = next;
    cPage = 1;
  }
  /** Bulk actions act on WHAT IS SHOWN, so the filter narrows them too. */
  function insSelectShown() {
    const next = new Set(insOff);
    for (const r of insSelectable) next.delete(r.key);
    insOff = next;
    cPage = 1;
  }
  function insClearShown() {
    const next = new Set(insOff);
    for (const r of insSelectable) next.add(r.key);
    insOff = next;
    cPage = 1;
  }

  // --------------------------------------------------------------- the ladder
  //
  // WHICH CONTROL CAN BE ANSWERED RIGHT NOW.
  //
  // The form's first line states the cascade and the cascade is real: the feed
  // is the root, and each control below answers a question the one above it
  // asked. A control whose parent is unanswered has no question to answer, so
  // it IS NOT DRAWN.
  //
  // Disabling was the wrong shape. A greyed control still occupies the page,
  // still has to be read, and still invites a click that does nothing — five
  // controls at once say "here are five decisions" when only one can be made.
  // Hiding says "here is the next decision", which is what the cascade means.
  //
  // `cut` is what makes the chain CUMULATIVE. Every control here has a default —
  // `swept`, `spot`, `1min` — so without it a rung would appear merely because
  // its own value happened to be set, three rungs below an unanswered feed.

  /** What a segment tick can actually REACH: ticked AND served by this form. */
  /**
   * IS THIS SEGMENT SERVED **BY THE SELECTED FEED**, which is not a property of
   * the segment.
   *
   * `SEGMENTS[].served` is a static field and it was written for a BROKER: both
   * vendors purge their master on expiry, `core::vendor::decode_master_row`
   * declines every FUT row as `Skip::LiveContract`, and `POST /pull/fno`
   * answers 503 because expired F&O has no HTTP transport. All true, and all
   * true of a BROKER only.
   *
   * AN ARCHIVE IS THE OPPOSITE CASE. TrueData and GDFL are folders of CSVs the
   * operator BOUGHT, and expired futures and expired options are precisely what
   * those files contain — 642 futures CSVs and 11,490 option members measured
   * in one GDFL day. Refusing the segments an archive exists to hold was the
   * static field applied to the wrong kind of feed.
   *
   * `kind === 'folder'` is `SourceKind`'s own word off `/feeds.json`, the same
   * value `kind_label` and `verb` are read from — not a second guess at the
   * transport.
   */
  const isArchiveFeed = $derived(active?.kind === 'folder');
  /**
   * EVERY SEGMENT IS OFFERED ON EVERY FEED — the operator's decision, stated
   * three times and overruling the per-feed rule that stood here.
   *
   * WHAT THAT COSTS, RECORDED RATHER THAN DISCOVERED. On an ARCHIVE this is
   * simply correct: expired futures and options are what the bought CSVs hold.
   * On a BROKER it offers something the vendor cannot answer, and all three
   * reasons remain true — the master purges on expiry so every FUT row decodes
   * as `Skip::LiveContract`, `POST /pull/fno` answers 503 for want of an HTTP
   * transport, and `FnoRequest` names one contract rather than a set.
   *
   * So a broker request for an expired segment WILL be refused. It is refused
   * LOUDLY and by name — §4's requirement is that a failure is named, not that
   * it is made unreachable — and the reason still sits on the row.
   */
  /** @param {Segment} _s */
  const segServed = (_s) => true;
  const segmentsReached = $derived(SEGMENTS.filter((s) => segSet.has(s.key) && segServed(s)));
  const segmentsUnserved = $derived(SEGMENTS.filter((s) => !segServed(s)));
  /**
   * HOW MANY RUNGS ARE TICKED — and therefore how many requests this form
   * sends. `/pull/spot` carries a single `granularity` field, so the factor
   * here is also the number of POSTs, never a widening of one.
   */
  const rungCount = $derived(rungsChosen.length);

  const ladder = $derived.by(() => {
    const steps = [
      {
        id: 'universe',
        parentReady: Boolean(active) && !archiveHidden,
        why: 'Select a broker feed in the first control of this strip — the universe below is that feed’s answer.'
      },
      {
        id: 'instruments',
        parentReady: Boolean(target),
        why: 'Choose a universe first — the instruments are whichever names it covers.'
      },
      {
        id: 'segment',
        parentReady: reachKnown && reach > 0,
        why: reachKnown
          ? `${feedName(feeds.active)} lists no instrument in this universe, so there is nothing to segment. Choose another universe, or a feed whose master carries these names.`
          : `The instruments in this universe are not counted yet, so nothing below them can be answered: ${reachWhy}`
      },
      {
        id: 'rung',
        parentReady: segmentsReached.length > 0,
        why: 'Tick a segment this form can fill. With none ticked there is no series to choose a bar length for.'
      },
      {
        id: 'window',
        parentReady: rungCount > 0,
        why: 'Choose a bar length first — a window with no rung names no series.'
      }
    ];
    let cut = false;
    return steps.map((s) => {
      const show = !cut && s.parentReady;
      if (!show) cut = true;
      return { id: s.id, why: s.why, show };
    });
  });

  const shows = $derived(new Map(ladder.map((s) => [s.id, s.show])));
  /**
   * THE FIRST UNANSWERED RUNG, and the only control a refusal may name.
   *
   * A refusal that misnames its cause sends the reader to fix something that is
   * not broken: "From has no date" is true and useless when the date field is
   * not on screen because the segment above it was unticked.
   */
  const gate = $derived(ladder.find((s) => !s.show) ?? null);

  /**
   * Instrument-months the request would fill if every name answered in every
   * month file it touches. An ESTIMATE, and the only denominator available: the
   * server states no total.
   *
   * THE MONTH FILE IS THE UNIT `/store.json` COUNTS IN, so this is counted in
   * month files even though the window is chosen in days — a day window that
   * covers three sessions of November still touches November's file, and the
   * store reports that file whole. It over-states a partial month on purpose
   * rather than inventing a per-day denominator no endpoint states, and the
   * line beside it says it is an estimate.
   *
   * EVERY FACTOR IS A REACHABLE COUNT. Reading the universe's nominal size here —
   * or a member list belonging to another feed — is what prints "785
   * instruments" beneath a header that has just said nothing is counted.
   */
  const expectedUnits = $derived(
    reachKnown ? insCount * segmentsReached.length * rungCount * windowMonths.length : 0
  );

  /** The product, with the factors that make it. They multiply, in every state. */
  const productLine = $derived(
    `${reachKnown ? n(insCount) : '—'} instrument(s)` +
      ` × ${n(segmentsReached.length)} segment(s)` +
      ` × ${n(rungCount)} rung(s)` +
      ` × ${n(windowMonths.length)} month file(s)` +
      ` = ${reachKnown ? n(expectedUnits) : '—'}`
  );

  /**
   * THE DIFFERENCE BETWEEN WHAT IS TICKED AND WHAT THE WIRE ASKS FOR, in
   * words, whenever the two are not the same number.
   *
   * THIS LINE SAID THE OPPOSITE, AND IT WAS WRONG IN THE DIRECTION THAT COSTS.
   * It read "the REQUEST cannot be narrowed to them ... no member field, so
   * target=X asks the server for all N either way", and it drew whenever the
   * ticked count differed from the reach — which is exactly the case where the
   * request IS narrowed. The whole chain honours a tick and has for some time:
   * `wireBodyFor` appends one `member=` per ticked name, `parse_spot` collects
   * them into `SpotRequest.members` and REFUSES rather than skips one it cannot
   * read, and `broker_run` puts every candidate through
   * `asked.members.contains(..)` — one hash probe each. The page was telling the
   * operator their selection was being ignored while the server honoured it.
   *
   * WHAT IS STILL WORTH SAYING IS THE EMPTY CASE, and only that. Naming no
   * member asks for the SET — what `target=` means on its own, and what the
   * autopilot sends. The run is NOT gated on a tick: `ladder` gates on the
   * universe having names, not on one of them being chosen. So an operator who
   * unticks everything gets all of them, and an empty selection does not look
   * like "everything" on a form.
   */
  const askGap = $derived.by(() => {
    if (!reachKnown || insCount > 0) return null;
    return (
      `Nothing is ticked, and an empty selection is not an empty request: naming no member asks for the ` +
      `whole set, so target=${target} pulls all ${n(reach)} instrument(s). Tick the names you want and the ` +
      `request carries them — one member= per tick, and the server refuses a name it cannot read rather ` +
      `than dropping it.`
    );
  });

  /** What the feed reaches, said in one line beside the feed's own name. */
  const scopeNote = $derived.by(() => {
    if (!active) return 'no feed selected — nothing below can be answered';
    if (!reachKnown) return reachWhy;
    if (!rosterFresh) {
      return `carries ${n(reach)} name(s) in this universe · what it does not carry is not measured`;
    }
    const miss = missingHere?.length ?? 0;
    return miss === 0
      ? `carries all ${n(universeWidth ?? reach)} name(s) any feed lists for this universe`
      : `carries ${n(reach)} of ${n(universeWidth ?? reach)} — ${n(miss)} listed only by another feed`;
  });

  // ══════════════════════════════ THE CENSUS ═══════════════════════════════
  //
  // ONE ROW PER SERIES IN THE WINDOW — instrument × segment × timeframe — and
  // every number in it is MEASURED. There was no such surface on this page at
  // all: the form asked for a window and nothing on screen said what the store
  // already held for it, so "is this window done" could only be answered by
  // starting a pull and watching.
  //
  // WHERE EACH NUMBER COMES FROM, and none of them is a guess:
  //
  //   stored     `/store.json?feed=<wire>` — one row per instrument-month-rung,
  //              read once per feed and re-read after every run. A month with
  //              no row is NEVER PULLED, which is not the same as zero bars and
  //              is never printed as one.
  //   expected   the NSE calendar this page already carries, counted over the
  //              days INSIDE the window, times the rung's bars per session.
  //              A rung with no stated bars-per-session gets no expectation at
  //              all — see RUNGS.per.
  //   in flight  `/ingest/status.json` — the sweep's own ladder. It is the only
  //              evidence on this machine that an attempt is being retried, so
  //              `retrying` is drawn only where that route names the row.
  //   halted     the same route's `waiting_on[].halted`, per feed.
  //   reach      the feed's floor at the ticked rungs, which is already what
  //              refuses a day in the date control.

  /**
   * The store, keyed `instrument|timeframe|month` — O(1).
   *
   * # THIS PAGE HELD TWO INDEPENDENTLY-CLOCKED COPIES OF ONE CENSUS
   *
   * `/store.json` was fetched from SIX places across this product and TWO of
   * them were on this page: this one, once per feed, and the run watcher below,
   * every five seconds while a pull was running. Nothing reconciled them, so
   * the census table and the progress bay could state different totals for the
   * same disk at the same moment and both were right about their own snapshot.
   * `/db` was a third opinion on a seventh clock.
   *
   * `$lib/store.svelte.js` reads it ONCE per (feed, generation), builds
   * `byCell` in the same pass that builds every other page's shape, and stamps
   * the answer with the feed it answered for. This object is the four names
   * this page already used, pointed at that one reading — and the run watcher
   * below now reads the SAME one, so the two can no longer disagree.
   *
   * `bars` and `rows` are gated on the feed stamp rather than assumed: a
   * reading that answered for another feed would put one broker's held counts
   * against this broker's window, which is the §4 failure this page's whole
   * census exists to prevent.
   */
  const storeMine = $derived(store.feed !== null && store.feed === (feeds.active ?? ''));
  const storeRead = $derived({
    at: storeMine && store.state === 'ready' ? (store.at ?? 0) : 0,
    feed: store.feed ?? '',
    bars: storeMine ? store.byCell : new Map(),
    rows: storeMine ? store.rows.length : 0,
    // NAMED, NOT SWALLOWED. A census built on a failed read would print
    // "never pulled" on every row, which is a finding this page would have
    // invented out of a network error.
    error: store.error,
    busy: store.state === 'reading'
  });

  /**
   * BARS BY (UNDERLYING, SEGMENT, RUNG, MONTH) — the index the census was
   * missing, and the reason it reported the same number three times.
   *
   * `store.byCell` is keyed `instrument|tf|month` where `instrument` is the
   * WHOLE store key. The census probed it with the MEMBER's key —
   * `NSE-INDEX-BANKNIFTY` — inside a loop over the chosen segments, and the
   * segment never entered the probe. So Spot, Expired futures and Expired
   * options all read the spot row and printed identical counts. Measured on the
   * running store: 58,305 bars reported under all three at 1 minute, when the
   * store holds 58,305 spot bars, 0 futures and 26 option contracts.
   *
   * That is the failure wearing a success's clothes that §4 bans: a segment
   * with nothing in it reported a full one's holdings, so "do I hold expired
   * futures" answered yes off another segment's data.
   *
   * ONE PASS OVER THE CELLS, and the segment comes from the KEY's own tail
   * rather than from what was ticked. A contract segment sums across every
   * contract of that underlying, because one (name, segment, rung, month) row
   * on the census is many keys in the store — 26 of them here.
   */
  const barsBySeg = $derived.by(() => {
    /** @type {Map<string, number>} */
    const by = new Map();
    if (!storeMine) return by;
    for (const c of store.readable) {
      const p = parseKey(c.instrument);
      const seg = segmentOf(p);
      if (seg === null || p.underlying === null) continue;
      const k = `${p.underlying}|${seg}|${c.timeframe}|${c.month}`;
      by.set(k, (by.get(k) ?? 0) + c.rows);
    }
    return by;
  });

  /** The sweep's ladder — what is in flight, and whether a feed has halted. */
  /**
   * @type {{
   *   at: number,
   *   inFlight: PilotFlight | null,
   *   feeds: PilotFeed[],
   *   state: string,
   *   error: string | null,
   *   busy: boolean
   * }}
   */
  let pilot = $state({ at: 0, inFlight: null, feeds: [], state: '', error: null, busy: false });
  let pilotAsked = false;

  async function readPilot() {
    if (pilot.busy) return;
    pilot = { ...pilot, busy: true, error: null };
    try {
      const r = await request('/ingest/status.json');
      if (!r.ok) throw new Error(`/ingest/status.json answered HTTP ${r.status}`);
      const j = await r.json();
      pilot = {
        at: Date.now(),
        inFlight: j.in_flight ?? null,
        feeds: Array.isArray(j.waiting_on) ? j.waiting_on : [],
        state: String(j.state ?? ''),
        error: null,
        busy: false
      };
    } catch (why) {
      pilot = { ...pilot, busy: false, error: String(why) };
    }
  }

  /* THE SUBSCRIPTION. One line, and it replaces the fetch, the token, the "have
     I asked for this feed" latch and the retry rule — the shared reading owns
     all four, and a Refresh pressed anywhere (or the poll below) arrives here. */
  $effect(() => syncStore(feeds.active));
  $effect(() => {
    if (pilotAsked) return;
    pilotAsked = true;
    readPilot();
  });

  /** This feed's rung of the sweep ladder, or `null` when it names none. */
  const feedLadder = $derived(
    pilot.feeds.find((f) => f.vendor === feeds.active || f.feed === feeds.active) ?? null
  );
  /** The halt, or `null`. A halt is terminal: attempts have stopped. */
  const feedHalt = $derived(feedLadder && String(feedLadder.halted ?? '').trim() ? String(feedLadder.halted).trim() : null);

  /**
   * SESSIONS PER MONTH INSIDE THE WINDOW — counted once, over the window's own
   * days, and read per series afterwards. Walking the days per row would be
   * 750 × 3,653; this is 3,653 once and a Map probe per month after it.
   */
  const sessionsByMonth = $derived.by(() => {
    const m = new Map();
    if (!windowOk) return m;
    let d = from;
    while (d <= to) {
      if (isSession(d)) {
        const ym = d.slice(0, 7);
        m.set(ym, (m.get(ym) ?? 0) + 1);
      }
      d = addDays(d, 1);
    }
    return m;
  });

  /**
   * THE SEVEN VERDICTS, spelled once: [tone, label, what it means].
   *
   * Read for the chip, for the pill and for the sort ordinal, so a label edited
   * here moves its own rank with it rather than dropping silently to the bottom
   * of a table that still carries the old spelling.
   */
  /** @type {Record<Verdict, [string, string, string]>} */
  const VERDICT = {
    fail: [
      'down',
      'failed',
      'The sweep ladder for this feed has HALTED and this series still has month files that are empty or short. Attempts have stopped — the halt reason is on the strip above, and a retry on its own will not help.'
    ],
    never: [
      'warn',
      'never pulled',
      'At least one month file in this window has no entry in /store.json at all. Nothing has asked the vendor for it yet — a pull IS warranted and will very likely settle it.'
    ],
    short: [
      'warn',
      'short',
      'Every month file in this window has been written, and at least one holds fewer bars than the NSE calendar expects for the days inside the window.'
    ],
    retry: [
      'info',
      'retrying',
      '/ingest/status.json names this instrument in flight right now. Nothing is waiting on you; it settles into verified, short or failed on its own.'
    ],
    unknown: [
      'info',
      'no yardstick',
      'This timeframe has no bars-per-session this page can state, so what is stored cannot be compared with the calendar. The bars are counted; the verdict is withheld rather than guessed.'
    ],
    beyond: [
      'info',
      'out of reach',
      'Every month file in this window is older than this feed answers for at this timeframe. No attempt was made and none would help — move the from date, the feed or the timeframe.'
    ],
    ok: [
      'up',
      'verified',
      'Every month file in the window holds at least the bars the NSE calendar expects for the days inside it.'
    ]
  };
  /** Worst first, and the order is a claim about what the reader has to DO. */
  /** @type {Verdict[]} */
  const VORDER = ['fail', 'never', 'short', 'retry', 'unknown', 'beyond', 'ok'];
  const VRANK = new Map(VORDER.map((k, i) => [k, i]));

  /**
   * WHERE IN THE WINDOW THE GAPS ARE — four blocks, oldest on the left.
   *
   * `Months unproved` says 32/32 and `Verdict` says `never pulled`. Neither says
   * WHERE: a series missing its first six months and a series missing its last
   * six read identically in both columns, and they call for opposite actions —
   * one is a floor you have to move, the other is a pull that has not caught up.
   * This is the column that separates them, and it is the reference's own
   * (`web/design/ingest.html` draws four blocks per row).
   *
   * FOUR BUCKETS OVER THE ROW'S OWN MONTHS, not over a fixed calendar: the
   * window is whatever the two date fields hold, so quarters of THAT are what a
   * reader can act on. Fewer than four months gives fewer than four blocks
   * rather than padding with empties, because an empty block and an unsettled
   * one look alike at 9px and only one of them is a fact.
   *
   * THE WORST VERDICT IN EACH BUCKET, never the best or the most common. A block
   * showing the majority would hide the single missing month it exists to point
   * at, and `VORDER` is already worst-first so the comparison is one Map lookup.
   *
   * Cost: O(months) per row, and it is computed once per view in `coverage`
   * below rather than per render — the lookup at draw time is one `Map.get`.
   *
   * @param {CensusRow['months']} months
   * @returns {{ tone: string, label: string, from: string, to: string, n: number }[]}
   */
  function coverageOf(months) {
    if (months.length === 0) return [];
    const buckets = Math.min(4, months.length);
    const out = [];
    for (let i = 0; i < buckets; i += 1) {
      const slice = months.slice(
        Math.floor((i * months.length) / buckets),
        Math.floor(((i + 1) * months.length) / buckets)
      );
      let worst = slice[0].k;
      for (const m of slice) {
        if ((VRANK.get(m.k) ?? 99) < (VRANK.get(worst) ?? 99)) worst = m.k;
      }
      out.push({
        tone: VERDICT[worst][0],
        label: VERDICT[worst][1],
        from: slice[0].month,
        to: slice[slice.length - 1].month,
        n: slice.length
      });
    }
    return out;
  }

  /**
   * EVERY SERIES IN THE WINDOW, MEASURED. Ticked instruments × reached
   * segments × ticked rungs, each with its month files resolved against the
   * store, the calendar and the feed's floor.
   */
  const censusRows = $derived.by(() => {
    /** @type {CensusRow[]} */
    const out = [];
    if (!reachKnown || !windowOk || windowMonths.length === 0) return out;
    const flight = pilot.inFlight;
    for (const m of ticked) {
      const key = censusKey(m);
      const named =
        flight != null && (flight.instrument === key || flight.instrument === m.key || flight.instrument === m.symbol);
      for (const s of segmentsReached) {
        for (const r of rungsChosen) {
          const months = [];
          let got = 0;
          let exp = 0;
          let unproved = 0;
          let missing = 0;
          const seen = { fail: 0, never: 0, short: 0, retry: 0, unknown: 0, beyond: 0, ok: 0 };
          for (const ym of windowMonths) {
            const sessions = sessionsByMonth.get(ym) ?? 0;
            /* SPOT ONLY. A DERIVATIVE MONTH HAS NO DERIVABLE EXPECTATION.
             *
             * `sessions * r.per` is the NSE calendar times the bars one session
             * holds — the right number for ONE continuous series, and the wrong
             * one for a segment that is a SET of contracts. A month of expired
             * options is however many strikes traded across however many
             * expiries; nothing on this page knows that number, and nothing can
             * until the vendor's discovery call has run.
             *
             * Measured on the operator's own screen 2026-08-21: an
             * `Expired futures · 1 day` row reading `0 / 1,708` — the spot
             * session count of the window, presented as that row's denominator.
             * A reader takes `0 / 1,708` as "none of the 1,708 I should have",
             * when the truthful statement is "I do not know how many there are".
             * `CLAUDE.md` §4: degrade loudly and name the reason, never invent a
             * number that reads like a measurement.
             *
             * `null` is already the "no expectation" value this row understands
             * — `r.per === null` produces it for a rung with no per-session
             * count — and it renders as the unknown verdict rather than as a
             * shortfall. So the derivative segments take the path the page
             * already has for "not derivable" instead of a new one.
             *
             * The BARS STORED half is unaffected and stays exact: what is on
             * disk is known, and it is the half worth reading here. */
            const derivable = s.key === 'spot';
            const expect = r.per === null || !derivable ? null : sessions * r.per;
            // WITH `s.key` IN IT. Without the segment this probe answered
            // the same number for every segment in the loop.
            const held = barsBySeg.get(`${m.symbol}|${s.key}|${r.dir}|${ym}`);
            const bars = held === undefined ? null : held;
            // A MONTH THE FEED CANNOT REACH IS NOT A GAP. The floor is the
            // same one the date control refuses a day with.
            // ONE READ, ONE NAME. `feedFloor.at` is `string | null`, and
            // `Boolean(x) && … < x` reads the property TWICE — the checker
            // cannot know the second read is the same value, and on a `$derived`
            // getter that is not merely pedantry. The ternary is what
            // `Boolean(…) &&` already meant for a string: empty and null both
            // answer false, and nothing else changes.
            const floorAt = feedFloor.at;
            const beyond = floorAt ? monthLastDay(ym) < floorAt : false;
            /** @type {Verdict} */
            let k;
            if (beyond) k = 'beyond';
            else if (bars === null) k = 'never';
            else if (expect === null) k = 'unknown';
            else if (bars >= expect) k = 'ok';
            else k = 'short';
            got += bars ?? 0;
            exp += expect ?? 0;
            if (k !== 'ok' && k !== 'beyond') unproved += 1;
            if (k === 'never' || k === 'short') missing += 1;
            seen[k] += 1;
            months.push({ month: ym, sessions, exp: expect, got: bars, k });
          }
          // The series verdict, worst first, and every branch has evidence
          // behind it: a halt from the ladder route, a flight from the same
          // route, a missing row from the store, a floor from the feed.
          /** @type {Verdict} */
          let state;
          if (seen.beyond === months.length) state = 'beyond';
          else if (named) state = 'retry';
          else if (feedHalt && missing > 0) state = 'fail';
          else if (seen.never > 0) state = 'never';
          else if (seen.short > 0) state = 'short';
          else if (seen.unknown > 0) state = 'unknown';
          else state = 'ok';
          out.push({
            id: `${key}|${s.key}|${r.dir}`,
            // TWO KEYS, AND THEY ARE NOT THE SAME KEY. `key` is the STORE's
            // spelling — EXCHANGE-SEGMENT-SYMBOL, what /store.json files a row
            // under. `mkey` is the MASTER's — what /instruments.json returns and
            // what the tick list, the outcome list and `askedKeys` compare on.
            // Passing one where the other is expected matches nothing and
            // produces an empty answer that looks like a measurement.
            key,
            mkey: m.key,
            sym: m.symbol,
            kind: m.kind ?? '',
            seg: s.key,
            segLabel: s.label,
            tf: r.dir,
            tfLabel: r.label,
            per: r.per,
            months,
            got,
            exp,
            unproved,
            missing,
            state
          });
        }
      }
    }
    return out;
  });

  /** Sessions inside the window, from the same calendar every row is judged by. */
  const windowSessions = $derived([...sessionsByMonth.values()].reduce((a, b) => a + b, 0));
  /**
   * WHY THE CENSUS IS EMPTY, when it is empty — always a control the reader can
   * reach, in the order the cascade asks for them.
   */
  function censusBlocker() {
    if (gate) return [gate.id === 'universe' ? 'No feed selected' : 'Nothing to census yet', gate.why];
    if (storeRead.error) return ['The store could not be read', `Nothing below is measured, so nothing is drawn rather than drawing zeros: ${storeRead.error}`];
    if (storeRead.at === 0) return ['The store has not been read yet', `Reading /store.json?feed=${feeds.active ?? ''} — every count below is one row of that answer.`];
    if (insCount === 0) return ['No instrument is ticked', `Open the instruments control above and tick at least one of the ${n(insPool.length)} name(s) in ${universeSpec.label}.`];
    if (segmentsReached.length === 0) return ['No segment is ticked', 'Tick Spot — it is the one segment POST /pull/spot fills.'];
    if (rungsChosen.length === 0) return ['No timeframe is ticked', 'Tick at least one bar length. A window with no rung names no series.'];
    if (!windowOk) return ['No window', 'Both ends of the window need a date before anything can be counted against it.'];
    if (windowMonths.length === 0) return ['No month file in that window', `${dayLabel(from)} – ${dayLabel(to)} touches no month file at all.`];
    if (cQuery.trim())
      return [
        `Nothing matches “${cQuery.trim()}”`,
        `Not one of the ${n(censusNames)} name(s) in this window contains that anywhere in its trading symbol. ` +
          `Clear the box to see all ${n(censusRows.length)} series. A shorter fragment finds more — the match is on any part of a symbol, not just its start.`
      ];
    return ['Nothing matches', 'Every row was filtered out. Widen the window, or tick another instrument, segment or timeframe above.'];
  }

  // ---- sort, filter, page

  let cSort = $state({ key: 'sym', dir: 1 });
  let cPage = $state(1);
  /**
   * WHAT IS IN THE SEARCH BOX. Text, and it never becomes a request.
   *
   * IT NARROWS WHAT IS DRAWN AND DELIBERATELY NOT WHAT IS ASKED. Not because
   * the request cannot carry names — `SpotRequest.members` exists and the tick
   * list above already fills it — but because ONE control should decide what a
   * run covers. Two of them narrowing one ask is the defect `members` was
   * added to remove: the picker said `1 of 213 ticked`, the receipt said
   * `ASKED 1 instrument(s)` and the run pulled all 213.
   *
   * The pager keeps the unfiltered total beside the filtered one, so a reader
   * can never mistake a shorter table for a smaller ask.
   */
  let cQuery = $state('');

  /** @param {string} key */
  function sortCensus(key) {
    if (cSort.key === key) cSort = { key, dir: -cSort.dir };
    // A count sorts biggest-first on the first press; a name sorts A→Z.
    else cSort = { key, dir: key === 'sym' ? 1 : -1 };
    cPage = 1;
  }

  /**
   * THE INDEX, REBUILT WHEN THE ROWS CHANGE — which is a selection or a window
   * change, never a keystroke. It is built over DISTINCT SYMBOLS, so 750 names
   * at three segments and six rungs is 13,500 rows and still 750 entries.
   */
  const censusIndex = $derived(buildFind(censusRows, (r) => r.sym));
  /** One `Map.get`. An empty box is `censusRows` itself, by reference. */
  const censusFound = $derived(probeFind(censusIndex, censusRows, cQuery));
  /** How many NAMES are on offer. Never the row count: a name is drawn once
      per segment per rung, so 750 names can be 13,500 rows. */
  const censusNames = $derived(symbolCount(censusIndex));

  const censusSorted = $derived.by(() => {
    const rows = [...censusFound];
    const d = cSort.dir;
    const k = cSort.key;
    rows.sort((a, b) => {
      // EVERY COMPARATOR IS THREE-WAY and every one falls through to the same
      // tie-break, so the order is total and a redraw cannot reshuffle equals.
      let first = 0;
      if (k === 'sym') first = cmpStr(a.sym, b.sym);
      else if (k === 'seg') first = cmpStr(a.segLabel, b.segLabel);
      else if (k === 'tf') first = cmpStr(a.tfLabel, b.tfLabel);
      else if (k === 'got') first = cmpNum(a.got, b.got);
      else if (k === 'unproved') first = cmpNum(a.unproved, b.unproved);
      else if (k === 'state') first = cmpNum(VRANK.get(a.state) ?? VORDER.length, VRANK.get(b.state) ?? VORDER.length);
      // 'Next step' ranks by the same number the button prints, which is the
      // count of unsettled month files — not by the button's own label.
      else if (k === 'act') first = cmpNum(a.unproved, b.unproved);
      return first * d || cmpStr(a.sym, b.sym) || cmpStr(a.tf, b.tf) || cmpStr(a.seg, b.seg);
    });
    return rows;
  });

  /**
   * HOW MANY COLUMNS ARE DRAWN. Segment and Timeframe appear only when more
   * than one is ticked — a column of one repeated value is a sort control over
   * nothing.
   *
   * IT SAID THIS COUNT "CAN NEVER SPAN THE WRONG NUMBER" AND IT WAS SPANNING
   * THE WRONG NUMBER. The base was 5 against SIX unconditional `<th>`, because
   * `Attempts` was added to the header and this constant was not bumped with
   * it. Measured in the browser with one segment and one rung ticked: six
   * header cells, `colspan="5"` on the empty row. The refusal sentence was
   * laid out to the wrong width and the last column hung off the end of it,
   * which is how it was noticed at all.
   *
   * SO THE GUARANTEE IS WITHDRAWN AND THE COLUMNS ARE NAMED INSTEAD. This is a
   * hand-kept number, it has drifted once, and the only thing standing between
   * it and drifting again is that the six are written down here where the next
   * person adding a seventh will read them:
   *
   *   Instrument · Bars stored/expected · Months unproved · Verdict ·
   *   Attempts · Next step
   *
   * Add a column to the header, add it to that list and add one here.
   */
  const censusColCount = $derived(
    6 + (segmentsReached.length > 1 ? 1 : 0) + (rungsChosen.length > 1 ? 1 : 0)
  );

  const censusPages = $derived(Math.max(1, Math.ceil(censusSorted.length / PAGE_SIZE)));
  /** The page never survives past the end of a list that just got shorter. */
  const censusPage = $derived(Math.min(Math.max(1, cPage), censusPages));
  const censusSlice = $derived(censusSorted.slice((censusPage - 1) * PAGE_SIZE, censusPage * PAGE_SIZE));

  /**
   * THE SEVEN COUNTS, WORST FIRST — the one thing the table cannot say.
   *
   * A table answers "what is this series" one row at a time. It cannot answer
   * "how much of this window is settled", because that is a property of the
   * whole set and the reader is looking at 25 of 50. The pager already states
   * the arithmetic of the VIEW; this states the arithmetic of the ANSWER.
   *
   * COUNTED OVER `censusRows`, NOT OVER WHAT IS DRAWN, and that is the only
   * honest denominator. `censusSorted` is what survived the search box, and the
   * search box narrows what is DRAWN and deliberately not what was asked —
   * `.cfind`'s own title says so at length. A summary that shrank when you typed
   * would be answering a different question from the one above it, which is the
   * `1 of 213 ticked` / `ASKED 1` / pulled-213 shape this page already carries a
   * rule against.
   *
   * ONE PASS, AND THE ORDER IS `VORDER`'s. O(rows) to count and O(7) to render,
   * with no sort: `VORDER` is already worst-first, so walking it is the display
   * order. Nothing here re-derives a row — `state` was decided by `censusRows`.
   *
   * @type {{ k: Verdict, tone: string, label: string, why: string, n: number, pct: number }[]}
   */
  const verdictTally = $derived.by(() => {
    /** @type {Map<Verdict, number>} */
    const seen = new Map();
    for (const row of censusRows) seen.set(row.state, (seen.get(row.state) ?? 0) + 1);
    const total = censusRows.length;
    return VORDER.map((k) => {
      const [tone, label, why] = VERDICT[k];
      const n = seen.get(k) ?? 0;
      // A ZERO-WIDTH SEGMENT IS NOT DRAWN AT ALL rather than drawn at 0% — a
      // 0%-wide flex child still takes its border and its gap, so seven of them
      // would paint a row of hairlines for verdicts nothing is in.
      return { k, tone, label, why, n, pct: total > 0 ? (n / total) * 100 : 0 };
    }).filter((seg) => seg.n > 0);
  });

  /** How many series the tally covers — the partition's own denominator. */
  const verdictTotal = $derived(censusRows.length);

  /**
   * THE DRAWN PAGE'S COVERAGE, COMPUTED ONCE PER VIEW.
   *
   * Keyed by `id` so the table's read is one `Map.get` per row rather than a
   * walk of that row's months on every render — and it is built over
   * `censusSlice`, the 25 rows actually on screen, never over all of them.
   * Paging or sorting rebuilds it; a hover or an unrelated state change does
   * not. Bound: 25 rows x the months in the window, once.
   */
  const coverage = $derived(new Map(censusSlice.map((r) => [r.id, coverageOf(r.months)])));

  /**
   * IS EVERY SERIES SETTLED? `ok` and `beyond` are the two verdicts that need
   * no action — one is proved, the other cannot be reached at this feed and
   * timeframe and no attempt would help. Everything else is work.
   */
  const verdictSettled = $derived(
    verdictTally.reduce((sum, seg) => (seg.k === 'ok' || seg.k === 'beyond' ? sum + seg.n : sum), 0)
  );

  /** 1 … 7 pages in full; beyond that the ends, the middle and a gap. */
  /** @param {number} cur @param {number} total @returns {(number | string)[]} */
  function pageList(cur, total) {
    if (total <= 7) return Array.from({ length: total }, (_, i) => i + 1);
    const s = new Set([1, total, cur, cur - 1, cur + 1]);
    if (cur <= 3) [2, 3, 4].forEach((x) => s.add(x));
    if (cur >= total - 2) [total - 1, total - 2, total - 3].forEach((x) => s.add(x));
    const l = [...s].filter((p) => p >= 1 && p <= total).sort((a, b) => cmpNum(a, b));
    /** @type {(number | string)[]} */
    const out = [];
    l.forEach((p, i) => {
      if (i && p - l[i - 1] > 1) out.push('gap' + p);
      out.push(p);
    });
    return out;
  }

  /**
   * THE DAYS ONE ROW IS SHORT, clipped to the window the operator chose.
   *
   * MONTHS ARE WHAT THE STORE FILES; DAYS ARE WHAT THE WIRE CARRIES. The row
   * knows which month files are unsettled, and this turns that into the day
   * range `parse_spot` reads — never wider than the chosen window, and never
   * older than the first unsettled month.
   */
  /** @param {CensusRow} row */
  function shortSpan(row) {
    const open = row.months.filter((x) => x.k !== 'ok' && x.k !== 'beyond');
    if (open.length === 0) return null;
    const firstDay = `${open[0].month}-01`;
    const lastDay = monthLastDay(open[open.length - 1].month);
    return { from: firstDay > from ? firstDay : from, to: lastDay < to ? lastDay : to };
  }

  /**
   * PULL ONE ROW. The same wire shape the main submit sends — `target`,
   * `vendor`, `from`, `to`, `granularity`, the two days as ISO — narrowed to
   * the DAYS this row is short and to the row's own rung.
   *
   * IT CANNOT BE NARROWED TO THE INSTRUMENT AND THE BUTTON SAYS SO. There is no
   * member field on `api::ingest::SpotRequest`, so the server resolves
   * `target=` and answers for the whole set. What this button genuinely
   * narrows is the WINDOW and the RUNG, which is real work saved, and the row
   * is what the outcome list is then built for.
   */
  /** @param {CensusRow} row */
  async function pullRow(row) {
    if (phase === 'running') return;
    const span = shortSpan(row);
    if (!span) return;
    await runPull(
      [{ dir: row.tf, label: row.tfLabel, body: wireBodyFor(row.tf, span.from, span.to) }],
      // THE MASTER'S KEY, because that is what `classifyBuild` compares on.
      new Set([row.mkey])
    );
  }

  // -------------------------------------------------------------- the run
  //
  // `phase` is the whole state machine: idle → running → done. Nothing else
  // decides what the right-hand column shows.

  let phase = $state('idle');
  let startedAt = $state(0);
  let finishedAt = $state(0);
  /**
   * THE ONE THE CARD SHOWS. `rung` is added by `runPull` and is optional on the
   * type for the same reason it is absent on `null`: the shape a request comes
   * back with is the server's, and the rung it was sent for is this page's.
   *
   */
  // ═══ WHY THESE THREE CARRY THE TYPE ON THE VALUE AND NOT ON THE `let` ═══
  //
  // A `@type` above the declaration states what the variable MAY hold. It does
  // not state what it holds AT THIS POINT, and the checker tracks the second
  // one: a `let` whose initialiser is `null` is narrowed to `null` for every
  // read that the flow reaches without an assignment in between — which, at the
  // top level of a component, is every `$derived(expr)` written below it.
  // `baseline && live ? live.units : 0` then type-checks its own live branch
  // against `never` and reports six errors about a field that is plainly there.
  //
  // Putting the type on the VALUE makes the initial flow type the union itself,
  // so a `$derived` reading it sees what a reader sees. It matters for exactly
  // the ones read from a top-level `$derived(…)` — a `$derived.by(() => …)` and
  // the markup are function bodies, where the flow starts from the declared
  // type again — but the three that need it are written the same way, because a
  // rule that applies at some declarations and not others is one nobody can
  // apply.
  let receipt = $state(/** @type {(Receipt & { rung?: string }) | null} */ (null));
  /**
   * ONE RECEIPT PER REQUEST, because a run is now one request PER TICKED RUNG.
   * `receipt` stays the one the card renders — the first that came back bad, or
   * the last — and this is the list, so a run that answered three times is not
   * reported as if it answered once.
   */
  /** @type {(Receipt & { rung: string })[]} */
  let receipts = $state([]);
  /** How far through the requests this run is. Counted, never estimated. */
  let sent = $state({ done: 0, of: 0, label: '' });
  /**
   * WHICH KEYS THIS RUN WAS ASKED FOR. The outcome list is built for these and
   * no others: a row-level pull that reported on all 750 names would be
   * reporting a result about instruments nobody asked for in that press.
   */
  let askedKeys = $state(new Set());
  /** @type {string | null} */
  let netError = $state(null);
  let aborted = $state(false);
  /** @type {AbortController | null} */
  let controller = null;

  /** Where the store stood the moment the request left, and where it stands now. */
  let baseline = $state(/** @type {StoreFold | null} */ (null));
  let live = $state(/** @type {StoreFold | null} */ (null));
  /** @type {string | null} */
  let pollError = $state(null);
  let lastGrowthAt = $state(0);
  /** {t, units, rows} samples, newest last. Bounded — only the tail is kept. */
  /** @type {{ t: number, units: number, rows: number }[]} */
  let samples = $state([]);

  const elapsedMs = $derived(
    phase === 'running' ? nowMs - startedAt : finishedAt > startedAt ? finishedAt - startedAt : 0
  );

  const unitsDone = $derived(
    baseline && live ? Math.max(0, live.units - baseline.units) : 0
  );
  const rowsGained = $derived(
    baseline && live ? Math.max(0, live.rows - baseline.rows) : 0
  );
  const unitsLeft = $derived(Math.max(0, expectedUnits - (baseline?.units ?? 0) - unitsDone));
  const share = $derived(
    expectedUnits > 0 ? Math.min(1, (baseline ? baseline.units + unitsDone : 0) / expectedUnits) : 0
  );

  /**
   * Instrument-months per second, from the OBSERVED samples only. `null` until
   * two samples exist that actually differ — a rate computed from one point is
   * a number with no measurement behind it.
   */
  const rate = $derived.by(() => {
    if (samples.length < 2) return null;
    const first = samples[0];
    const last = samples[samples.length - 1];
    const secs = (last.t - first.t) / 1000;
    const gained = last.units - first.units;
    if (secs < 5 || gained <= 0) return null;
    return gained / secs;
  });

  const etaSecs = $derived(rate && unitsLeft > 0 ? Math.round(unitsLeft / rate) : null);

  /**
   * THE GOVERNOR IS NOT VISIBLE FROM HERE, and this is the closest honest thing
   * to seeing it. `crates/pull/src/rate.rs` runs an AIMD governor inside the
   * synchronous POST; nothing reports its state. What CAN be observed is that
   * the store has stopped growing while the request is still open — which is
   * what a waiting governor looks like from outside, and also what a slow
   * vendor looks like. It is named as an inference, not as a reading.
   */
  const stalledSecs = $derived(
    phase === 'running' && lastGrowthAt > 0 ? Math.round((nowMs - lastGrowthAt) / 1000) : 0
  );
  const stalled = $derived(phase === 'running' && stalledSecs >= 20);
  /**
   * WHETHER ANYTHING HAS ACTUALLY LANDED. Without this the panel read "Bars are
   * landing" three seconds into a run that had written nothing — a claim with
   * no measurement under it, which is the exact failure the rest of this page
   * is built to avoid.
   */
  const everGrew = $derived(rowsGained > 0 || unitsDone > 0);

  // ------------------------------------------------------------ store census

  /**
   * One reading of the store, narrowed to the window and reduced to two numbers
   * and a map. The array is ~266 KB at 3,353 rows today and grows toward the
   * ~93,776 the store is heading for, so nothing here holds on to it.
   *
   * IT IS NO LONGER A SEVENTH FETCH. `refreshStore` bumps the shared generation
   * and returns the ONE request that bump causes — the same answer the census
   * table thirty lines up is reading, so a run's baseline and the "held" column
   * beside it can no longer come from two different snapshots of one disk.
   * `foldMonths` walks only the months the window names, off the shared
   * `byMonth` index, and memoises on (read, months) so a re-render never
   * re-folds.
   */
  /**
   * THE ASK'S OWN POPULATION — the census keys and the rungs this request
   * reaches, and nothing else.
   *
   * # The 100% meter this removes
   *
   * `snapshot()` counted EVERY row in the window months — every instrument at
   * every rung — while `expectedUnits` counted only this request's reach
   * (instruments × segments × rungs × months). With anything else already held
   * in those months (the 750-name NIFTY Total Market backfill is the ordinary
   * case) the numerator was in the hundreds against a denominator of 2,
   * `Math.min(1, …)` pinned the bar at 100.0%, and `unitsLeft` clamped to 0 —
   * which silently removed the ETA line as well — with half the request still
   * on the wire. It took two pre-existing rows, not 750.
   *
   * Both readings are taken through this, so the numerator and the denominator
   * are over one population. What lands outside it is not discarded: the fold
   * counts it as `outside` and the card names it.
   */
  const askScope = $derived({
    instruments: new Set(ticked.map(censusKey)),
    rungs: new Set(rungsChosen.map((r) => r.dir))
  });

  /** @returns {Promise<StoreFold>} */
  async function snapshot() {
    await refreshStore();
    // REFUSED, NOT ZEROED. Every outcome on this page is a difference against
    // this reading; measuring against a failed one would credit the run with
    // whatever the store already held.
    if (store.state !== 'ready')
      throw new Error(store.error ?? '/store.json could not be read, and it named no reason');
    return foldMonths(windowMonths, askScope);
  }

  /**
   * The watcher. It is no longer a loop this page owns: `watchStore` is ONE
   * timer for the whole product, held at the finest period any page asked for
   * and released on cleanup, and it bumps the shared generation rather than
   * fetching — so the five-second reading a run is measured against IS the
   * reading every other surface is showing.
   *
   * The awaited poll still cannot stack requests on top of each other; that
   * rule moved into the shared clock with the timer.
   */
  /** @type {(() => void) | null} */
  let releaseWatch = null;
  /** The last landed read this page has already folded into `live`. */
  let seenRead = 0;

  $effect(() => {
    const reads = store.reads;
    const state = store.state;
    const why = store.error;
    untrack(() => {
      if (phase !== 'running') return;
      if (state === 'error') {
        // NAMED, NOT SWALLOWED. A watcher that quietly stops is a progress
        // display that silently freezes, which is the failure this page exists
        // to remove.
        pollError = why ?? '/store.json could not be read, and it named no reason';
        return;
      }
      if (state !== 'ready' || reads === seenRead) return;
      seenRead = reads;
      const shot = foldMonths(windowMonths, askScope);
      // THE READING'S OWN CLOCK, AND IT IS A NUMBER ON THIS LINE.
      //
      // `store.at` is `null` until a read SUCCEEDS and `Date.now()` from that
      // moment on — `$lib/store.svelte.js` sets it beside `state = 'ready'` and
      // clears it beside every other state. The guard four lines up returned
      // unless the state IS `ready`, so the null arm is unreachable here and
      // the assertion records that argument where a reader can check it. A
      // `?? 0` would be a different thing entirely: a default standing in for a
      // measurement, which is the §4 shape.
      const at = /** @type {number} */ (shot.at);
      pollError = null;
      if (!live || shot.units > live.units || shot.rows > live.rows) lastGrowthAt = at;
      live = shot;
      samples = [...samples, { t: at, units: shot.units, rows: shot.rows }].slice(-12);
    });
  });

  // ------------------------------------------------------------- the receipt

  /**
   * The server's own answer, read rather than reconstructed.
   *
   * `DOMParser` on `text/html` builds an INERT document: no browsing context,
   * so no script runs and no resource is fetched. Every value below comes out
   * as `textContent`, so a symbol carrying `&` — M&M, M&MFIN are real
   * instruments — is a string and never markup. Nothing on this page is ever
   * assigned through `innerHTML`.
   */
  /**
   * @param {string} html
   * @param {boolean} ok
   * @param {number} status
   * @param {string | null} ctype
   * @param {string | null} marker
   * @returns {Receipt}
   */
  function readReceipt(html, ok, status, ctype, marker) {
    // A PAGE IS NOT A RECEIPT. See `$lib/receipt.js`: the content-type is
    // tested before anything is believed, and the parsed document must carry
    // the verdict element every answer this route gives carries.
    const wrong = notAReceipt({ html, status, ctype, marker, hasVerdict: true });
    if (wrong) return wrong;
    const doc = new DOMParser().parseFromString(html, 'text/html');
    const facts = [];
    for (const tr of doc.querySelectorAll('table.kv tr')) {
      const k = tr.querySelector('th')?.textContent?.trim() ?? '';
      const v = tr.querySelector('td')?.textContent?.trim() ?? '';
      if (k) facts.push({ k, v });
    }
    const badge = doc.querySelector('.badge');
    // THE SECOND TEST. `wrong` above cleared the content-type; this one is the
    // document's own verdict element, and its absence is the answer that used
    // to render as a green OK.
    const noVerdict = notAReceipt({ html, status, ctype, marker, hasVerdict: Boolean(badge) });
    if (noVerdict) return noVerdict;
    const halt = doc.querySelector('.halt');
    const scope = halt?.querySelector('b')?.textContent?.trim() ?? '';
    let reason = halt?.textContent?.trim() ?? '';
    if (scope && reason.startsWith(scope)) reason = reason.slice(scope.length).trim();
    return {
      ok,
      status,
      verdict: badge?.textContent?.trim() || (ok ? 'OK' : `HTTP ${status}`),
      good: badge?.classList.contains('good') === true,
      scope,
      reason,
      facts,
      raw: html
    };
  }

  const factValue = $derived.by(() => {
    const m = new Map();
    for (const f of receipt?.facts ?? []) if (!m.has(f.k)) m.set(f.k, f.v);
    return m;
  });

  /** Every `Failed` row the receipt carried — the server sends at most five. */
  const namedFailures = $derived((receipt?.facts ?? []).filter((f) => f.k === 'Failed'));
  const failedCount = $derived.by(() => {
    const raw = factValue.get('Members failed');
    const parsed = Number(String(raw ?? '').replace(/[^0-9]/g, ''));
    return Number.isFinite(parsed) ? parsed : 0;
  });
  /**
   * THE TRUNCATION, STATED. `landed_answer` writes
   * `for f in done.failures.iter().take(5)`, so a run with 407 failures puts
   * five on the wire and the count beside them. The page can see the gap and
   * says so rather than presenting five as the whole story.
   */
  const hiddenFailures = $derived(Math.max(0, failedCount - namedFailures.length));

  // -------------------------------------------------------------- outcomes
  //
  // ONE ROW PER INSTRUMENT THE REQUEST NAMED, whatever happened to it. The
  // reasons come from the server's own words where it gave them and from the
  // measured census where it did not; nothing here is a guess dressed as a
  // verdict.

  /**
   * The six outcomes, spelled ONCE. Read for the label and read for the order,
   * so the two cannot disagree — a label edited here moves its own rank with it
   * rather than dropping silently to the bottom of a table that still carries
   * the old spelling.
   */
  const GROUP = {
    failed: 'Failed',
    refused: 'Refused',
    silent: 'No bars landed',
    outside: 'Outside the universe',
    held: 'Already held',
    stored: 'Stored'
  };

  /**
   * THE OUTCOME ORDER IS AN ORDINAL, NOT THE ALPHABET.
   *
   * Sorting by `group.localeCompare` ranked these six by the first letter of a
   * sentence written for a reader: `Already held` first, `Stored` before them
   * all if it had been spelled `Ok`, and `Failed` buried in the middle. That is
   * not an order any outcome has — it is an accident of English, and it changes
   * the moment a label is reworded, which is a silent reordering nobody would
   * connect to a copy edit.
   *
   * Most actionable first, and each rank is a claim about what the reader has
   * to DO:
   *
   *   0 Failed            the run named a reason, one per instrument — each is
   *                       its own fix, and they are the only rows carrying the
   *                       server's own words.
   *   1 Refused           one cause standing in front of the whole set. Fewer
   *                       rows to read than `Failed`, but fixing it unblocks
   *                       every one of them at once.
   *   2 No bars landed    nothing landed and NOBODY SAID WHY. Nothing to fix
   *                       yet, so it ranks under the two that name a cause —
   *                       but it has to be investigated, which puts it above
   *                       everything that needs no attention at all.
   *   3 Outside the universe  something wrote what this request did not ask for.
   *                       Not this run's failure; still evidence of a second
   *                       writer, and worth seeing before the rows that worked.
   *   4 Already held      the store had it and gained nothing. Correct, but not
   *                       a thing THIS run achieved, so it sits above the rows
   *                       that did.
   *   5 Stored            it worked. Nothing to do, so it sorts last.
   */
  const GROUP_ORDER = [
    GROUP.failed,
    GROUP.refused,
    GROUP.silent,
    GROUP.outside,
    GROUP.held,
    GROUP.stored
  ];
  const GROUP_RANK = new Map(GROUP_ORDER.map((label, i) => [label, i]));
  /**
   * An unranked label sorts AFTER every ranked one, never silently first. A
   * group this table has never heard of is a group that was added without a
   * decision about where it belongs, and burying it at the top would be the
   * fallback that hides the omission.
   */
  /** @param {string} label */
  function groupRank(label) {
    return GROUP_RANK.get(label) ?? GROUP_ORDER.length;
  }

  /** @type {OutcomeRow[]} */
  let outcomes = $state([]);
  /** The prefix index — 1..`MAX_PREFIX` characters to the rows that start with them.
   *  @type {Map<string, OutcomeRow[]>} */
  let outcomeIndex = new Map();

  function classifyBuild() {
    if (!baseline) {
      outcomes = [];
      outcomeIndex = new Map();
      return;
    }
    const after = live ?? baseline;
    const failedBySymbol = new Map();
    for (const f of namedFailures) {
      // `landed_answer` writes `{instrument} — {why}`.
      const split = f.v.split(' — ');
      const who = (split.shift() ?? '').trim();
      const why = split.join(' — ').trim() || f.v;
      if (who) failedBySymbol.set(who.toUpperCase(), why);
    }
    /**
     * A WHOLESALE REFUSAL AND A RUN THAT FAILED PART-WAY ARE NOT THE SAME
     * THING, and stamping the run's verdict onto every member conflated them:
     * a run that read 785 members and failed 407 came back with 745 rows all
     * labelled "Refused", quoting a sentence that was never about them.
     *
     * `Members read` is the discriminator. A refusal that never reached the
     * members — the parser's, or the broker path's before the socket — emits no
     * such fact, and only then does the run's own reason belong on every row.
     */
    const refused = factValue.get('Refused') ?? null;
    const reachedMembers = factValue.has('Members read');
    const runReason = reachedMembers
      ? refused
      : (refused ?? (receipt && !receipt.good ? receipt.reason : null));

    /** @type {OutcomeRow[]} */
    const rows = [];
    const seen = new Set();
    for (const m of members) {
      // ONLY WHAT THIS PRESS ASKED FOR. A row-level pull names one instrument
      // and the form names the ticked ones; reporting on the rest would print
      // "not accounted for" against names nobody asked about in this run.
      if (askedKeys.size > 0 && !askedKeys.has(m.key)) continue;
      const key = censusKey(m);
      seen.add(key);
      const before = baseline.byInstrument.get(key) ?? 0;
      const now = after.byInstrument.get(key) ?? 0;
      const gained = now - before;
      const named =
        failedBySymbol.get(m.symbol.toUpperCase()) ??
        failedBySymbol.get(m.key.toUpperCase()) ??
        failedBySymbol.get(key.toUpperCase());
      let group;
      let reason;
      let tone;
      if (named) {
        group = GROUP.failed;
        reason = named;
        tone = 'down';
      } else if (gained > 0) {
        group = GROUP.stored;
        reason = `${n(gained)} bar(s) landed across ${windowMonths.length} month(s) of the window`;
        tone = 'up';
      } else if (runReason) {
        group = GROUP.refused;
        reason = runReason;
        tone = 'down';
      } else if (before > 0) {
        group = GROUP.held;
        reason = `The store already carried ${n(before)} bar(s) for this window and gained none — nothing new was written and the run named no failure for it`;
        tone = 'info';
      } else {
        group = GROUP.silent;
        reason =
          hiddenFailures > 0
            ? `The store gained nothing for this instrument and no reason was sent for it. The run recorded ${n(failedCount)} failed member(s) and put only ${n(namedFailures.length)} reason(s) on the wire, so this may be one of the ${n(hiddenFailures)} whose reason the server truncated.`
            : 'The run reported no failure for this instrument and the store gained nothing for it. It was named by the universe and is not accounted for.';
        tone = 'warn';
      }
      rows.push({ key, symbol: m.symbol, kind: m.kind, gained, before, group, reason, tone });
    }

    // Anything that GREW but was not in the target set. Surprising, and worth
    // seeing: it means something wrote outside the request.
    for (const [key, now] of after.byInstrument) {
      if (seen.has(key)) continue;
      const before = baseline.byInstrument.get(key) ?? 0;
      if (now - before <= 0) continue;
      rows.push({
        key,
        symbol: key.split('-').slice(2).join('-') || key,
        kind: key.split('-')[1] ?? '',
        gained: now - before,
        before,
        group: GROUP.outside,
        reason: `${n(now - before)} bar(s) landed for an instrument this request did not name. Another run may be writing at the same time.`,
        tone: 'info'
      });
    }

    outcomes = rows;
    // O(1) PER KEYSTROKE, the same prefix index `$lib/index.svelte.js` uses.
    // Built once per run — O(n) and paid a single time — so the search box is a
    // Map probe rather than a scan of 750.
    /** @type {Map<string, OutcomeRow[]>} */
    const next = new Map();
    for (const row of rows) {
      const s = row.symbol.toUpperCase();
      for (let i = 1; i <= Math.min(MAX_PREFIX, s.length); i += 1) {
        const k = s.slice(0, i);
        let bucket = next.get(k);
        if (!bucket) next.set(k, (bucket = []));
        bucket.push(row);
      }
    }
    outcomeIndex = next;
  }

  /** Reasons, grouped and counted, most common first. */
  const groups = $derived.by(() => {
    const m = new Map();
    for (const row of outcomes) {
      let g = m.get(row.group);
      if (!g) m.set(row.group, (g = { group: row.group, tone: row.tone, count: 0, reasons: new Map() }));
      g.count += 1;
      g.reasons.set(row.reason, (g.reasons.get(row.reason) ?? 0) + 1);
    }
    return [...m.values()].sort((a, b) => b.count - a.count);
  });

  // ------------------------------------------------------- the verdict strip
  //
  // ═════════════ DID THE WINDOW HE ASKED FOR LAND? ═════════════
  //
  // One sentence at the head of the table, the non-zero counts beside it as
  // filters, and the one bulk action. A reader who stops at the strip has the
  // answer he came for; the pills are the exceptions to it and the rows are the
  // evidence.
  //
  // IT RECOMPUTES NOTHING. Every number here is read off `groups`, which is one
  // pass over `outcomes` — the same rows the table draws and the same
  // `row.group` its Verdict column renders. There is no second classifier and
  // no second definition of "landed", so the strip and the column cannot
  // disagree about a single instrument. The ONLY thing added is arithmetic on
  // counts that already exist.
  //
  // WORST FIRST, because the row reads left to right and the bucket that needs
  // him must not be last. `GROUP_ORDER` already states that order for the
  // table's sort; the pills read the same list rather than a second one.
  const pills = $derived(
    [...groups].sort((a, b) => groupRank(a.group) - groupRank(b.group))
  );
  /** Counts by group, off the same pass. `0` for a group no row is in. */
  const countOf = $derived(new Map(groups.map((g) => [g.group, g.count])));
  /** @param {string} label */
  const countIn = (label) => countOf.get(label) ?? 0;
  /**
   * The instruments this window did not land for: named failures, wholesale
   * refusals, and the ones nothing accounted for. `Already held` is not short —
   * the store has it — and `Outside the universe` was never asked for.
   */
  /**
   * THE ONE VERDICT, in the four states that answer four different questions.
   * Collapsing any two would answer a question nobody asked: "as complete as it
   * gets" is a SUCCESS and looks identical to a failure in any count that does
   * not separate it.
   */
  const landed = $derived.by(() => {
    const total = outcomes.length;
    if (total === 0) return null;
    const failed = countIn(GROUP.failed);
    const refused = countIn(GROUP.refused);
    const silent = countIn(GROUP.silent);
    const held = countIn(GROUP.held);
    const stored = countIn(GROUP.stored);
    const outside = countIn(GROUP.outside);
    const span = `${dayLabel(from)} – ${dayLabel(to)}`;
    const aside =
      outside > 0
        ? ` ${n(outside)} instrument(s) outside this request also gained bars — something else is writing.`
        : '';
    if (failed > 0) {
      return {
        tone: 'bad',
        title: `Not complete — ${n(failed)} of ${n(total)} instrument(s) failed and need you`,
        why: `The run named a reason for each of them and they are the only rows carrying the server's own words. A second pull on its own will not help: fix the cause on the row, then ask for ${span} again.${aside}`
      };
    }
    if (refused > 0) {
      return {
        tone: 'bad',
        title: `Not complete — the run refused ${n(refused)} of ${n(total)} instrument(s)`,
        why: `One cause is standing in front of the whole set, so there are fewer rows to read than there are instruments — and fixing it unblocks every one of them at once.${aside}`
      };
    }
    if (silent > 0) {
      return {
        tone: 'warn',
        title: `Not accounted for — ${n(silent)} of ${n(total)} instrument(s) gained nothing and named no reason`,
        why: `Nothing landed for them and nobody said why, so there is nothing to fix yet — there is something to investigate. ${hiddenFailures > 0 ? `The server truncated ${n(hiddenFailures)} reason(s), so some of these may have had one.` : 'The run reported no failure for any of them.'}${aside}`
      };
    }
    if (stored === 0) {
      return {
        tone: 'warn',
        title: `Nothing was written — the store already carried all ${n(total)}`,
        why: `Every instrument this request named already held bars for ${span} and gained none. Nothing failed and nothing is outstanding: asking again returns the same nothing.${aside}`
      };
    }
    return {
      tone: 'good',
      title:
        total === 1
          ? 'The window landed — the one instrument this request named is accounted for'
          : `The window landed — all ${n(total)} instruments this request named are accounted for`,
      why: `${n(stored)} gained bars across ${span}${held > 0 ? ` and ${n(held)} already held them` : ''}. Nothing failed, nothing was refused and nothing is unexplained.${aside}`
    };
  });
  /**
   * WHY THE ONE BULK ACTION IS DISABLED, or `null` when it can be pressed. One
   * source for the `disabled` attribute AND the sentence beside it, so a greyed
   * button whose stated reason has gone stale is not possible.
   */
  const rerunBlock = $derived.by(() => {
    if (phase === 'running') return 'a pull is already on the wire — this is one synchronous POST and no route amends it.';
    if (problems.length > 0) {
      return `the request above has ${n(problems.length)} thing(s) to fix first: ${problems[0].why}`;
    }
    return null;
  });
  /**
   * THE SAME REFUSAL, SHORT ENOUGH TO PUT ON THE PAGE.
   *
   * `rerunBlock` is a whole sentence and it was rendered THREE TIMES on one
   * screen — in the refusal list, beside this button, and again on the button's
   * own `title` — for a single mistyped date. Two of those copies were the same
   * words the reader had just read six inches higher, and the visible one ran
   * off the right edge of the card because the row it sits in does not wrap.
   *
   * So the sentence is kept where it is READ ONCE — the list above, and the
   * tooltip for the reader who is on the button rather than the list — and what
   * is drawn beside the control is the COUNT and the field, which is the part
   * that says whether this button is the thing to fix. Nothing is lost and
   * nothing is repeated: `rerunBlock` still carries every word.
   */
  /* REMOVED with the outcome card: its symbol filter (`q`), its group
     narrowing, its sort, and the virtualiser that drew it -- `vEl`, `vTop`,
     `vH`, `vFirst`, `vCount`, `vSlice`, `onScroll`, the ResizeObserver that
     measured the box and the effect that kept the scroll inside it. None of
     it had a reader once the card went, and a virtualiser with nothing to
     virtualise is the orphaned-machinery pattern this page has been
     shedding. */

  // ------------------------------------------------------------ the calendar
  //
  // ONE PANEL AT A TIME. `cal.field` is the latch — a single value, so two
  // panels cannot coexist to overlap each other.
  //
  // IT PICKS DAYS, BECAUSE THE WINDOW IS DAYS. A day grid is the honest shape
  // for a control whose value is a day: every day of the month visible, one
  // click each, the ones no bar can exist for drawn faint, and the ones a bound
  // refuses struck through with the reason on them — never a click that
  // silently does nothing.
  //
  // TWO SELECTS IN THE HEADER, NOT JUST ARROWS. Paging one month at a time is
  // how a reader ends up thirty clicks from where they meant to be; a month
  // select and a year select reach any day in the eleven-year span in two
  // picks, and the span is a data question rather than a layout one.
  //
  // Every mechanism the month grid carried is still here: roving focus, the
  // orphan reclaim, clamping the cursor to what can be committed, opening
  // upward when there is no room below, retracting with its rung, and the whole
  // keyboard.

  /**
   * WHICH FACE THE PANEL IS SHOWING, AND WHY IT IS A FACE RATHER THAN A MENU.
   *
   * The month and the year used to be two `Picker`s in the header. `Picker` is
   * the STRIP's control -- 42px rows, an 18px radio, a 380px scroll box, a
   * panel that hangs `position: absolute` off the button it belongs to. Inside
   * a 272px calendar header that panel has nowhere to hang but OVER THE GRID:
   * opening the year drew a twelve-row scrolling list of radio buttons across
   * the forty-two days it was supposed to be steering, so the reader lost sight
   * of the thing he was navigating at the exact moment he navigated it.
   *
   * A calendar does not need a menu for this. It needs the same box to show a
   * different face: days, then the twelve months, then the years, each one
   * REPLACING the grid in place and drilling back down to it. Nothing overlaps
   * anything, nothing scrolls, and the panel never changes size -- `.calbody`
   * carries the day grid's own height so the footer does not move under the
   * pointer when the face changes.
   *
   * `Picker` itself is untouched. It is still the right control for a rung in
   * the strip and it is shared with three other pages; what was wrong was
   * putting a rung control inside a popover.
   *
   * @type {{ field: 'from' | 'to' | null, cursor: string, view: string, up: boolean, pad: 'day' | 'month' | 'year' }}
   */
  let cal = $state({ field: null, cursor: '', view: '', up: false, pad: 'day' });
  /** @type {HTMLDivElement | null} */
  let calEl = $state(null);

  /** Tall enough for the header, six week rows and the footer. */
  const CAL_H = 340;
  /** The day grid is seven across, so Up/Down is a week and Left/Right a day. */
  const CAL_COLS = 7;
  /** NSE weeks read Monday-first. */
  const WEEK = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

  /** Which month the open panel is looking at — `YYYY-MM`, the view, not the value. */
  const calView = $derived(cal.view || maxMonth);
  const calYear = $derived(Number(calView.slice(0, 4)));
  const calMonthNo = $derived(Number(calView.slice(5)));
  const calYears = $derived.by(() => {
    const out = [];
    for (let y = FLOOR_YEAR; y <= parts(maxDay).y; y += 1) out.push(y);
    return out;
  });

  /**
   * The forty-two cells of the shown month, Monday-first, as ISO days. Six full
   * weeks always, so the panel does not change height as the reader pages
   * through it and the grid never reflows under the pointer.
   */
  const calDays = $derived.by(() => {
    const first = `${calView}-01`;
    // 0 = Sunday from the platform; the grid reads Monday-first.
    const lead = (weekdayOf(first) + 6) % 7;
    const out = [];
    for (let i = 0; i < 42; i += 1) out.push(addDays(first, i - lead));
    return out;
  });
  /**
   * Whether the month on screen holds ANY day this field can take. A month the
   * reader can reach but cannot pick from says so ONCE, in the footer, rather
   * than in forty-two identical tooltips he would have to hover to discover
   * that his click was wasted.
   */
  const nothingInView = $derived(
    calDays.every((d) => d.slice(0, 7) !== calView || dayBlock(d) !== null)
  );

  /* ---- the three faces -------------------------------------------------
     One caption, one pair of arrows, three bodies. Each face states its own
     step so the header is never a control whose meaning the reader has to
     infer from what is underneath it. */

  /** The twelve months of the year on screen, each carrying its own refusal. */
  const calMonths = $derived(
    MON.map((label, i) => {
      const ym = `${pad(calYear, 4)}-${pad(i + 1)}`;
      return { ym, label, off: ym < minMonth || ym > maxMonth };
    })
  );

  /** What the caption says, which is always the unit the arrows move. */
  const calCaption = $derived(
    cal.pad === 'day'
      ? `${MON[calMonthNo - 1]} ${calYear}`
      : cal.pad === 'month'
        ? String(calYear)
        : `${calYears[0]} – ${calYears[calYears.length - 1]}`
  );
  /** What the caption OPENS, named on its own tooltip rather than guessed at. */
  const calCaptionTitle = $derived(
    cal.pad === 'day'
      ? 'Pick a month instead of paging one at a time.'
      : cal.pad === 'month'
        ? 'Pick a year.'
        : 'Back to the days.'
  );
  const calBack = $derived(
    cal.pad === 'day'
      ? { on: calView > minMonth, why: 'Previous month' }
      : cal.pad === 'month'
        ? { on: calYear > calYears[0], why: 'Previous year' }
        : { on: false, why: 'Every year this field may take is on screen' }
  );
  const calFwd = $derived(
    cal.pad === 'day'
      ? { on: calView < maxMonth, why: 'Next month' }
      : cal.pad === 'month'
        ? { on: calYear < calYears[calYears.length - 1], why: 'Next year' }
        : { on: false, why: 'Every year this field may take is on screen' }
  );
  /** The arrows move the unit the caption names, whichever face is up.
   * @param {number} by */
  function calStep(by) {
    if (cal.pad === 'day') pickView(addMonths(calView, by));
    else if (cal.pad === 'month') pickView(clampView(`${pad(calYear + by, 4)}-${pad(calMonthNo)}`));
  }
  /* THE CAPTION DRILLS UP AND A CHOICE DRILLS BACK DOWN -- days to months to
     years, and a picked year lands on the months of that year rather than
     jumping straight back to a grid the reader has not chosen a month for. */
  function calZoom() {
    cal = { ...cal, pad: cal.pad === 'day' ? 'month' : cal.pad === 'month' ? 'year' : 'day' };
  }

  /** @param {'from' | 'to'} field */
  function openCal(field) {
    if (cal.field === field) {
      cal = { ...cal, field: null };
      return;
    }
    const current = field === 'from' ? fromDay : toDay;
    const seed = isValidIso(current) ? current : maxDay;
    // OPENS UPWARD WHEN THERE IS NO ROOM BELOW. A picker that drops off the
    // bottom of a scroll container is a picker the operator has to hunt for,
    // and this form sits in one.
    const box = document.getElementById(`cal-${field}`)?.getBoundingClientRect();
    const up = box ? box.bottom + CAL_H > window.innerHeight && box.top > CAL_H : false;
    // IT OPENS ON THE DAY THE FIELD HOLDS, EVEN WHEN THAT DAY IS NOW REFUSED.
    // The view follows the value and the cursor follows what can be committed,
    // which are two different questions: a `from` that fell below a floor when
    // the feed changed is exactly the day the reader is looking for, and
    // jumping him to the floor's month would hide the struck-through cell that
    // explains what happened.
    // ALWAYS ON THE DAYS. A panel that reopened on the month pad because that
    // is where it was left would answer a question the reader did not ask
    // twice in a row.
    cal = { field, cursor: clampDay(seed), view: clampView(seed.slice(0, 7)), up, pad: 'day' };
    // FOCUS HAS TO ENTER THE PANEL, ONCE. The keydown handler is on the panel,
    // so with focus left on the trigger every arrow key went to the page
    // instead: the picker opened and then ignored the keyboard entirely.
    enterOnOpen = true;
  }
  function shutCal(refocus = true) {
    const field = cal.field;
    cal = { ...cal, field: null };
    if (refocus && field) document.getElementById(`cal-${field}`)?.focus();
  }
  /**
   * THE ONE WRITER OF A DAY, and it writes BOTH halves.
   *
   * The text field and the ISO value are two views of one day, so nothing may
   * set one without the other — a label left behind by a commit is a field
   * reading `02 Sep 2024` over a window that starts in March, and the screen
   * would look right the whole time.
   */
  /** @param {'from' | 'to'} field @param {string} isoDay */
  function setDay(field, isoDay) {
    if (field === 'from') {
      fromDay = isoDay;
      fromText = isoDay ? dayLabel(isoDay) : '';
    } else {
      toDay = isoDay;
      toText = isoDay ? dayLabel(isoDay) : '';
    }
    // A VALUE THAT TOOK CLEARS THE COMPLAINT ABOUT THE ONE THAT DID NOT.
    typedErr = { ...typedErr, [field]: null };
  }
  /** @param {string} isoDay */
  function commit(isoDay) {
    if (dayBlock(isoDay)) return;
    // Same as typing one — see `windowTouched`.
    windowTouched = true;
    // THE LATCH HOLDS A FIELD WHENEVER THIS RUNS. `commit` is reachable from
    // the open panel and from nowhere else — `openCal` sets `cal.field` to one
    // of the two names before the panel mounts, and `shutCal` clears it
    // together with the button that calls this. The assertion records that
    // argument; widening `setDay` instead would give it a third case it has no
    // branch for, and `typedErr` no key to clear.
    setDay(/** @type {'from' | 'to'} */ (cal.field), isoDay);
    showProblems = true;
    shutCal();
  }
  /**
   * WHY A DAY CANNOT BE PICKED HERE, or `null` when it can.
   *
   * Forty-two buttons go grey with no word between them, and the bounds that
   * produced them are stated nowhere the reader is looking — the panel covers
   * the paragraph below it. This is the sentence, attached to the day it is
   * about, and it is the SAME rule `problems` states: both read `maxDay`,
   * `feedFloor` and `minDay` rather than restating a number that could drift.
   *
   * THE CEILING IS NAMED FIRST because it is the one bound that is the same for
   * every feed, and naming a vendor for a day that has not happened yet would
   * be absurd. The feed's floor comes before the page's own, because it is the
   * fact the reader can act on.
   */
  /** @param {string} isoDay */
  function dayBlock(isoDay) {
    if (dayNum(isoDay) > dayNum(maxDay)) {
      return `${dayLabel(isoDay)} is after ${dayLabel(maxDay)}. ${ceilReason}`;
    }
    /* THE FEED'S FLOOR NO LONGER BLOCKS THE DAY, at the operator's instruction:
       a window is picked ONCE and is asked of every ticked feed, so a floor
       that moves with the feed made the same calendar offer different days from
       one press to the next. Now the grid offers the whole span the server's
       picker offers and the floor is a matter for the PULL -- which already
       states it: `problems` still refuses a window that starts before the
       strictest ticked floor, by name and with the date to move to, so nothing
       under-pulls in silence. `CLAUDE.md` §4 wants the failure loud, not the
       control narrow. */
    if (dayNum(isoDay) < dayNum(minDay)) {
      return `${dayLabel(isoDay)} is before ${dayLabel(minDay)}, the oldest day the server's own picker offers (crates/api/src/calendar.rs).`;
    }
    return null;
  }
  /**
   * WHAT A TAKEABLE DAY IS, IN WORDS. Not a refusal — the tooltip of a day that
   * CAN be picked, saying whether it is a session at all and whether the
   * holiday table actually covers it. A weekend inside the window is not an
   * error and must not be drawn as one; it is a day that will return no bars,
   * and the reader is owed that before he counts a shortfall.
   */
  /** @param {string} isoDay */
  function dayNote(isoDay) {
    const ns = noSessionWhy(isoDay);
    const approx = holidaysKnownFor(isoDay)
      ? ''
      : ` · this page's NSE holiday table covers ${dayLabel(HOL_FROM)} – ${dayLabel(HOL_THRU)} only, so outside it a weekday is assumed to be a session`;
    const ceiling = isoDay === maxDay ? ` · the newest day that can be asked for. ${ceilReason}` : '';
    return `${dayLabel(isoDay)} · ${ns ?? 'trading session'}${approx}${ceiling}`;
  }
  /**
   * A PANEL LEFT OPEN UNDER A RETRACTED RUNG HAS TO CLOSE.
   *
   * Unticking the last segment takes the date fields off the page while their
   * calendar is still latched open. The panel unmounts with it, so nothing is
   * visibly wrong — until the rung comes back and the calendar springs open
   * over a control nobody clicked. `cal.field` is the latch, so releasing it is
   * the whole fix.
   */
  $effect(() => {
    if (!shows.get('window') && cal.field) cal = { ...cal, field: null };
  });
  /**
   * THE CURSOR NEVER LANDS ON A DAY THAT CANNOT BE COMMITTED.
   *
   * Not tidiness — a correctness fix. A day outside the bounds renders as a
   * `disabled` button, and a disabled button cannot take focus: one PageDown
   * past the ceiling dropped focus to `<body>`, after which the panel's keydown
   * handler never fired again and every subsequent key, Enter included, went
   * nowhere. Clamping keeps the cursor on a real target, so focus always has
   * somewhere to be.
   *
   * The FEED's floor clamps it too, and that is deliberate: a cursor sitting on
   * a day this feed refuses is a cursor Enter cannot commit.
   */
  /** @param {string} isoDay */
  function clampDay(isoDay) {
    if (!isValidIso(isoDay)) return maxDay;
    /* CLAMPED TO THE PAGE'S OWN FLOOR, NOT THE FEED'S -- see `dayBlock`. */
    const lo = minDay;
    if (dayNum(isoDay) < dayNum(lo)) return lo;
    if (dayNum(isoDay) > dayNum(maxDay)) return maxDay;
    return isoDay;
  }
  /** @param {string} isoDay */
  function placeCursor(isoDay) {
    const at = clampDay(isoDay);
    cal = { ...cal, cursor: at, view: at.slice(0, 7) };
  }
  /** @param {number} days */
  function moveCursor(days) {
    placeCursor(addDays(cal.cursor || maxDay, days));
  }
  /**
   * ═══════════ LOOKING IS UNBOUNDED; THE CURSOR IS NOT ═══════════
   *
   * The two header selects and the ‹ › steps move the VIEW, and the view is
   * clamped only to what this page can DRAW — 2015 to the ceiling's month. The
   * CURSOR is a different value with a different bound: it is the day Enter
   * would commit, so `clampDay` keeps it inside what can actually be taken.
   *
   * They were one value and that was a bug with a rule's voice: navigating to
   * 2015 under a feed whose floor is 2021 snapped the view straight back to
   * 2021, so a month the reader is entitled to LOOK at was unreachable and the
   * page never got to say why its days are struck through. He now gets the
   * month he asked for, forty-two refusals each naming the feed, and the
   * footer's one line saying the same thing once.
   */
  /** @param {string} ym */
  function clampView(ym) {
    if (ym < minMonth) return minMonth;
    if (ym > maxMonth) return maxMonth;
    return ym;
  }
  /** @param {string} ym */
  function pickView(ym) {
    cal = { ...cal, view: clampView(ym) };
  }
  /** @param {KeyboardEvent} e */
  function onCalKey(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      shutCal();
      return;
    }
    // THE MONTH AND YEAR PADS ARE NOT A GRID OF DAYS, so nothing below this
    // line applies to them. Every chip on those two faces is a real button in
    // the tab order — Tab walks them, Enter and Space press them — and the day
    // cursor must not move under a face that is not showing it.
    //
    // THIS GUARD USED TO NAME `<select>`, and it had been dead since the two
    // header selects became `Picker`s: `tagName === 'SELECT'` matched nothing,
    // so every arrow key aimed at the open month menu was preventDefault-ed
    // here and the menu could not be walked at all. A guard that names a
    // control the page no longer has is not a guard.
    if (cal.pad !== 'day') return;
    // KEYED BY THE KEY NAME, WHICH IS A STRING. The four names are the whole
    // table and `e.key in map` is the membership test right below it; declaring
    // the record is what lets the lookup on the next line be the same lookup
    // the test just made.
    /** @type {Record<string, number>} */
    const map = {
      ArrowLeft: -1,
      ArrowRight: 1,
      ArrowUp: -CAL_COLS,
      ArrowDown: CAL_COLS
    };
    if (e.key in map) {
      e.preventDefault();
      moveCursor(map[e.key]);
    } else if (e.key === 'PageUp' || e.key === 'PageDown') {
      e.preventDefault();
      const step = e.key === 'PageUp' ? -1 : 1;
      const cur = cal.cursor || maxDay;
      const ym = addMonths(cur.slice(0, 7), step);
      const [y, m] = ym.split('-').map(Number);
      placeCursor(iso(y, m, Math.min(Number(cur.slice(8)), daysInMonth(y, m))));
    } else if (e.key === 'Home') {
      e.preventDefault();
      placeCursor(`${calView}-01`);
    } else if (e.key === 'End') {
      e.preventDefault();
      placeCursor(monthLastDay(calView));
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      commit(cal.cursor);
    }
  }
  /**
   * Roving focus: the cursor day is the only day button in the tab order.
   *
   * It moves focus on OPEN — once, which is what makes the arrow keys reachable
   * at all — and after that only while focus is already on a day. Without that
   * second condition the nav arrows and the two selects would lose focus to the
   * grid the instant they were used.
   */
  let enterOnOpen = false;
  $effect(() => {
    if (!cal.field || !calEl) return;
    // `HTMLElement`, BECAUSE THE THING BEING LOOKED FOR IS A BUTTON.
    // `querySelector` promises only `Element`, which has no `focus` — and focus
    // is the entire reason this effect exists. Every `[data-day]` node in the
    // grid below is a `<button class="cday">`, so the narrowing is a statement
    // about this page's own markup and not about the DOM in general.
    const el = /** @type {HTMLElement | null} */ (
      calEl.querySelector(`[data-day="${cal.cursor}"]`)
    );
    // THE CURSOR IS NOT IN THE MONTH ON SCREEN — which happens the moment the
    // panel opens on a day a bound has since refused. Focus still has to enter
    // the panel or its keydown handler never fires and the picker is dead to
    // the keyboard; the panel itself carries `tabindex="-1"` for exactly this.
    if (!el && enterOnOpen) {
      enterOnOpen = false;
      calEl.focus();
      return;
    }
    if (!el || document.activeElement === el) return;
    const now = document.activeElement;
    const onADay = now?.classList?.contains('cday') ?? false;
    // ARROWING ACROSS A MONTH BOUNDARY DESTROYS THE FOCUSED BUTTON. The grid is
    // re-rendered for the new month, the node focus was on is gone, and the
    // browser drops focus to `body` — after which the panel's keydown handler
    // never fires again and the picker is dead to the keyboard from the second
    // Down arrow onward. An orphaned focus is therefore reclaimed, while a nav
    // button or a select — which survive the re-render — keep theirs.
    const orphaned = !now || now === document.body || now === calEl;
    if (enterOnOpen || onADay || orphaned) {
      enterOnOpen = false;
      el.focus();
    }
  });

  /**
   * TYPING IS THE OTHER WAY IN, and it never leaves a stale day behind.
   *
   * An unreadable string sets the parsed value to empty rather than keeping the
   * last good one: a field reading `02 Spe 2024` over a window still starting
   * in September would send a pull the operator can see they did not ask for,
   * and `problems` would have nothing to complain about. The refusal
   * `parseDay` wrote is kept beside the characters that earned it.
   */
  /** @param {'from' | 'to'} field @param {string} text */
  function typeDay(field, text) {
    // FROM HERE THE WINDOW IS HIS. See `windowTouched`.
    windowTouched = true;
    const r = parseDay(text);
    if (field === 'from') {
      fromText = text;
      fromDay = r.iso ?? '';
    } else {
      toText = text;
      toDay = r.iso ?? '';
    }
    typedErr = { ...typedErr, [field]: r.err ?? null };
  }

  // ------------------------------------------------------------------ submit

  /**
   * `e` is optional because there are two ways in and only one of them is a
   * form submit: the button at the foot of the request, and the strip's bulk
   * action at the head of the table. Both send the SAME request — there is only
   * one — so they call the same function rather than growing a second path that
   * could drift from it.
   */
  /**
   * THE ONE PLACE A PULL IS STARTED, and both callers reach it: the form's own
   * Start button with one body per ticked rung, and a census row's Pull with a
   * single body over the days that row is short.
   *
   * ONE REQUEST AT A TIME, IN ORDER. `/pull/spot` is synchronous and the store
   * appends — two in flight would interleave two vendors' writes into one
   * month file — so the rungs are sent one after another and the card counts
   * them.
   */
  /** @param {Event} [e] */
  /* THE THREE PASS CONSTANTS ARE GONE FROM THIS FILE, AND THAT IS THE POINT.
     `MAX_PASSES`, `CLEAN_EMPTY_PASSES` and `RETRY_WAIT_MS` described a loop
     this page ran. It runs in `crates/api/src/pullrun.rs` now, which holds all
     three with the reasoning that goes with them. Keeping declarations here
     would leave a second copy of a rule only one of them enforces — and the
     copy nobody remembers exists is always the one that drifts. */

  /** What the last multi-pass run did, for the card to state plainly. */
  let passSummary = $state(/** @type {string | null} */ (null));

  /**
   * THE ONE PRESS, AND THE SERVER OWNS EVERYTHING AFTER IT.
   *
   * # What this used to do, and why it stopped
   *
   * It used to BE the run: it fired every leg, re-read the census, decided
   * whether the window was satisfied, and fired the whole set again — up to
   * four hundred times. It was correct about what to retry and it put the
   * operator's backfill inside a browser tab.
   *
   * The symptom was a stop that looked like a restart. A leg answering 502
   * ended the pass; twenty seconds later the set went out again; the operator
   * watched `Pull running` turn into `NOT STARTED` and then, unasked, a fresh
   * run. Reported on 2026-08-20: *"why stopped and again auto repulled ... only
   * one pull from the webpage and automatically everything should be entirely
   * taken care internally"*.
   *
   * So the loop moved into `crates/api/src/pullrun.rs`. This function now sends
   * ONE request, which returns at once, and then WATCHES. Closing the tab no
   * longer stops the run; reopening it picks the run back up, because the state
   * lives on the server rather than in this closure.
   *
   * # The wire shape
   *
   * One `leg` field per leg, `route|vendor|dir|label|body`, with the body
   * encoded a SECOND time inside the field. Every form body contains `&` and
   * some contain `|`, so without the inner encoding the server could not tell a
   * separator from a payload.
   *
   * The parallel-across-feeds, sequential-within-one rule went with it: the
   * server groups these legs by vendor and spawns one chain each. It is not a
   * preference — a rate budget is per vendor, so two legs fired at one broker
   * together spend one ceiling twice, while two brokers share none.
   */
  /**
   * PICKS UP A RUN THAT WAS ALREADY GOING WHEN THIS PAGE LOADED.
   *
   * The run is a task on the server, not a closure in this tab, so closing the
   * page does not stop it — and opening the page must not pretend nothing is
   * happening. A tab that showed an idle form during a backfill would offer a
   * Pull button the server then refuses, and the operator would meet "a run is
   * already in flight" for a run this page gave no sign of.
   *
   * Quiet when there is nothing to pick up, which is the common case: a server
   * with no run answers a whole document saying so, and this returns without
   * touching anything the page shows.
   *
   * THE ELAPSED CLOCK COUNTS FROM HERE, not from when the run started. The
   * status document carries no start time, and inventing one would be a number
   * nobody measured; what the card shows after a resume is how long THIS TAB
   * has been watching.
   */
  async function resumeRun() {
    let doc;
    try {
      const r = await request('/pull/run.json', { ms: 15_000 });
      if (!r.ok) return;
      doc = await r.json();
    } catch {
      /* A status this page could not read is not a run this page may claim.
         The Pull button stays offered; the server refuses it by name if a run
         really is going, which is a better answer than a page that locked
         itself out on one failed read. */
      return;
    }
    if (doc?.running !== true) return;

    runState = doc;
    phase = 'running';
    startedAt = Date.now();
    lastGrowthAt = startedAt;
    finishedAt = 0;
    releaseWatch?.();
    seenRead = store.reads;
    releaseWatch = watchStore(5000);
    try {
      baseline = await snapshot();
      live = baseline;
    } catch {
      /* Without a baseline the card cannot show a difference, but the run is
         real and worth watching; the census table reads the store on its own
         clock either way. */
    }
    await watchRun();
  }

  /** @param {Event} [e] */
  async function start(e) {
    e?.preventDefault?.();
    showProblems = true;
    if (problems.length > 0 || phase === 'running') return;

    /* THE SAME LIST THE PAGE COUNTS, AND THAT IS THE WHOLE POINT.
     *
     * This built `[...wireBodies, ...fnoBodies]` inline while the feed picker's
     * summary counted `wireBodies` ALONE — so a run with three segments ticked
     * announced `3 feeds · 6 request(s)` and submitted NINE. The label, the
     * counter and the run were three answers to one question, which is the
     * shape `SpotTarget::names` was added to remove one layer down and the
     * §4 failure this page is otherwise careful about.
     *
     * `allBodies` is now the only expression that joins them, so a counter and
     * a submission cannot drift apart again — there is nothing left to keep in
     * step. Spot first, then derivatives: the server sorts each vendor's legs
     * onto `pull::fold`'s ladder itself, so this decides what is IN the run,
     * never the order they are sent. */
    const bodies = allBodies;
    if (bodies.length === 0) return;

    const form = bodies
      .map((b) =>
        `leg=${encodeURIComponent(
          [
            b.route ?? '/pull/spot',
            b.vendor ?? '',
            b.dir,
            b.label,
            encodeURIComponent(b.body)
          ].join('|')
        )}`
      )
      .join('&');

    receipt = null;
    receipts = [];
    outcomes = [];
    outcomeIndex = new Map();
    samples = [];
    netError = null;
    pollError = null;
    aborted = false;
    passSummary = null;
    runState = null;
    askedKeys = new Set(ticked.map((m) => m.key));
    sent = { done: 0, of: bodies.length, label: bodies[0].label };

    // THE BEFORE READING IS TAKEN FIRST AND IS NOT OPTIONAL. Every count the
    // card shows is a difference against it.
    try {
      baseline = await snapshot();
      live = baseline;
    } catch (why) {
      baseline = null;
      live = null;
      netError = `The store could not be read before starting, so nothing this run does could be measured against it: ${why}`;
      return;
    }

    startedAt = Date.now();
    lastGrowthAt = startedAt;
    finishedAt = 0;
    phase = 'running';
    releaseWatch?.();
    seenRead = store.reads;
    releaseWatch = watchStore(5000);
    controller = new AbortController();

    try {
      const r = await request('/pull/run', {
        ms: 30_000,
        method: 'POST',
        headers: { 'content-type': 'application/x-www-form-urlencoded' },
        body: form,
        signal: controller.signal
      });
      const answer = await r.json();
      if (!r.ok || answer?.started !== true) {
        /* REFUSED, AND THE SERVER SAID WHY. A refusal here is a decision — a
           run already in flight, or a leg that could not be read — not a
           transient failure to retry around, so it ends the press and says so
           rather than looping. */
        netError =
          answer?.why ??
          `The run was refused and gave no reason, which is itself the fault: HTTP ${r.status}.`;
        phase = 'done';
        finishedAt = Date.now();
        return;
      }
    } catch (why) {
      netError = `The run could not be started: ${why}. Nothing was asked of any vendor and nothing was written.`;
      phase = 'done';
      finishedAt = Date.now();
      return;
    }

    await watchRun();
  }

  /**
   * How often the page asks the server what its run is doing.
   *
   * Two seconds. The document is one lock take on the server and a few hundred
   * bytes on the wire, so this is cheap — and it is the ONLY thing this page
   * does while a run is on. It is not a retry: a poll that fails changes
   * nothing about the run.
   */
  const POLL_MS = 2000;

  /** The server's own account of the run, or `null` before the first answer. */
  let runState = $state(/** @type {any} */ (null));

  /**
   * WATCHING, NOT DRIVING.
   *
   * A failed poll is reported and the watch CONTINUES. This is the distinction
   * the old loop could not make: when the page WAS the run, a broken request
   * meant a broken pull. Now the run is a task on the server, so a dropped poll
   * means only that this tab lost sight of it for two seconds.
   *
   * It ends when the server says `running:false` and on nothing else — not on a
   * poll error, not on a timeout, not on the operator switching tabs.
   */
  async function watchRun() {
    for (;;) {
      await new Promise((done) => setTimeout(done, POLL_MS));
      if (controller?.signal.aborted && aborted) {
        /* The operator pressed Stop AND the server was told. The run winds down
           at its next leg boundary; the summary arrives on a later poll, so the
           watch keeps going rather than guessing at one here. */
      }
      let doc;
      try {
        const r = await request('/pull/run.json', { ms: 15_000 });
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        doc = await r.json();
      } catch (why) {
        pollError = `The run is still going; this page just could not read its status: ${why}. Retrying every ${POLL_MS / 1000}s.`;
        continue;
      }
      pollError = null;
      runState = doc;

      /* ANNOTATED BECAUSE THE DOCUMENT IS UNTYPED ON PURPOSE. `runState` holds
         whatever `/pull/run.json` sent; pinning a shape here would be a second
         declaration of it, and the server's is the one that is true. */
      /** @type {{ legs?: number, legsDone?: number, doing?: string, lastError?: string }[]} */
      const feeds = doc.feeds ?? [];
      const legs = feeds.reduce((sum, f) => sum + (f.legs ?? 0), 0);
      const done = feeds.reduce((sum, f) => sum + (f.legsDone ?? 0), 0);
      const doing = feeds.find((f) => f.doing)?.doing ?? '';
      sent = { done, of: legs, label: doing };

      if (doc.running === false) {
        passSummary = doc.finished ?? null;
        /* THE FIRST FEED THAT FAILED NAMES THE CAUSE. A run can finish having
           retried past a failure, so this is shown beside the summary rather
           than instead of it. */
        const hurt = feeds.find((f) => f.lastError);
        // `?? null` AND NOT A BARE ASSIGNMENT. `lastError` is optional on the
        // status document's feed row, so it reads `string | undefined`, while
        // `netError` is `string | null` — and the two absences are not
        // interchangeable here: every other writer of `netError` uses `null`,
        // and `{#if netError}` would treat either as false while a later
        // `netError === null` check would not. One spelling of "no error".
        if (hurt) netError = hurt.lastError ?? null;
        phase = 'done';
        finishedAt = Date.now();
        try {
          live = await snapshot();
        } catch {
          /* The census refreshes on its own five-second clock; a failed read
             here is not worth overwriting the summary for. */
        }
        return;
      }
    }
  }

  /**
   * THE PARAMETER TYPE WAS WRITTEN BY HAND AND THEN A FIELD WAS ADDED.
   *
   * This read `{ dir, label, body, vendor? }[]` — four keys, spelled out inline
   * — while both builders emit six, and the two it missed are the two that
   * decide where a leg goes and how far back it reaches: `route` and `from`.
   * `route` is read once, at `request(b.route ?? '/pull/spot', …)`, so the
   * checker reported the gap there rather than here, and the fallback made it
   * look like an optional field with a sensible default instead of a required
   * one the type had dropped. Annotating the call site, the map, and the
   * derivation itself each moved the error to the next line that reads `route`;
   * this is where it actually came from.
   *
   * `Leg` is the shape both builders now assert, so the four keys cannot go out
   * of step with the six again.
   *
   * @param {Leg[]} bodies
   * @param {Set<string>} asked
   */
  async function runPull(bodies, asked) {
    if (bodies.length === 0 || phase === 'running') return;

    receipt = null;
    receipts = [];
    askedKeys = asked;
    sent = { done: 0, of: bodies.length, label: bodies[0].label };
    netError = null;
    pollError = null;
    aborted = false;
    outcomes = [];
    outcomeIndex = new Map();
    samples = [];

    // THE BEFORE READING IS TAKEN FIRST AND IS NOT OPTIONAL. Every outcome
    // below is a difference against it; without one there is nothing to
    // subtract and the page would have to guess.
    try {
      baseline = await snapshot();
      live = baseline;
    } catch (why) {
      baseline = null;
      live = null;
      netError = `The store could not be read before starting, so nothing this run does could be measured against it: ${why}`;
      return;
    }

    startedAt = Date.now();
    lastGrowthAt = startedAt;
    finishedAt = 0;
    phase = 'running';
    // HELD, NOT OWNED. One five-second clock for the whole product, released
    // the moment this run stops needing it.
    releaseWatch?.();
    seenRead = store.reads;
    releaseWatch = watchStore(5000);

    controller = new AbortController();
    // CAPTURED ONCE, AND THAT IS WHAT MAKES IT TYPE. `controller` is a
    // module-scope `AbortController | null` that teardown sets back to null, and
    // TypeScript cannot narrow such a binding across the awaits in the chains
    // below -- so every `controller.signal` in them read as possibly-null and
    // `svelte-check` was right to say so. The signal is the only thing those
    // chains need and it cannot change for the life of this run, so reading it
    // here is both the narrower type and the more honest statement of intent.
    const signal = controller.signal;
    // PARALLEL ACROSS FEEDS, SEQUENTIAL WITHIN ONE. The operator's rule, and it
    // is the shape the rate budget dictates rather than a preference: a budget
    // is per VENDOR, so two rungs fired at one broker together double the rate
    // against a single ceiling, while two different brokers share nothing at
    // all. Firing everything at once is faster right up to the 429 that loses
    // the run.
    //
    // GROUPED IN ONE PASS, in the order the feeds were ticked, so the chains
    // start in a defined order even though they finish in whatever order the
    // vendors answer.
    /** @type {Map<string, typeof bodies>} */
    const byFeed = new Map();
    for (const b of bodies) {
      const k = b.vendor ?? '';
      const g = byFeed.get(k);
      if (g) g.push(b);
      else byFeed.set(k, [b]);
    }

    /* COARSEST RUNG FIRST, WHICH IS NOT THE ORDER THEY ARE DISPLAYED IN.
       `RUNGS` runs finest-first — ticks, minute, day — because that is how an
       operator reads a timeframe list. Sending them in that order made the
       server refuse every minute request it received: `pull::fold`'s ladder
       requires the DAY pass for a month before the minute pass for it, so a
       minute request that arrives first is answered "the 1day pass comes
       first" and nothing is stored.
       Measured on 2026-08-19 with both feeds and both rungs ticked: journal
       ordinals 0 and 1 are the two minute requests, both NOT STARTED, and 2
       and 3 are the two day requests, both STORED. The day pass landed, the
       minute pass was refused, and the operator saw a run that reported bars
       and left the rung they actually sweep empty.
       Sorted here rather than by reordering `RUNGS` because the display order
       is a separate decision that this must not silently change.

       THE DERIVATIVE SEGMENTS ARE ON THE SAME LIST, and after both spot rungs.
       `pull::fold`'s ladder is spot, then futures, then options — each waiting
       on the one before it to FINISH — so a single ordered list per feed is the
       whole sequencing rule. Across feeds nothing is ordered at all: the chains
       are awaited together, and two brokers share no rate ceiling, no census
       and no manifest lock. */
    const PULL_ORDER = ['1day', '1min', '1s', 'futures', 'options'];
    const ladderRank = (/** @type {string} */ dir) => {
      const at = PULL_ORDER.indexOf(dir);
      /* A RUNG NOBODY RANKED GOES LAST, NEVER FIRST. `indexOf` answers -1 for
         an unknown rung, and -1 sorts ahead of every real rank — so a rung
         added to `RUNGS` and forgotten here would quietly become the first
         thing sent, which is the one position that breaks the ladder. */
      return at === -1 ? PULL_ORDER.length : at;
    };
    for (const group of byFeed.values()) {
      group.sort((a, b) => ladderRank(a.dir) - ladderRank(b.dir));
    }

    // ONE CHAIN PER FEED. Each awaits its own requests in turn; the chains
    // themselves are awaited together.
    //
    // A FAILING FEED STOPS ITS OWN CHAIN AND NOT THE OTHERS, which is a change
    // the fan-out forces and an improvement on its own terms: under the old
    // single-feed loop `break` ended the run, and ending every vendor's work
    // because one vendor's socket died would throw away answers already paid
    // for. `netError` keeps the FIRST reason so the card names a cause rather
    // than the last thing to go wrong.
    const chain = async (/** @type {typeof bodies} */ group) => {
      for (const b of group) {
        sent = { done: sent.done, of: bodies.length, label: b.label };
        try {
          const r = await request(b.route ?? '/pull/spot', {
            // A PULL IS THE ONE ROUTE WHOSE WORK IS NOT LOCAL, so it carries
            // its own ceiling rather than the 15 s every other read gets. The
            // operator's Cancel signal below still applies: `ask` answers to
            // both.
            ms: 120_000,
            method: 'POST',
            headers: { 'content-type': 'application/x-www-form-urlencoded' },
            body: b.body,
            signal
          });
          const one = {
            ...readReceipt(
              await r.text(),
              r.ok,
              r.status,
              r.headers.get('content-type'),
              r.headers.get(RECEIPT_HEADER)
            ),
            rung: b.label
          };
          /* APPEND IS SAFE WITHOUT A LOCK and not by luck: JavaScript runs one
             task at a time, so a read-modify-write between two `await`s cannot
             interleave with another chain's. The chains are concurrent, never
             parallel. */
          receipts = [...receipts, one];
          // THE ONE THE CARD SHOWS IS THE FIRST BAD ONE. A later success must
          // not paint over a rung that refused — the list carries all of them.
          if (!receipt || (receipt.good && !one.good)) receipt = one;
        } catch (why) {
          if (controller?.signal.aborted) {
            aborted = true;
            return;
          }
          if (!netError) netError = String(why);
          /* CONTINUE, DO NOT ABANDON THE REST OF THIS FEED'S CHAIN.
             This was `return`, and `return` here throws away every request
             still queued for the feed — which is how Dhan's expired options
             went unpulled with nothing on screen naming them.
             Measured 2026-08-20: `PULL_ORDER` is
             ['1day','1min','1s','futures','options'], so options is LAST. Dhan's
             chain ran 1day, then 1min — a request the server answered by
             storing 58,572 bars — and the browser's connection dropped before
             the reply arrived. The fetch threw, this `return` fired, and the
             OPTIONS body was never sent. The page reported `HTTP 0`, the store
             held a short month, and `fno_roll` never wrote an audit record
             because it was never called.
             A dropped socket on one request says nothing about the next: the
             bodies are independent, one per (rung × segment), and the server
             has already committed whatever landed. The error is recorded in
             `netError` and shown; the chain goes on.
             WHAT THIS DOES NOT PRETEND: `PULL_ORDER` exists because the fold
             ladder needs the day pass before the minute pass. If the 1day body
             is the one that dropped, the minute body that follows may be
             refused by the server with "the 1day pass comes first" — which is a
             LOUD refusal on the receipt, and strictly better than a silent hole
             where a segment was never asked for at all. */
          continue;
        }
        sent = { done: sent.done + 1, of: bodies.length, label: b.label };
      }
    };

    // THE CHAINS RUN TOGETHER AGAIN, BECAUSE THE SERVER NOW LETS THEM.
    //
    // This was serialised in e902e65 for a real reason: `server.rs` held ONE
    // seat for the whole process, so a fan-out produced one run and N-1 × 409.
    // Measured from `logs/events.ndjson`, 18 Aug 22:16:54 — four POSTs inside
    // seven milliseconds, three refused, one storing 11 bars.
    //
    // THE SEAT IS PER FEED NOW. `autopilot.rs` carries `seats: AtomicU8` with
    // one bit per feed and `take_seat(feed)` claiming only that feed's bit, and
    // its own doc names the same incident and the same 299µs. The thing the
    // seat guards is the census lock, and the census is per vendor —
    // `dhan.man.lock` and `groww.man.lock` are different files — so two feeds
    // writing their own manifests never could race. Serialising here now costs
    // exactly what the fan-out was written to buy.
    //
    // PARALLEL ACROSS FEEDS, SEQUENTIAL WITHIN ONE, which is the rule the rate
    // budget dictates: a budget is per VENDOR, so two rungs at one broker
    // double the rate against a single ceiling while two brokers share nothing.
    //
    // A FAILING FEED STILL STOPS ONLY ITS OWN CHAIN — `chain` catches per
    // request, so one vendor's dead socket cannot throw away answers another
    // has already paid for.
    await Promise.all([...byFeed.values()].map(chain));
    controller = null;
    finishedAt = Date.now();
    phase = 'done';
    releaseWatch?.();
    releaseWatch = null;

    // THE AFTER READING. Taken even when the request failed — a run that
    // refused halfway still wrote whatever landed before it, and pretending
    // otherwise is the fallback that hides a failure.
    try {
      live = await snapshot();
      pollError = null;
    } catch (why) {
      pollError = String(why);
    }
    classifyBuild();
    // THE CENSUS IS A READING OF THE STORE AND THE STORE JUST MOVED. Leaving it
    // stale would show "never pulled" beside an outcome row saying bars landed.
    // The `snapshot()` above already bumped the shared generation, so the
    // census table is reading that same answer and no second fetch is needed.
    readPilot();
  }

  /**
   * STOP MEANS STOP THE RUN, NOT STOP LOOKING AT IT.
   *
   * The run is a task on the server now. Aborting the poll would end the
   * WATCHING and leave the run going — a button that says Stop and hides a
   * backfill instead of ending it, which is worse than no button. So the server
   * is told first, and a failure to tell it is REPORTED rather than swallowed.
   *
   * The server stops at the next leg boundary and not inside one: a leg that
   * has already asked the vendor for bars must be allowed to write them, or
   * pressing Stop would throw away answers that were already paid for.
   */
  async function stopWatching() {
    aborted = true;
    try {
      await request('/pull/run/stop', { ms: 10_000, method: 'POST' });
    } catch (why) {
      pollError = `Stop could not be delivered, so the run may still be going: ${why}. Reload this page to see what it is doing.`;
      return;
    }
    controller?.abort();
  }


  function reset() {
    phase = 'idle';
    // CLEARED WITH THE REST. A pass summary left standing over a cleared
    // receipt describes a run whose evidence is gone.
    passSummary = null;
    receipt = null;
    receipts = [];
    sent = { done: 0, of: 0, label: '' };
    askedKeys = new Set();
    netError = null;
    outcomes = [];
    outcomeIndex = new Map();
    baseline = null;
    live = null;
    samples = [];
    aborted = false;
  }

  let showRaw = $state(false);

  /* ══════════════════ THE SELECTION IN THE ADDRESS BAR ══════════════════

     WHAT THIS FIXES, MEASURED. Every control here was in-memory state that
     nothing wrote down. Recorded from the running page, reloaded, and diffed:

       universe      F&O Underlyings  ->  NIFTY 50        RESET
       instruments   All 211 ticked   ->  All 50 ticked   RESET
       feed          Groww            ->  Groww           kept, by accident
       segments      Spot             ->  Spot            kept, by accident
       timeframe     1 minute         ->  1 minute        kept, by accident

     The three that "survived" were already sitting on the default. In the
     operator's words: "whatever we have selected before pull, even after pull
     it needs to be the same -- if not, how will I know which one got selected?"

     THE FEED IS THE ONE THAT HAD TO BE WRITTEN DOWN RATHER THAN RECOMPUTED.
     The others reset to a CONSTANT; the feed does not. `pick.js` chooses it by
     "most bars wins", so the default moves whenever the store does -- and a
     pull moves the store. Measured after one run: groww 132,132 bars, dhan 0.
     An operator who chose Dhan, pulled, and reloaded came back on Groww with
     every count silently rescoped. So an explicit feed in the URL beats the
     computed default, which is what `urlFeedPinned` below is for.

     IT IS THE URL AND NOT STORAGE, so the answer to "which one got selected"
     is readable, bookmarkable and sendable, rather than hidden in a browser
     nobody can inspect. */

  /** What the address bar asked for, drained as each prerequisite arrives. */
  let urlWant = $state(/** @type {ReturnType<typeof decodeSel> | null} */ (null));
  /** Set once the feed named in the URL has been applied, so `pick.js` cannot
      win a later race against it. */
  let urlFeedPinned = $state(false);
  /** Set once the ticked list has been rebuilt, which needs the catalogue. */
  let urlMembersDone = $state(false);
  /** No writing until the reading is finished, or hydration overwrites itself. */
  let urlHydrated = $state(false);

  /** The selection as this page would spell it. One definition, read by the
      writer below and by nothing else. */
  const selectionSearch = $derived(
    encodeSel({
      feeds: pullFeeds.size > 0 ? pullFeeds : feeds.active ? [feeds.active] : [],
      universe,
      // ALL TICKED IS `null`, WHICH IS NO FIELD AT ALL -- the same rule
      // `wireBodyFor` follows on the wire, and it keeps a 750-name selection
      // from putting 750 fields on a query string to say "everything".
      members: insOff.size === 0 ? null : ticked.map((m) => m.symbol),
      segs: segSet,
      rungs: rungSet,
      from: fromDay,
      to: toDay
    })
  );

  onMount(() => {
    /* A RUN OUTLIVES THIS TAB. See `resumeRun`: the first thing this page does
       is ask whether one is already going, because the server owns the run and
       this page is only watching it. */
    void resumeRun();
    const want = decodeSel(window.location.search);
    urlWant = want;
    // THE FIELDS WITH NO ASYNC PREREQUISITE, applied at once.
    if (want.universe) universe = want.universe;
    if (want.segs) segSet = new Set(want.segs);
    if (want.rungs) {
      rungSet = new Set(want.rungs);
      // A RUNG THAT CAME FROM THE URL IS THE OPERATOR'S, and the effect that
      // moves an untouched rung onto whatever the feed serves must not treat
      // it as a default it may overwrite.
      rungTouched = true;
    }
    if (want.from) setDay('from', want.from);
    if (want.to) setDay('to', want.to);
    if (want.from || want.to) windowTouched = true;
    urlHydrated = true;
  });

  /* THE FEED, ONCE THE FEED LIST HAS LANDED. `loadFeeds` sets `feeds.active`
     from the store; this puts the operator's own choice back on top of it. */
  $effect(() => {
    const wantFeeds = urlWant?.feeds;
    // NARROWED HERE AND NOT INSIDE `untrack`. The guard cannot travel across a
    // callback boundary, so the value it proved has to.
    if (!wantFeeds || urlFeedPinned || feeds.all.length === 0) return;
    untrack(() => {
      const named = [...wantFeeds].filter((w) => feeds.all.some((f) => f.wire === w));
      if (named.length > 0) {
        feeds.active = named[0];
        // ONE NAMED FEED IS THE SCOPE FEED AND NOT A FAN-OUT. `pullFeeds`
        // empty means "the scope feed", so seeding it with a single entry
        // would say the same thing twice and drift from `feeds.active`.
        pullFeeds = named.length > 1 ? new Set(named) : new Set();
      }
      urlFeedPinned = true;
    });
  });

  /* THE TICKED LIST, ONCE THE CATALOGUE HAS LANDED. The URL carries the ticked
     SYMBOLS; this page stores the EXCLUDED keys, so the two are converted here
     and nowhere else. A symbol the master no longer lists simply does not
     match, which un-ticks it -- and that is visible on the control's own face
     as a smaller count rather than being silently widened back to everything. */
  $effect(() => {
    const keep = urlWant?.members;
    if (!keep || urlMembersDone || insPool.length === 0) return;
    untrack(() => {
      insOff = new Set(insPool.filter((m) => !keep.has(m.symbol)).map((m) => m.key));
      urlMembersDone = true;
    });
  });

  /* THE WRITER. `replaceState` and never `pushState`: a selection is not a
     place you navigated to, and one history entry per tick would make the back
     button walk the operator backwards through their own form one checkbox at
     a time. Guarded by `sameSel` so a derivation re-running does not write an
     address that has not changed. */
  $effect(() => {
    const next = selectionSearch;
    if (!urlHydrated) return;
    untrack(() => {
      const now = window.location.search.replace(/^\?/, '');
      if (sameSel(now, next)) return;
      const url = next ? `${window.location.pathname}?${next}` : window.location.pathname;
      window.history.replaceState(window.history.state, '', url);
    });
  });

  onMount(() => {
    const tick = setInterval(() => (nowMs = Date.now()), 500);
    /** @param {PointerEvent} e */
    const away = (e) => {
      // THE PRESS LANDED ON AN ELEMENT. `EventTarget` is what the DOM types
      // promise and `Element` is what a pointerdown in a document delivers;
      // both reads below need the narrower one, and naming it once keeps the
      // two tests reading the same value rather than asserting twice.
      const t = /** @type {Element} */ (e.target);
      if (cal.field && calEl && !calEl.contains(t) && !t.closest?.('.cal-open')) shutCal(false);
      // THE SAME PRESS CLOSES THE STRIP’S OWN MENU. `.dd` is the whole
      // control — the button and the panel it opens — so a press inside the
      // open one is not away, and a press on any other control, a Picker
      // included, is. One value, so two can never be open at once.
      if (drop !== null && !t.closest?.('.dd')) drop = null;
    };
    document.addEventListener('pointerdown', away, true);
    return () => {
      clearInterval(tick);
      document.removeEventListener('pointerdown', away, true);
      releaseWatch?.();
      releaseWatch = null;
      controller?.abort();
    };
  });
</script>

<div class="pane">
  <div class="pane-head">
    <!-- ══ THE TITLE AND THE THREE CHIPS ARE GONE, AT THE OPERATOR'S
         INSTRUCTION ══
         "INGEST · DHAN · REST API · 1 MINUTE" restated four things the page
         states better a few pixels lower: the nav above has `Ingest` marked
         current, and the feed, its transport and the rung are each the FACE of
         a control in the strip. A band that repeats the controls under it is
         the running commentary this page has been shedding all along.

         THE REFUSAL BRANCH IS NOT LOST. `source kind not stated` drew here when
         the server sent no kind — §4 — and it still draws, on the form itself,
         where the shape of the request is decided. Checked before this was
         cut, not assumed.

         The run status and the link to /audit stay: neither is a restatement,
         and the status is the only thing on the page that says a pull is on the
         wire while you are looking somewhere else. -->
    <span class="spacer"></span>
    {#if phase === 'running'}
      <span class="status busy" aria-live="polite">
        <span class="dot acc live"></span>
        <span class="txt">{Verb} running</span>
        <span class="ms">{clockOf(elapsedMs)}</span>
      </span>
    {:else if phase === 'done' && receipt}
      <span class="status" class:bad={!receipt.good}>
        <span class="dot" class:up={receipt.good} class:down={!receipt.good}></span>
        <span class="txt">{receipt.verdict}</span>
        <span class="ms">{clockOf(elapsedMs)}</span>
      </span>
    {/if}
    <a class="link" href="/audit">The record of every run</a>
  </div>

  <!-- THE CONTROL COMES WITH THE REFUSAL. Each of these three states is the
       page saying "not with this feed", and the one action every one of them
       asks for is a change of feed — so the control is drawn inside the
       refusal rather than named in it. It used to point at the top bar; the
       top bar no longer carries a picker on this route, and a page that both
       refuses and offers no way out is a dead end. `.blankpick` is the one
       rule that keeps the dropdown from stretching to the width of a page. -->
  {#if feeds.error}
    <div class="blank">
      <h2>The feed list could not be read</h2>
      <p>Nothing can be ingested until the server names a feed to ingest into.</p>
      <p class="mono down">{feeds.error}</p>
      <div class="blankpick">{@render feedRung()}</div>
    </div>
  {:else if !active}
    <div class="blank">
      <h2>No feed is selected</h2>
      <p>
        Ingest writes into exactly one feed's store, so one has to be chosen before a window means
        anything. Pick one here — it is the same control that stands first in the strip once a feed
        is chosen, and it is the only one on this page.
      </p>
      <div class="blankpick">{@render feedRung()}</div>
    </div>
  {:else if archiveHidden}
    <div class="blank">
      <h2>{active.display} has nothing to ingest yet</h2>
      <p>
        This is an archive feed: it reads CSV files that have to be bought first. The server refuses
        it for a reason of its own, quoted here rather than paraphrased.
      </p>
      <p class="warn mono">{active.why}</p>
      <p class="alt">
        <!-- `.lab`, NOT `.lbl`, AND THE PAGE NOW HAS ONE LABEL FACE INSTEAD OF
             TWO. This was the only `.lbl` on the page against six `.lab`, and
             the local rule that styled it also overrode `theme.css`'s own
             `--fs-mini` down to `--fs-micro` — so the one odd label was odd in
             two ways at once, in a different face and at a size neither `.lbl`
             nor `.lab` actually specifies anywhere else. -->
        <span class="lab">What to do</span>
        Buy the archive and give the feed a store prefix, or select a broker feed below. The folder
        form appears here the day this feed reports itself ready — nothing on this page names a
        vendor.
      </p>
      <div class="blankpick">{@render feedRung()}</div>
    </div>
  {:else}
    <div class="grid body">
      <!-- TWO COLUMNS WHILE THERE ARE TWO CARDS, ONE WHEN THE RUN CARD IS NOT
           DRAWN. Without this the form kept its 5fr and the vacated 4fr stayed
           on screen as a column of nothing — the panel removed, its hole left
           behind. -->
      <div class="cols">
        <!-- ====================================== THE CONTROL STRIP =======
             ONE PANEL, and every control in it is a labelled dropdown standing
             in the same row as its peers: the feed across the top, then
             universe · instruments · segments · timeframe as equal columns,
             then the two days.

             WHAT LEFT THE RENDERED PAGE, AND WHERE EACH FACT WENT. The four
             tiles, the "The request" heading and its chip, the "not an NSE
             universe" block, the "counted here, not requestable" paragraph,
             the ceiling paragraph and the "Dhan is a broker" sentence are gone
             from the page. Nothing they said is denied — each one is now the
             `title` of the control it is about, or one clause under it:

               names this feed reaches  -> the Instruments control's own face,
                                           and the clause under it
               window · month files     -> the clause under Days to pull
               instrument-months asked  -> the ask line at the foot of the
                                           strip; it is `productLine` verbatim
               held in this window      -> the same line, beside it
               refused universes        -> each row's own `title`, which is
                                           `universeRefusal(u)` in full, plus
                                           `refusalSummary` on the control
               the swept pair           -> the last row of the same menu, said
                                           on the row rather than beside it
               the ceiling              -> `ceilReason` on the To date control
                                           and in the grid it opens
               broker window splitting  -> the Days to pull label's `title`
             ============================================================== -->
        <section class="sel">
          <form onsubmit={start} class="form rise">
            <!-- THE GRID IS THE FIELDSET. A wrapper between the two would take
                 the grid's tracks for itself and leave the controls stacked
                 inside one column, which is the shape this page is being
                 rewritten out of. -->
            <fieldset class="pickers" disabled={phase === 'running'}>
              <!-- THE FEED IS THE FIRST RUNG OF THE CASCADE, AND THEREFORE THE
                   FIRST CONTROL IN THE ROW. It stood across the top of the
                   strip as a full-width SCOPE LINE — a label, the feed's name
                   wearing a control's face with no affordance, and a clause —
                   which is a paragraph pretending to be a control. It is a
                   dropdown again, and it is a PEER: same track, same label
                   style, same button height and the same one baseline as
                   Universe · Instruments · Segments · Timeframe.

                   ONE CONTROL FOR ONE VALUE, STILL. The top bar's picker is not
                   drawn on the routes that carry their own — see FEED_OWNED in
                   `+layout.svelte`. Two controls writing one `feeds.active`
                   teach the reader they are independent when they are not, and
                   that is the defect the scope line was introduced to fix; the
                   fix is which control exists, not whether the page has one.

                   THE SCOPE SENTENCE SURVIVES AS THE CONTROL'S OWN CLAUSE,
                   which is where it was always headed: "everything below is
                   this feed's answer · carries N name(s) · what it does not
                   carry is not measured" is what tells a reader the whole page
                   is scoped to one vendor, and it is load-bearing. The long
                   sentence is on the button's `title`.

                   A REFUSED FEED IS DRAWN AND REFUSED, never omitted:
                   `/feeds.json` marks a feed not ready and states why, and the
                   row quotes that reason rather than paraphrasing it. -->
              {@render feedRung()}

              {#if shows.get('universe')}
                <!-- THE UNIVERSE RUNG, AND IT IS A DROPDOWN. The eight in NSE's
                     order, then the swept pair, which is not one of them and is
                     labelled as what it is on its own row. A refused row is
                     drawn, disabled and carries `universeRefusal(u)` whole —
                     the same sentence the deleted paragraph printed, on the
                     row it is about. -->
                <div class="field">
                  <!-- THE SUB-CLAUSE IS ON THE CONTROL, NOT IN THE LABEL. "The
                       set this feed is asked for" is a true and useful sentence
                       and it is the FIRST thing `universeTitle` now says. In the
                       label it was a second line in one cell and no line in the
                       next, which is half of why this row had no baseline. -->
                  <span class="lab">Universe</span>
                  <!-- THE MENU ONLY. Everything below this control -- the reach
                       clause and the whole archive-census branch a folder feed
                       draws -- is untouched. This rung renders two different
                       things depending on whether the feed is HTTP or a folder
                       of files, and only the HTTP half was ever a menu; a
                       previous attempt replaced the WHOLE rung and took the
                       census with it.

                       `tuck` REPLACES uniTuck/uniOpen/uniTucked/uniTuckLabel,
                       which existed to fold the sets no request can name behind
                       one counted line -- exactly what `tuck` does, and what the
                       Segments rung already uses it for. A refused set stays
                       DRAWN, DISABLED and carrying `universeRefusal`'s own
                       sentence: CLAUDE.md section 4, since a deleted row reads
                       as "this set does not exist".

                       THE SWEPT PAIR IS NOT ADDED. Its row sat behind a
                       permanently-false guard, so it was already unrendered; an
                       earlier attempt at this conversion put it back into the
                       live list, which handed the operator NSE-NIFTY +
                       NSE-BANKNIFTY as a selectable set again. Dead code stays
                       dead. -->
                  <Picker
                    single
                    filter
                    tuck
                    label="universes"
                    summary={universeSpec.label}
                    title={universeTitle}
                    rows={UNIVERSES.map((u) => {
                      const why = universeRefusal(u);
                      return {
                        key: u.id,
                        name: u.label,
                        disabled: why !== null,
                        why: why ?? undefined,
                        title:
                          why ??
                          `Pulls with target=${u.target}. ${feedName(feeds.active)} reaches ${reachKnown ? n(reach) : 'an uncounted number of'} name(s) in it, counted from /instruments.json?feed=${feeds.active ?? ''}.${u.note ? ` ${u.note}.` : ''}`
                      };
                    })}
                    selected={new Set([universe])}
                    onchange={(/** @type {Set<string>} */ sel) => {
                      const next = [...sel][0];
                      if (next) universe = next;
                    }}
                  />
                  <!-- NARRATION CUT, REFUSAL KEPT. This drew
                       "N reachable · target=n50" on every load — a count the
                       control's own face already carries and a wire slug the
                       operator did not ask about. It is a REMARK, not a
                       refusal: it names nothing anybody could act on and it
                       cost a line under every field on the strip.
                       `reachWhy` is the other half and it stays, because that
                       one IS a refusal — §4 requires it named, and it is drawn
                       only when there is something to name. -->
                  {#if !reachKnown}
                    <span class="note warn" title={reachWhy}>{reachWhy}</span>
                  {/if}

                  <!-- ═══════ WHAT AN ARCHIVE HAS INSTEAD OF A UNIVERSE ═══════

                       The line above counts what a MASTER reaches, and the two
                       archive feeds publish none — `Vendor::MASTERED` is the
                       three REST feeds and nothing else, so every tier resolves
                       to nothing for TrueData and GDFL and the count above is
                       an honest zero about the wrong question.

                       The right question for a folder is what is IN it, and the
                       only way to answer it is to walk it. `/folder.json` did
                       that walk the moment this feed was chosen, and its six
                       facts were being spent on one line at the foot of a
                       calendar popover. They are the closest thing this feed
                       HAS to a universe, so they belong on the universe control.

                       NOTHING HERE IS DECLARED. Every number is off the wire,
                       and the state a folder is in decides which sentence gets
                       drawn — empty, blank and days are three different answers
                       and a halt is not an answer at all. D-0141. -->
                  {#if archiveCensus}
                    {@const c = archiveCensus}
                    {#if c.state === 'reading'}
                      <span class="note quiet">reading {active.display}'s folder…</span>
                    {:else if c.state === 'halted'}
                      <span class="note wrap warn" title={c.why}>
                        the folder could not be read{c.path ? ` — ${c.path}` : ''}. This is a HALT
                        and not an empty folder: nothing was counted, so nothing below is a census
                        of anything.
                      </span>
                    {:else if c.state === 'days'}
                      <span
                        class="note quiet wrap"
                        title={`Walked by GET /folder.json?feed=${feeds.active ?? ''} when this feed was selected — pull::folder::read_reach, O(members). ${c.path ?? ''}`}
                      >
                        {n(c.files)} file(s) · {n(c.rows)} row(s) · {dayLabel(c.earliest)} – {dayLabel(
                          c.latest
                        )} — {c.segment
                          ? `the ${c.segment} archive`
                          : "this feed's universe is the folder"}, read from it
                      </span>
                    {:else if c.state === 'blank'}
                      <span class="note wrap warn">
                        {n(c.files)} file(s) are there and not one carries a row. They were bought
                        and they are blank — which is a different thing from not having bought them,
                        and this is the only place the two are told apart.
                      </span>
                    {:else}
                      <span class="note wrap warn">
                        the folder is there and holds nothing{c.path ? ` — ${c.path}` : ''}. That is
                        an ANSWER, not a failure: the months have not been bought and put there yet.
                      </span>
                    {/if}
                    <!-- THE NAMES. `/folder.json` carries `instruments` since
                         D-0141: `pull::folder::read_census` collects what
                         `pull::archive::Member` already took off each file
                         name, sorted, whole, never truncated.

                         THREE STATES AND THEY ARE NOT TWO. `null` is a server
                         that sent no such field — a running binary that
                         predates it, which names its own fix. `[]` is a folder
                         that names nothing, which is an ANSWER. A list is a
                         list. Collapsing the first two would print "0
                         instruments" at a server that was never asked.

                         FOUR STATES, THEN — AND THE FOURTH IS "THERE IS NO
                         CENSUS YET". `reading` and `halted` are shapes that
                         carry no `instruments` key at all, and this chain used
                         to begin at `=== null`, so on those two it fell through
                         to `.length` on `undefined` and THREW. Measured on the
                         running binary: `TypeError: Cannot read properties of
                         undefined (reading 'length')`, from this line, on any
                         archive feed. Drawing nothing is right here rather than
                         drawing a reason: the branch above has already printed
                         "reading …" or named the halt, and a second sentence
                         about the instrument list would be the same fact twice.
                    -->
                    {#if c.state === 'reading' || c.state === 'halted'}
                      <!-- deliberately empty; the state chain above already spoke -->
                    {:else if c.instruments === null}
                      <span
                        class="note wrap warn"
                        title="GET /folder.json emits `instruments` — every distinct name pull::archive::Member took off a file name in that folder, sorted. This server sent no such field, so this page cannot name what is in the folder and does not guess: the file names are the ONLY identity an archive has, and there is no ISIN, no security id and no master to fall back on. Restart the API on a build that emits it."
                      >
                        this server sends no instrument list for the folder — what is in it is not
                        known here, and this page will not synthesise names from a file count
                      </span>
                    {:else if c.instruments.length > 0}
                      <span
                        class="note quiet wrap"
                        title={`${c.instruments.length} distinct name(s), read off the file names in ${c.path ?? 'the folder'} and sorted. This is the whole identity an archive has — there is no ISIN, no security id and no master — so it is also the whole of what a universe could mean for this feed. First twenty: ${c.instruments.slice(0, 20).join(', ')}`}
                      >
                        {n(c.instruments.length)} instrument(s) named — {c.instruments
                          .slice(0, 6)
                          .join(', ')}{c.instruments.length > 6
                          ? ` and ${n(c.instruments.length - 6)} more`
                          : ''}
                      </span>
                      <!-- TWO FILES CLAIMING ONE NAME, SAID RATHER THAN
                           DEDUPLICATED IN SILENCE. It is the `ambiguous` bucket
                           of D-0141 on the folder side, and a reader who saw
                           only the distinct list could not tell a clean folder
                           from a colliding one. -->
                      <!-- FILES OF ANOTHER PRODUCT, NAMED. One of these used to
                           halt the whole folder: a loose BACKADJUSTED CSV of
                           nine OHLC fields beside a TICK archive of ten made
                           GDFL answer nothing at all, and the page said so
                           about the folder. The census reads past it now and
                           the file is a finding rather than the end. Each
                           carries the decoder's own words, so the operator sees
                           WHICH file and WHY. -->
                      {#if c.rejected.length > 0}
                        <span
                          class="note wrap warn"
                          title={c.rejected
                            .slice(0, 8)
                            .map((r) => `${r.path} — ${r.why}`)
                            .join('\n')}
                        >
                          {n(c.rejected.length)} file(s) in this folder are a DIFFERENT PRODUCT and
                          were read past, not counted — hover for each one and the decoder's own
                          reason. They are not a broken folder; they are files this vendor's
                          declared layout does not describe.
                        </span>
                      {/if}
                      {#if c.collisions > 0}
                        <span
                          class="note wrap warn"
                          title="pull::folder::census_of counts the members whose instrument name a previous member had already claimed, rather than deduplicating in silence. GDFL nests Options/ and Futures/, so one stem can appear under both. Two files claiming one instrument is the `ambiguous` case D-0141 names on the folder side: it is not resolved by whichever was walked first."
                        >
                          {n(c.collisions)} file(s) name an instrument another file already named —
                          two members claiming one name are not resolved by walk order
                        </span>
                      {/if}
                    {:else}
                      <span class="note wrap warn">
                        the folder names no instrument at all — this is the list being EMPTY, which
                        the server answered, and not a list it failed to send
                      </span>
                    {/if}
                  {/if}
                </div>
              {/if}

              {#if shows.get('instruments')}
                <!-- TICKED, ONE BY ONE, AND THE LIST IS THE CONTROL. This was a
                     roster: two summary lines and nothing selectable, with its
                     own label conceding "read, not chosen". Every name in the
                     universe is a checkbox now, searchable and sorted, with the
                     two bulk actions over WHAT IS SHOWN and a face that counts
                     what is ticked.

                     THE ONE THING THE TICKS CANNOT DO IS NARROW THE REQUEST,
                     and that is said on the control and again under the ask
                     line rather than being worked around: SpotRequest has no
                     member field. What they narrow is what this page counts,
                     measures and reports on — the ask, the census, the outcome
                     rows. -->
                <div class="field">
                  <span class="lab">Instruments in that universe</span>
                  <div class="dd">
                    <button
                      class="ddb"
                      type="button"
                      aria-expanded={drop === 'ins'}
                      title={`Ticked — this is what is counted. ${insSummary}. ${reachKnown ? `${n(reach)} of the ${n(catalogue.rows.length)} instrument(s) ${active.display} contributes to a tracked universe carry this one. That is NOT the size of its master: /instruments.json returns the merged tracked catalogue — index, F&O underlyings and NIFTY Total Market — while the master itself holds every listing the vendor publishes and /health reports that separately.` : reachWhy} A tick decides what this page counts — the ask line, the census below and the outcome list after a run — AND what the run walks: each ticked name goes out as its own member= field, api::ingest::parse_spot collects them into SpotRequest.members, and broker_run filters every candidate through them. Ticking every name sends no member at all, deliberately: naming all and naming none are the same ask, and target=${target} then covers all ${reachKnown ? n(reach) : '—'}.`}
                      onclick={(e) => {
                        e.stopPropagation();
                        drop = drop === 'ins' ? null : 'ins';
                      }}>{insSummary}</button
                    >
                    {#if drop === 'ins'}
                      <div class="ddm wide" role="group" aria-label="Instruments in that universe">
                        <!-- THE HEADER IS OUTSIDE EVERY KEYED BLOCK, so Svelte
                             keeps this exact input node across updates: a
                             detached element is blurred, the caret vanishes and
                             the next character is dropped. -->
                        <div class="srch">
                          <input
                            class="search"
                            type="search"
                            bind:value={insQ}
                            placeholder="Search {n(insRows.length)} name(s) — ticker, key or kind"
                            aria-label="Search the instruments in this universe"
                            onclick={(e) => e.stopPropagation()}
                          />
                          <p class="shint">
                            Matches the trading symbol, the store key or the kind. {n(
                              insShown.length
                            )} shown of {n(insRows.length)}.
                          </p>
                          <div class="sact">
                            <button
                              type="button"
                              disabled={insSelectable.length === 0}
                              title={insSelectable.length === 0
                                ? 'Nothing here can be ticked: every row shown is a name this feed does not list.'
                                : `Ticks the ${n(insSelectable.length)} name(s) currently shown. A search narrows this button with the list.`}
                              onclick={insSelectShown}
                            >
                              {insQ.trim()
                                ? `Select all ${n(insSelectable.length)} shown`
                                : `Select all ${n(insSelectable.length)}`}
                            </button>
                            <button
                              type="button"
                              disabled={insCount === 0}
                              title={insCount === 0
                                ? 'Nothing is ticked, so there is nothing to clear.'
                                : 'Unticks the names currently shown. A search narrows this button with the list.'}
                              onclick={insClearShown}
                            >
                              {insQ.trim() ? 'Clear all shown' : 'Clear all'}
                            </button>
                          </div>
                        </div>

                        <div class="inslist">
                          {#if insEmptyWhy}
                            <p class="insnone">{insEmptyWhy}</p>
                          {:else}
                            <!-- A ROW THIS FEED CANNOT SERVE IS DRAWN, REFUSED
                                 AND CARRIES THE REASON. Deleting it would teach
                                 the reader the name does not exist, which is
                                 the opposite of the finding. -->
                            {#each insShown.slice(0, 400) as r (r.key)}
                              <label class:off={r.off !== null} title={r.off ?? ''}>
                                <input
                                  type="checkbox"
                                  checked={r.off === null && !insOff.has(r.key)}
                                  disabled={r.off !== null}
                                  onchange={() => toggleIns(r.key)}
                                  onclick={(e) => e.stopPropagation()}
                                />
                                <span class="nm mono">{r.sym}</span>
                                <span class="ct" class:warn={r.off !== null}>
                                  {r.off === null ? r.detail : 'not on this feed'}
                                </span>
                              </label>
                            {/each}
                            {#if insShown.length > 400}
                              <p class="insnone">
                                {n(insShown.length - 400)} more are not drawn. Narrow the search to
                                reach them — the count on the button is the whole of it, and Select
                                all acts on all {n(insSelectable.length)} shown, drawn or not.
                              </p>
                            {/if}
                          {/if}
                        </div>

                        <!-- THE MEASUREMENT THAT FILLS THE REFUSED ROWS. It is
                             one request per feed and it rebuilds a census
                             server-side each time, which is why it is a press
                             and not a page load. -->
                        {@render instrumentRung()}
                      </div>
                    {/if}
                  </div>
                  <!-- "of 869 tracked" IS A DENOMINATOR, AND IT HAD NO
                       NUMERATOR BESIDE IT. The control's face reads
                       `All 50 ticked`; a second line under it naming the size of
                       the catalogue those 50 came out of is a fact about the
                       SERVER printed under a control about the SELECTION, on
                       every load. It is on the button's `title` with the whole
                       measurement around it.
                       `reachWhy` STAYS. That branch is the §4 case — the reach
                       could not be read at all — and it is drawn only when it
                       is true. -->
                  {#if !reachKnown}
                    <span class="note warn" title={reachWhy}>{reachWhy}</span>
                  {/if}
                </div>
              {/if}

              {#if shows.get('segment')}
                {@render segmentRung()}
              {/if}

              {#if shows.get('rung')}
                <!-- "Timeframe" is the cascade's word for this rung and the word
                     /db and /markets use for the same axis; "bar length" is what
                     it means, and `granularity` is what it is called on the
                     wire. All three, once, on the one control that carries
                     them.

                     TWO ROWS ON A BROKER, THREE ON AN ARCHIVE, AND NEVER
                     ELEVEN. The owner's rule of 14 Aug 2026: one minute and one
                     day alone, plus the tick rung for TrueData and GDFL alone.
                     The eight that left were rungs NO feed in this build
                     declares — the union of all four descriptors' declared
                     granularities is exactly the three that remain — so they
                     were drawn live and annotated "not fetched by this build"
                     on every feed, forever. Everything coarser than a minute is
                     a fold of bars already on this disk, which is why there is
                     no "add a timeframe" here and nothing missing that one
                     would add. See `RUNGS`.

                     WHICH TWO FEEDS SHOW THE THIRD ROW IS NOT DECIDED HERE. It
                     is `/feeds.json`'s granularity floor: a feed that bottoms
                     out at a minute has the second rung permanently refused and
                     `rungRows` drops it. No vendor is named in this file. -->
                <!-- `rung`: the ONE control on this strip whose menu needs a
                     ceiling. See `.field.rung` in the stylesheet. -->
                <div class="field rung">
                  <span
                    class="lab"
                    title="Bar lengths — the granularity field on the wire. Timeframe is the cascade's word for this rung and the word /db and /markets use for the same axis; bar length is what it means; granularity is what it is called on the wire. All three name one thing."
                    >Timeframe</span
                  >
                  <Picker
                    label="bar lengths"
                    title={feedFinest
                      ? `${feedFinest.short} One record at ${feedFinest.rungPhrase} is a ${feedFinest.label}. ${feedFinest.because} — ${feedFinest.source}`
                      : 'Bar lengths — the granularity field on the wire. This server stated no granularity floor for the active feed, so nothing here refuses any rung for it.'}
                    summary={rungsChosen.length === 0
                      ? 'No timeframe ticked'
                      : rungsChosen.length === rungTally.live
                        ? `All ${n(rungTally.live)} offered`
                        : rungsChosen.map((r) => r.label).join(', ')}
                    rows={rungRows}
                    selected={rungSet}
                    onchange={(/** @type {Set<string>} */ sel) => {
                      rungSet = sel;
                      // FROM HERE THE TIMEFRAME IS HIS, and the default stops
                      // following the feed. See `rungTouched`.
                      rungTouched = true;
                      // The drop notice is the operator's to dismiss, and
                      // touching this control is how he does it: he has just
                      // answered it.
                      droppedRungs = null;
                      cPage = 1;
                    }}
                  />

                  <!-- ══════════ WHAT THE FEED CHANGE TOOK, AND WHY ══════════
                       Never a silent keep and never a silent clear. It stands
                       until the operator moves the timeframe himself or changes
                       the feed again — the change that caused it is over in a
                       frame, and a notice that dies with it is one nobody
                       reads. -->
                  {#if droppedRungs}
                    <p class="caution loud drop" aria-live="polite">
                      <span class="tag down">dropped</span>
                      <span class="msg">
                        {n(droppedRungs.rows.length)} ticked timeframe(s) were removed when the feed
                        became <b>{droppedRungs.feed}</b>, which can never serve them:
                        {#each droppedRungs.rows as d (d.dir)}
                          <br /><b class="mono">{d.label}</b> — {d.why}
                        {/each}
                        <br />{droppedRungs.kept === 0
                          ? 'Nothing is ticked now, so nothing below can be counted until you pick a bar length this feed serves.'
                          : `${n(droppedRungs.kept)} timeframe(s) survived the change and are still ticked.`}
                      </span>
                    </p>
                  {/if}

                  <!-- THE FEED'S OWN FLOOR, STATED ONCE, WHERE IT BINDS. The
                       row-level sentences say what each rung is refused for;
                       this says what the FEED is, which is the fact that
                       decides whether the second rung is one of the rows
                       above. -->
                  <!-- THE FEED'S FLOOR IS A FACT, AND IT WAS THE LONGEST
                       NARRATION ON THE STRIP.
                       "Dhan — REST API. It serves one minute and coarser. One
                       record at one minute is a bar — open, high, low, close."
                       ran to five lines under a control whose face reads
                       `1 minute`, on every load, and it is what made the row
                       four lines taller at one end than the other. It is on the
                       menu's `title` now, with its provenance, which is where
                       the same fact already lived in its long form.
                       WHAT IT PROTECTS IS UNCHANGED. The floor still decides
                       which rows `rungRows` draws, and a rung this feed refuses
                       is still a row in that menu carrying its own why. Nothing
                       is enforced here and nothing was. -->
                  {#if !feedFinest && feeds.active}
                    <!-- A MISSING FIELD, NAMED AS ONE. This used to read "no
                         granularity floor stated", which was true of a browser
                         holding a transcription that did not cover the feed.
                         There is no transcription now, so the only way to be
                         here is a server that did not send `finest` — and that
                         is a build to restart, not a vendor to wonder about. -->
                    <span
                      class="note wrap warn"
                      title="crates/pull carries a granularity floor for every feed it can name, and /feeds.json emits it as `finest` on every row — the finest rung, whether one record there is a bar, a conflated snapshot or a real tick stream, why nothing finer exists in the vendor's own terms, and where those words were read. This page holds no copy of that table: a hardcoded fallback would look exactly as authoritative as a reading, and the operator could not tell them apart. So nothing is refused here, and an unstated floor is not an unlimited one."
                    >
                      this server sent no granularity floor for {feedName(feeds.active)} — nothing
                      here refuses any rung for it
                    </span>
                  {/if}
                  <!-- MULTI-SELECT, AND EVERY TICK IS A REQUEST. The wire
                       carries one `granularity` per POST, so N ticks are N
                       POSTs sent in the ladder's order — the wire fold prints
                       every one and the run card counts them.

                       THERE IS NO "+ ADD A TIMEFRAME" HERE, AND THE REASON IS
                       NOT THAT ONE IS HARD TO BUILD. Everything coarser than a
                       minute is a fold of one-minute bars already on this disk,
                       so asking a vendor for it spends a request, a quota and a
                       window on arithmetic `crates/pull` does for nothing. The
                       owner's rule, 14 Aug 2026. And a rung invented in the
                       browser would be refused anyway:
                       `api::ingest::parse_granularity` matches the directory
                       name exactly.

                       WHAT IS DRAWN IS PER FEED. A rung the active vendor does
                       not publish is filtered out — `rungRows`, D-0137 —
                       because no work closes it, and that filter is what puts
                       the second rung on the two archive feeds and nowhere
                       else. The rungs drawn WHILE refused are the ones a person
                       could close: unfetched, or unstored. -->
                  <!-- NARRATION CUT, AND THE ELEMENT GOES WITH IT.
                       The `{n} ticked of {n} offered · {n} with a store
                       directory · {n} request(s) per read` tail drew on EVERY
                       load and every clause of it was already on screen: the
                       ticked set is the control's own face, and the store
                       directory and the request count are on the rows
                       themselves. Three counts restating three visible things
                       is the running commentary that made this strip unreadable.

                       THE `{#if}` IS OUTSIDE THE SPAN, not inside it. A
                       `.note` carries its marker as `::before`, so an empty
                       one is not invisible — it is a bare `·` sitting under a
                       control with nothing after it. Emptying the element and
                       keeping the element is how a cut leaves litter.

                       The empty state stays: that one is not narration, it says
                       the form cannot be submitted and why, which is the §4
                       case, and it appears only when it is true. -->
                  {#if rungsChosen.length === 0}
                    <span
                      class="note warn"
                      title="Three rungs are offered — one minute, one day, and the archives' one second — because they are the whole of what the four descriptors in pull::vendor declare between them, and because everything coarser is folded from bars this repository already holds. A rung the active vendor cannot publish at all is not listed: /feeds.json's granularity floor says no pull, entitlement, purchase or code change makes it exist, so there is no work behind the row. A rung cannot be ADDED from here: parse_granularity matches the directory name exactly and refuses anything else."
                    >
                      no timeframe ticked — nothing below can be counted
                    </span>
                  {/if}
                  <!-- WHAT THE SERVER DID NOT SAY, SAID. `history[].served` is
                       the bit `crates/api`'s `served()` gates the POST on; a
                       binary that predates the field answers nothing, and the
                       page reports that rather than assuming every rung is
                       fetched. An absent answer is a state, not a default. -->
                  {#if rungServed === null && active}
                    <span
                      class="note wrap warn"
                      title="GET /feeds.json emits a history array carrying, per rung, whether Descriptor::granularities declares it. This server sent no such array, so this control cannot say which rungs the build actually fetches and does not guess. The server still refuses an undeclared rung by name — it will just be the run that tells you rather than this line."
                    >
                      this server states no per-rung `served` flag — which rungs the build fetches is
                      not known here
                    </span>
                  {/if}
                </div>
              {/if}

              {#if shows.get('window')}
                <!-- DAYS TO PULL — and DAYS is the honest unit. The vendor
                     answers a day range and the wire carries one:
                     `api::ingest::parse_spot` reads `from` and `to` as
                     `YYYY-MM-DD`. A month picker stood here and named a unit of
                     the STORE as if it were the unit of the REQUEST.

                     NOT A NATIVE `<input type="date">`: it renders `02/09/2024`,
                     which is 2 September to one reader and 9 February to
                     another. These are text fields over an ISO value that never
                     leaves ISO, and the ▦ opens the grid. -->
                <div class="field wide">
                  <!-- "DAYS TO PULL" IS GONE AND ONLY THE HEADER WAS.
                       It stacked a group heading above two fields that already
                       carry their own — FROM DATE and TO DATE say what they are,
                       and a third label above them naming the pair is the
                       hierarchy the four rungs beside it do not have. Every
                       other control on this strip is one label over one control.

                       "THE RANGE THE VENDOR ANSWERS" IS A REST FACT AND IS
                       FALSE FOR A FOLDER FEED. There is no vendor in that
                       sentence: the range is the files somebody bought and put
                       in a directory on this machine, and nothing about it is
                       answered by anybody. It moves to the pair it is about
                       rather than being deleted with the heading that carried
                       it — `.dates` is the element that IS the window. -->
                  <div
                    class="dates"
                    title={(isFolderFeed
                      ? 'The range is whatever files are in the folder. '
                      : 'The range the vendor answers. ') +
                      (isBroker
                        ? `${active.display} is a broker. The window is split to its per-request cap by the server and the rate budget is charged per request. Both ends are inclusive here; on the wire crates/pull must send the day after "to", which Dhan documents as non-inclusive.`
                        : 'Both ends are inclusive here and in every count on this page.')}
                  >
                    <div
                      class="dcell field cal-open"
                      class:bad={(showProblems || staleWindow) && problemFor.has('from')}
                    >
                      <span class="dlbl">From date</span>
                      <div class="dwrap">
                        <input
                          class="din mono"
                          type="text"
                          autocomplete="off"
                          spellcheck="false"
                          placeholder="dd Mon yyyy"
                          aria-label={`First day to ${verb}`}
                          aria-invalid={Boolean(fromText.trim()) && !isValidIso(fromDay)}
                          value={fromText}
                          oninput={(e) => typeDay('from', e.currentTarget.value)}
                        />
                        <button
                          type="button"
                          id="cal-from"
                          class="dbtn"
                          aria-haspopup="dialog"
                          aria-expanded={cal.field === 'from'}
                          aria-label="Choose the first day"
                          onclick={() => openCal('from')}>▦</button
                        >
                      </div>
                      {#if cal.field === 'from'}{@render dayGrid()}{/if}
                    </div>
                    <div
                      class="dcell field cal-open right"
                      class:bad={(showProblems || staleWindow) && problemFor.has('to')}
                    >
                      <span class="dlbl">To date</span>
                      <div class="dwrap">
                        <input
                          class="din mono"
                          type="text"
                          autocomplete="off"
                          spellcheck="false"
                          placeholder="dd Mon yyyy"
                          aria-label={`Last day to ${verb}`}
                          aria-invalid={Boolean(toText.trim()) && !isValidIso(toDay)}
                          value={toText}
                          oninput={(e) => typeDay('to', e.currentTarget.value)}
                        />
                        <!-- THE CEILING RIDES THE CONTROL IT BOUNDS. It used to
                             be a paragraph under the form that spoke only when
                             the newest askable day was behind the clock; it now
                             says the same thing on the button that opens the
                             grid, where the reader meets it at the moment it
                             costs him a day, and on every struck cell inside. -->
                        <button
                          type="button"
                          id="cal-to"
                          class="dbtn"
                          aria-haspopup="dialog"
                          aria-expanded={cal.field === 'to'}
                          aria-label="Choose the last day"
                          title={`Newest day that can be asked for: ${dayLabel(maxDay)}. ${ceilReason}`}
                          onclick={() => openCal('to')}>▦</button
                        >
                      </div>
                      {#if cal.field === 'to'}{@render dayGrid()}{/if}
                    </div>
                    <!-- THE RATE, AND ONLY WHERE IT DOES ANYTHING.
                         Drawn when an F&O body is going out and not otherwise:
                         a spot bar has no strike, no expiry and nothing to
                         price, so a rate box beside a spot-only pull would be a
                         control that changes nothing — which is worse than a
                         missing one, because the reader fills it and expects an
                         effect.
                         EMPTY IS THE DEFAULT AND IS A REAL ANSWER. See
                         `rateText`: no page in docs/00-charter.md records a
                         rate, so this build will not supply one, and the
                         receipt says "not computed" rather than quietly using a
                         number nobody chose. -->
                    {#if fnoBodies.length > 0}
                      <div class="dcell field">
                        <span class="dlbl">Risk-free rate</span>
                        <div class="dwrap">
                          <input
                            class="din mono"
                            type="text"
                            autocomplete="off"
                            spellcheck="false"
                            inputmode="decimal"
                            placeholder="leave empty for no greeks"
                            aria-label="Risk-free rate for the greeks, as a decimal"
                            title={'A DECIMAL, NOT A PERCENTAGE: 0.0655 for 6.55%. Anything outside ±1.0 is refused by name, because a percentage typed where a decimal was meant prices every option wrongly and errors nowhere.\n\nLeave it empty and no greek is computed — docs/00-charter.md records no rate, so this build will not invent one. What you type is stored beside every priced row, so the result can be reproduced.'}
                            value={rateText}
                            oninput={(e) => (rateText = e.currentTarget.value)}
                          />
                        </div>
                      </div>
                    {/if}
                  </div>
                  <!-- THE WINDOW LINE SAID WHAT THE TWO FIELDS ABOVE IT AND
                       THE TILE BELOW IT WERE ALREADY SAYING.
                       "06 Jan 2015 – 05 Feb 2016 · 396 day(s) · 14 month
                       file(s) Jan 2015 – Feb 2016" sat between two boxes
                       reading `06 Jan 2015` and `05 Feb 2016`, and the WINDOW
                       tile four inches down reads `396` over `day(s) · 14 month
                       file(s)`. Every clause of it was on screen twice, in the
                       reader's own words, at a larger size.
                       THE TWO REFUSALS STAY. Neither is a restatement: one says
                       the pair cannot make a window, the other that there is no
                       pair yet, and neither is drawn unless it is true. -->
                  {#if !windowOk}
                    <span class="note warn">
                      {#if fromDay && toDay}
                        the two dates are the wrong way round
                      {:else}
                        no window picked
                      {/if}
                    </span>
                  {/if}
                  <!-- WHICH TICKED FEEDS WILL NOT BE ASKED FOR F&O, AND WHY.
                       `fnoDeclined` has existed for a long time carrying a
                       comment that says silently sending fewer requests than
                       the operator ticked is the §4 fallback that hides a
                       failure — and it was NEVER RENDERED. One occurrence in
                       the whole file: its own definition. The list that exists
                       to stop a feed being dropped silently was itself dropped
                       silently, which is how Dhan went unasked for weeks with
                       nothing on screen to say so.
                       `fnoPartial` is the finer case beside it: a feed that is
                       asked for SOME segments. Dhan answers expired options and
                       has no expired-futures series at all, so ticking both
                       sends it one — and that gap is exactly where a silent
                       drop lives. -->
                  {#if fnoDeclined.length > 0 || fnoPartial.length > 0}
                    <!-- `.rows` RATHER THAN AN INLINE `display:flex`, AND THE
                         MARKER IS WHY. `.note.warn` puts its `▲` on the
                         container's `::before`; making that container a COLUMN
                         flex box turns the pseudo-element into a flex item of
                         its own, so the triangle was laid out on its own line
                         with the sentence it belongs to 4px underneath it.
                         Measured on the running page: one orphaned glyph above
                         two lines of amber text.
                         The class moves the marker onto each ROW instead, which
                         is also the more truthful arrangement — every row here
                         is a separate feed's separate refusal, so each one gets
                         its own triangle rather than the group sharing one.
                         Inline styles were the other half of the problem: this
                         was the only `style=` in the block, so the rule could
                         not be seen from the stylesheet that owns `.note`. -->
                    <div class="note warn rows">
                      {#each fnoPartial as p (p.feed)}
                        <span><strong>{p.feed}</strong> {p.why}</span>
                      {/each}
                      {#each fnoDeclined as d (d.feed)}
                        <span><strong>{d.feed}</strong> {d.why}</span>
                      {/each}
                    </div>
                  {/if}
                  <!-- WHAT ACTUALLY GOES OUT, WHEN IT IS NOT WHAT IS ABOVE.
                       The two boxes are the operator's intent and are the same
                       for every ticked feed. A feed cannot always answer for
                       all of it — Dhan rolls five years back, Groww starts at a
                       fixed 01 Jan 2020, Zerodha rolls ten — so each request is
                       clamped to its own vendor's floor before it is sent.
                       IT ONLY EVER NARROWS. A window inside every ticked feed's
                       reach — a week, a day, a month — goes out exactly as
                       typed, `narrowings` is empty, and this line does not draw.
                       When it does draw it is because the request differs from
                       the form, which §4 does not allow to be silent. -->
                  {#if narrowings.length > 0}
                    <span
                      class="note warn wrap"
                      title="Each floor is declared in pull::vendor with the operator's own observation as its source, and the server clamps to the same floors again on arrival — this states the narrowing, it does not perform it alone."
                    >
                      asked from {dayLabel(from)}; {n(narrowings.length)} request(s) start later
                      because their feed answers no earlier —
                      {narrowings.map((w) => `${w.feed} · ${w.rung} from ${dayLabel(w.at)}`).join(', ')}
                    </span>
                  {/if}

                  <!-- ══ THE WINDOW, DRAWN ══
                       The sentence above names a date; this names a PROPORTION,
                       which is the thing a reader can act on. Six and a half
                       years of dead track under a window opening 01 Jan 2015 is
                       a picture the prose cannot make.
                       Not a control: there is nothing to click, and the two date
                       fields directly above are what change it. -->
                  {#if reachLanes.length > 0}
                    <div class="reach">
                      {#each reachLanes as lane (lane.key)}
                        <div class="lane">
                          <span class="lname" title={`${lane.feed} · ${lane.rung}`}
                            >{lane.feed} · {lane.rung}</span
                          >
                          <span
                            class="ltrack"
                            role="img"
                            aria-label={lane.whole
                              ? `${lane.feed} at ${lane.rung} answers for the whole window.`
                              : `${lane.feed} at ${lane.rung} answers for ${Math.round(lane.livePct)}% of the window, from ${dayLabel(lane.at)}. The earlier ${Math.round(lane.deadPct)}% returns nothing.`}
                          >
                            {#if lane.deadPct > 0}
                              <i class="ldead" style="width:{lane.deadPct}%"></i>
                            {/if}
                            <i class="llive" style="width:{lane.livePct}%"></i>
                          </span>
                          <span class="lat" class:dim={lane.whole}>
                            {lane.whole ? 'whole window' : dayLabel(lane.at)}
                          </span>
                          <!-- WHAT IS ON DISK FOR THIS FEED, AND THE THREE
                               STATES ARE NOT TWO. A store that cannot be READ
                               is broken and no pull will fix it; a store that
                               reads and holds nothing has simply never been
                               asked; a store with bars in it has been. Those
                               call for three different actions, so they get
                               three different words. -->
                          <span
                            class="lhold"
                            class:down={lane.broken !== null}
                            class:up={lane.broken === null && lane.bars > 0}
                            title={lane.broken !== null
                              ? `${lane.feed}'s store could not be read: ${lane.broken}. That is a broken store rather than an empty one, and a pull will not settle it.`
                              : lane.bars > 0
                                ? `${lane.feed}'s store holds ${n(lane.bars)} bar(s) across every instrument, timeframe and month it has — NOT only this window. What this window holds is the census below, and that is scoped to the feed the counts are stamped with.`
                                : `${lane.feed}'s store reads cleanly and holds nothing at all. Nothing has been asked of this vendor yet.`}
                          >
                            {lane.broken !== null
                              ? 'store unreadable'
                              : lane.bars > 0
                                ? `${n(lane.bars)} held`
                                : 'nothing held'}
                          </span>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>

                {#if isFolderFeed}
                  <div class="field" class:bad={showProblems && problemFor.has('folder')}>
                    <span class="lab">Folder</span>
                    <input
                      class="din mono"
                      bind:value={folder}
                      placeholder={folderReach.body?.path ?? '/path/to/vendor/csvs'}
                      title="CSV files already bought. No token, no rate budget and no 'not today' — a file's last day is not a question about the clock."
                    />
                    <!-- WHERE THE REACH WAS READ, WHICH IS THE FOLDER THIS
                         BUILD RESOLVES ON ITS OWN.
                         `pull::folder::root` reads BRUTEX_ARCHIVES, else
                         $HOME/.brutex/vendor-data — the same two-step the store
                         and the masters roots use, with no literal path in any
                         tracked file. The box above is still what the REQUEST
                         carries, so the two are shown together rather than
                         assumed equal: a reach read from one folder beside a
                         run against another is exactly the silent disagreement
                         this page exists to surface. -->
                    <span class="note quiet">no token · no rate budget</span>
                    <!-- THE PATH WRAPS. A `.note` clips and ellipses by
                         default, and a PATH with its end cut off is worse than
                         no path at all — it looks like an answer and cannot be
                         acted on. Same defect the `.wrap` modifier exists for
                         one comment above. -->
                    {#if folderReach.body?.path}
                      <!-- `full`: a PATH is the one caption on this strip that
                           may not be clipped to the uniform block, for the
                           reason the comment above already gives — an ellipsed
                           path looks like an answer and cannot be acted on. -->
                      <span class="note quiet wrap full">
                        resolved: <button
                          class="pathbtn mono"
                          type="button"
                          onclick={() => {
                            // GUARDED, like the placeholder two lines above
                            // already is. Without it a resolved-path button
                            // rendered before the probe answered would assign
                            // `undefined` into the folder box and the archive
                            // path would silently become empty.
                            if (folderReach.body?.path) folder = folderReach.body.path;
                          }}
                          title="Use the folder this build resolves on its own — BRUTEX_ARCHIVES, else $HOME/.brutex/vendor-data, one subfolder per feed. Resolved exactly the way the store and masters roots are."
                          >{folderReach.body.path}</button
                        >
                      </span>
                    {/if}
                  </div>
                {:else if sourceKindUnstated}
                  <!-- THE FORM'S SHAPE IS THE ANSWER TO A QUESTION THIS SERVER
                       DID NOT ANSWER, so the shape is refused rather than
                       guessed. A folder field drawn here for a broker asks for
                       a path nothing will read; one omitted for an archive
                       hides the only input that feed has. §4: name the
                       failure. -->
                  <p class="caution loud" role="alert">
                    <span class="tag warn">source kind not stated</span>
                    <span class="msg">{SOURCE_KIND_UNSTATED}</span>
                  </p>
                {/if}
              {/if}

              <!-- THE ASK TILES ARE GONE, AT THE OPERATOR'S INSTRUCTION.
                   Asked / Held / Instruments / Window stood here as a four-tile
                   row. Two of the four restated controls that are three inches
                   above them — Instruments is the tick count the menu's own face
                   carries, and Window is the two date fields subtracted — and
                   the row was the last thing between the form and the button.
                   WHAT LEAVES WITH THEM, STATED RATHER THAN LOST: `Asked` was
                   the only rendering of the estimated instrument-month product,
                   and `Held` the only AGGREGATE of the store reading. The
                   per-series census below still measures both per instrument —
                   `BARS STORED / EXPECTED` and `MONTHS UNPROVED` — and its
                   header still names the window, the sessions and the series
                   count, so nothing here is the only place a number exists. -->

              <!-- ══ THE TWO BULK ACTIONS ARE GONE, AT THE OPERATOR'S
                   INSTRUCTION ══
                   "Clear all instruments" unticked all fifty — the instruments
                   menu has its own "Clear all" inside it, which is the same act
                   next to the thing it acts on. "Delete all stored bars" was
                   permanently disabled and always would be: there is no DELETE
                   route in crates/api at all and §3 rule 8 makes a month file
                   unmutatable in place and unremovable. It stood as a visible
                   absence; the absence is now stated here instead, where it
                   costs no control on the form. -->
            </fieldset>

            <!-- THE LOCK NAMES ITSELF. `<fieldset disabled>` greys every control
                 above at once and says nothing about why, so a form that has
                 gone flat looks broken rather than busy — the spinner is on a
                 button far below the controls it explains. It is not a refusal
                 to fix, so it is stated once and calmly. -->
            {#if phase === 'running'}
              <!-- ONE CLAUSE, NOT THE PARAGRAPH IT WAS. `<fieldset disabled>`
                   greys every control at once and says nothing about why, so a
                   form gone flat looks broken rather than busy — but the reason
                   is one line and the rest of it is a note about how the route
                   works, which belongs on the line rather than under it. -->
              <p
                class="hint"
                title="The request is one synchronous POST already on the wire and no route amends it, so an edit now could only change what a NEXT run asks for. The outcome list below is a difference measured against the store as it stood when this one left."
              >
                <span class="tag acc">locked</span>
                Busy, not broken — every control above answers again when this run does.
              </p>
            {/if}

<!-- A GATED FORM STATES ITS BLOCKER WITHOUT BEING ASKED. The field-level
                 messages wait for a submit, because telling someone their date is
                 missing before they have opened the calendar is nagging. A retracted
                 rung is not nagging: the controls below it have just left the page,
                 and the reason has to be on screen the moment they do. -->
            {#if (showProblems || gate || staleWindow) && problems.length > 0}
              <ul class="probs panel-in" aria-live="polite">
                {#each problems as p (p.field + p.why)}
                  <li>
                    <span class="tag down">{p.field}</span>
                    <span class="msg">{p.why}</span>
                    <!-- A REFUSAL THAT KNOWS THE ANSWER SHOULD OFFER IT. Every
                         sentence here already names the day to move to; making
                         the reader retype it into the field above is the whole
                         of what stood between him and a legal window. Only the
                         refusals with ONE reading carry a button — see `fix` in
                         `problems`. -->
                    {#if p.fix}
                      <button type="button" class="btn sm pfix" onclick={p.fix.run}>
                        {p.fix.label}
                      </button>
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}

            <div class="actions">
              <button class="btn primary" type="submit" disabled={phase === 'running'}>
                {#if phase === 'running'}
                  <span class="spin ring" aria-hidden="true"></span> Running…
                {:else}
                  <!-- ONE WORD, AT THE OPERATOR'S INSTRUCTION. It read
                       "Start broker pull" -- a verb, a noun and a second verb
                       for a button with one job.

                       IT IS STILL THE SOURCE KIND'S WORD AND NOT THIS BUTTON'S,
                       and that is why this is `{Verb}` rather than the literal
                       "Pull" that was asked for. A broker is PULLED over the
                       network; a folder of bought files is READ off a disk, and
                       hardcoding "Pull" would print the wrong diagnostic frame
                       on the control that starts it -- the exact defect the
                       comment this replaces was written to record. `verb` comes
                       from /feeds.json, which takes it from
                       pull::vendor::SourceKind::verb, so a broker reads "Pull"
                       and an archive reads "Read". -->
                  {Verb}
                {/if}
              </button>
              {#if phase === 'running'}
                <!-- STOP IS A REAL CONTROL NOW, AND IT HAS TO BE.
                     `stopWatching` existed before this and had NO call site —
                     harmless while the run lived in this tab, because closing
                     the tab ended it. The run is a task on the server now, and
                     a failed pass retries WITHOUT limit, so without this button
                     the only way to end one is to restart the process.
                     It tells the server BEFORE it stops watching: aborting the
                     poll alone would end the looking and leave the run going,
                     which is worse than offering no button at all. -->
                <button class="btn ghost" type="button" onclick={stopWatching}>Stop</button>
              {/if}
              {#if phase === 'done'}
                <button class="btn ghost" type="button" onclick={reset}>Clear the result</button>
              {/if}

            </div>
            <!-- WHAT THE MULTI-PASS RUN DID, AND WHY IT STOPPED.
                 One press is now several passes — a window can be bigger than
                 one sitting at a legal rate, and the operator's requirement is
                 that it keeps going until the window is satisfied. Without this
                 line the difference between "finished", "hit the ceiling" and
                 "gained nothing twice" is invisible, and those three want three
                 different next actions from the reader.

                 THE SENTENCE IS THE SERVER'S NOW, not this page's. It comes
                 from `pullrun::summary_of` on the last poll, because the server
                 is what counted the passes. This page only shows it. -->
            <!-- WHAT THE RUN IS DOING RIGHT NOW, PER FEED.
                 The run moved to the server and the page stopped being able to
                 show per-leg receipts, because it no longer makes the per-leg
                 requests. Without this block a run reports NOTHING until it
                 ends — the operator presses Pull, the store fills for twenty
                 minutes, and the page says nothing at all. That is a report
                 with nothing behind it in the other direction, and it is the
                 first thing an operator asks about. -->
            {#if runState && (runState.running || runState.passes > 0)}
              <div class="runcard">
                <div class="runtop">
                  <span><b>{n(runState.rowsNow - runState.rowsAtStart)}</b> bar(s) landed</span>
                  <span>pass <b>{n(runState.passes)}</b></span>
                  {#if runState.retries > 0}<span><b>{n(runState.retries)}</b> retried</span>{/if}
                  <span class="runwhere">{runState.running ? (runState.stopping ? 'stopping at the next leg' : 'running on the server') : 'finished'}</span>
                </div>
                <table class="runfeeds">
                  <tbody>
                    {#each runState.feeds ?? [] as f (f.vendor)}
                      <tr>
                        <td class="rf-v">{feedName(f.vendor)}</td>
                        <td class="rf-n">{n(f.legsDone)}/{n(f.legs)}</td>
                        <td class="rf-d">{f.finished ? '—' : (f.doing || 'waiting for its turn')}</td>
                        <td class="rf-e">{f.lastError ? 'retrying after a failure' : ''}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {/if}
            {#if passSummary}

              <p class="note" style="margin-top:10px">{passSummary}</p>
            {/if}
          </form>
        </section>

        <!-- ================================================ THE RUN ======= -->
        <!-- IT IS NOT ON THE PAGE UNTIL THERE IS A RUN.
             Idle, the whole card said one thing: that nothing was running and
             where the progress WOULD be measured from. A panel whose only
             content is why it is empty is a panel that costs half the width of
             the form and answers nothing he asked. It comes back the instant
             `phase` leaves `idle` and stays for the whole of `done`, so the
             receipt, the facts table and the elapsed clock are never hidden —
             `reset` is the only thing that takes it off again, and that is a
             press. -->
        <!-- REMOVED: THE RUN card — elapsed, store growth, the share estimate,
             the request ladder and the per-rung outcome box. Every one of those
             is narrative about the run; the outcome per instrument is a COLUMN
             in the results table below, which is where it belongs. -->

        <!-- ══ THE BAND UNDER THE PANEL IS GONE, AND WHAT IT HELD IS RECORDED ══

             `.belowsel` was a full-width row beneath the form, and by the time
             it was measured it contained no elements at all — only the notes
             below. Rendered: zero children, zero pixels tall, with a class, a
             grid span and a flex column still describing it in the stylesheet.
             An empty box that is styled and documented reads to the next person
             as a slot waiting to be filled rather than as a thing that ended.

             WHAT LEFT, IN THE ORDER IT LEFT, because each was a decision:

               * The four-factor sentence — where the ticked count and the
                 reachable count differ, the difference stated in words. It sat
                 in a 248px track and wrapped into a ribbon.

               * `askGap`'s banner, saying the request cannot be narrowed to the
                 ticked instruments. True, and a permanent property of
                 `api::ingest::SpotRequest` for as long as this route exists — a
                 fact about an API drawn as a yellow alert on every load is not a
                 warning, it is furniture. It moved into `cautions`, where its
                 count is visible and its sentence is one click away.

               * The caution disclosure. Every one of those is a real caution and
                 §4 requires each to be named — what §4 does not require is all
                 of them open at once above the button. Five stacked yellow
                 blocks is how a page teaches an operator to scroll past yellow
                 blocks, and then the sixth is the one that mattered. The COUNT
                 stays visible; the sentences are one click away.

               * "What goes on the wire" — the request body, collapsed.

             Nothing above is lost and nothing was summarised; all of it lives in
             `cautions` and the wire fold. Only the emptied container is gone. -->
      </div>

      <!-- ============================================== THE OUTCOMES ====== -->
      <!-- IT ARRIVES WITH THE RUN AND STAYS UNTIL THE RESULT IS CLEARED.
           Idle, this whole card was a promise: a header, a border and a
           sentence saying rows would appear here once a run had happened. The
           table itself is the point of the page and is never touched — what
           left is the placeholder standing in for it. The two branches that
           SAY something still do: "not built yet" while the second store
           reading is outstanding, and the diagnostic when a finished run
           produced no rows at all. -->
      <!-- REMOVED, AT THE OPERATOR'S INSTRUCTION: the whole "Every instrument
           this request named" card. It drew only after a run and carried a
           verdict strip, its own symbol filter, a sort bar and a virtualised
           outcome list -- 191 lines between the form and the census.

           WHAT LEAVES WITH IT, STATED: the per-instrument outcome of a run
           (which instrument gained how many bars, and why), the second
           search box on this page, and the group filter over it. The run's
           receipt is untouched and still names the verdict; the census below
           still measures what the store holds, per series, and it is the
           surface that outlives the run. -->

      <!-- ══════════════════════════ THE CENSUS ══════════════════════════
           EVERY SERIES IN THE WINDOW, AND WHAT THE STORE ACTUALLY HOLDS FOR
           IT. There was no such surface here at all: the form asked for a
           window and nothing on screen said whether that window was already
           filled, so the only way to find out was to start a pull.

           EVERY NUMBER IS MEASURED and the header of each column says its
           unit. Stored comes from /store.json, expected from the NSE calendar
           over the days INSIDE the window, in flight from /ingest/status.json,
           and out-of-reach from the same feed floor that refuses a day in the
           date control above. Nothing here is generated and nothing is
           extrapolated. -->
      <!-- ══ NOTHING BELOW EXISTS UNTIL THERE IS A WINDOW TO COUNT ══

           This section drew on every load. With no dates picked it rendered a
           heading, a verdict strip reading "No window", a paginated table with
           five column headers and their unit captions, an empty body repeating
           the same sentence, a pager reading "0 of 0 series", and two footer
           paragraphs defining terms for numbers that were not on screen —
           SEVEN blocks, all of them saying the same thing: you have not picked
           a window yet.

           The date field says that already, in three words, where the window
           is chosen. A census of nothing is not a census, and drawing its
           chrome so it can announce its own emptiness is the noise that made
           this page unreadable before it was ever used.

           WHEN THERE IS A WINDOW, EVERY WORD OF IT COMES BACK. Nothing here is
           deleted and no refusal is hidden: this is a section that has nothing
           to report until it does. -->
      {#if windowOk}
      <section class="census">
        <!-- ══ THE HEADING AND THE TWO STRIPS ARE GONE, AT THE OPERATOR'S
             INSTRUCTION ══
             What stood between the form and the table: a heading restating the
             window the two date fields above already hold, a verdict strip of
             filter pills with a bulk pull button, and a bar naming when the
             store was read. Three bands of chrome over a table that says all of
             it per row — INSTRUMENT, BARS STORED / EXPECTED, MONTHS UNPROVED,
             VERDICT, NEXT STEP.
             WHAT LEAVES WITH THEM, STATED: the verdict FILTER (clicking a pill
             narrowed the table to one verdict), the "Pull the N month file(s)"
             bulk action, and the "Re-read the store" press. The per-row NEXT
             STEP button still pulls, and the form above still runs the whole
             window; the reading still refreshes when a run finishes. -->

        <!-- ══ THE PARTITION. A READING, AND DELIBERATELY NOT A CONTROL ══

             WHAT THE TABLE CANNOT SAY. A table answers "what is this series",
             one row at a time, and the reader is looking at 25 of 50. It cannot
             answer "how much of this window is settled" — that is a property of
             the whole set. The pager states the arithmetic of the VIEW; this
             states the arithmetic of the ANSWER.

             NOTHING HERE IS CLICKABLE, AND THAT IS THE WHOLE BOUNDARY. The
             verdict strip that stood here before carried FILTER PILLS and a
             bulk-pull button, and both were removed at the operator's
             instruction. They do not come back. This page already holds a
             hard-won rule — one control chooses what is PULLED and it is the
             tick list; the search box chooses what is DRAWN — written after the
             button read `1 of 213 ticked`, the receipt read `ASKED 1`, and the
             run pulled 213. A clickable chip would be a third control narrowing
             one question, which is how those three answers came apart.

             COUNTED OVER THE WHOLE WINDOW, NOT OVER WHAT IS DRAWN. See
             `verdictTally`: a summary that shrank as you typed in the search box
             would answer a different question from the table beneath it.

             ONE `role="img"` WITH THE WHOLE SENTENCE ON IT. Seven coloured
             segments are a picture; read one at a time by a screen reader they
             are seven unlabelled boxes. The label carries the same reading the
             chips carry, so nothing here is available only to a sighted reader.
        -->
        {#if verdictTotal > 0}
          <div class="tally">
            <div
              class="tallybar"
              role="img"
              aria-label={`${n(verdictSettled)} of ${n(verdictTotal)} series need nothing. ` +
                verdictTally.map((s) => `${n(s.n)} ${s.label}`).join(', ') + '.'}
            >
              {#each verdictTally as seg (seg.k)}
                <span
                  class="tseg {seg.tone}"
                  style="width:{seg.pct}%"
                  title={`${n(seg.n)} of ${n(verdictTotal)} series — ${seg.label}. ${seg.why}`}
                ></span>
              {/each}
            </div>
            <div class="tchips">
              {#each verdictTally as seg (seg.k)}
                <!-- THE COUNT AND THE WORD ARE ONE TEXT NODE, NOT TWO FLEX
                     ITEMS. Separated by a `gap` they looked right and read
                     wrong: the flex gap is layout, not content, so the chip's
                     text was "50never pulled" — one token to anything reading
                     the DOM rather than looking at it. The space is a real
                     space now, and the gap only separates the dot from the
                     words it belongs to. -->
                <span class="tchip" title={`${seg.label} — ${seg.why}`}>
                  <i class="dot {seg.tone}"></i>
                  <span class="tword"><b>{n(seg.n)}</b> {seg.label}</span>
                </span>
              {/each}
              <span class="spacer"></span>
              <span class="tsum" title="Settled means verified or out of reach — the two verdicts no pull would change. Everything else is work this window still owes.">
                <b>{n(verdictSettled)}</b> of {n(verdictTotal)} settled
              </span>
            </div>
          </div>
        {/if}

        <div class="cgrid">
          <!-- THE SEARCH. One control, no caption: the count it changes is
               already in the pager at the foot, and a line here restating it
               would be the narration this page had removed from it.

               IT NARROWS THE VIEW AND IT MUST NOT NARROW THE ASK -- and the
               reason is NOT that the request cannot carry names. It can:
               `SpotRequest.members` exists and the tick list above already
               fills it. That is exactly why this box must not: two controls
               narrowing one ask is how they drift apart, which is the defect
               `members` was added to FIX -- the button said `1 of 213 ticked`,
               the receipt said `ASKED 1` and the run pulled 213.
               One control chooses what is PULLED and it is the tick list. This
               one chooses what is DRAWN. The pager keeps the unfiltered total
               beside the filtered one, because a table that got shorter after
               typing looks exactly like a request that got smaller. -->
          <div class="cfind">
            <input
              class="search"
              type="search"
              bind:value={cQuery}
              oninput={() => (cPage = 1)}
              placeholder="Find among {n(censusNames)} name(s) in the census — any part of the symbol"
              aria-label="Find a trading symbol in the census"
              title={`Narrows what this table DRAWS and nothing else. The ask above is unchanged by anything typed here, and the pager keeps the unfiltered total beside the filtered one. What narrows a PULL is the instrument tick list — it sends one member= per name and the server filters on it — and this box deliberately does not, because two controls narrowing one request is how the button, the receipt and the run come to give three answers to one question. Matches ANY PART of a symbol, not just the start: BANK finds AXISBANK, HDFCBANK, ICICIBANK and KOTAKBANK as well as BANKNIFTY — the outcome list above, after a run, is a different table and matches from the START instead. ${n(censusNames)} name(s) across ${n(censusRows.length)} series in this window.`}
            />
          </div>
          <div class="cscroll">
            <table>
              <thead>
                <tr>
                  <th>
                    <button class="sort" type="button" onclick={() => sortCensus('sym')}>
                      <span class="hrow"
                        >Instrument{cSort.key === 'sym' ? (cSort.dir > 0 ? ' ▲' : ' ▼') : ''}</span
                      >
                      <span class="hsub">NSE trading symbol</span>
                    </button>
                  </th>
                  {#if segmentsReached.length > 1}
                    <th>
                      <button class="sort" type="button" onclick={() => sortCensus('seg')}>
                        <span class="hrow"
                          >Segment{cSort.key === 'seg' ? (cSort.dir > 0 ? ' ▲' : ' ▼') : ''}</span
                        >
                        <span class="hsub">what the route fills</span>
                      </button>
                    </th>
                  {/if}
                  {#if rungsChosen.length > 1}
                    <th>
                      <button class="sort" type="button" onclick={() => sortCensus('tf')}>
                        <span class="hrow"
                          >Timeframe{cSort.key === 'tf' ? (cSort.dir > 0 ? ' ▲' : ' ▼') : ''}</span
                        >
                        <span class="hsub">one bar covers</span>
                      </button>
                    </th>
                  {/if}
                  <th class="num">
                    <button
                      class="sort"
                      type="button"
                      title="Stored is counted from /store.json. Expected is the NSE sessions inside the chosen window times the rung's bars per session — weekends and the holidays this page carries excluded. A rung with no stated bars-per-session has no expectation and prints a dash."
                      onclick={() => sortCensus('got')}
                    >
                      <span class="hrow"
                        >Bars stored / expected{cSort.key === 'got'
                          ? cSort.dir > 0
                            ? ' ▲'
                            : ' ▼'
                          : ''}</span
                      >
                      <span class="hsub">bars, from the NSE calendar</span>
                    </button>
                  </th>
                  <th class="num">
                    <button class="sort" type="button" onclick={() => sortCensus('unproved')}>
                      <span class="hrow"
                        >Months unproved{cSort.key === 'unproved'
                          ? cSort.dir > 0
                            ? ' ▲'
                            : ' ▼'
                          : ''}</span
                      >
                      <span class="hsub">of {n(windowMonths.length)} in the window</span>
                    </button>
                  </th>
                  <!-- NOT SORTABLE, AND THAT IS NOT AN OMISSION. Every other
                       header here is a button because its column holds ONE
                       value that a run can be ordered by. This column holds
                       four, and any single key you sorted it on — worst block,
                       first bad block, count of bad blocks — would be a
                       different column from the one being drawn. `Months
                       unproved` beside it already sorts on the number this
                       summarises. -->
                  <th>
                    <span class="hrow">Coverage</span>
                    <span class="hsub">oldest → newest</span>
                  </th>
                  <th>
                    <button
                      class="sort"
                      type="button"
                      title={`Worst first, and the order is a claim about what you have to do: ${VORDER.map((k) => VERDICT[k][1]).join(' → ')}. Not alphabetical — the label is prose and prose has no order.`}
                      onclick={() => sortCensus('state')}
                    >
                      <span class="hrow"
                        >Verdict{cSort.key === 'state' ? (cSort.dir > 0 ? ' ▲' : ' ▼') : ''}</span
                      >
                      <span class="hsub">measured, not reported</span>
                    </button>
                  </th>
                  <!-- ══ ATTEMPTS — AND IT IS HONEST ABOUT HAVING ALMOST
                       NOTHING TO SAY ══
                       The column answers "how many times was this asked for",
                       and the answer for a manual pull is ALWAYS ONE, because
                       /pull/spot makes one attempt per cell and counts none.
                       Only the sweep counts attempts, and only for the month it
                       is on right now — `attempts` / `attempts_max` off
                       /ingest/status.json, which this page already polls.

                       SO THE EMPTY CELL SAYS THE FINDING RATHER THAN A DASH.
                       "1 · not counted" is the fact: one ask was made and
                       nothing is keeping score, so a row that came back empty
                       cannot be told apart from a row that was never reachable.
                       A bare `—` would read as "no data available", which is
                       the §4 silence — this is the same absence, named.

                       NOT SORTABLE, deliberately. Every other header here ranks
                       a number that differs per row; this one is the same value
                       on every row but the one the sweep is holding, and a sort
                       control that cannot reorder anything is a control that
                       lies about what it does. -->
                  <!-- NOT `.num`. `.num` right-aligns a column because a
                       COLUMN OF FIGURES is compared down its last digit —
                       `0 / 4,500` under `0 / 4,500`. This column holds a
                       phrase, "1 · not counted", and right-aligning a phrase
                       between two left-aligned neighbours is what put a band of
                       air between Verdict and Next step. It reads left, with
                       the two text columns it belongs to. -->
                  <th>
                    <span class="hrow">Attempts</span>
                    <span class="hsub">asked, and counted</span>
                  </th>
                  <th>
                    <button
                      class="sort"
                      type="button"
                      title="Ranks by the number the button prints: how many month files inside the window are neither settled nor out of reach."
                      onclick={() => sortCensus('act')}
                    >
                      <span class="hrow"
                        >Next step{cSort.key === 'act' ? (cSort.dir > 0 ? ' ▲' : ' ▼') : ''}</span
                      >
                      <span class="hsub">pull, or nothing</span>
                    </button>
                  </th>
                </tr>
              </thead>
              <tbody>
                {#if censusSlice.length === 0}
                  {@const why = censusBlocker()}
                  <tr>
                    <td colspan={censusColCount}>
                      <div class="empty">
                        <b>{why[0]}</b>
                        {why[1]}
                      </div>
                    </td>
                  </tr>
                {:else}
                  {#each censusSlice as r (r.id)}
                    <tr>
                      <td>
                        <b class="mono">{r.sym}</b>
                        <span class="kindtag">{r.kind}</span>
                      </td>
                      {#if segmentsReached.length > 1}
                        <td class="mono">{r.segLabel}</td>
                      {/if}
                      {#if rungsChosen.length > 1}
                        <td class="mono">{r.tfLabel}</td>
                      {/if}
                      <td
                        class="num mono"
                        title={r.per === null
                          ? `${r.tfLabel} has no bars-per-session this page can state, so ${n(r.got)} stored bar(s) are counted and nothing is claimed about what they should be.`
                          : `${n(r.got)} stored against ${n(r.exp)} expected — ${n(windowSessions)} NSE session(s) inside the window × ${n(r.per)} bar(s) per session, over ${n(r.months.length)} month file(s).`}
                      >
                        <span class:up={r.per !== null && r.got >= r.exp} class:warn={r.per !== null && r.got < r.exp}
                          >{n(r.got)}</span
                        >
                        / {r.per === null ? '—' : n(r.exp)}
                        <!-- THE RATIO THE TWO NUMBERS ALREADY STATE, AS A
                             LENGTH. `0 / 11,29,875` and `11,04,320 / 11,29,875`
                             are both two long tabular numbers, and telling them
                             apart means reading eight digits and dividing. The
                             fill is that division, done once, at the width the
                             eye reads without counting.
                             DRAWN ONLY WHEN THERE IS A DENOMINATOR: `r.per` is
                             null for a rung with no bars-per-session this page
                             can state, and a bar against an unknown total would
                             be inventing the very yardstick the `no yardstick`
                             verdict exists to refuse. -->
                        {#if r.per !== null && r.exp > 0}
                          <i
                            class="fill"
                            class:up={r.got >= r.exp}
                            style="width:{Math.min(100, (r.got / r.exp) * 100)}%"
                          ></i>
                        {/if}
                      </td>
                      <td
                        class="num mono"
                        class:warn={r.unproved > 0}
                        class:up={r.unproved === 0}
                        title={r.unproved > 0
                          ? `${n(r.unproved)} of the ${n(r.months.length)} month file(s) in the window are not settled: ${r.months.filter((m) => m.k !== 'ok' && m.k !== 'beyond').map((m) => `${monthLabel(m.month)} ${VERDICT[m.k][1]}`).join(', ')}.`
                          : `All ${n(r.months.length)} month file(s) in the window are settled — matched the calendar, or out of this feed's reach.`}
                      >
                        {n(r.unproved)}/{n(r.months.length)}
                      </td>
                      <td>
                        <!-- ONE `Map.get`, NOT A WALK. `coverage` was built for
                             the drawn page; this cell only reads it. -->
                        <span
                          class="cov"
                          role="img"
                          aria-label={`Coverage, oldest to newest: ${(coverage.get(r.id) ?? []).map((q) => q.label).join(', ')}.`}
                        >
                          {#each coverage.get(r.id) ?? [] as q (q.from)}
                            <i
                              class="q {q.tone}"
                              title={`${monthLabel(q.from)}${q.n > 1 ? ` – ${monthLabel(q.to)}` : ''} — ${q.n} month file(s), worst verdict ${q.label}.`}
                            ></i>
                          {/each}
                        </span>
                      </td>
                      <td>
                        <span
                          class="tag"
                          class:up={VERDICT[r.state][0] === 'up'}
                          class:down={VERDICT[r.state][0] === 'down'}
                          class:warn={VERDICT[r.state][0] === 'warn'}
                          class:info={VERDICT[r.state][0] === 'info'}
                          title={VERDICT[r.state][2]}>{VERDICT[r.state][1]}</span
                        >
                      </td>
                      <!-- THE ONLY ROW WITH A REAL COUNT IS THE ONE THE SWEEP IS
                           HOLDING. `r.state === 'retry'` is set from
                           /ingest/status.json naming this instrument in flight,
                           and the same answer carries `attempts` and
                           `attempts_max` for the feed it is on — so where the
                           count exists it is READ, and where it does not the
                           cell says why instead of drawing a dash. -->
                      <td class="mono">
                        {#if r.state === 'retry' && feedLadder}
                          <span
                            class="info"
                            title={`The sweep is on this instrument now — attempt ${n(feedLadder.attempts)} of ${n(feedLadder.attempts_max)} for ${feedLadder.month}. Read from /ingest/status.json.`}
                            >{n(feedLadder.attempts)} / {n(feedLadder.attempts_max)}</span
                          >
                        {:else}
                          <span
                            class="dash"
                            title="One. A pull from the form above asks for each instrument-month exactly once and keeps no attempt count, so a series that came back empty cannot be told apart here from one that was never reached. Only the sweep counts attempts, and only for the month it is holding."
                            >1 · not counted</span
                          >
                        {/if}
                      </td>
                      <td>
                        {#if r.unproved === 0}
                          <span
                            class="dash"
                            title={r.state === 'beyond'
                              ? VERDICT.beyond[2]
                              : 'Nothing to ask for: every month file in this window is settled.'}
                            >—</span
                          >
                        {:else}
                          {@const sp = shortSpan(r)}
                          <button
                            class="btn sm"
                            type="button"
                            disabled={phase === 'running' || problems.length > 0 || sp === null}
                            title={phase === 'running'
                              ? 'A pull is already on the wire — /pull/spot is synchronous and this page sends one at a time.'
                              : problems.length > 0
                                ? `The request above has ${n(problems.length)} thing(s) to fix first: ${problems[0].why}`
                                : `POST /pull/spot — target=${target}, vendor=${feeds.active}, granularity=${r.tf}, from=${sp?.from}, to=${sp?.to}. That is ${dayLabel(sp?.from ?? '')} – ${dayLabel(sp?.to ?? '')}: the days covering the ${n(r.unproved)} unsettled month file(s) of this series, clipped to the window you chose. It is NOT narrowed to ${r.sym}: this body carries whatever is ticked above, so the server answers for all of those. That is this page's limit and not the route's — SpotRequest carries a member set and broker_run filters on it; pullRow simply reuses the ticked body. What this DOES narrow is the window and the rung, and the outcome list below is built for this row.`}
                            onclick={() => pullRow(r)}
                          >
                            Pull {n(r.unproved)}
                          </button>
                        {/if}
                      </td>
                    </tr>
                  {/each}
                {/if}
              </tbody>
            </table>
          </div>

          <!-- THE PAGER. Six controls and the count they move through. Every
               disabled one says on its face why it is disabled. -->
          <div class="pager">
            <span class="of">
              <b class="mono"
                >{censusSorted.length === 0
                  ? '0'
                  : `${n((censusPage - 1) * PAGE_SIZE + 1)}–${n(Math.min(censusPage * PAGE_SIZE, censusSorted.length))}`}</b
              >
              of {n(censusSorted.length)} series{cQuery.trim()
                ? ` matching “${cQuery.trim()}” · ${n(censusRows.length)} in all`
                : ''}
            </span>
            <span class="pages">
              <button
                class="pg"
                type="button"
                disabled={censusPage === 1}
                title={censusPage === 1 ? 'First page — already on it' : 'First page'}
                onclick={() => (cPage = 1)}>«</button
              >
              <button
                class="pg"
                type="button"
                disabled={censusPage === 1}
                title={censusPage === 1 ? 'Previous page — this is the first' : 'Previous page'}
                onclick={() => (cPage = censusPage - 1)}>‹</button
              >
              {#each pageList(censusPage, censusPages) as it (it)}
                {#if typeof it === 'string'}
                  <span class="pg gap" title="Pages between the two shown">…</span>
                {:else}
                  <button
                    class="pg"
                    type="button"
                    class:on={it === censusPage}
                    aria-current={it === censusPage ? 'page' : undefined}
                    title={it === censusPage ? `Page ${n(it)} — you are on it` : `Page ${n(it)}`}
                    onclick={() => (cPage = it)}>{n(it)}</button
                  >
                {/if}
              {/each}
              <button
                class="pg"
                type="button"
                disabled={censusPage === censusPages}
                title={censusPage === censusPages ? 'Next page — this is the last' : 'Next page'}
                onclick={() => (cPage = censusPage + 1)}>›</button
              >
              <button
                class="pg"
                type="button"
                disabled={censusPage === censusPages}
                title={censusPage === censusPages ? 'Last page — already on it' : 'Last page'}
                onclick={() => (cPage = censusPages)}>»</button
              >
            </span>
          </div>
        </div>

        <!-- ══ TWO FOOTERS FOLDED INTO ONE DISCLOSURE ══

             Both were true, both were long, and both drew under the census on
             every load whether or not anything on screen depended on them. One
             defined `stored` and `expected`; the other stated the holiday
             table's own bounds. Together they were a wall of prose under a
             table, which is where a reader stops reading and therefore where a
             page should put the least.

             NOT DELETED — §4's limit-stating is exactly what the second one
             does, and `docs/06-limits.md` is the other place it lives. Folded,
             so the reader who wants the definitions clicks once and the reader
             who wants the numbers is not made to scroll past them.

             The fold OPENS ITSELF when the window reaches outside the holiday
             table, because there the second paragraph stops being a definition
             and becomes a caveat on a number that is on screen. A limit that
             applies right now is not something to make somebody hunt for. -->
      <!-- REMOVED: "What stored and expected mean" — a glossary under the table. Not a control and not a column. -->
      </section>
      {/if}
    </div>
  {/if}
</div>

<!-- ===================================================================== -->
<!-- THE INSTRUMENTS RUNG.                                                  -->
<!--                                                                        -->
<!-- IT USED TO SAY THESE NAMES WERE NOT SELECTABLE, "and the reason is a    -->
<!-- missing FIELD, not a missing widget" — that SpotRequest had no member   -->
<!-- list, so a subset had nothing to travel on and a picker would take      -->
<!-- ticks and send a request that ignored them.                             -->
<!--                                                                        -->
<!-- BOTH HALVES HAVE SINCE STOPPED BEING TRUE, and the comment one level    -->
<!-- down already knew it: "now that the list above is the control". The     -->
<!-- picker exists, `SpotRequest.members` exists, `wireBodyFor` sends one    -->
<!-- member= per tick and `broker_run` filters on them.                      -->
<!--                                                                        -->
<!-- What it DOES answer is the question the count alone cannot: which names -->
<!-- this feed cannot serve. Those are shown and marked rather than absent,  -->
<!-- because absence is indistinguishable from completeness.                 -->
<!-- ===================================================================== -->
{#snippet instrumentRung()}
  <div class="roster">
    <!-- WHAT THIS PANEL IS FOR, NOW THAT THE LIST ABOVE IS THE CONTROL.
         It used to be the whole rung: two counts and nothing selectable. The
         count of reachable names is on the control's own face, and the names
         another feed lists are DRAWN IN THE LIST ABOVE as refused rows — which
         is where a reader meets them. What is left here is the one thing
         neither of those can be: the MEASUREMENT that fills them, which costs
         one request per feed and is therefore a press. -->
    <div class="rrow">
      <span class="rk">Listed by another feed, not by this one</span>
      <span class="rv mono" class:warn={(missingHere?.length ?? 0) > 0}>
        {missingHere ? n(missingHere.length) : '—'}
      </span>
      <span class="rn">
        {#if missingHere}
          measured across {n(feeds.all.length)} feed(s) · the union of every master, which is the
          widest membership any endpoint here can state
        {:else if roster.busy}
          reading every feed's master…
        {:else}
          not measured. <span class="mono">/instruments.json</span> answers for ONE feed and there
          is no <span class="mono">/universe.json</span>, so the complete membership costs one
          request per feed and is not taken unless asked for.
        {/if}
      </span>
    </div>

    {#if !missingHere}
      <div class="ract">
        <!-- ONE CONDITION DISABLES THIS BUTTON AND THE LINE BESIDE IT NAMES
             WHICH. `rosterBlock` is the single source of both the `disabled`
             attribute and the sentence, so the two can never disagree — a
             greyed control whose stated reason has gone stale is worse than
             one that says nothing. -->
        <button
          class="btn ghost sm"
          type="button"
          onclick={measureRoster}
          disabled={rosterBlock !== null}
        >
          {roster.busy
            ? 'Reading…'
            : `Measure what ${active?.display ?? 'this feed'} is missing — ${n(feeds.all.length)} request(s)`}
        </button>
        {#if rosterBlock}
          <span class="hint warn">Cannot be measured yet: {rosterBlock}</span>
        {:else}
          <span class="hint">
            Each request rebuilds the census server-side, which is why it is a press rather than a
            page load.
          </span>
        {/if}
      </div>
    {/if}

    {#if roster.error}
      <p class="caution">
        <span class="tag down">not measured</span>
        <span class="msg">
          The union could not be read, so the count above stays a dash rather than becoming a zero
          that would read as "nothing missing": {roster.error}
        </span>
      </p>
    {/if}

    {#if missingHere && missingHere.length > 0}
      <!-- THE NAMES THEMSELVES ARE NOT LISTED TWICE. Every one of them is a
           row in the tick list above — drawn, refused, and carrying which feed
           does list it — which is where the reader is already looking. -->
      <span class="hint">
        All {n(missingHere.length)} of them are in the list above, drawn and refused, each with the
        feed that does list it on its own row. Choose that feed in the first control of the strip and they become
        selectable; nothing here is hidden and nothing is deleted from the store.
      </span>
    {/if}
  </div>
{/snippet}

<!-- ===================================================================== -->
<!-- THE FEED RUNG — FIRST IN THE ROW, AND A PEER OF THE FOUR IT GOVERNS.    -->
<!--                                                                        -->
<!-- A snippet rather than markup in place, because the strip is not the     -->
<!-- only place it has to appear: the three blank states above render it too -->
<!-- (no feed selected, an archive with nothing to ingest, and the feed list -->
<!-- failing), and each of those is exactly the state in which the operator  -->
<!-- most needs to change the feed. A control that vanishes precisely when   -->
<!-- it is needed is the dead end this snippet exists to make impossible.    -->
<!--                                                                        -->
<!-- It is `.dd`/`.ddb`/`.ddr` and NOT a `Picker`, for the reason the        -->
<!-- Universe and Instruments menus are not: a Picker row is a checkbox and  -->
<!-- the feed is a SELECTION — exactly one, never a set — and a refused feed -->
<!-- needs a row that is drawn, dead, and carrying `/feeds.json`'s own why.  -->
<!-- ===================================================================== -->
{#snippet feedRung()}
  <div class="field">
    <span class="lab">Broker feed</span>
    <!-- THE SAME CONTROL THE SEGMENTS RUNG DRAWS, and that is the whole point:
         the operator named Segments as the wanted look -- checkboxes, bulk
         buttons, a detail column, the panel that rises -- and asked why the
         feed rung was not it. It is now.

         MULTI-SELECT, AND IT REACHES THE WIRE. This was single until now, on
         the correct reasoning that `api::ingest::SpotRequest` carries ONE
         `feed` so a second tick could not be sent. That reasoning was right
         about the REQUEST and wrong about the RUN: `runPull` has always taken
         an ARRAY of bodies and POSTed each in turn, and `wireBodies` was
         already one entry per ticked TIMEFRAME. Adding the feed to that
         product is the whole fan-out -- N requests, one per (feed x rung),
         each naming its own vendor, each with its own receipt. Nothing about
         the loop, the abort or the receipts changed, because they were written
         for N from the start.

         EMPTY MEANS "the feed this page is scoped to". So the control starts
         agreeing with the strip above it and needs no seeding, and clearing
         every tick is not an empty run -- it is the ordinary one. -->
    <Picker
      filter
      label="feeds"
      summary={feedsChosen.length === 0
        ? 'No feeds'
        : feedsChosen.length === 1
          ? feedName(feedsChosen[0])
          : `${n(feedsChosen.length)} feeds · ${n(allBodies.length)} request(s)`}
      title={`Everything below is this feed's answer — ${scopeNote}. Which vendors this run asks: ${n(wireBodies.length)} spot request(s) — ${n(feedsChosen.length)} feed(s) x ${n(rungsChosen.length)} timeframe(s) — plus ${n(fnoBodies.length)} expired-derivative request(s), ${n(allBodies.length)} in all, each with its own receipt. The page's COUNTS stay scoped to ${feedName(feeds.active)} — a count is measured against one master and one store, and no page in this product puts two feeds' numbers side by side.`}
      rows={feeds.all.map((f) => ({
        key: f.wire,
        name: f.display,
        detail: f.ready === true
          ? (f.kind_label ?? 'kind not stated')
          : `${f.kind_label ?? 'kind not stated'} · unavailable`,
        disabled: f.ready !== true,
        /* NOT `skipBulk`. A ready feed is a feed "Select all" may tick: unlike
           the two segments that answer 503, every ready feed here can be sent. */
        why: f.ready === true
          ? undefined
          : (f.why ??
            'the server marked this feed not ready and stated no reason, which is itself the thing to fix.'),
        title: f.ready === true
          ? `Adds ${f.display} to this run. Its bars are pulled from ${f.display} alone and filed under it — nothing is merged with another feed.`
          : `${f.display} is refused by the server: ${f.why ?? 'no reason stated.'}`
      }))}
      selected={new Set(feedsChosen)}
      onchange={(/** @type {Set<string>} */ sel) => {
        /* BACK TO EMPTY WHEN THE PICK IS JUST THE SCOPE FEED, so the control
           returns to "follows the page" rather than pinning a value that then
           stops tracking a scope change. */
        pullFeeds =
          sel.size === 1 && [...sel][0] === feeds.active ? new Set() : new Set(sel);
      }}
    />
    <!-- ONE FEED SAYS NOTHING HERE; MORE THAN ONE HAS SOMETHING ONLY THIS
         LINE CAN SAY.
         "everything below is this feed's answer · <scope>" drew under a control
         whose face already reads `Dhan`, on every load, forever — the running
         commentary the cut two hundred lines down already named and removed one
         instance of. It is on the control's own `title` now, which is where §4
         sends a fact that does not fit.
         The multi-feed branch is NOT narration and stays: with two feeds ticked
         the counts below belong to ONE of them, and nothing else on this page
         says which. It appears only when it is true. -->
    {#if feedsChosen.length > 1}
      <span class="note quiet">
        {n(feedsChosen.length)} feed(s) · {n(wireBodies.length)} request(s) · counts below are {feedName(
          feeds.active
        )}'s
      </span>
    {/if}
  </div>
{/snippet}

<!-- ===================================================================== -->
<!-- THE SEGMENTS RUNG. Every segment the store keys on is DRAWN. Spot is    -->
<!-- offered because `/pull/spot` fills it; the two expired ones are drawn   -->
<!-- dead, in place, each carrying the three measured reasons no build can   -->
<!-- fetch it. See `SEGMENTS` for all three and where each was read.         -->
<!-- ===================================================================== -->
{#snippet segmentRung()}
  <!-- A ONE-ITEM DROPDOWN IS AN UNANSWERED QUESTION. The two expired segments
       were FILTERED OUT of this menu and summarised in a clause beside it, and
       the operator asked the only question that shape provokes: why are expired
       futures and options not here. A hidden row cannot answer it. Both are
       rows now — disabled, in the store's own order, with the short reason
       VISIBLE on the row and the long one on its `title` — which is the shape
       `$lib/Picker.svelte` documents `disabled`/`why` for and the shape the
       universe menu beside it has always used.

       `segmentsReached` is unchanged and still gates everything downstream:
       ticked AND served. A disabled row cannot be ticked, so nothing below this
       control can be reached by a segment this build cannot fill. -->
  <div class="field">
    <span class="lab">Segments</span>
    <!-- `tuck`: expired futures and expired options are `served: false` because
         only POST /pull/spot exists — there is no expired-contract route in
         this build at all. Two of the three rows were dead on every feed. -->
    <Picker
      tuck
      label="segments"
      title={`The store keys on three segments and all three are drawn. ${segmentsUnserved
        .map((s) => `${s.label} — ${s.why}`)
        .join('\n\n')}`}
      summary={segmentsReached.length > 0
        ? segmentsReached.map((s) => s.label).join(', ')
        : 'No segment selected'}
      rows={SEGMENTS.map((s) => ({
        key: s.key,
        name: s.label,
        detail: s.note,
        // NEVER `disabled`, AND THAT IS THE OPERATOR'S RULE. Every segment is
        // offered on every feed — stated three times — because on an ARCHIVE
        // the expired contracts are exactly what the bought CSVs hold. See
        // `segServed`.
        //
        // NO `skipBulk`, AND THIS IS THE OPERATOR'S CALL, TAKEN.
        //
        // It was `skipBulk: s.short !== null`, so "Select all" reached only the
        // segments with no refusal on them. With Spot the sole unrefused row and
        // Spot already ticked, the bulk press had NOTHING to add: the count read
        // 1, the click changed no input, and the control read as broken. The note
        // that stood here recorded the tension and said the decision "belongs to
        // whoever owns the control's meaning — not to a checker run". It has now
        // been made: Select all selects all three.
        //
        // WHAT THAT COSTS IS STATED RATHER THAN HIDDEN. Ticking the two expired
        // segments builds a request the server answers 503 to, and the page does
        // not pretend otherwise — each row still carries its three measured
        // reasons on its own `title`, the run card still reports the refusal
        // verbatim, and `segmentsUnserved` still names them. A bulk action that
        // can reach a row a single click can already reach is consistent;
        // silently skipping it while announcing a count that included it was not.
        //
        // The rows were never `disabled` — that is deliberate and unchanged, so
        // the only thing that moves is whether the BULK press reaches what a
        // single press always could.
        why: s.short,
        title: s.why
      }))}
      selected={segSet}
      onchange={(/** @type {Set<string>} */ sel) => (segSet = sel)}
    />
    <!-- "1 of 3 active · 2 refused, drawn with the reason" IS GONE, AND THE
         TWO FACTS IN IT ARE BOTH STILL ON SCREEN.
         The control's face names the ticked segments. The refused two are rows
         inside it — drawn, dead, each carrying its own why — behind Picker's
         own drawer line, which states its count and says "show why". This line
         was a third rendering of a thing the reader can see twice, under a
         control it made taller. The long form is the menu's `title`. -->
  </div>
{/snippet}

<!-- ===================================================================== -->
<!-- THE DAY GRID. One pane: a month and a year in the header, that month's  -->
<!-- forty-two cells in the body. Six full weeks always, so the panel never  -->
<!-- changes height under the pointer as it is paged.                        -->
<!--                                                                         -->
<!-- A day that CANNOT be taken is drawn unable to take it and says which     -->
<!-- bound refused; a day that holds no session is drawn faint and says why.  -->
<!-- Those are two different facts and they get two different marks — a       -->
<!-- Saturday is not a refusal, it is a day with nothing in it.               -->
<!--                                                                         -->
<!-- One panel exists at a time because `cal.field` is a single value, so two -->
<!-- can never overlap each other.                                           -->
<!-- ===================================================================== -->
{#snippet dayGrid()}
  <div
    class="cal panel-in"
    class:up={cal.up}
    role="dialog"
    aria-label="Choose a day"
    bind:this={calEl}
    onkeydown={onCalKey}
    tabindex="-1"
  >
    <!-- THE HEADER IS A CAPTION BETWEEN TWO ARROWS, and the caption is the
         control. It was two `Picker`s; see `cal.pad` for why a strip control
         inside a 272px popover could only ever draw over the grid it steers.
         The arrows always move the unit the caption names, so the pair reads
         as one instrument on all three faces. -->
    <div class="cal-h">
      <button
        type="button"
        class="nav"
        aria-label={calBack.why}
        title={calBack.why}
        disabled={!calBack.on}
        onclick={() => calStep(-1)}>&lsaquo;</button
      >
      <button
        type="button"
        class="calcap"
        aria-label={`${calCaption}. ${calCaptionTitle}`}
        title={calCaptionTitle}
        aria-expanded={cal.pad !== 'day'}
        onclick={calZoom}
      >
        <span class="capt">{calCaption}</span>
        <span class="capc" class:on={cal.pad !== 'day'} aria-hidden="true">&#9662;</span>
      </button>
      <button
        type="button"
        class="nav"
        aria-label={calFwd.why}
        title={calFwd.why}
        disabled={!calFwd.on}
        onclick={() => calStep(1)}>&rsaquo;</button
      >
    </div>

    <!-- ONE BOX, THREE FACES, ONE HEIGHT. The days, the twelve months and the
         years each fill the same block, so the note and the footer under it do
         not move when the reader changes face. -->
    <div class="calbody">
      {#if cal.pad === 'month'}
        <div class="calpad" role="group" aria-label="Months of {calYear}">
          {#each calMonths as m (m.ym)}
            <button
              type="button"
              class="padc"
              disabled={m.off}
              class:on={m.ym === calView}
              title={m.off ? `${m.label} ${calYear} is outside ${dayLabel(minDay)} – ${dayLabel(maxDay)}.` : `Show ${m.label} ${calYear}.`}
              onclick={() => {
                pickView(m.ym);
                cal = { ...cal, pad: 'day' };
              }}
            >
              {m.label}
            </button>
          {/each}
        </div>
      {:else if cal.pad === 'year'}
        <div class="calpad years" role="group" aria-label="Years this field may take">
          {#each calYears as y (y)}
            <button
              type="button"
              class="padc"
              class:on={y === calYear}
              title={`Show the months of ${y}.`}
              onclick={() => {
                pickView(clampView(`${pad(y, 4)}-${pad(calMonthNo)}`));
                cal = { ...cal, pad: 'month' };
              }}
            >
              {y}
            </button>
          {/each}
        </div>
      {:else}
        <div class="calwk" aria-hidden="true">
          {#each WEEK as w (w)}<span>{w}</span>{/each}
        </div>

        <div class="dgrid" role="group" aria-label="Days of {monthLabel(calView)}">
      {#each calDays as d (d)}
        {@const why = dayBlock(d)}
        <!-- A REFUSED DAY CARRIES ITS OWN REASON, and a takeable one carries
             what it is. `title` is never empty: Svelte drops the attribute when
             the expression is null, and a tooltip that says nothing is worse
             than none. -->
          <button
            type="button"
            class="cday"
            data-day={d}
            disabled={why !== null}
            title={why ?? dayNote(d)}
            tabindex={d === cal.cursor ? 0 : -1}
            aria-pressed={d === (cal.field === 'from' ? fromDay : toDay)}
            class:picked={d === (cal.field === 'from' ? fromDay : toDay)}
            class:cursor={d === cal.cursor}
            class:oth={d.slice(0, 7) !== calView}
            class:nos={noSessionWhy(d) !== null}
            class:now={d === maxDay}
            onclick={() => commit(d)}
          >
            {Number(d.slice(8))}
          </button>
        {/each}
        </div>
      {/if}
    </div>

    <!-- THE BOUNDS THE STRUCK-THROUGH DAYS ARE ENFORCING, WRITTEN OUT.
         `title` on a disabled control is unreliable — Chrome suppresses
         tooltips on them — so the rule cannot live only there. It is one line,
         always on screen while the grid is, and it reads the same `minDay`,
         `feedFloor` and `maxDay` the grid and the refusals do rather than
         restating a number that could drift from them. -->
    <p class="calnote">
      {dayLabel(minDay)} – {dayLabel(maxDay)} can be picked. Faint days hold no session;
      struck-through days are outside what this field may take.
      {#if nothingInView}
        <span class="far">
          No day in {monthLabel(calView)} can be picked here.
          {#if feedFloor.at && `${calView}-01` < feedFloor.at}
            {floorSentence} Looking is allowed — this month is drawn so the refusal can be read on it.
          {:else}
            Every day in it is outside {dayLabel(minDay)} – {dayLabel(maxDay)}.
          {/if}
        </span>
      {/if}
    </p>

    <!-- THE REACH CAVEAT, HEADLINE OUT AND REASONING FOLDED — AND THAT IS NOT
         THE §4 FALLBACK.
         What stood here was eight lines of grey prose inside a 272px popover:
         the bounds, then the feed's floor with its rolling clause and its
         citation, then the folder's span, then the month warning. Four separate
         facts of four different kinds set as one paragraph, so the two that
         matter — a date, and a warning — read as the same weight as the two
         that explain them. A reader who meets that wall reads none of it, which
         is the same outcome as saying nothing and is why it had to change.

         WHAT IS FOLDED IS THE ARGUMENT, NEVER THE FACT. The summary carries the
         DATE and the feed's name, on screen, unfolded; only the sentence
         explaining what to do about it is one click away. §4 bans a fallback
         that HIDES a failure, and nothing here is hidden: the same idiom, with
         the same reasoning, is what `$lib/Picker.svelte` already uses for the
         rows a feed cannot serve. The month warning above is NOT in here — a
         refusal that applies to what is on screen right now stays on screen. -->
    {#if (feedFloor.at && feedFloor.at > minDay) || folderReachSentence}
      <details class="calwhy">
        <summary>
          {#if feedFloor.at && feedFloor.at > minDay}
            {feedName(feeds.active)} answers from {dayLabel(feedFloor.at)}{feedFloor.rolling
              ? ' (rolling)'
              : ''}
          {:else}
            {active?.display} holds files, not a floor
          {/if}
        </summary>
        {#if feedFloor.at && feedFloor.at > minDay}
          <!-- THE FLOOR IS STATED, NOT ENFORCED. It used to be both the sentence
               AND the bound, so this line advertised the feed's floor as the
               pickable span. The grid now offers the server's whole span and the
               floor is a property of the ASK, so the sentence has to say what the
               vendor will actually return rather than what the control will let
               you choose. -->
          <p>
            Earlier days are pickable and this feed will not answer for all of them:
            {floorSentence} A day before that returns nothing from this feed — pick a feed or a
            timeframe that reaches further back to fill it.
          </p>
        {/if}
        <!-- A FOLDER FEED HAS NO FLOOR TO PRINT, AND THIS IS WHAT IT HAS INSTEAD.
             Not a rule about how far back a vendor will answer — there is no
             vendor in this sentence — but the span of the files actually in the
             folder, read from it, with the path beside it. The grid does NOT
             strike days outside it: those days are askable and will simply come
             back with nothing, and striking them would claim a refusal nobody
             made. -->
        {#if folderReachSentence}
          <p>
            No floor applies — {active?.display} is a folder of bought files, so its range is
            whatever is in it: {folderReachSentence}. Days outside it can still be picked; they hold
            no file, so they come back empty rather than refused.
          </p>
        {/if}
      </details>
    {/if}
    <div class="cal-f">
      <button type="button" class="btn ghost sm" onclick={() => commit(maxDay)}>
        {dayLabel(maxDay)}
      </button>
      <span class="spacer"></span>
      <button type="button" class="btn ghost sm" onclick={() => shutCal()}>Close</button>
    </div>
  </div>
{/snippet}

<style>
  /* Everything below is built from the tokens in $lib/theme.css. No colour,
     radius, duration or step is spelled twice — a hex code here would be a
     second theme that the toggle does not reach. */

  /* ---- density ----
     DENSE ROWS, GENEROUS SPACING BETWEEN GROUPS. The page is read at a glance
     by someone deciding whether to spend thirty-two minutes, so the gaps that
     separate one QUESTION from the next stay wide (`--s6`) while the gaps
     inside one answer tighten (`--s5`, `--s3`). Every step is a token; none is
     a number typed here. */
  .body {
    padding: var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s5);
  }
  /* ONE COLUMN, ALWAYS. THE RUN CARD GOES UNDER THE FORM, NEVER BESIDE IT.
     This was `5fr 4fr` the moment a run started, so pressing Start took the
     form — six controls that had just been laid out across the full width —
     and squeezed it into five ninths, while the run card took the other four.
     Every control reflowed under the operator's hands at the exact moment he
     had stopped touching them, and the strip that had been one clean row
     became two ragged ones beside a panel.

     A form and its outcome are SEQUENTIAL, not parallel: you fill the first,
     then you watch the second. Putting them side by side says they are two
     things to look at at once, which is false, and it costs the form the width
     its own layout was designed against. Full width for both, stacked, and the
     page does not move when the run begins. */
  .cols {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--s5);
    align-items: start;
  }
  /* NO BREAKPOINT HERE, AND THE ABSENCE IS THE CORRECTION. A
     `@media (max-width: 1100px)` block stood here restating
     `grid-template-columns: minmax(0, 1fr)` — the value `.cols` already carries
     four lines above, byte for byte. It was the leftover of the `5fr 4fr` split
     that rule replaced, and it changed nothing at any viewport width.
     A breakpoint that sets what is already set reads as a responsive decision
     to the next person and is not one; the run card going under the form is
     unconditional, so there is nothing here to make conditional. */

  /* NO `overflow: hidden` HERE. It squared off the header's top corners neatly
     and CLIPPED THE CALENDAR: the panel is taller than the space left under the
     date row, so the last week of the month and the whole footer were cut off
     by the card edge. The header rounds its own two corners instead, which
     costs one line and does not eat a popover. */
  
  
  .spacer {
    flex: 1;
  }

  /* ---- the form ---- */
  /* GENEROUS BETWEEN GROUPS, DENSE INSIDE ONE. The strip, the wire fold and
     the press are three different questions and get the wide gap; the rows
     inside the strip get the tight one. */
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--s6);
    padding: var(--s5);
  }
  fieldset {
    border: 0;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    /* THE RUNGS ARE THE GROUPS. One question per rung, so this is the one gap
       on the page that stays wide — tightening it runs the universe row into
       the instruments panel and the cascade stops reading as a cascade. */
    gap: var(--s6);
    min-width: 0;
  }
  fieldset:disabled {
    opacity: 0.55;
  }
  /* THE FIRST OF THREE `.field` RULES IS GONE, AND IT NEVER RENDERED.
     It set `gap: var(--s3)`; the rule in the control-strip section below sets
     `gap: var(--s2)` at IDENTICAL specificity and later in source order, so
     every `.field` on this page has always taken 4px and this 6px was
     unreachable. Deleting it changes no pixel — verified by measurement, not by
     reasoning — and leaves one definition of the box instead of a pair that
     disagreed, where whichever you edited was a coin flip. */
  /* `.lbl` IS GONE FROM THIS FILE, RULE AND USE BOTH. It styled exactly one
     span — "What to do", in the archive blank state — while six other labels on
     the page used `.lab`, and it also re-sized `theme.css`'s `.lbl` from
     `--fs-mini` to `--fs-micro`, which is a change every OTHER `.lbl` in the
     product would have inherited had one ever been added here. That span now
     carries `.lab` like its six peers. `theme.css` keeps the global `.lbl` for
     the feed strip that actually wants it. */
  .hint {
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--dim);
  }
  
  /* A `.hint` CARRYING A TONE CLASS HAS TO TAKE THE TONE. The scoped `.hint`
     rule above outranks the global `.warn` colour on specificity, so without
     this the sentence naming why a control is refused rendered in exactly the
     same grey as the neutral hint it replaces — the tone said nothing, and the
     only cue that the line was a refusal was reading it. Amber and not red on
     purpose: every branch of `rosterBlock` is a "not yet", not a failure. */
  .hint.warn {
    color: var(--warn);
  }


  /* ======================= THE CONTROL STRIP =========================
     ONE PANEL, AND THE CONTROLS STAND SIDE BY SIDE IN IT.

     What was here before was a column: a row of text-link universe tabs, a
     bordered box of instrument rows, then two full-width stacked dropdowns,
     with four tiles above the lot and three explanatory paragraphs between
     them. Five decisions read as five documents.

     The shape below is one strip. Every rung is the same object — a caps
     label, one dropdown button, one clause under it — laid out on an
     auto-fitting track so four of them sit across at console width and fold to
     two and then one as the panel narrows. Dense rows, wide gaps BETWEEN
     groups, monospace wherever a value lives, and no paragraph anywhere: a
     fact that matters is the control's `title` or its one clause.
     ================================================================== */

  /* THE THREE ALIASES `$lib/Picker.svelte` ASKS FOR AND `theme.css` DOES NOT
     DEFINE. Picker reads `--raise`, `--panel2` and `--accdeep` with hardcoded
     dark fallbacks, so its menu ground is a literal that the light theme never
     reaches and that nothing in this strip could match. Custom properties
     inherit, so naming them here points Picker's own menus at the real ramp
     and makes them the same object as the menus below — without touching
     another file and without a hex code on this page. */
  .sel {
    --raise: var(--bg-2);
    --panel2: var(--panel-2);
    --accdeep: var(--acc-soft);

    background: linear-gradient(180deg, var(--panel-2), var(--panel));
    border: 1px solid var(--line);
    border-radius: var(--r4);
    box-shadow: var(--e1);
    min-width: 0;
  }

  /* auto-fit, not a fixed four: the panel is half the width while a run card
     is beside it, and four 248px tracks cannot fit there. The track floor is
     the width at which a monospace instrument name and its caret stop
     colliding.

     ══════════ WHY THIS ROW WAS NOT A ROW, AND WHAT ACTUALLY BROKE IT ══════════

     `align-items: end` was the cause, and it is worth naming exactly, because
     the symptom pointed at the wrong control. Bottom-aligning grid items makes
     every cell's LAST pixel share a line. The cells do not have the same
     content below their control: Universe carries one `.note`; Timeframe
     carries up to four (the feed's granularity floor, the dropped-rung notice,
     the ticked/refused tally, and the missing-`served` warning); Days to pull
     carries a two-field `.dates` block and its own note. Align the bottoms of
     cells with two, five and seven lines of tail and their TOPS land three
     different places — so Timeframe's label floated far above Universe's, and
     Days to pull's did the same. Nothing was wrong with Timeframe. It simply
     had the most to say.

     Two changes, and they are the cause rather than a nudge:

       1. `align-items: start`. The tops share a line, so every label starts at
          the same y and every control sits directly under its own label. A
          longer tail now hangs BELOW its cell, which is where a tail belongs
          and where it cannot move anything above it.
       2. One-line labels (see `.lab`). Top-alignment alone is not enough: a
          label that wraps to two lines in one cell and one line in the next
          pushes that cell's control down by a line, and the row breaks again
          one level lower. Every sub-clause moved to the control's `title` —
          §4's own remedy — and what is left is a single short noun phrase that
          is held to one line by rule, not by hope.

     A NUDGE WOULD NOT HAVE HELD. A margin tuned against today's four notes is
     wrong the moment a fifth appears, and the fifth is a `{#if}` away in half
     these cells. */
  /* `auto-fill`, NOT `auto-fit`, AND THE DIFFERENCE IS THE RAGGED ROW.
     Five controls into a three-track row leaves two on the second, and
     `auto-fit` COLLAPSES the empty tracks and stretches the survivors across
     them — so Segments and Timeframe rendered half again as wide as the three
     above them, and the strip read as two different grids stacked. `auto-fill`
     keeps the empty tracks, so every control is the same width at every count
     and the columns line up down the rows. The leftover is one clean gap at
     the end of the short row instead of distortion spread across it. */
  .pickers {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(248px, 1fr));
    gap: var(--s6) var(--s5);
    align-items: start;
    border: 0;
    margin: 0;
    padding: 0;
    min-width: 0;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    min-width: 0;
  }
  /* THE DAY WINDOW STARTS ITS OWN ROW, WHICH IS WHAT THE MOCKUP DRAWS: the
     dropdowns across the top, the two dates below them. `1 / -1` rather than
     `span 2` does that at EVERY track count — a spanning item is placed on the
     first line that fits it, so `span 2` let it finish a row of dropdowns
     whenever an even number of tracks left a gap, and the row it finished was
     the row it then broke. Full width also removes the old hazard the media
     query existed for: `span 2` in a one-track grid grew an implicit second
     column and pushed the panel past its own box, and `1 / -1` cannot, because
     `-1` is whatever the last line happens to be.

     `.dates` is capped rather than stretched. Two date fields spread across
     1,200px would read as the panel's most important control, and they are its
     last one. */
  .field.wide {
    grid-column: 1 / -1;
  }
  .field.wide .dates {
    max-width: 560px;
  }
  /* ONE LINE, ALWAYS, AND THAT IS THE SECOND HALF OF THE ROW FIX. A label is a
     short noun phrase — Broker feed, Universe, Instruments in that universe,
     Segments, Timeframe, Days to pull — and each one used to carry an italic
     sub-clause after it ("bar lengths — the granularity field", "ticked — this
     is what is counted"). At a 248px track those wrapped, and they wrapped to
     DIFFERENT heights per cell, which moves that cell's control down a line and
     breaks the baseline the row is supposed to have. Every one of those clauses
     is now the first sentence of its control's `title`, which is where §4 sends
     a fact that does not fit; the label is held to one line here so the rule
     cannot be broken by a longer word later. */
  /* THE REFERENCE'S `.lab`, AND IT IS THE SAME RULE RATHER THAN A COPY OF IT.
     `web/design/ingest.html` drew this label in MONO at `--fs-micro` with the
     caps tracking, and this drew it in the sans face at `--fs-xs` — so the two
     were different labels for the same control and "why does the app not look
     like the mockup" was partly this line. theme.css section 8 now carries the
     reference's version under `.lab`; every property below is that rule, and a
     page that adopts the class name outright loses nothing.

     `line-height: var(--lab-h)` is what makes a label occupy the strip's label
     ROW exactly, which is the height the grid reserves for it.

     ONLY ONE DECLARATION IS LEFT, AND THE OTHER TEN ARE GONE BECAUSE THEY WERE
     THE SAME TEN. The face, size, weight, tracking, case, colour, line-height
     and the three ellipsis properties were a verbatim second copy of
     `theme.css`'s `.lab` — the copy this comment's own first paragraph says the
     page "loses nothing" by adopting outright. Two copies of one rule drift the
     day one of them is edited, and the drift is invisible: both labels keep
     rendering, just differently.

     `min-width: 0` IS NOT PART OF THAT RULE AND MUST STAY. `theme.css` does not
     set it. Without it a label that is a grid or flex item takes its content as
     its floor and refuses to shrink, so `text-overflow: ellipsis` never fires —
     the label pushes its own track wider instead of clipping, and the 248px
     auto-fill floor that keeps the strip a row stops holding. It is the one
     line here that is about this page's layout rather than about the label. */
  .lab {
    min-width: 0;
  }
  /* THE FEED PICKER INSIDE A BLANK STATE. The three refusals above the strip
     render the same `feedRung` snippet, and a `.field` is a flex column that would
     take the whole width of an empty page. Clamped to one track's worth so it
     reads as the control it is in the strip. */
  .blankpick {
    max-width: 320px;
    margin-top: var(--s5);
  }
  /* `.belowsel` IS GONE — see the markup, where the band's whole history is
     kept. It described a full-width row under the form holding everything that
     had been crowding the panel; measured, it held nothing at all, and a styled
     grid span with a flex column and a gap is an expensive way to describe zero
     pixels. The rule outlived its contents by several commits, which is exactly
     what a dead rule does: it goes on reading like a layout. */
  /* ONE LINE, AND IT CLIPS RATHER THAN WRAPS. This is where a fact that used
     to be a paragraph now lives, and a clause that can grow to three lines
     would reflow the whole strip the moment a feed answered slowly. The full
     sentence is on the control's `title`. */
  /* THE REFERENCE'S `.note`, MARKER AND ALL — theme.css section 8.
     A note carries `▲` when it needs attention and `·` when it merely is, so a
     refusal is distinguishable from a remark BEFORE it is read. That is the
     difference between a strip a reader scans and a strip a reader has to
     parse, and it is most of what made the mockup look calmer than the page:
     the mockup had two kinds of note and this had one grey line for both. */
  /* THE MARKER IS `::before` AND THE NOTE STAYS A BLOCK, deliberately.
     A flex row would have made the text an anonymous flex item, and an
     anonymous item cannot be selected — so `text-overflow: ellipsis` would
     have had nothing to apply to and every one-line note would have lost its
     clip. The reference uses flex because its notes are short by construction;
     these are generated and are not, so the marker is inline here and the
     clip stays where it was. Same two glyphs, same two meanings. */
  .note {
    font-family: var(--mono);
    font-size: var(--fs-micro);
    line-height: 1.35;
    color: var(--faint);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .note::before {
    content: '· ';
    color: var(--faint);
  }
  .note.warn {
    color: var(--warn);
  }
  .note.warn::before {
    content: '▲ ';
    font-size: 8px;
    color: var(--warn);
  }
  /* A NOTE THAT IS SEVERAL REFUSALS, ONE PER ROW.
     The container stops carrying the marker and each row carries its own, which
     is both the fix and the more honest reading: every row here is a different
     feed declining for a different reason, so a single shared triangle was
     understating the count as well as sitting in the wrong place. Suppressing
     `::before` on the container is REQUIRED, not tidiness — as a column flex box
     the pseudo-element becomes a flex item and lands on its own line above the
     text. `white-space: normal` because these are sentences, not the one-line
     clipped captions the base `.note` is shaped for. */
  .note.rows {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    white-space: normal;
    overflow: visible;
    line-height: var(--lh-base);
  }
  .note.rows::before {
    content: none;
  }
  .note.warn.rows > span::before {
    content: '▲ ';
    font-size: 8px;
    color: var(--warn);
  }
  /* ══ THE STRIP IS A ROW, AND ITS CAPTIONS ARE WHAT STOPPED IT BEING ONE ══

     `.pickers` is a grid of five controls with `align-items: start`, so each
     cell is exactly as tall as what is in it. The captions underneath were not
     the same height as each other and could not be: a `.note` is one clipped
     line and a `.note.wrap` is however many lines its sentence needs, and the
     timeframe's — "Dhan — REST API. It serves one minute and coarser. One
     record at one minute is a bar — open, high, low, close." — ran to five
     against neighbours that ran to one. Five controls laid out on one baseline,
     and under them a row four lines taller at one end than the other.

     A FIXED TWO-LINE BLOCK, RESERVED WHETHER IT IS FILLED OR NOT. Two lines
     rather than one because one was already cutting "everything below is this
     feed's answer" mid-word; reserved rather than fitted because a caption that
     grows when a feed answers slowly reflows the whole strip under the reader's
     hands, which is the hazard the one-line rule was protecting against and it
     is protected against here too. The box does not change size, so nothing
     below it can move.

     QUIET CAPTIONS ONLY, AND THAT DISTINCTION IS §4's. A `.note.quiet` is a
     remark — it carries `·` and it says what a control IS. A `.note.warn`
     carries `▲` and is a refusal, and an ellipsed refusal is the failure this
     repository keeps writing comments about, so warnings are not clamped and a
     cell that grows because something is wrong is a cell that should. */
  .pickers .field > .note.quiet:not(.full) {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    white-space: normal;
    overflow: hidden;
    min-height: calc(2 * 1.35 * var(--fs-micro));
  }
  /* A `pknote` is a one-line tail by default — nowrap, clipped, ellipsed. A
     SENTENCE put in one is a sentence with its end cut off, which is the same
     defect as an ellipsed refusal. The vendor floor and the missing-`served`
     notice are sentences, so they wrap. */
  .note.wrap {
    white-space: normal;
    overflow: visible;
    text-overflow: clip;
    line-height: var(--lh-base);
  }
  /* THE RESOLVED FOLDER, WHICH IS A PATH AND THEREFORE BREAKS ANYWHERE.
     `overflow-wrap: anywhere` rather than `break-word`: a path has no spaces to
     break at, so `break-word` would leave it overflowing its column and push
     the strip wider than the viewport. It is a button because it DOES
     something — it fills the field beside it — and it is styled flat so it
     reads as the path it is rather than as a second submit control. */
  .pathbtn {
    font: inherit;
    font-size: inherit;
    padding: 0;
    border: 0;
    background: none;
    color: var(--acc);
    cursor: pointer;
    text-align: left;
    overflow-wrap: anywhere;
    text-decoration: underline dotted;
    text-underline-offset: 2px;
  }
  .pathbtn:hover {
    text-decoration: underline solid;
  }

  /* ---- the dropdown, and it is the SAME OBJECT `$lib/Picker.svelte` draws ----
     Segments and Timeframe are Pickers; Feed, Universe and Instruments cannot
     be, because a Picker row is a checkbox and these three need a row that is
     drawn, disabled and carries its own refusal. Every measurement below is
     Picker's, so the five read as one control. */
  .dd {
    position: relative;
    min-width: 0;
  }
  /* ONE HEIGHT FOR EVERY BUTTON IN THE STRIP, INCLUDING THE TWO THAT ARE NOT
     OURS. `$lib/Picker.svelte`'s `.pbtn` already agrees with `.ddb` on every
     metric — 15px semibold mono, 11px/14px padding, 9px radius — but it does
     not clip, not a reaction to one
     string: Instruments and Universe reach the same width on a long name.
     Picker is shared and is not edited from here; the clip is applied from
     this page, to Pickers inside this strip only. */
  .field :global(.pbtn) {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* ══ ONE CONTROL NEEDED A CEILING, AND IT IS NOT ALL FIVE ══

     `Picker` sizes its panel `width: max-content` up to `min(92vw, 560px)`,
     and for four of the five rungs that is right — the feed names, the universe
     names and the segment sentences all want the room and truncate visibly
     without it. A first attempt pinned every menu in the strip to its button's
     width and it was wrong in the obvious way: "Global Datafeeds" became
     "Global D…" and "contracts that have already settled" became "contracts
     t…", which is an ellipsed refusal — the thing this page keeps removing.

     TIMEFRAME IS THE EXCEPTION because its rows carry the longest detail on the
     strip ("1min · 375 bar(s) per session") on the RIGHTMOST control, so
     `max-content` opened it from its own left edge to the window's right and it
     hung across Segments. A ceiling, not a pin: it still sizes to its content,
     it just stops before it becomes a banner. */
  .field.rung :global(.pmenu) {
    max-width: 420px;
  }

  /* ══ THE ROW IS THE CONTROL. A COLUMN OF EMPTY CIRCLES IS NOT ══

     What read as dated here was never the panel or the type — it was the
     gutter. Six universes drawn as six hollow rings down the left edge, five
     of them empty, is a 1998 form control, and it is dated for a reason that
     is not fashion: the ring is a SECOND thing to look at that says what the
     row already says. The eye has to find a 15px circle to answer "which one
     is picked" on a list where the picked row is already tinted and already
     railed.

     So the box goes and the ROW carries the state — the accent ground and the
     rail `Picker` already draws, plus a check at the end of the line the way a
     macOS menu, a command palette and every current picker mark a choice.

     THE INPUT IS STILL THERE AND STILL REAL. It is moved out of the flow, not
     removed: it keeps its `type`, its `name`, its `:checked`, its change event
     and its place in the tab order, so the keyboard, the screen reader and
     `toggle()` all behave exactly as before. This is a rendering change and
     nothing else.

     WHAT IS LOST, HONESTLY: `Picker`'s own comment argues the circle-vs-square
     is load-bearing — it is "the only thing on screen saying whether ticking
     this one unticks the rest". That was true when the glyph was the only mark.
     It is not the only mark now: a single-choice menu has exactly one tinted
     row and no bulk header, and a multi-choice one has "Select all N / Clear
     all" across its top and tints every row you tick. Scoped to this page, so
     the component's own reasoning still governs /db, /markets and /autopilot. */
  /* RADIOS ONLY. The first version of this hid EVERY control in a Picker menu
     and that was a straight inconsistency: Segments and Timeframe lost their
     boxes while the Instruments list twelve pixels away kept its native
     `accent-color` checkbox, so one page ended up with two answers to "can I
     tick more than one of these".

     The rule is the one the affordance already implies. A CHECKBOX SAYS "you
     may tick several" and it stays, everywhere, because that is a promise the
     row cannot make on its own. A RADIO says "ticking this unticks the rest",
     which is a thing the list ALREADY shows — exactly one row is tinted — so
     the ring is the second mark that says nothing, and a column of five empty
     ones is the gutter that read as dated. */
  .field :global(.pmenu .plist input[type='radio']) {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    padding: 0;
    opacity: 0;
    pointer-events: none;
  }
  /* The reason under a row was indented past a box that is no longer in the
     flow, which left it floating in the middle of the row.

     `order: 1` IS WHAT KEEPS THE CHECK ON THE FIRST LINE. `.pwhy` is
     `flex: 0 0 100%`, so it takes a row of its own, and the check below is an
     `::after` — last in source order, which put it on a THIRD line under the
     sentence, floating alone at the left. Ordering the sentence after it puts
     the check back beside the name where it belongs, with no pixel offset to
     go stale: the wrap is still the flex container's to decide. */
  .field :global(.pmenu .plist label:has(input[type='radio']) .pwhy) {
    padding-left: 0;
    order: 1;
  }
  /* THE CHECK, DRAWN RATHER THAN TYPED — two borders on a rotated box, the
     same technique `Picker` uses for its own tick, so it cannot come out as a
     missing glyph on a machine without the font. */
  .field :global(.pmenu .plist label:has(input[type='radio']:checked))::after {
    content: '';
    flex: 0 0 auto;
    align-self: center;
    width: 5px;
    height: 10px;
    margin: 0 2px 3px 0;
    border: solid var(--acc);
    border-width: 0 2px 2px 0;
    transform: rotate(43deg);
  }
  /* FOCUS HAS TO LAND SOMEWHERE VISIBLE. The input carried the ring and the
     input is out of the flow, so the row takes it — inset, so it reads as the
     row being focused rather than as a second border around it. */
  .field :global(.pmenu .plist label:has(input[type='radio']:focus-visible)) {
    outline: 2px solid var(--acc);
    outline-offset: -2px;
  }
  /* A row that cannot be chosen shows no check and no ground — it is struck
     through and dimmed, which `Picker` already does, and the gutter it used to
     need for a disabled box is gone with every other row's. */
  .field :global(.pmenu .plist label.pdis)::after {
    content: none;
  }
  /* THE CHECKBOX MATCHES THE ONE TWELVE PIXELS AWAY. `.inslist input` is a
     native box at 14px tinted with `accent-color`; Picker draws its own at
     18px. Two sizes of the same idiom on one strip is the drift this page
     keeps removing, so the drawn one is brought to the native one's size. */
  .field :global(.pmenu .plist input[type='checkbox']) {
    width: 14px;
    height: 14px;
    /* AND THE CORNER COMES DOWN WITH THE BOX. Picker's 6px radius is right on
       an 18px square; on a 14px one it is nearly half the side, so the box
       rendered as a ROUNDEL — which is the one shape a checkbox may not have,
       because round means radio and radio means the others untick. Shrinking
       the square without shrinking its corner turned a multi-select into
       something that looked single-select, twelve pixels from a native box
       that still looked square. */
    border-radius: 4px;
  }
  /* The tick inside a 14px box, scaled to it — Picker sizes its own for 18px
     and it overhung the smaller square by about a pixel on each side. */
  .field :global(.pmenu .plist input[type='checkbox']:checked)::after {
    width: 3.5px;
    height: 7px;
    margin-top: -1px;
  }
  .ddb {
    appearance: none;
    width: 100%;
    text-align: left;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 9px;
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-base);
    font-weight: var(--w-semi);
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    padding: 11px 36px 11px 14px;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    background-image: linear-gradient(45deg, transparent 50%, var(--acc) 50%),
      linear-gradient(135deg, var(--acc) 50%, transparent 50%);
    background-position: calc(100% - 17px) 55%, calc(100% - 12px) 55%;
    background-size: 5px 5px, 5px 5px;
    background-repeat: no-repeat;
  }
  .ddb:hover:not(:disabled) {
    border-color: var(--dim);
  }
  .ddb:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: 2px;
  }
  .ddb:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .ddm {
    position: absolute;
    left: 0;
    top: calc(100% + 7px);
    z-index: 40;
    width: max-content;
    min-width: 100%;
    max-width: min(92vw, 560px);
    max-height: 380px;
    overflow-y: auto;
    background: var(--raise);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 5px;
    box-shadow: var(--e3);
  }
  /* The census is a panel of rows and a press, not a list of names, so it
     needs the room a two-column row costs. */
  .ddm.wide {
    min-width: 420px;
    padding: var(--s5);
  }
  
  
  
  /* Drawn and refused, never absent: a row missing from the list reads as a
     set that does not exist rather than one this route cannot spell. The whole
     reason is on the row's own `title`. */
  

  /* ---- the two days ----
     A flex pair rather than a 1fr/1fr grid, so that when the rung loses its
     second track the two cells wrap instead of being squeezed to half a
     monospace date each. */
  /* `flex-start`, FOR THE REASON `.pickers` IS. Bottom-aligning two cells makes
     their labels move apart the instant one of them grows — the `.bad` state
     and the calendar are both per-cell — and FROM DATE / TO DATE sitting at two
     heights is the same broken row one level down. Their tops share a line now,
     so the two labels share a baseline and the two fields share a top edge
     whatever either cell is doing.

     THE FLOOR IS WHAT MAKES THE WRAP CLEAN. `flex: 1 1 0` with `min-width: 0`
     let both cells shrink without limit, so at a narrow width they went on
     shrinking side by side until "02 Sep 2024" no longer fit its own box
     instead of dropping to a second line. A basis wide enough for the text and
     the ▦ makes the second cell wrap whole. */
  .dates {
    display: flex;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: var(--s5);
  }
  .dcell {
    flex: 1 1 190px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .dlbl {
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    white-space: nowrap;
  }
  /* THE FIELD IS THE SAME HEIGHT AND THE SAME FACE AS THE DROPDOWN BUTTONS
     BESIDE IT. At the old 13px/7px it sat two pixels short of them and the
     strip read as two rows of different controls. Two class names deep so it
     beats the base `.din` wherever that rule sits in the file. */
  .dates .din {
    font-size: var(--fs-base);
    padding: 11px 13px;
    border-radius: 9px;
  }
  .dates .dbtn {
    font-size: var(--fs-base);
    padding: 11px 10px;
    border-radius: 9px;
  }

  /* ---- the size of the ask, in one line ----
     Everything the four deleted tiles carried that is not already on a
     control: the product with its factors, and the measured store reading
     beside it. `productLine` verbatim, so the factors still multiply to the
     total printed next to them in every state. */
  /* `.ask` IS GONE WITH ITS ELEMENT. It styled one flex line carrying two
     numbers and eleven words between them; that line is now the `.tiles` row
     above, drawn with the reference's own `.metric`. Svelte reported `.ask .k`
     as an unused selector the moment the element left, which is the compiler
     doing what a dead-CSS gate would: a rule with nothing to match is a rule
     the next reader has to prove is dead before touching anything near it.

     The tile row inherits the border-top this used to draw — see `.tiles`. */
  /* The submit rung closes the panel the way the strip's own rungs are
     separated — a rule above it and the press on the right. */
  .form > .actions {
    /* LEFT, AT THE OPERATOR'S INSTRUCTION. It sat at `flex-end`, so the one
       control that starts a run was the furthest thing on the strip from the
       controls that configure it -- the eye crossed the whole form to reach it
       and crossed back to check what it had chosen. */
    justify-content: flex-start;
    padding-top: var(--s5);
    border-top: 1px solid var(--line);
  }

  /* The instruments rung: three facts and their provenance, never a control.
     `.rv` is the number, and it is a dash whenever nothing counted it. */
  /* The census. It is the panel the Instruments control opens, so it draws no
     border and no ground of its own — `.ddm.wide` is already both, and a box
     inside a box reads as two panels arguing about which one is the answer. */
  .roster {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    min-width: 0;
  }
  .rrow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: baseline;
    gap: var(--s2) var(--s4);
  }
  .rk {
    font-size: var(--fs-sm);
    color: var(--ink);
  }
  .rv {
    font-size: var(--fs-base);
    font-weight: var(--w-bold);
    color: var(--dim);
  }
  .rn {
    grid-column: 1 / -1;
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--dim);
  }
  .ract {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s3) var(--s4);
  }
  /* `.miss` and its three children are gone with the second list they styled:
     the names another feed lists are drawn in the tick list itself now, as
     refused rows, which is where the reader is already looking. */

  /* ---- the month fields and their panel ----
     The old day button and its three-pane drill-down went with the unit. What
     replaces it is the shape the design settled on: a monospace text field that
     reads and writes `Sep 2024`, a ▦ that opens the grid, and the grid itself —
     one year of months, the year a <select> in its header. */
  /* THE CALENDAR'S ANCHOR, NAMED FOR THE CONTROL IT ANCHORS.
     This was `.field { position: relative }` — a third rule on `.field`, at the
     same specificity as the two above, written for the month fields and applied
     to EVERY field on the page. That made a containing block out of the feed
     cell, the universe cell, the instruments cell and the segments cell, none
     of which asked for one, and it meant any absolutely-positioned descendant
     added to the strip later would anchor to whichever cell it happened to land
     in rather than to a box someone chose.
     Measured before narrowing it, because this is the one rule here that is
     load-bearing: `.cal` resolves its `offsetParent` to
     `DIV.dcell.field.cal-open`, so the anchor is real and removing it outright
     would drop the day grid to the page. `.ddm` does NOT depend on it — it
     anchors to `.dd`, which sets its own `position: relative` — and Picker
     positions `.pmenu` itself.
     `.cal-open` rather than `.dcell` because that class already means "this
     cell owns a calendar": `onMount`'s outside-press handler tests
     `t.closest('.cal-open')` to decide whether a click is inside the control.
     One name, two readers, and the anchor now says what it is for. */
  .cal-open {
    position: relative;
  }
  .dwrap {
    display: flex;
    align-items: center;
    gap: var(--s3);
    min-width: 0;
  }
  /* TYPEABLE, AND MONOSPACE SO THE TWO FIELDS ALIGN. `Sep 2024` and `Dec 2024`
     are the same width in the mono face and are not in the sans one, which is
     what made a From/To pair look like two different controls. */
  .din {
    flex: 1 1 auto;
    min-width: 0;
    appearance: none;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r);
    color: var(--ink);
    font: inherit;
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-base);
    font-weight: var(--w-semi);
    padding: 7px var(--s5);
    outline: none;
  }
  .din:focus {
    border-color: var(--acc);
    box-shadow: 0 0 0 3px var(--acc-soft);
  }
  .din::placeholder {
    color: var(--faint);
    font-weight: var(--w-mid);
  }
  /* A STRING THIS PAGE COULD NOT READ LOOKS UNREAD. The refusal quoting it is
     in the block below the form; this is the field saying which one it is
     about, so the two are never hunted for separately. */
  .din[aria-invalid='true'] {
    color: var(--down);
    border-color: var(--down);
  }
  .dbtn {
    flex: 0 0 auto;
    appearance: none;
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--dim);
    font: inherit;
    font-size: var(--fs-base);
    line-height: 1;
    padding: 8px var(--s4);
    border-radius: var(--r);
    cursor: pointer;
  }
  .dbtn:hover {
    color: var(--acc);
    border-color: var(--acc);
  }
  .dbtn[aria-expanded='true'] {
    color: var(--acc);
    border-color: var(--acc);
    box-shadow: 0 0 0 3px var(--acc-soft);
  }
  /* A REFUSED FIELD LOOKS REFUSED WHEREVER IT SITS. `.field` is the day cell
     inside the strip; `.field` is the archive folder rung, which is a whole
     column of its own. Both carry `bad` from the same `problemFor` set. */
  .field.bad .din,
  .field.bad .din {
    border-color: var(--down);
    background: var(--down-soft);
  }

  .cal {
    position: absolute;
    z-index: 40;
    top: calc(100% + var(--s2));
    left: 0;
    /* Seven columns of tabular digits plus the panel's own padding. Narrower
       and the 30th of a month wraps its own column. */
    width: 272px;
    padding: var(--s4);
    background: var(--panel);
    border: 1px solid var(--line-hard);
    border-radius: var(--r4);
    box-shadow: var(--e3);
  }
  .cal.up {
    top: auto;
    bottom: calc(100% + var(--s2));
  }
  /* THE RIGHT-HAND CELL HANGS ITS PANEL FROM THE RIGHT EDGE. Left-anchored, the
     To panel ran past the card and clipped its own footer. */
  .field.right .cal {
    left: auto;
    right: 0;
  }
  .cal-h {
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin-bottom: var(--s4);
  }
  .cal-h .nav {
    font: inherit;
    font-size: var(--fs-md);
    line-height: 1;
    color: var(--dim);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    background: var(--panel);
    cursor: pointer;
    width: 26px;
    height: 26px;
    flex: 0 0 auto;
  }
  .cal-h .nav:hover:not(:disabled) {
    color: var(--acc);
    border-color: var(--acc);
  }
  .cal-h .nav:disabled {
    opacity: 0.3;
    cursor: not-allowed;
  }
  /* THE CAPTION IS THE CONTROL, and it takes the width the two menus took.
     One target between the arrows, so the header reads as a single instrument
     rather than as four things that happen to sit on a line. */
  .calcap {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--s3);
    height: 26px;
    padding: 0 var(--s4);
    appearance: none;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--r2);
    color: var(--ink);
    font: inherit;
    font-family: var(--mono);
    font-size: var(--fs-sm);
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    letter-spacing: var(--track-caps);
    cursor: pointer;
  }
  .calcap:hover {
    background: var(--panel-2);
    border-color: var(--line);
    color: var(--acc);
  }
  .calcap:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: 1px;
  }
  .capt {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* THE CARET SAYS WHICH WAY THE PANEL IS ABOUT TO GO. Down while the days are
     showing — press and a face opens; up while a pad is showing — press and it
     closes back to the days. */
  .capc {
    flex: 0 0 auto;
    /* 11px, not 9. At 9 the triangle rendered as a speck beside the caption and
       read as a stray dot — the one mark saying this text is pressable. */
    font-size: 11px;
    line-height: 1;
    color: var(--dim);
  }
  .calcap:hover .capc {
    color: var(--acc);
  }
  .capc.on {
    transform: rotate(180deg);
    color: var(--acc);
  }

  /* ONE HEIGHT FOR THREE FACES. 203px is the day face measured, not chosen:
     the weekday strip is one --fs-mini line at --lh-base over 2px and 4px of
     padding (~25px), and the grid is six 28px rows with five --s1 gaps (178px).
     Without this the panel would shrink when the months face opened and the
     note, the disclosure and the footer would all jump up under the pointer. */
  .calbody {
    min-height: 203px;
    display: flex;
    flex-direction: column;
  }
  /* THE MONTHS AND THE YEARS. Three columns, filling the block the days leave,
     so twelve chips are one glance rather than a scroll. `auto-fill` on the
     rows rather than a fixed four: the year span is FLOOR_YEAR to the ceiling's
     year and grows by one every January, and a hardcoded row count would start
     clipping the newest year the first time it did. */
  .calpad {
    flex: 1 1 auto;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-auto-rows: minmax(38px, 1fr);
    gap: var(--s2);
    align-content: stretch;
    overflow-y: auto;
  }
  .padc {
    display: flex;
    align-items: center;
    justify-content: center;
    appearance: none;
    font: inherit;
    font-family: var(--mono);
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    border: 1px solid transparent;
    border-radius: var(--r2);
    background: none;
    color: var(--ink);
    cursor: pointer;
  }
  .padc:hover:not(:disabled) {
    background: var(--panel-2);
    border-color: var(--line);
    color: var(--acc);
  }
  /* STRUCK, NOT ABSENT — the same rank the day grid gives a refused day, so a
     month outside the span reads the same way on both faces. */
  .padc:disabled {
    color: var(--faint);
    cursor: not-allowed;
    text-decoration: line-through;
    text-decoration-color: var(--down);
  }
  .padc.on {
    background: var(--acc);
    border-color: var(--acc);
    color: var(--on-acc);
    font-weight: var(--w-bold);
  }
  .padc:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: 1px;
  }
  /* SEVEN COLUMNS, MONDAY-FIRST, and the weekday strip shares the grid so the
     two can never fall out of step. */
  .calwk,
  .dgrid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: var(--s1);
  }
  .calwk span {
    text-align: center;
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    /* --dim. This row was 1.57:1 AND its container carries aria-hidden="true",
       so the weekday names reached neither the eye nor a screen reader. */
    color: var(--dim);
    padding: var(--s1) 0 var(--s2);
  }
  .cday {
    font: inherit;
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    height: 28px;
    border: 1px solid transparent;
    border-radius: var(--r2);
    background: none;
    color: var(--ink);
    cursor: pointer;
  }
  .cday:hover:not(:disabled) {
    background: var(--panel-2);
    color: var(--acc);
  }
  /* A NEIGHBOURING MONTH'S DAY IS STILL TAKEABLE — it is drawn quiet rather
     than hidden, because a grid with holes in it reads as broken. */
  .cday.oth {
    color: var(--faint);
  }
  /* NO SESSION IS NOT A REFUSAL. Weekends and NSE holidays hold no bars, so
     they are drawn light — a different mark from the line-through below, which
     means a bound refused the day. */
  .cday.nos {
    /* --dim, and the distinction is now visible instead of promised.
       `dayBlock()` never tests sessions, so these days are ENABLED and
       clickable -- the WCAG exemption for inactive controls does not apply, and
       at 1.57:1 the caption below ("Faint days hold no session") told the
       operator to read something he could not see. Three ranks now: a session
       day is --ink, a no-session day is --dim and clickable, an out-of-bounds
       day is --faint with a line-through. */
    color: var(--dim);
    font-weight: var(--w-mid);
  }
  .cday:disabled {
    /* --faint, which is now AA in its own right. Genuinely inactive, and the
       line-through remains the primary mark rather than the colour. */
    color: var(--faint);
    cursor: not-allowed;
    text-decoration: line-through;
    text-decoration-color: var(--down);
  }
  .cday.now {
    box-shadow: inset 0 0 0 1px var(--line-hard);
  }
  .cday.cursor {
    border-color: var(--acc);
  }
  .cday.picked:not(:disabled) {
    background: var(--acc);
    border-color: var(--acc);
    color: var(--on-acc);
    font-weight: var(--w-bold);
  }
  .calnote {
    margin: var(--s4) 0 0;
    font-size: var(--fs-micro);
    line-height: var(--lh-base);
    color: var(--faint);
  }
  /* THE FOLDED CAVEAT. The summary is a FACT — a feed name and a date — so the
     line carries its own answer closed, and only the paragraph explaining what
     to do about it opens. Marker suppressed and redrawn, because the platform
     triangle is drawn by the OS at the OS's size and is the one mark on this
     panel no rule here reaches. */
  .calwhy {
    margin-top: var(--s3);
    font-size: var(--fs-micro);
    line-height: var(--lh-base);
    color: var(--faint);
  }
  .calwhy summary {
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: var(--s2) 0;
    color: var(--dim);
    font-family: var(--mono);
    cursor: pointer;
    list-style: none;
  }
  .calwhy summary::-webkit-details-marker {
    display: none;
  }
  .calwhy summary::before {
    content: '\25B8';
    flex: 0 0 auto;
    /* Same reasoning as `.capc`: below about 10px a triangle in a mono face
       stops reading as a direction and starts reading as punctuation. */
    font-size: 10px;
    line-height: 1;
    color: var(--acc);
  }
  .calwhy[open] summary::before {
    transform: rotate(90deg);
  }
  .calwhy summary:hover {
    color: var(--ink);
  }
  .calwhy p {
    margin: var(--s2) 0 0;
    padding-left: var(--s5);
  }
  /* The reader navigated somewhere this field cannot take a day from. That is a
     legitimate place to be and it gets an answer in words, on its own line — not
     forty-two identical tooltips. */
  .calnote .far {
    display: block;
    margin-top: var(--s2);
    color: var(--warn);
  }
  .cal-f {
    display: flex;
    align-items: center;
    gap: var(--s3);
    margin-top: var(--s4);
    padding-top: var(--s4);
    border-top: 1px solid var(--line-soft);
  }
  .btn.sm {
    padding: var(--s2) var(--s4);
    font-size: var(--fs-xs);
  }

  /* ---- validation and cautions ----

     A REFUSAL IS AN EDGE, NOT A WALL. This was a full crimson border around a
     crimson fill, at card width, and it drew the same way for one date that is
     a day out as for a form with nothing in it — so the loudest object on the
     page was permanently loud and stopped carrying information. The rail on the
     left is the mark now, at full strength; the ground behind it is six per
     cent of the same hue over the panel, which reads as "this list is the
     refusals" without shouting it. The hue has not changed and nothing is
     quieter to a colour-blind reader: the rail is a position, the chip on every
     row still says which field, and the sentence still says what happened. */
  .probs {
    list-style: none;
    margin: 0;
    padding: var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    background: color-mix(in srgb, var(--down) 6%, var(--panel));
    border: 1px solid var(--line);
    border-left: 3px solid var(--down);
    border-radius: var(--r3);
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
  }
  .probs li {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s3) var(--s4);
    align-items: baseline;
  }
  /* THE SENTENCES START ON ONE LINE. `from` and `to` are four letters apart, so
     a chip sized to its own text left every message at a different x and the
     list read as ragged rather than as a column of refusals. Wide enough for
     the longest field name this page can put in one — `folder`. */
  .probs li > .tag {
    min-width: 56px;
    text-align: center;
  }
  /* THE FIX SITS AT THE END OF THE SENTENCE THAT EARNED IT. `margin-left: auto`
     rather than a column, so a row with no fix does not reserve a gutter for
     one — and `flex-wrap` above puts it on its own line rather than crushing
     the sentence when the card is narrow. */
  .probs .pfix {
    flex: 0 0 auto;
    margin-left: auto;
    align-self: center;
  }
  .caution {
    display: flex;
    gap: var(--s4);
    align-items: baseline;
    margin: 0;
    padding: var(--s4) var(--s5);
    background: var(--warn-soft);
    border-left: 2px solid var(--warn);
    border-radius: var(--r2);
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--ink-2);
  }
  .caution.loud {
    background: var(--down-soft);
    border-left-color: var(--down);
    margin: var(--s5) var(--s5) 0;
  }
  /* Inside a picker column the notice has to line up with the control it is
     about. `.loud`'s inset is for a caution sitting in a card that already has
     its own padding; here it would leave the message indented from the button
     that caused it. */
  .caution.drop {
    margin: 0;
    white-space: normal;
  }
  /* THE MESSAGE IS ONE FLEX ITEM, NOT SEVEN. Without this wrapper every inline
     `<b>` and `<span class="mono">` inside a `.caution` became a flex item of
     its own, and the truncation banner laid itself out as a row of words with
     ragged gaps between them instead of a sentence. */
  .probs .msg,
  .caution .msg {
    flex: 1;
    min-width: 0;
  }

  /* THE FOLDED CAUTIONS. Same chrome as `.wire` below it — one disclosure
     idiom on this page, and repeating it on every line was the stutter that made five
     of these unreadable. */
  
  /* `.tiles` IS GONE WITH THE ROW IT DREW. Leaving the rule behind is the
     exact pattern Gate W4 exists to catch — styling whose markup was deleted,
     with the comment beside it still asserting a layout the page no longer
     has. `.metric` is NOT removed: it is theme.css section 8 and other pages
     draw with it. */
  
  
  /* `.wire iframe` is gone with the frame it styled — see the receipt
     fold above for why a second application nested in this one had to go. */

  .actions {
    display: flex;
    align-items: center;
    gap: var(--s5);
    flex-wrap: wrap;
  }
  .ring {
    width: 12px;
    height: 12px;
    border: 2px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    display: inline-block;
  }

  /* ---- the run ---- */
  
  
  
  
  
  

  
  

  /* ---- the verdict strip ----
     ONE ROW AT THE HEAD OF THE TABLE. The sentence takes the space it needs and
     the rest hold their own width, so a long verdict wraps inside its own
     column instead of pushing the pills off the card. */
  
  
  
  
  
  
  
  
  
  /* IT MAY SHRINK AND IT MAY WRAP. `flex: 0 0 auto` on a row holding a button
     AND a sentence is a row that cannot give any width back, so the sentence
     pushed the strip wider than the card and ran off the right edge of the
     window — visible in the refused state, which is the one state where the
     sentence matters. `min-width: 0` is the half that lets the hint shrink at
     all; without it the flex item's automatic minimum size is its content. */
  
  

  /* ---- the outcome list ---- */
  
  
  
  
  
  
  
  
  
  /* `max-height`, NEVER `height`, AND THE DIFFERENCE IS THE WHOLE COMPLAINT.
     This was `height: min(46vh, 520px)`, so the outcome list stood 520px tall
     whether it held two hundred rows or one. A run that names a single
     instrument -- one ticked name, which is the ordinary case for a targeted
     pull -- drew one 30px row and then 490px of nothing, and the census below
     it was pushed off the bottom of the screen. Reported from the running page
     as "before the pull this view is awesome, after the pull it changes": the
     census had not gone anywhere, it was under half a metre of empty box.
     The virtualiser is safe with a box that shrinks: `vH` is read from a
     ResizeObserver on the element itself, so a short list reports a short
     height and `vCount` follows it. Above the cap the box stops growing and
     scrolls, which is the behaviour that was wanted in the first place. */
  
  
  
  
  
  
  
  

  /* ---- the instrument tick list ----
     A SEARCH BOX BIG ENOUGH TO TYPE IN. It is the full width of the menu and
     it sits above the list it filters, not beside it. */
  .srch {
    position: sticky;
    top: 0;
    z-index: 2;
    padding: var(--s4);
    background: var(--bg-2);
    border-bottom: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }
  .shint {
    margin: 0;
    font-size: var(--fs-mini);
    color: var(--faint);
  }
  .sact {
    display: flex;
    gap: var(--s3);
  }
  .sact button {
    flex: 1;
    padding: var(--s3) var(--s4);
    border-radius: var(--r2);
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    cursor: pointer;
  }
  .sact button:hover:not(:disabled) {
    border-color: var(--line-hard);
    background: var(--panel-2);
  }
  .sact button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .inslist {
    max-height: 46vh;
    overflow-y: auto;
    overscroll-behavior: contain;
    border-bottom: 1px solid var(--line);
  }
  .inslist label {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--s4);
    padding: var(--s3) var(--s5);
    cursor: pointer;
    border-bottom: 1px solid var(--line-soft);
  }
  .inslist label:hover {
    background: var(--panel-2);
  }
  /* PRESENT, VISIBLE, AND REFUSED. The row is dimmed, never removed — its
     title carries which feed does list it. */
  .inslist label.off {
    cursor: not-allowed;
    opacity: 0.62;
  }
  .inslist input {
    accent-color: var(--acc);
    width: 14px;
    height: 14px;
  }
  .inslist .nm {
    font-weight: var(--w-semi);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .inslist .ct {
    font-size: var(--fs-mini);
    color: var(--faint);
    white-space: nowrap;
  }
  .inslist .ct.warn {
    color: var(--warn);
  }
  .insnone {
    margin: 0;
    padding: var(--s5);
    font-size: var(--fs-xs);
    color: var(--faint);
    line-height: var(--lh-base);
  }

  /* ---- the two bulk actions ---- */
  /* A CAUTION UNDER THE ASK LINE IS A SENTENCE, NOT A COLUMN. Left in a
     248px track it wraps into a ribbon nobody reads. */
  .pickers > .caution {
    grid-column: 1 / -1;
  }
  /* BOTTOM RIGHT, WHICH IS WHERE THE MOCKUP PUTS THEM AND WHERE A DESTRUCTIVE
     CONTROL BELONGS: past everything the reader came here to set, on the far
     side of a hairline, so neither is on the path to the form's own controls.
     They were left-aligned in the flow of the strip with a paragraph of grey
     prose beside them, which read as three more things to fill in. */
  /* `.acts`, `.qb` and `.qb.danger` are gone with the two buttons they drew.
     Neither was reported by Gate W4 — the same blind spot `.cbar` sat in — so
     the test is again "does any markup match", not "did the compiler complain". */

  /* ---- the census ---- */
  .census {
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    min-width: 0;
  }

  /* ---- the partition ----
     A BAR AND A LEGEND, AND NEITHER IS A BUTTON. Every colour here comes from
     the four semantic hues; nothing invents one, so a verdict's colour in the
     bar, in its chip and on its row in the table is the same value from the
     same token. */
  .tally {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    min-width: 0;
  }
  /* THE WELL IS THE UNCOUNTED REMAINDER, and it should never show: the seven
     segments sum to the whole set by construction. It is here so that a rounding
     gap reads as a gap rather than as the page ending early. */
  .tallybar {
    display: flex;
    height: 7px;
    border-radius: var(--r-full);
    background: var(--well);
    overflow: hidden;
    min-width: 0;
  }
  /* NO GAP AND NO BORDER BETWEEN SEGMENTS. A partition whose parts are separated
     by a hairline reads as seven bars rather than as one quantity divided, and
     at a 1% segment the hairline is wider than the fact. */
  .tseg {
    height: 100%;
    min-width: 2px;
  }
  .tseg.up {
    background: var(--up);
  }
  .tseg.down {
    background: var(--down);
  }
  .tseg.warn {
    background: var(--warn);
  }
  .tseg.info {
    background: var(--info);
  }
  .tchips {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s3) var(--s5);
    min-width: 0;
  }
  /* THE COUNT IS TABULAR AND THE LABEL IS NOT. A row of chips is read down the
     numbers, so the digits hold their column; the words beside them are prose
     and take the sans face. */
  .tchip {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
    font-size: var(--fs-micro);
    color: var(--dim);
    white-space: nowrap;
  }
  .tchip b {
    font-family: var(--num);
    font-variant-numeric: tabular-nums;
    font-weight: var(--w-bold);
    color: var(--ink);
  }
  .tsum {
    font-size: var(--fs-micro);
    color: var(--faint);
    white-space: nowrap;
  }
  .tsum b {
    font-family: var(--num);
    font-variant-numeric: tabular-nums;
    color: var(--ink-2, var(--ink));
  }

  /* ---- the reach lanes ----
     THREE COLUMNS, AND THE TRACK IS THE ONLY ONE THAT STRETCHES. Name and date
     size to their content so every lane's track starts and ends at the same x —
     without that the bars are different lengths for a reason that has nothing
     to do with reach, and comparing two feeds becomes impossible. */
  .reach {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    max-width: 560px;
    min-width: 0;
  }
  .lane {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto auto;
    align-items: center;
    gap: var(--s3);
    min-width: 0;
  }
  .lname {
    font-family: var(--mono);
    font-size: var(--fs-micro);
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ltrack {
    display: flex;
    height: 7px;
    border-radius: var(--r-full);
    background: var(--well);
    overflow: hidden;
    min-width: 0;
  }
  /* DEAD TRACK IS DRAWN, NOT LEFT EMPTY. An empty gutter reads as "the bar has
     not loaded"; a filled-but-muted span reads as "this part returns nothing",
     which is the fact. The `--down` tint rather than plain grey because it is a
     refusal — days asked for that no request will answer. */
  .ldead {
    height: 100%;
    background: var(--down-soft);
    border-right: 1px solid var(--down);
  }
  .llive {
    height: 100%;
    background: var(--up);
  }
  .lat {
    font-family: var(--num);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-micro);
    color: var(--warn);
    white-space: nowrap;
  }
  /* A LANE THAT LOSES NOTHING SAYS SO QUIETLY. It is still drawn — silence
     reads as "not checked" as easily as "nothing wrong" — but it does not take
     the amber that means a date was moved. */
  .lat.dim {
    color: var(--faint);
  }
  /* THE HOLDING, AND ITS DEFAULT TONE IS THE QUIET ONE. Neutral means "reads
     cleanly, holds nothing" — the ordinary state before a first pull, and not
     something to colour as a problem. Only the two ends take a hue: red for a
     store that cannot be read, green for one with bars in it. */
  .lhold {
    font-family: var(--num);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-micro);
    color: var(--faint);
    white-space: nowrap;
  }
  .lhold.up {
    color: var(--up);
  }
  .lhold.down {
    color: var(--down);
    font-family: var(--sans);
  }

  /* ---- the completeness fill ----
     A RULE UNDER THE NUMBER, NOT A BAR BESIDE IT. The cell is already two
     tabular numbers and a slash; a second object competing for width would push
     the column wider for a fact the number states exactly. Two pixels welded to
     the bottom edge of the cell cost no width at all, and the row height is
     unchanged because it is positioned out of flow.
     `--well` is NOT painted behind it. An empty track on a row that has stored
     nothing draws a line the width of the column and reads as a full bar at a
     glance — the exact opposite of the truth. Zero stored means zero drawn. */
  .cscroll td .fill {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 2px;
    background: var(--warn);
    border-radius: var(--r-full);
    pointer-events: none;
  }
  .cscroll td .fill.up {
    background: var(--up);
  }
  /* The cell that holds a fill is its own containing block — see the single
     `.cscroll td.num` rule further down, which now carries `position: relative`
     alongside the scale and the alignment. It is declared THERE and not here:
     writing a second `.cscroll td.num` at this point would have recreated, in
     the same file and within the same hour, the identical-specificity duplicate
     removed from this block earlier today. */

  /* ---- the coverage blocks ----
     FOUR SQUARES, READ LEFT TO RIGHT AS TIME. Squares and not a bar: a bar
     would say "this much of the window", which `Months unproved` already says
     better as a number. Discrete blocks say WHICH PART, and four is the count
     the reference draws.
     The same four semantic hues the partition and the verdict tag use, so one
     verdict is one colour everywhere on this page. */
  .cov {
    display: inline-flex;
    gap: 2px;
    align-items: center;
  }
  /* THE WELL IS THE DEFAULT, so a tone that is somehow unset draws as an empty
     slot rather than inheriting the row's text colour and reading as settled. */
  .cov i {
    width: 9px;
    height: 9px;
    border-radius: 2px;
    background: var(--well);
    flex: none;
  }
  .cov i.up {
    background: var(--up);
  }
  .cov i.down {
    background: var(--down);
  }
  .cov i.warn {
    background: var(--warn);
  }
  .cov i.info {
    background: var(--info);
  }
  /* `h2.sec`, its two children and `.cbar` all go with the markup they drew.

     `.cbar` IS REMOVED EVEN THOUGH GATE W4 DID NOT ASK FOR IT. The compiler
     reported the three `h2.sec` selectors and not this one, so the ratchet was
     already back at its ceiling with a dead rule still in the file — a reminder
     that W4 is a floor on carelessness and not a proof of its absence. The rule
     had no markup left; that is the whole test, and it is the one this
     repository keeps saying matters. */
  .cgrid {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    box-shadow: var(--e1);
    overflow: hidden;
    min-width: 0;
  }
  /* THE TABLE SCROLLS INSIDE ITS OWN BOX. A wide table that widens the page
     puts the form's own controls off screen. */
  /* The pager's twin at the head of the panel: same padding, same ground,
     same hairline, turned the other way up. A search bar that invented its
     own band would be a second idiom for one job. */
  .cfind {
    padding: var(--s3) var(--s5);
    border-bottom: 1px solid var(--line);
    background: var(--bg-2);
  }

  .cscroll {
    max-height: min(58vh, 640px);
    overflow: auto;
    overscroll-behavior: contain;
  }
  /* ══ ONE TYPE SCALE PER ROW, AND THIS TABLE HAD TWO ══

     Measured on the running page, not guessed: `ADANIENT` rendered at 12.5px,
     `0 / 4,80,375` at 17px, `1 · not counted` at 12.5px, `Pull 61` at 13.5px.
     A 4.5px jump between neighbouring cells in the same row.

     The cause is that `.num` is not a table rule at all. In `$lib/theme.css` it
     shares its declaration with `.stat > .v` — the big value on a METRIC TILE —
     so it carries `--fs-data`, a 17px DISPLAY scale, and applying it to a cell
     drops a headline into a data row. `table` sets 12.5px underneath it, and
     the two never met.

     Both complaints came out of that one fault. The row read as two tables
     because two of six columns were a third larger; and the header row read as
     unaligned because `vertical-align: bottom` lines up boxes of different
     heights by their bottom edge, which puts text of different sizes on
     different baselines.

     So: one size for every cell in this table, at `--fs-sm` — up from 12.5,
     which was genuinely small for fifty rows, and down from a display scale
     that was never meant for a cell. THE FIGURES DO NOT LOSE THEIR EMPHASIS:
     they keep `--w-semi`, `tabular-nums` and `--ink-hi` from `.num`, so they
     still lead the row — by weight and colour, which is how a data table
     should do it, rather than by being physically bigger than their neighbours.

     Scoped to `.cscroll`. `.num` is shared with the tiles on three other pages
     and is not edited from here. */
  .cscroll th,
  .cscroll td {
    font-size: var(--fs-sm);
  }
  /* `td.num`'s SCALE MOVED DOWN TO JOIN ITS ALIGNMENT — see the single
     `.cscroll td.num` rule near the foot of this block. It was declared here
     too, and the note down there had already worked out why that was a trap;
     the two halves of one column's styling now live in one place. */
  /* THE CHIP AND THE BUTTON ARE NOT BODY TEXT and keep their own scales. A
     first pass set every descendant with `:global(*)` and blew the VERDICT chip
     up to 15px — the one thing on this row the operator had already called
     legible at 12. A chip is sized to be a chip. The button moves to the row's
     scale because it is the only cell whose content a reader has to READ and
     then PRESS, and at 13.5 it was the smallest thing in the row. */
  .cscroll td :global(.btn) {
    font-size: var(--fs-sm);
  }
  /* THE HEADER'S TWO SCALES ARE SET ONCE, ~90 LINES DOWN, AND NOT HERE.
     `.cscroll .hrow` and `.cscroll .hsub` were each declared twice in this
     block at identical specificity. The pair here carried nothing but the same
     `font-size` the later pair restates before adding weight, tracking, case
     and colour — so these two rules could never change a pixel, whichever way
     either was edited. That is the exact hazard the note on `.cscroll td.num`
     further down already names in its own words: "identical specificity, so the
     later rule wins and the override silently did nothing. One rule per thing."
     Applied here rather than only observed there. */
  .cscroll th {
    vertical-align: bottom;
  }
  /* THE SLACK IS THE DEFAULT `auto` DISTRIBUTION AGAIN, AND THAT IS A FIX
     UNDOING ITS OWN WORKAROUND.

     `th:first-child { width: 100% }` stood here to stop the leftover width
     landing as ragged empty space inside six columns. It worked, and it was
     treating a symptom: the columns sized unevenly because two of them carried
     a 17px display scale and four carried 12.5px, so `auto` was balancing boxes
     whose contents were a third apart. Handing all the slack to the name column
     hid that — and cost 592px of empty band beside a 150px name.

     With one type scale across the row, `auto` measured at 1440px gives
     245 / 311 / 229 / 237 / 227 / 164 — the even rhythm the rule was written to
     force, arrived at by the columns actually agreeing rather than by one of
     them absorbing everything. Measured both ways in the browser before this
     was removed. */
  /* A HEADER THAT IS NOT A BUTTON STILL STACKS ITS UNIT UNDER ITS NAME.
     `.sort` is the flex column that puts "bars, from the NSE calendar" on its
     own line under "Bars stored / expected"; the one column with no sort
     control inherited none of that and ran "Attempts  asked, and counted"
     along a single line, half a size larger than every header beside it.
     The direct-child combinator is what keeps this off the sortable headers —
     theirs are nested inside the button and are already handled. */
  .cscroll th > .hrow,
  .cscroll th > .hsub {
    display: block;
  }
  .cscroll .sort {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    color: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    cursor: pointer;
  }
  /* ══ THE VOID UNDER A RIGHT-ALIGNED HEADER ══

     Measured on the running page: every column WAS aligned — each header's
     text and its data agreed on an edge to the pixel. What was wrong is that
     the two `.num` columns agreed on the RIGHT one, while their headers are far
     wider than their values:

         BARS STORED / EXPECTED   header 194px   data 109px   85px of void
         MONTHS UNPROVED          header 132px   data  45px   87px of void

     Right-aligning pins the value to the far edge, so `61/61` sat under the
     last third of a label that began 87px to its left, and the eye stopped
     reading them as one thing. Worse at the Months/Verdict boundary, which is
     where it was reported: the last right-aligned value and the first
     left-aligned one end up crammed together with all the whitespace piled on
     one side.

     RIGHT ALIGNMENT IS FOR MAGNITUDES, and neither of these is one. Both are
     `x / y` PAIRS — stored of expected, unproved of total — and nobody scans
     them for the largest; they are read for whether the two halves differ. The
     `y` half is identical on every row of a window, so nothing lines up better
     on the right than on the left, and `tabular-nums` keeps the digits in
     columns either way.

     Left, therefore, on all six: `voidLeft` measured 0 for every column, and
     the header label sits over the first character of its own data. Verified
     in the browser both ways before this was written. */
  .cscroll th.num {
    text-align: left;
  }
  .cscroll th.num .sort {
    align-items: flex-start;
    width: auto;
  }
  .cscroll .sort:hover {
    color: var(--ink);
  }
  .cscroll .hrow {
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
  }
  /* A COLUMN THAT CANNOT STATE ITS OWN UNIT IN THE HEADER is a column whose
     meaning lives in a tooltip. The second line is not decoration. */
  .cscroll .hsub {
    font-size: var(--fs-micro);
    font-weight: var(--w-reg);
    letter-spacing: 0;
    text-transform: none;
    color: var(--ghost, var(--faint));
  }
  /* LEFT — see the note on `.cscroll th.num .sort` for the measurement. THIS IS
     NOW THE ONLY `.cscroll td.num` IN THE BLOCK, which is what the earlier
     version of this note was asking for: a second one used to sit ~120 lines
     above carrying the `font-size` that has moved in beside the alignment. Two
     rules of identical specificity meant the later silently won and any edit to
     the earlier did nothing at all. One rule per thing. */
  .cscroll td.num {
    font-size: var(--fs-sm);
    text-align: left;
    /* THE CONTAINING BLOCK FOR `.fill`, which is welded to the cell's bottom
       edge and must measure against this cell rather than against whatever
       ancestor happens to be positioned. Scoped to `td.num` and not to every
       `td` — the `.field { position: relative }` rule removed from this file
       earlier was exactly that mistake, written for one box and applied to
       every box that looked like it. */
    position: relative;
  }
  .cscroll td.num.warn {
    color: var(--warn);
  }
  .cscroll td.num.up {
    color: var(--up);
  }
  .cscroll .kindtag {
    margin-left: var(--s3);
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--faint);
  }
  .cscroll .dash {
    color: var(--faint);
  }
  /* A REFUSAL YOU HAVE TO SCROLL SIDEWAYS TO READ IS A REFUSAL NOBODY MEETS.
     The empty row spans every column, so its cell is as wide as the widest
     data row -- measured at 1,274px inside a 742px window -- and `.empty`'s
     centring put the sentence 504px past the right edge, where the only clue
     that anything had been said was a stray fragment of it.
     `sticky` pins the block to the SCROLLER's left edge rather than the
     cell's, so it stays where the eye already is however wide the columns
     grow, and the text reads from the left like every other sentence on the
     page. */
  .cscroll .empty {
    position: sticky;
    left: 0;
    max-width: 68ch;
    text-align: left;
    /* `nowrap` IS INHERITED FROM THE TABLE and it defeats `max-width` on its
       own: the BOX obeys the 68ch and the TEXT runs straight out of it. That
       is why the first attempt at this measured as fixed and was not --
       `getBoundingClientRect()` on the div reported 25..560, inside the
       window, while a Range over its text nodes reported 41..1259, still 504px
       past the right edge. A sentence is not a column heading; it wraps. */
    white-space: normal;
  }

  .cscroll .empty b {
    display: block;
    color: var(--ink);
    margin-bottom: var(--s2);
  }

  /* ---- the pager ---- */
  .pager {
    display: flex;
    align-items: center;
    gap: var(--s5);
    flex-wrap: wrap;
    padding: var(--s3) var(--s5);
    border-top: 1px solid var(--line);
    background: var(--bg-2);
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .pager .of b {
    color: var(--ink);
    font-weight: var(--w-semi);
  }
  .pages {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: var(--s2);
  }
  .pg {
    min-width: 26px;
    padding: var(--s2) var(--s3);
    border-radius: var(--r2);
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .pg:hover:not(:disabled) {
    border-color: var(--line-hard);
    background: var(--panel-2);
  }
  .pg:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .pg.on {
    border-color: var(--acc);
    color: var(--acc);
    background: var(--acc-soft);
    font-weight: var(--w-bold);
  }
  .pg.gap {
    border-color: transparent;
    background: none;
    cursor: default;
    color: var(--faint);
  }

  .blank .alt {
    display: block;
    margin-top: var(--s6);
    text-align: left;
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--dim);
  }

  /* EVERY transition and animation this file adds is inside the guard, the
     same rule $lib/theme.css holds itself to. A reduced-motion operator gets
     the whole console with nothing moving. */
  @media (prefers-reduced-motion: no-preference) {
    /* THE PARTITION MOVES ONLY WHEN THE PARTITION CHANGED, which is the whole
       test for whether motion belongs on this page.

       A segment's width IS the count it stands for. When a pull settles a
       month, `never pulled` shrinks and `verified` grows by exactly as much —
       so animating the width is not decoration, it is the one moment the bar
       has something to report, and a bar that jumps between two states makes
       the reader diff two pictures from memory instead of watching the fact
       move. Nothing else here animates: no hover flourish, no entrance on every
       redraw, no pulse. A redraw that changes no count produces no motion at
       all, because the width it transitions to is the width it already has.

       `--d-enter` rather than a hover duration: this is a quantity travelling,
       not a control acknowledging a press, and it has to outlast a saccade to
       read as movement rather than as a flicker. It is the same 220ms the
       panels enter on, which is deliberate — one page, one sense of pace. */
    .tseg,
    .ldead,
    .llive {
      transition: width var(--d-enter) var(--ease-out);
    }
    .din,
    .dbtn,
    .cday,
    .padc,
    .calcap,
    .cal-h .nav {
      transition:
        background-color var(--d-hover) var(--ease-out),
        border-color var(--d-hover) var(--ease-out),
        color var(--d-hover) var(--ease-out),
        box-shadow var(--d-state) var(--ease-out);
    }
    /* The caret turns rather than swapping glyph, so the press reads as the
       same control changing state instead of two controls alternating. The
       disclosure's marker turns for the same reason and on the same timing. */
    .capc,
    .calwhy summary::before {
      transition:
        transform var(--d-state) var(--ease-out),
        color var(--d-hover) var(--ease-out);
    }
    /* THE FACE ARRIVES FROM WHERE IT WAS OPENED. Twelve chips that simply
       exist read as a repaint; twelve that rise the last few pixels read as
       the grid folding into them. --d-enter is the token the row animations
       already use, so this is the page's own timing and not a second one. */
    .calpad {
      animation: bx-pop-in var(--d-enter) var(--ease-out);
    }
  }
  /* THE SERVER-OWNED RUN'S OWN CARD. Deliberately quiet: it is on screen for
     the whole of a long backfill, so it reads as instrumentation rather than
     as an alert. */
  .runcard {
    margin-top: 10px;
    border: 1px solid var(--line, #2a333c);
    border-radius: 8px;
    overflow: hidden;
  }
  .runtop {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    padding: 8px 12px;
    font-size: 12.5px;
    border-bottom: 1px solid var(--line, #2a333c);
  }
  .runtop .runwhere { margin-left: auto; opacity: .7; }
  table.runfeeds { width: 100%; border-collapse: collapse; font-size: 12.5px; }
  table.runfeeds td { padding: 6px 12px; border-bottom: 1px solid var(--line, #2a333c); }
  table.runfeeds tr:last-child td { border-bottom: 0; }
  td.rf-v { font-weight: 600; white-space: nowrap; }
  td.rf-n { white-space: nowrap; font-variant-numeric: tabular-nums; opacity: .8; }
  td.rf-d { width: 99%; opacity: .8; }
  td.rf-e { white-space: nowrap; opacity: .8; }
</style>


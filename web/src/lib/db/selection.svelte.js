/**
 * /db — THE SELECTION: every rung of the cascade, and the effects that seed it.
 *
 * Feed -> universe -> instrument -> segment -> timeframe -> expiry -> strike ->
 * moneyness -> side, each as `$state`, and every derived set between them, down
 * to `matched` — what every grid on the page reads.
 *
 * Every line below was MOVED out of `routes/db/+page.svelte` by WP-00 (the /db
 * page split). The only edits to a moved line are the ones the move needs: a
 * page-level name that now lives in another file is imported, or, for state,
 * read as `db.<name>`; a plain module exports what it declares; and a callback
 * over such a value marks its parameter `any`, the way the page already did
 * elsewhere. Nothing was fixed in the move: every defect the audit found is
 * still present byte for byte, and is fixed by the package that owns this file
 * next.
 *
 * Owner after WP-00: WP-50 (selection model, controls and URL state).
 *
 * HOW A FACTORY WORKS. Each `export function x(db)` declares its runes exactly as
 * the page script did, then publishes every name onto `db` as a getter (and a
 * setter for `$state`). A name declared in ANOTHER factory is read as
 * `db.<name>`. The
 * shell calls the factories in the order the page script created its effects,
 * so every `$effect` is still created — and still runs within a flush — in
 * exactly the order it did before the split.
 */
import { feeds, selectFeed } from '../feeds.svelte.js';
/* THE FEED'S OWN MASTER, ALREADY ON HAND. `+layout.svelte` calls
   `loadCatalogue(feeds.active)` on every feed change, so the universe rung
   costs this page NO request — it is a join against a list the layout has
   already paid for. It also arrives stamped with the feed it answers for,
   which is the only thing that makes a count under this feed's name
   checkable. See `universeRefusal`. */
import { catalogue } from '../index.svelte.js';
import { UNIVERSES, EVERYTHING, tokensOf, LEGACY_BLIND, censusKeyOf, SEG_VIEW, VIEWS } from './notes-controls.js';
import { fmt, txt, tfCmp, TF_SECS } from './notes-labels.js';
import { parseKey } from '../instrument.js';
import { monthLabel, dayLabel, istDay, timeLabel } from '../dates.js';
import { strikeText } from './decoration.js';
import { replaceState } from '$app/navigation';
import { page } from '$app/state';
import { ask } from '../ask.js';
import { askCalendar } from '../calendar-owed.js';
import { createPageRequests } from '../page-requests.js';
import { encodeDb, decodeDb } from '../urlstate.js';
import {
  segmentOf,
  contractRungsOf,
  foldInstruments,
  heldSpan,
  eraOf,
  sessionGrid,
  expiryTags,
  calendarOf,
  dayCode,
  keepOffered,
  linkOfKey,
  rebound,
  timeBound,
  DEFAULT_INSTRUMENT,
  DEFAULT_SEGMENT
} from './selection-model.js';
/**
 * THE DB VIEW — what the store actually holds, for ONE feed.
 *
 * Named `/db` because that is what it is: a database view. It never shows two
 * feeds side by side; the selector in the top bar chooses one and this shows
 * that one's rows.
 *
 * # Why it is built the way it is
 *
 * **Summary before detail.** Twenty thousand rows answer "which
 * instrument-month" and answer nothing about the shape of the store. The
 * strip at the top is the question an operator actually opens this page with:
 * how much is here, how much of it is complete, and where are the holes. The
 * month band underneath it is the same question at the only other altitude
 * that matters — a hole is almost never one instrument, it is a day the pull
 * did not finish.
 *
 * **Sortable, because the interesting rows are never alphabetical.** The
 * instrument missing 8,478 bars matters more than the one that is full, and
 * alphabetical order buries it at whatever letter it starts with. Every
 * column that HOLDS DATA sorts, and the direction is on screen — an arrow and
 * `aria-sort`, not a convention the operator has to remember.
 *
 * **Windowed.** Only the visible slice is in the DOM. Twenty thousand rows is
 * already past what a browser lays out comfortably and 93,776 — the census
 * size `docs/06-limits.md` §34 projects for the end of the stated backfill —
 * is not survivable at all. Rows are absolutely positioned inside a spacer of
 * the exact total height, so the scrollbar is honest at any row count and no
 * rounding accumulates down the list.
 *
 * **Indexed filtering.** A prefix Map is built once per dataset in yielding
 * chunks. Longer text and membership filters inspect only the chosen prefix
 * bucket; that bucket can be large. No request is made per character.
 *
 * **The two percentage columns are LIVE, and most of their cells are still a
 * dash.** They used to be refused outright, because `/store.json` carried no
 * price at all; D-0067 put the month's first and last close in the census and
 * D-0069 made the server compute the ratio. What arrives now is INTEGER BASIS
 * POINTS — `125` is +1.25% — or `null` and one of five reason codes.
 *
 * A dash is therefore never "nothing happened" and never a zero: it is one
 * named fact out of five, and hovering it says which. The commonest by far is
 * `corporate_action_unverified` — no split-and-bonus threshold is sourced
 * anywhere in this repository, so `docs/05-decisions.md` D-0018 refuses every
 * equity rather than printing an unadjusted 1:5 split as an 80% crash.
 * Indices never split and do show a number. `CLAUDE.md` §4: degrade loudly
 * and name the reason.
 */
import { untrack } from 'svelte';


/**
 * THE SELECTION — every rung's state and every derived set of the cascade, down
 * to `matched`, plus the narrowing, refusal and reset that describe it.
 *
 * Creates no effect, so where the shell calls it does not change behaviour.
 *
 * @param {any} db the page's shared state object (see the shell)
 */
export function selectionState(db) {
  let filter = $state('');
  let month = $state(''); /* '' = every month */
  let kind = $state(''); /* '' = every kind  */
  let holesOnly = $state(false);

  /* ---- THE CASCADE'S NEW RUNGS ---------------------------------------
     Feed → universe → instruments, and a month WINDOW across the top of the
     table. Every one of them is stored in its KEY form and never in its label:
     `universe` is a slug out of `UNIVERSES`, and `fromMonth` / `toMonth` are
     raw `YYYY-MM` — the same string `r.month` carries, so `>=` and `<=` are
     chronological string comparisons and never alphabetical ones over
     `Apr 2020`. The rule is stated at the top of `$lib/dates.js`; this is the
     page where breaking it would be invisible, because the table would render
     perfectly and return nothing.

     `''` MEANS "NO BOUND" IN ALL THREE, never "the empty month". A range with
     one end open is the ordinary case — "everything since March 2020" — and it
     has to be expressible without inventing a sentinel date the store has
     never heard of. */
  let universe = $state(''); /* '' = Everything: no membership join at all */
  /* THE MONTH RANGE IS RETIRED, and these two are the last of it. Kept as
   * declarations only so `reset()` and the `filtered` guard keep naming the
   * same set of state; both are permanently empty and nothing writes them. */
  let fromMonth = $state('');
  let toMonth = $state('');

  /* ---- THE DAY WINDOW, FOR SPOT AND FOR A CONTRACT ---------------------
   *
   * A spot series runs forever, so "show me this week" is a real question and
   * a calendar is the only way to ask it. It is just as real for a contract:
   * a 4.5-year NIFTY option stored from 2018 is not one question, and the
   * server narrows a contract's bars to the chosen days exactly as it does a
   * spot series (DB-046, DB-G89). The labels used to say "spot only"; the code
   * never did.
   *
   * `YYYY-MM-DD`, the same shape `b.day` carries, so the compare is a string
   * compare and never a Date. Empty means unbounded on that side. The fields
   * open on the span the CENSUS says this selection holds (`heldDays`), and
   * `daySeed` remembers that span so "narrowed" means "differs from it".
   */
  let fromDay = $state('');
  let toDay = $state('');

  /**
   * THE TIME-OF-DAY HALF OF THE SAME WINDOW.
   *
   * The day pair above narrows to a DATE, and at `1min` a date is 375 rows —
   * so "show me 27 Aug at 14:32" meant setting both dates to that day and then
   * paging sixty-three times at six rows a page, reading the Time column. The
   * grid has always HAD the minute; there was no way to ask for one.
   *
   * `HH:MM`, zero-padded, which is why these are compared as STRINGS and never
   * parsed: `'09:15' <= '14:32' <= '15:30'` is already true lexicographically,
   * so the bound needs no clock arithmetic and no timezone of its own.
   *
   * COMPARED AGAINST `timeLabel(row.ts)` — the SAME function that renders the
   * Time column, not a second one that agrees with it today. A filter that
   * computes the minute differently from the cell it filters is a filter that
   * hides the row it is pointing at, and both would look right in isolation.
   *
   * Empty means unbounded on that side, exactly like the day pair.
   */
  let fromTime = $state('');
  let toTime = $state('');

  /**
   * WHERE TO LAND, which is a different question from what to SHOW.
   *
   * A window answers "only these rows". This answers "put me at this row" —
   * it filters nothing, it moves the page to whichever bar is nearest the
   * moment asked for and marks it, so the rows either side of it stay on
   * screen. Reading a print needs its neighbours; a filter that returned one
   * row would take them away.
   */
  let goTime = $state('');
  /** The `rk` of the row a jump landed on, so the grid can mark it. */
  let landedRk = $state('');

  /* ---- THE BAR-LENGTH RUNG, WHICH THIS PAGE PRINTED AND COULD NOT CHOOSE
     The rung has been a COLUMN since the server stopped stamping `"1m"` on
     every row, and it has never been a CONTROL: an operator could see that a
     row was `1day` and had no way to ask for only those. It is the axis
     `/ingest` and `/markets` both call "Timeframe", so it is called that here.

     `''` MEANS EVERY RUNG THE STORE HOLDS, and it is the opening value on
     purpose. Defaulting to one rung would silently narrow the table on load
     and move every counted figure on this page — bars, sessions, coverage,
     months — while the control read like an untouched default. The same rule
     `month`, `kind` and `universe` already follow.

     THE VALUE IS THE STORE'S OWN SPELLING — `1min`, `1day`, the exact string
     `store::path::Timeframe::as_str` writes onto `/store.json`'s `timeframe`
     field and the exact directory the bars are filed under. It is compared
     with `===` against `r.timeframe`, so it can never be a label. */
  let timeframe = $state(''); /* '' = every rung the store holds */

  /* ---- THE CONTRACT RUNG'S TWO SELECTIONS -----------------------------
     AN EMPTY SET MEANS NO JOIN AT ALL, exactly as `universe = ''` does, and it
     is NOT "every strike". The two are different and the difference is the
     whole of this page: selecting every strike keeps only rows that HAVE one,
     so every index, every equity and every future in the store drops out of
     the table. An empty set is the ordinary case here — a census whose rows
     are overwhelmingly not options — and it has to be expressible without
     inventing a strike the store has never held.

     THEY ARE REPLACED, NEVER MUTATED. `$state` proxies plain objects and
     arrays; a `Set` is neither, so `strikePick.add(x)` would change the value
     and notify nothing, and the table would stay on the previous selection
     while the panel showed the new one. Every writer below builds a new Set
     and assigns it.

     THE KEYS ARE STRINGS ON BOTH. `Picker` filters by `r.key.toUpperCase()`,
     so a numeric key would throw inside a shared component the moment the
     filter box was typed into. The strike's own paisa integer stays the
     integer on the row; only the selection key is its decimal spelling.
     @type {Set<string>} */
  /**
   * WHICH MENU IN THE PANEL IS OPEN — one value for the whole page, so "two are
   * open at once" is not a state this component can reach.
   *
   * The same rule `$lib/Picker.svelte` holds for itself, and for the same
   * reason: a per-menu `open` flag plus an outside-click close never shuts the
   * first menu, because a click on the SECOND menu's button is not outside the
   * first one's `.dd` — it is inside that one. Picker's own module-level id
   * closes its menus when anything else on the page is pressed, and
   * `onWindowDown` below closes this one for the same press, so the two never
   * overlap in either direction.
   *
   * THE SET IS DECLARED TO THE CHECKER AND NOT ONLY TO THE READER. Without
   * the `@type` below, `$state(null)` infers `null` and every assignment in
   * this file — `drop = 'feed'`, `drop = 'month'` — is an error `svelte-check`
   * reports and nobody reads. Six of them stood here before the three new
   * rungs were added; one annotation answers all nine.
   *
   * @type {'feed' | 'uni' | 'tf' | 'month' | 'exp' | 'side' | null}
   */
  /** @type {string | null} */
  let drop = $state(null);

  let strikePick = $state(new Set());
  /** @type {Set<string>} */
  let mnyPick = $state(new Set());

  /**
   * THE EXPIRY, AND IT IS A GATE RATHER THAN A FILTER.
   *
   * The owner's rule, in his words: *for futures and options without the
   * current selected expiry don't display anything.* So `''` here does NOT
   * mean "every expiry" the way `universe = ''` means "no membership join".
   * It means NO CONTRACT HAS BEEN CHOSEN, and every row that carries one is
   * held back until one is — because a futures bar belongs to ONE contract,
   * and putting two expiries under one heading would show a series no contract
   * ever traded.
   *
   * A row with NO expiry — every index and every equity — is untouched by it.
   * Gating those on a contract they do not have would be this page inventing
   * one for them.
   *
   * RAW ISO DAY, ALWAYS: `2026-10-29`, the exact string `Display for Expiry`
   * writes into the instrument name. It is the `{#each}` key, the `===` in the
   * gate and the sort key; `dayLabel` is applied at the render site only.
   * `YYYY-MM-DD` is fixed-width and most-significant-first, so `<` on the
   * string IS chronological order and `29 Oct 2026` is not.
   */
  let expiry = $state(''); /* '' = no contract chosen; contract rows are held */

  /**
   * WHICH SIDE OF THE CONTRACT — `''` (both), `'CE'` or `'PE'`.
   *
   * `''` is the ordinary "no join at all", the same meaning `strikePick`'s
   * empty Set carries: it narrows nothing and it is NOT "every option", so an
   * index does not drop out of the table for having no side. `'CE'` and `'PE'`
   * are the two strings `OptionSide::as_str` emits and the two `contractOf`
   * reads back off a name — never a label, never lower case.
   */
  let side = $state('');

  /**
   * THE SEARCH TEXT, APART FROM THE PICK (D-3337 item 1, DB-001).
   *
   * `filter` above is the EXACT instrument key the Instrument rung picked —
   * the store's own underlying spelling, matched by equality, never a prefix.
   * (Its name is the legacy one; `instrument` is published as its alias.) The
   * text a search box holds is a different thing and lives here, so typing
   * `NIFT` can list NIFTY, NIFTYIT and NIFTY100 without the grid blending
   * them: only a pick narrows the rows.
   */
  let query = $state('');

  /**
   * SHOW MONTHS OUTSIDE THE OPERATOR'S WINDOW (D-3338).
   *
   * Zerodha's descriptor carries the operator's window, 2018-01-01 to
   * 2026-09-30; 115 indices hold 82,079 `1day` bars dated before it from an
   * earlier run. Those months are hidden by default and shown on this toggle,
   * labelled "outside your window". Nothing is deleted.
   */
  let oow = $state(false);

  /**
   * THE FULL SPAN THE DAY FIELDS WERE LAST SEEDED WITH. The window narrows
   * exactly when `fromDay`/`toDay` differ from it (`dayNarrowing`).
   */
  let daySeed = $state({ from: '', to: '' });

  /**
   * TODAY'S IST DAY, which splits expired contracts from unexpired ones
   * (D-3337 item 6). Read once when the page opens; a page left open across
   * midnight keeps the day it opened on until it is reloaded.
   */
  const today = istDay(Date.now());

  /**
   * THE EXCHANGE CALENDAR OF RECORD for the active feed and rung (D-3336),
   * read from `/calendar.json`: the closed days the date fields strike and the
   * holidays an expiry shift is measured against. `null` until it answers;
   * `calWhy` names a failure.
   * @type {{from: number, days: string, fromDate: string, toDate: string} | null}
   */
  let calendar = $state.raw(null);
  let calWhy = $state('');
  let calFor = $state('');

  /** Why the address bar could not be written, or `''` (CLAUSE: §4). */
  let urlWhy = $state('');
  /** What a link asked for that this page could not honour, named. */
  /** @type {string[]} */
  let linkNotes = $state([]);


  /* ======================================================================
     THE UNIVERSE RUNG — feed → UNIVERSE → instruments
     ----------------------------------------------------------------------
     THIS PAGE IS BACKWARD-LOOKING AND EVERY WORD HERE OBEYS THAT. `/ingest`
     asks what a vendor can be asked for, so a name it cannot serve reads "not
     on this feed". `/db` asks what is on disk, so the same name reads "not
     stored", and a count reads "N of M held". The two are not synonyms: the
     gap between "the vendor lacks it" and "you never pulled it" is the entire
     reason this page exists, and `/store.json` can only ever answer the second
     — it knows what a pull wrote, never what a pull could have written.

     FOUR OF THE EIGHT NAMES ONCE COULD NOT BE COMPUTED. THEY CAN NOW, AND
     THIS PARAGRAPH IS THE RECORD OF WHY THOSE ROWS CHANGED STATE WITH NO EDIT
     TO THIS FILE. `crates/core/src/universe.rs` carries the ranked constituent
     lists — `NIFTY_50` at 50 names, `NIFTY_100` at 100, `NIFTY_200` at 200,
     `NIFTY_500` at 500 — each behind its own bit, and
     `crates/api/src/server.rs` appends those bits to `UNIVERSE_TOKENS` as
     `n50`, `n100`, `n200`, `n500`. It puts them in the `universes` ARRAY, beside
     the `universe` STRING it deliberately froze at three tokens so an older
     reader keeps working (D-0089 / D-0090). `tokensOf` below reads the array
     first and splits the string only when there is no array, so the membership
     this rung needs arrives on the wire and the four rows enable themselves
     the moment a master carrying the field answers for the selected feed.

     WHAT HAS NOT CHANGED IS THE PROHIBITION, and it is why nothing here counts
     a tier for itself. `NIFTY_TOTAL_MARKET` arrives in ALPHABETICAL order, so
     slicing its first fifty names would put `360ONE` in the NIFTY 50 and print
     it as a fact; `CLAUDE.md` §3 rule 1 forbids exactly that. A tier this page
     cannot read off the wire stays DISABLED and names the field that is
     missing — `tierRefusal` is that sentence — and the row is never dropped:
     the list is what exposes the gap, and a list with the gap removed looks
     complete.
     ====================================================================== */

  /**
   * A feed's own name for itself, never the wire string.
   *
   * A FUNCTION DECLARATION, so it hoists above every reader. `feedName` below
   * is the same lookup and now defers to this one — two spellings of "what is
   * this feed called" is how one of them goes stale, and this file has already
   * paid for that lesson twice (see the FORMATTING block at the top).
   *
   * `null` is a real argument: `feeds.active` is null before the list lands
   * and `catalogue.feed` is null whenever the last master read FAILED, and
   * both are asked "what is this feed called" while they hold it. No wire
   * string is ever null, so the lookup misses and the dash is the answer.
   *
   * @param {string | null} wire
   */
  function feedDisplay(wire) {
    return feeds.all.find((f) => f.wire === wire)?.display ?? '—';
  }

  /**
   * Whether the instrument list on hand answers for the feed on screen.
   *
   * `$lib/index.svelte.js` stamps every answer with the feed it was asked for.
   * Between a feed change and its reload — and forever after a reload that
   * failed — those rows belong to a DIFFERENT feed, and a membership count
   * printed under this feed's name would be a number produced by a query
   * nobody re-ran. Same guard `/ingest` uses, and for the same reason.
   */
  const catalogueIsThisFeed = $derived(catalogue.ready && catalogue.feed === feeds.active);

  /**
   * This feed's master, read as a plain array of rows.
   *
   * `$lib/index.svelte.js` declares `rows: []` with no element type, so
   * `svelte-check` infers `never[]` and every field read off a row becomes an
   * error that says nothing about this page. The annotation lives HERE because
   * that file is shared and not this pass's to touch. It changes no behaviour
   * and asserts no shape beyond "these are objects", which is all this join
   * reads: `exchange`, `segment`, `symbol`, `universe`.
   *
   * A FUNCTION, called inside each derived, so the read of `catalogue.rows`
   * happens during that derived's evaluation and is tracked. Hoisting it into
   * a `$derived` of its own would be one more thing to keep in step for no
   * gain.
   *
   * @returns {any[]}
   */
  function masterRows() {
    return catalogue.rows ?? [];
  }

  const chosenUniverse = $derived(UNIVERSES.find((u) => u.key === universe) ?? EVERYTHING);

  /**
   * EVERY MEMBERSHIP TOKEN THIS FEED'S MASTER ACTUALLY CARRIES — counted from
   * the rows on hand, never a list this file asserts about a server.
   *
   * This replaced a hardcoded `token: null` on the four NIFTY rows. That field
   * was the front end stating, as a constant, what a binary it cannot see does
   * and does not send — the kind of claim `CLAUDE.md` §6 calls a measurement
   * nobody took. It was also wrong in the safe direction only by luck: the day
   * the API is rebuilt it would keep the tiers dark with a sentence saying they
   * are impossible.
   *
   * EMPTY WHEN THE CATALOGUE IS NOT THIS FEED'S, and every reader treats that
   * as NOT MEASURED rather than as absent — `membershipWhy` is the sentence for
   * that state, and it is checked first everywhere this set is consulted. A
   * tier is refused only when a master that DOES answer for this feed has been
   * read and does not carry the token.
   *
   * ONE PASS over a list bounded at ~800 rows by decision, recomputed only when
   * the catalogue changes.
   */
  const tokensHere = $derived.by(() => {
    const s = new Set();
    if (!catalogueIsThisFeed) return s;
    for (const r of masterRows()) for (const t of tokensOf(r)) s.add(t);
    return s;
  });

  /**
   * Whether a universe row can be joined at all right now.
   *
   * Three states, and they are not two: `'ready'` — the token is in the master
   * on hand. `'unmeasured'` — no master answers for this feed yet, so nothing
   * is known either way. `'absent'` — a master for this feed WAS read and does
   * not carry the token. Only `'absent'` disables a row, because only `'absent'`
   * is a finding.
   *
   * @param {{ token: string | null }} u
   */
  const tierState = (u) => {
    if (!u.token) return 'ready'; /* Everything joins nothing, and always can */
    if (!catalogueIsThisFeed) return 'unmeasured';
    return tokensHere.has(u.token) ? 'ready' : 'absent';
  };

  /**
   * Whether the master on hand carries the ARRAY field at all — one probe, not
   * an assumption about a version.
   *
   * This is the fact that tells a MISSING FIELD from an EMPTY SET, and the two
   * are different findings with different fixes. Without it the page would
   * answer "rebuild the API" for a feed whose master simply lists nothing in
   * F&O, which is a diagnosis pointing at the wrong system.
   */
  const carriesArray = $derived.by(() => {
    if (!catalogueIsThisFeed) return false;
    for (const r of masterRows()) if (Array.isArray(r?.universes)) return true;
    return false;
  });

  /**
   * WHY A TIER IS NOT ON THE WIRE — and WHICH of the two reasons it is.
   *
   * Every clause is checkable against a file in this repository or against a
   * count taken here, which is the point: the next person does not have to
   * re-derive any of it, and nobody has to guess whether the gap is a
   * front-end bug, a server build, or an empty set.
   *
   * @param {string} label
   * @param {string} token
   */
  const tierRefusal = (label, token) =>
    !carriesArray && LEGACY_BLIND.has(token)
      ? `/instruments.json?feed=${feeds.active ?? ''} cannot express ${label} on the API this page is talking to. Its rows answer with the legacy "universe" string only, and crates/api/src/server.rs freezes that string at three tokens — index, fno and ntm — through LEGACY_UNIVERSE_TOKENS. The full "universes" array, which names n500, n200, n100 and n50, is emitted beside it by that same file (D-0089/D-0090): the source has this tier and the running binary predates it. Rebuild and restart the API and this row lights up with no change to this page. What will not happen is a tier synthesised here — NIFTY Total Market arrives in alphabetical order, and slicing its first fifty names would print 360ONE as a NIFTY 50 constituent.`
      : `${feedDisplay(feeds.active)}'s master lists no instrument in ${label}. Counted over all ${fmt(masterRows().length)} row(s) /instruments.json returned for this feed: the field that would name this set is present on them and not one carries it. That is an EMPTY SET, not a missing endpoint — nothing needs rebuilding, and there is simply nothing here to hold the store against.`;

  /**
   * WHY the universe narrowing is not in force, or `null` when it is.
   *
   * A selected filter that quietly does not apply is the fallback that hides a
   * failure `CLAUDE.md` §4 bans, and it is the exact failure this rung invites:
   * the join needs a list this page does not fetch, so the list can be absent,
   * stale, or for another feed entirely. When it is, the rows are NOT narrowed
   * — every stored row keeps rendering, which is the honest answer for a page
   * whose subject is what you hold — and this sentence says so on screen.
   */
  /**
   * Why NO membership can be counted at all, or `null` when it can.
   *
   * DELIBERATELY INDEPENDENT OF WHICH UNIVERSE IS SELECTED. The catalogue is
   * either on hand for this feed or it is not, and that is one fact about the
   * page rather than eight facts about eight chips — so the tooltip on a chip
   * nobody has clicked names the same reason the line under the row does, and
   * the two cannot come apart.
   */
  const membershipWhy = $derived.by(() => {
    if (catalogueIsThisFeed) return null;
    if (catalogue.error) return `/instruments.json could not be read: ${catalogue.error}`;
    if (!catalogue.ready) return 'reading /instruments.json…';
    return `the instrument list on hand answers for ${feedDisplay(catalogue.feed)}, not for ${feedDisplay(feeds.active)} — no count on this page is built from another feed's master`;
  });

  /* MEMBERSHIP IS CHECKED BEFORE THE TOKEN IS, and the order is the whole
     correctness of this expression. `tokensHere` is empty while no master
     answers for this feed, so asking it first would report every tier as
     "absent from the master" when the truth is that no master has been read —
     a finding invented out of a loading state. */
  const universeRefusal = $derived.by(() => {
    if (chosenUniverse.token === '') return null; /* Everything joins nothing */
    if (membershipWhy !== null)
      return `${membershipWhy}. Every stored row is still shown — nothing was hidden by a join that did not run.`;
    if (tierState(chosenUniverse) === 'absent')
      return tierRefusal(chosenUniverse.label, chosenUniverse.token);
    return null;
  });

  /**
   * The chosen universe's members as UNDERLYINGS, or `null` for NO JOIN.
   *
   * `null` is returned both for Everything and for every state the refusal
   * above names, so there is ONE expression deciding whether rows are narrowed
   * and one sentence explaining it. Two would drift.
   *
   * UNDERLYINGS AND NOT CENSUS KEYS, and that is a fix rather than a rename.
   * It held the master's own keys -- `NSE-FNO-BANKNIFTY` -- and `universed`
   * compared them against `r.instrument`, which for a contract row is
   * `NSE-FNO-BANKNIFTY-2026-07-28-4810000-PE`. Those are never equal, so
   * choosing ANY membership universe hid every stored contract: with "F&O
   * underlyings" selected both contract segments reported "nothing stored under
   * this segment" while 78 instrument-months were on disk, and the underlying
   * itself was a member of that very universe -- the picker said "1 of 211
   * held".
   *
   * A membership universe lists UNDERLYINGS; it does not and cannot list every
   * expiry and strike. So the join is by name on both sides, which is the same
   * correction the Instrument rung took: a contract belongs to a universe when
   * its underlying does.
   */
  const universeNames = $derived.by(() => {
    const token = chosenUniverse.token;
    if (!token || tierState(chosenUniverse) !== 'ready') return null;
    const s = new Set();
    for (const r of masterRows()) {
      if (!tokensOf(r).includes(token)) continue;
      const k = censusKeyOf(r);
      if (!k) continue;
      s.add(parseKey(k).underlying ?? k);
    }
    return s;
  });

  /** The `/feeds.json` row of the feed on screen, or `null`. */
  const activeFeed = $derived(feeds.all.find((f) => f.wire === feeds.active) ?? null);
  /** The feed's archive end (GDFL: 2026-09-24), or `null` (D-3338). */
  const archiveTo = $derived(/** @type {any} */ (activeFeed)?.archive?.to ?? null);
  /** The span this feed answers for: GDFL's archive, Zerodha's window. */
  const era = $derived(eraOf(activeFeed, oow));

  /**
   * THE CENSUS SPLIT BY THE OPERATOR'S WINDOW (D-3338). A month outside a
   * feed's `window` is held back unless `oow` is on, and COUNTED, so the
   * toggle can say how many it shows. One pass per census — the window's
   * month bounds are cut once, and the toggle only chooses which half to
   * read, so pressing it does not walk the census again.
   */
  const windowSplit = $derived.by(() => {
    const w = /** @type {any} */ (activeFeed)?.window;
    /** @type {any[]} */
    const out = [];
    if (!w || typeof w.from !== 'string' || typeof w.to !== 'string') return { rows: db.deco, out };
    const lo = w.from.slice(0, 7);
    const hi = w.to.slice(0, 7);
    /** @type {any[]} */
    const rows = [];
    for (const r of db.deco) (r.month < lo || r.month > hi ? out : rows).push(r);
    return { rows, out };
  });
  const inWindow = $derived({ rows: oow ? db.deco : windowSplit.rows });
  /** How many instrument-months the window holds back (0 with `oow` on). */
  const oowHidden = $derived(oow ? 0 : windowSplit.out.length);
  /** How many census months lie outside the window, held back or shown. */
  const oowMonths = $derived(windowSplit.out.length);
  /**
   * How many of those belong to the selection on screen — this instrument,
   * segment and rung — so the toggle says what it would add HERE (NIFTY 1day:
   * 36 months, 2015-01 to 2017-12), beside the feed's whole count. Walks the
   * out-of-window rows only.
   */
  const oowHere = $derived.by(() => {
    if (!filter) return 0;
    let n = 0;
    for (const r of windowSplit.out) {
      if (r.underlying !== filter || (timeframe && r.timeframe !== timeframe)) continue;
      if (kind && segOf(r) !== kind) continue;
      n += 1;
    }
    return n;
  });

  const universed = $derived(
    universeNames === null
      ? inWindow.rows
      : inWindow.rows.filter((/** @type {any} */ r) => universeNames.has(r.underlying))
  );

  /**
   * WHETHER THIS FEED HAS NO INSTRUMENT MASTER AT ALL (DB-041).
   *
   * GDFL publishes none; `/instruments.json?feed=gdfl` now lists its census
   * (every stored spot series and option underlying) with `master: null` and
   * membership from `core::universe`'s own lists. A row reading "in this
   * master" was false for every one of them.
   */
  const noMaster = $derived(
    catalogueIsThisFeed && masterRows().length > 0 && masterRows().every((r) => r?.master === null)
  );

  /**
   * `N of M held`, per universe — the backward-looking count, and the only
   * shape a count may take on this page.
   *
   * M is membership, out of this feed's master. N is how many of those the
   * store actually holds at least one month of. `M − N` is "not stored", never
   * "not on this feed" — this page cannot see a vendor's catalogue of what it
   * could serve, only the master's list of names and the store's list of files.
   *
   * ONE PASS PER UNIVERSE over a list bounded at ~800 rows by decision, and it
   * recomputes only when the store or the catalogue changes — never per
   * keystroke and never per scroll frame.
   */
  /* HELD IS COUNTED BY UNDERLYING. A master row for an option-only
     underlying is `NSE-FNO-ACC`, a key no census row carries (its rows are
     `NSE-FNO-ACC-<expiry>-<strike>-CE`), so a key join counted every such
     member "not held" over a store holding its contracts. */
  const universeCount = $derived.by(() => {
    const out = new Map();
    if (!catalogueIsThisFeed) return out;
    const stored = new Set();
    for (const r of db.deco) stored.add(r.underlying);
    for (const u of UNIVERSES) {
      if (!u.token || !tokensHere.has(u.token)) continue;
      let members = 0;
      let held = 0;
      for (const r of masterRows()) {
        if (!tokensOf(r).includes(u.token)) continue;
        members += 1;
        const k = censusKeyOf(r);
        if (stored.has(parseKey(k).underlying ?? k)) held += 1;
      }
      out.set(u.key, { members, held });
    }
    return out;
  });

  /**
   * STORED DATA THIS FEED'S MASTER DOES NOT LIST — counted, named, never hidden.
   *
   * `CLAUDE.md` §1 keeps BSE history on disk although the engine no longer
   * sweeps it; `/instruments.json` filters to what the feed's master gave an id
   * AND to the tracked surface. So a real, held instrument-month can be absent
   * from every membership universe through no fault of the pull. On a page
   * whose entire subject is what you hold, letting those rows disappear behind
   * a membership chip would be the worst outcome available — so the residue is
   * counted here and printed beside the rung, and `Everything` reaches it by
   * doing no join at all.
   *
   * `null` means NOT MEASURED, which is not zero: without a catalogue for this
   * feed there is no list to be absent from.
   */
  /* BY UNDERLYING, for the reason `universeCount` gives: a contract-month is
     listed when its underlying is. Joined on the full key this counted every
     GDFL option month "not listed" — 2,46,147 months across 1,34,519
     "instruments", which was every contract in the store (DB-041). */
  const outsideMaster = $derived.by(() => {
    if (!catalogueIsThisFeed) return null;
    const listed = new Set();
    for (const r of masterRows()) {
      const k = censusKeyOf(r);
      listed.add(parseKey(k).underlying ?? k);
    }
    const names = new Set();
    let months = 0;
    for (const r of db.deco) {
      if (listed.has(r.underlying)) continue;
      months += 1;
      names.add(r.underlying);
    }
    return { months, instruments: names.size };
  });

  const typed = $derived(filter.trim().toUpperCase());

  /**
   * THE EXACT-INSTRUMENT INDEX (DB-001): every row of the universe, folded by
   * underlying, once per census or universe change. A pick is then ONE Map
   * probe and the rows it returns are that underlying's and no other's.
   */
  const instrumentIndex = $derived(foldInstruments(universed));

  const instrumentRows = $derived([...instrumentIndex.values()].sort((a, b) => txt(a.key, b.key)));

  /**
   * O(1): the STORE'S OWN SPELLING of the instrument the text box currently
   * names, or `''`.
   *
   * A Map and not a Set, and the value is the raw key rather than the typed
   * text. `typed` is upper-cased — it has to be, because it is a probe against
   * the prefix index — so returning it would put a DISPLAY-cased string into
   * `picked`, and `picked` is not a label: it is the `selected` key the
   * instrument `Picker` matches its rows against, the key `narrowing` reports,
   * and the name the anchor prints. One character of case drift there and the
   * ticked row silently stops being ticked while the table filters correctly,
   * which is the same class of failure as an ISO date used as a label.
   */
  const instrumentKeys = $derived.by(() => {
    const s = new Map();
    for (const r of instrumentRows) s.set(r.key.toUpperCase(), r.key);
    return s;
  });
  const picked = $derived(instrumentKeys.get(typed) ?? '');

  /**
   * THE ROWS THE INSTRUMENT RUNG LEAVES — EXACTLY ONE UNDERLYING (DB-001,
   * D-3337 item 1).
   *
   * This was `textMatched`, a PREFIX match: picking NIFTY on Zerodha brought
   * about a hundred NIFTY* indices (page 2 of 1day showed NIFTY100's
   * 28-30 Sep 2026 closes above NIFTY's own), GDFL LT listed LTM's months, and
   * the NIFTY 2019-03-28 strike menu offered NIFTYIT's 15,400, 15,500 and
   * 16,000. Now a picked key is matched by equality through the index above.
   * A key the store does not hold (a master member never pulled) returns no
   * row, which is the honest answer; nothing picked returns the universe.
   */
  const instrumentMatched = $derived(
    picked ? (instrumentIndex.get(picked)?.rows ?? []) : filter ? [] : universed
  );

  /* ---- facets ---------------------------------------------------------
     A FACET'S OWN SELECTION IS EXCLUDED FROM ITS OWN COUNT, and every other
     selection is applied. Otherwise a toggle promises a number and delivers a
     different one: "INDEX 60" while a month holding 23 of them is selected is
     a count of rows the click cannot produce.

     The month band is the deliberate exception, and only for `holesOnly`: the
     cards are a ROLL-UP, not a filter chip — "2026-07 is 92.9% complete" is a
     statement about the month, and narrowing it to the incomplete rows would
     make every card read 0% complete and say nothing. */

  const kinds = $derived.by(() => {
    const c = new Map();
    for (const it of instrumentMatched) c.set(it.kind, (c.get(it.kind) ?? 0) + 1);
    // THE TIE-BREAK IS THREE-WAY, and `txt` is what makes it so. `a < b ? -1 :
    // 1` answers "greater" for two EQUAL keys, in both directions at once, and
    // a comparator that contradicts itself leaves the order of those elements
    // unspecified — the same counts could render in a different order on the
    // next pass. Same inputs, same order, everywhere.
    return [...c.entries()].sort((a, b) => b[1] - a[1] || txt(a[0], b[0]));
  });

  /**
   * EVERY INSTRUMENT THE CHOSEN UNIVERSE HOLDS, STORED OR NOT.
   *
   * # Why the store was the wrong source
   *
   * `instrumentRows` folds the STORE, so the rung offered only names that
   * already have bars. With one instrument stored it offered one row — and
   * "does this store hold TCS?" was a question the control could not be asked,
   * because TCS was not in the list to ask about. A rung that can only name what
   * you already have cannot tell you what you are missing, which on a page whose
   * whole job is reporting the store is the question worth asking.
   *
   * /ingest sources the same rung from `/instruments.json` and offers all 213
   * F&O underlyings whether or not a bar exists for any of them. This is that,
   * on the reading this page already has: `masterRows()` is the catalogue and
   * `universeNames` is the membership join, both already here and both already
   * used by `universeCount` two hundred lines up.
   *
   * # What each row carries
   *
   * `held` is the instrument-month count from the store, so a name with nothing
   * behind it says so on its own row rather than being absent. That is the
   * difference between "we hold none of it" and "it does not exist", which §4
   * does not allow a control to blur.
   *
   * # Everything is still Everything
   *
   * `universeNames === null` means NO JOIN — Everything, or a universe whose
   * membership could not be read. There is no catalogue set to offer then, so it
   * falls back to what the store holds, which is the only honest answer
   * available: with no membership list there is nothing to be missing FROM.
   */
  const instrumentOffered = $derived.by(() => {
    const held = new Map(instrumentRows.map((r) => [r.key, r]));
    /* THE ROW SHOWS WHAT /ingest's SHOWS — the trading SYMBOL and the kind.
       `insRows` there is `{ sym: m.symbol, detail: m.kind.toLowerCase() }`, so
       its menu reads `3600NE  equity`. This rung read the STORE KEY off the same
       instrument — `NSE-CASH-360ONE` — and printed a held count where the kind
       goes, so the two pages named the same thing two different ways and only
       one of them was the name a human uses.

       The KEY stays the store key, because that is what the table filters on.
       Only what is DISPLAYED changes. */
    const meta = new Map();
    for (const r of masterRows()) {
      const k = censusKeyOf(r);
      if (!k) continue;
      // BY UNDERLYING, to match what `instrumentRows` now folds by. The
      // master's own rows are spot keys, whose underlying is the symbol -- so
      // this is a no-op for them and the reason it is written anyway is the
      // day the master carries a contract.
      const u = parseKey(k).underlying ?? k;
      meta.set(u, {
        sym: r?.symbol ?? u,
        kind: r?.kind ? String(r.kind).toLowerCase() : '',
        master: r?.master ?? null
      });
    }
    /** @param {string} k @param {{months:number,bars:number,short:number,spotMonths:number,contractMonths:number}=} h */
    const decorate = (k, h) => ({
      key: k,
      sym: meta.get(k)?.sym ?? k,
      kind: meta.get(k)?.kind ?? '',
      /* `null` when the feed has no master (GDFL) or the name is not in it. */
      master: meta.get(k)?.master ?? null,
      months: h?.months ?? 0,
      bars: h?.bars ?? 0,
      short: h?.short ?? 0,
      spotMonths: h?.spotMonths ?? 0,
      contractMonths: h?.contractMonths ?? 0
    });
    if (universeNames === null) return instrumentRows.map((r) => decorate(r.key, r));
    /* ALREADY UNDERLYINGS AND ALREADY UNIQUE -- `universeNames` is a Set of
       names, so the conversion and the dedupe that used to sit here are the
       source's job now and happen once rather than per rung. */
    const out = [];
    for (const u of universeNames) out.push(decorate(u, held.get(u)));
    return out.sort((a, b) => txt(a.sym, b.sym) || txt(a.key, b.key));
  });

  /**
   * THE INSTRUMENT THE PAGE OPENS ON AND RESETS TO (D-3337 item 2): NIFTY when
   * this feed offers it with rows, else the first offered name with rows,
   * else the first offered. The same rule the seed below applies.
   */
  const defaultPick = $derived(
    (instrumentOffered.find((/** @type {any} */ r) => r.key === DEFAULT_INSTRUMENT && r.months > 0) ??
      instrumentOffered.find((/** @type {any} */ r) => r.months > 0) ??
      instrumentOffered[0])?.key ?? ''
  );

  /**
   * WHICH CONTRACT RUNGS THE CHOSEN SEGMENT ACTUALLY HAS.
   *
   * All four were drawn together, always, all four dead on a spot store. That is
   * wrong twice over: wrong that they are dead, and wrong that they are the SAME
   * four for every segment, because the three segments do not have the same
   * parts.
   *
   *   Spot             no expiry, no strike, no side. A continuous series has no
   *                    contract to narrow at all.
   *   Expired futures  an EXPIRY and nothing else. A future settles on a day; it
   *                    carries no strike and no call/put side.
   *   Expired options  expiry, strike, side — and moneyness, a distance measured
   *                    along a strike chain, which means nothing until strikes
   *                    exist.
   *
   * A strike box under Expired futures is not a disabled control, it is a control
   * that does not exist for that thing — drawing it dead says "this could be
   * filled" about something that never could.
   */
  const contractRungs = $derived(contractRungsOf(kind));

  /**
   * Which segment a stored row belongs to, off its own name, today's IST day
   * and the feed's archive end (D-3337 item 6, DB-G67): spot, an expired
   * future or option, one that settled after the archive ended, or one that
   * has not expired. See `segmentOf`.
   *
   * @param {{expiry: string | null, instrument: string}} r
   */
  function segOf(r) {
    return segmentOf(r, today, archiveTo);
  }

  /**
   * How many instrument-months of each segment this FEED holds — what decides
   * whether the cut and unexpired rows are drawn at all, and what the notes
   * count. One pass per census.
   */
  const feedSegments = $derived.by(() => {
    const n = new Map();
    for (const r of inWindow.rows) {
      const k = segOf(r);
      n.set(k, (n.get(k) ?? 0) + 1);
    }
    return n;
  });

  /**
   * HOW THIS FEED'S CONTRACTS GOT INTO THE STORE, in the descriptor's words
   * (DB-050). A folder feed was imported from the vendor's archive files by
   * `cli gdfl-import` (D-2804); a REST feed was pulled through its API.
   */
  const contractSource = $derived.by(() => {
    const f = /** @type {any} */ (activeFeed);
    if (!f) return '';
    if (f.kind === 'folder') return `imported from ${f.display}'s archive files${f.wire === 'gdfl' ? ' by cli gdfl-import (D-2804)' : ''}`;
    return `pulled through ${f.display}'s API`;
  });

  /**
   * THE SEGMENT ROWS, each with what this instrument holds of it and what the
   * feed holds overall (DB-050). The cut and unexpired rows are drawn only
   * when the feed holds such a contract or one is selected.
   */
  const segmentRows = $derived.by(() => {
    const n = new Map();
    for (const r of instrumentMatched) {
      const k = segOf(r);
      n.set(k, (n.get(k) ?? 0) + 1);
    }
    const name = /** @type {any} */ (activeFeed)?.display ?? feedDisplay(feeds.active);
    return SEG_VIEW.filter((s) => s.always || (feedSegments.get(s.key) ?? 0) > 0 || s.key === kind).map((s) => {
      const c = n.get(s.key) ?? 0;
      const all = feedSegments.get(s.key) ?? 0;
      const contract = s.key !== 'spot';
      /* WHAT THE STORE HOLDS, never a build note. A contract segment the feed
         holds none of says so and names what /feeds.json says about the
         feed's expired F&O; one it holds counts them and names their source. */
      const feedSays =
        !contract
          ? ''
          : all > 0
            ? ` ${name} holds ${fmt(all)} such instrument-month(s), ${contractSource}.`
            : ` ${name} holds none${/** @type {any} */ (activeFeed)?.fno === 'none' ? ' — /feeds.json says it serves no expired F&O (fno: none)' : ''}.`;
      return {
        key: s.key,
        name: s.name,
        detail: c > 0 ? `${fmt(c)} held` : 'nothing stored',
        title: `${c > 0 ? `${fmt(c)} instrument-month(s) held for this instrument.` : 'Nothing stored for this instrument.'} ${s.note}.${feedSays}`,
        /* NOT `disabled`. A segment with nothing stored is a legitimate thing to
           select — the answer is an empty table, which IS the answer to "do I
           hold any of these". Disabling it would refuse the question. */
        why: c === 0 ? 'nothing stored under this segment for this instrument' : undefined
      };
    });
  });

  /** Text and kind: the base the month cards roll up. */
  /* `segOf`, NOT `r.kind`. `kind` now holds one of /ingest's three — spot,
     futures, options — and those are derived from a row's expiry rather than
     read off the store's second field. See `SEG_VIEW`. */
  const segmented = $derived(kind ? instrumentMatched.filter((/** @type {any} */ r) => segOf(r) === kind) : instrumentMatched);

  /* ======================================================================
     THE BAR-LENGTH RUNG — feed → universe → TIMEFRAME → months
     ----------------------------------------------------------------------
     IT SITS HERE, ABOVE THE MONTH WINDOW, AND THAT PLACEMENT IS THE POINT. A
     row is one instrument-MONTH AT ONE RUNG: `fullestKey` is `(month, rung)`,
     `barsPerSession` is a function of the rung, and `monthCards` gives up and
     prints a dash the moment one month holds two rungs at once ("A month
     holding both 1min and 1day rows has no single session size"). Every month
     figure below this line is therefore cleaner when the rung is already
     chosen, and none of them is wrong when it is not — they say which.

     Placed BELOW the window instead, the window would offer months the chosen
     rung does not hold, and picking one would empty the table for a reason no
     control on screen was pointing at.

     THE COUNT BESIDE EACH RUNG IS TAKEN WITH THIS RUNG'S OWN SELECTION LEFT
     OUT and every coarser one applied — the facet rule stated further down,
     and the reason the number on a row is the number clicking it returns.
     ====================================================================== */

  /** Every rung the chosen universe holds, in LENGTH order, with its row count. */
  const tfAll = $derived.by(() => {
    const c = new Map();
    for (const it of segmented) c.set(it.timeframe, (c.get(it.timeframe) ?? 0) + 1);
    /* BY LENGTH, NEVER BY TEXT — see `tfCmp`. As text these read `15min`,
       `1day`, `1min`, which is an ordering by first character and not by
       anything a bar length has. */
    return [...c.entries()].sort((a, b) => tfCmp(a[0], b[0]));
  });

  /**
   * The universe, narrowed to one rung — the base every finer control reads.
   *
   * `timeframe === ''` falls through to `universed` UNCHANGED rather than
   * filtering on a sentinel, so "every rung" costs no pass at all and cannot
   * accidentally drop a row whose rung string this page has never heard of.
   */
  /* `segmented` WHEN NO RUNG IS CHOSEN, and it read `universed`: the moment
     between a feed change and the rung's re-seed then counted every
     instrument and every segment as "at this rung". */
  const timeframed = $derived(
    timeframe ? segmented.filter((/** @type {any} */ r) => r.timeframe === timeframe) : segmented
  );

  /**
   * WHY THE RUNG CANNOT BE CHOSEN, or `null` when it can.
   *
   * `CLAUDE.md` §4: a control that cannot be used says so on its own face.
   * "Nothing has been read yet" and "this universe holds nothing" are opposite
   * instructions — one is a wait, the other is a finding about the selection
   * above this one — and one greyed control cannot say both.
   */
  const tfRefusal = $derived.by(() => {
    if (tfAll.length > 0) return null;
    if (db.deco.length === 0)
      return `nothing has been read for ${feedDisplay(feeds.active)} yet, so no row has named a rung — this is an unread store, not a store without bar lengths`;
    /* NAME THE CONTROL THAT ACTUALLY NARROWED IT, AND NEVER ADVISE WIDENING
       ONE ALREADY AT ITS WIDEST.

       This blamed the universe unconditionally. MEASURED with Everything
       selected and the segment set to Expired futures, over a store holding
       2,187 instrument-months, it read:

         "Zerodha holds 2,187 instrument-month(s) and none of them is in
          Everything, so there is no row here to read a rung off. Widen the
          universe above and the rungs come back."

       Both halves are wrong. "None of them is in Everything" is a
       contradiction — Everything is the widest set there is and nothing can sit
       outside it. And "widen the universe" is advice that cannot work, because
       the universe is already as wide as it goes. The real cause was the
       SEGMENT, which the sentence never mentioned. A refusal that names the
       wrong rung sends a reader to a control that will not fix it, which is
       worse than one that says only "nothing matched". */
    const wideOpen = universe === '';
    const segName = kind ? (SEG_VIEW.find((s) => s.key === kind)?.name ?? kind) : null;
    if (wideOpen && segName) {
      /* NO ARTICLE BEFORE THE SEGMENT NAME. "a Expired futures series" is what
         a fixed `a` produces, and the three labels do not agree on which
         article they take — Spot wants "a", Expired futures wants "an". The
         preposition sidesteps the question rather than special-casing it. */
      return `${feedDisplay(feeds.active)} holds ${fmt(db.deco.length)} instrument-month(s) and none of them is stored under ${segName}, so there is no row here to read a rung off. The universe is already at Everything — it is the segment above that is narrowing this to nothing.`;
    }
    if (wideOpen) {
      return `${feedDisplay(feeds.active)} holds ${fmt(db.deco.length)} instrument-month(s) and none of them survives the rungs above, so there is no row here to read a rung off. The universe is already at Everything, so the narrowing is one of the rungs below it.`;
    }
    return `${feedDisplay(feeds.active)} holds ${fmt(db.deco.length)} instrument-month(s) and none of them is in ${chosenUniverse.label}, so there is no row here to read a rung off. Widen the universe above and the rungs come back.`;
  });

  /** Every month in the chosen universe AT THE CHOSEN RUNG, newest first. */
  const monthsAll = $derived.by(() => {
    const c = new Map();
    for (const it of timeframed) c.set(it.month, (c.get(it.month) ?? 0) + 1);
    return [...c.entries()].sort((a, b) => txt(b[0], a[0]));
  });

  /* THE MONTH CALENDAR IS GONE, AND SO IS EVERYTHING THAT DROVE IT.
   *
   * The from/to range went when MONTH stopped being a rung; the 186-line
   * snippet that drew it went next; this is the state and the handlers behind
   * both. `openCal`, `setCal` and `calKey` had no caller left, so `calOpen`
   * could never leave null, so `closeCal`, `stepYear`, `heldYears`,
   * `monthRows`, `monthKey`, `MONTH_SLOTS`, `monthsHeldIn`, `calYear`,
   * `calEl`, `calBtn` and `rangeInverted` were all unreachable behind it.
   *
   * Checked one at a time before cutting: every reference to each of them was
   * inside this block. `onWindowDown` stays — it also closes the pickers, and
   * only its `closeCal(false)` line went.
   */


  /* THE POINTER CLOSES IT TOO, and the button that opened it is excluded so
     the two do not fight: without that, pointerdown closes the panel and the
     click that follows reopens it, and the control never shuts. */
  /** @param {PointerEvent} e */
  function onWindowDown(e) {
    const t = e.target;
    if (!(t instanceof Node)) return;
    /* THE SAME PRESS CLOSES THE PANEL'S OWN MENU. `.picker[data-drop]` is the
       whole control — the button and the panel it opens — so a press inside the
       open one is not away, and a press on any other control is.

       THE ATTRIBUTE IS WHAT MAKES THE TEST EXACT, and it is not decoration.
       `$lib/Picker.svelte`'s own root element is ALSO `.picker`, so a bare
       `.closest('.picker')` would report a press inside a Picker's menu as a
       press inside this control and leave both open at once — two menus over
       one strip, one of them unreachable underneath the other. `data-drop` is
       carried only by the three menus `drop` actually governs. One value, so
       two of THOSE can never be open together either. */
    const inDrop =
      t instanceof Element
        ? t.closest('.picker[data-drop]')
        : t.parentElement?.closest('.picker[data-drop]');
    if (drop !== null && !inDrop) drop = null;
  }


  /* RAW KEY COMPARISON, BOTH ENDS. `YYYY-MM` is fixed-width and
     most-significant-first, so `<=` on the string IS chronological order. The
     labels never enter this function. */

  /**
   * The universe, narrowed to the month window — the base every finer control
   * reads.
   *
   * IT SITS ABOVE THE PREFIX INDEX ON PURPOSE. The index is rebuilt when this
   * set changes, which is a click and not a keystroke, and every keystroke
   * afterwards is still ONE Map probe into a bucket that already respects both
   * of the coarser rungs. Filtering after the probe instead would make the
   * text box the coarsest control on the page rather than the finest.
   */
  /* `timeframed`, NOT `universed`. The rung is the cascade step directly above
     this one — see the block on `tfAll` — so reading the set from before it
     would make the window, the prefix index, the instrument list and every
     count under them describe rows the chosen rung excludes. */
  /* `windowed` IS `timeframed`. The month WINDOW was the rung between them and
   * there is no month rung any more, so the stage collapses. Kept as a name
   * rather than dissolved into its readers because the cascade is easier to
   * read as one identifier per stage, and because `scoped` below is where a
   * month rung would return if it ever did. */
  const windowed = $derived(timeframed);


  /** Everything except the holes toggle — what that toggle counts against. */
  const scoped = $derived(month ? windowed.filter((/** @type {any} */ r) => r.month === month) : windowed);
  const holed = $derived(scoped.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + (r.short > 0 ? 1 : 0), 0));

  /**
   * EVERYTHING ABOVE THE CONTRACT RUNG.
   *
   * This is what the two contract panels count against and what their
   * narrowings are applied to. It is deliberately the set with BOTH of them
   * excluded rather than one each: a strike and a rung are two views of the
   * same axis — a rung IS a strike, seen from one side of the money — so a
   * count of one taken with the other applied would be a count of rows the
   * click cannot produce, which is exactly the rule the facet block above
   * states. The counted line under the rung says which set it is.
   */
  const matchedPreContract = $derived(holesOnly ? scoped.filter((/** @type {any} */ r) => r.short > 0) : scoped);

  /* ======================================================================
     THE EXPIRY GATE — the one control on this page that REFUSES BY DEFAULT
     ----------------------------------------------------------------------
     THE OWNER'S RULE, IN HIS WORDS: *for futures and options without the
     current selected expiry don't display anything.*

     So this is not a filter with an "all" setting. Every other control here
     opens at "no join at all" and narrows when it is touched; this one opens
     HOLDING BACK every row that carries a contract, and releases them one
     expiry at a time. The reason is the data, not the UI: a bar belongs to ONE
     contract, and a table that put October's and November's NIFTY futures
     under one heading would show a series no contract ever traded — the same
     class of invention `CLAUDE.md` §3 rule 1 forbids.

     IT IS NOT A HIDE. §4 bans the fallback that hides a failure, so the rows
     that are held back are COUNTED, the count is on the control's own clause,
     it is named in `narrowing`, it has its own branch in `blocked`, and the
     empty-table panel says in one sentence that clearing the filters will not
     release them and choosing an expiry will. A reader is never left to work
     out why the table is short.

     A ROW WITH NO EXPIRY IS NOT GATED. `r.expiry === null` is every index and
     every equity in the store, and gating those would be this page inventing a
     contract for an instrument that has none. The test is written against
     `null` and never against falsity, for the same reason `strike` is.

     THE LIST IS BUILT FROM `deco` AND THE COUNTS FROM `matchedPreContract`,
     which is the split the strike chain already uses: what contracts EXIST is
     a property of the disk and must not shrink when the table narrows; how
     many rows each one would return is a property of the selection.
     ====================================================================== */

  /** Every expiry the store names, ascending. Raw ISO days, so `txt` IS chronological. */
  const expiriesAll = $derived.by(() => {
    const s = new Set();
    for (const r of db.deco) if (r.expiry !== null) s.add(r.expiry);
    return [...s].sort(txt);
  });

  /** Rows per expiry, over everything above this rung — what clicking returns. */
  const expiryCount = $derived.by(() => {
    const c = new Map();
    for (const r of matchedPreContract) {
      if (r.expiry !== null) c.set(r.expiry, (c.get(r.expiry) ?? 0) + 1);
    }
    return c;
  });

  /* THE EXPIRIES THIS SELECTION ACTUALLY REACHES, sorted, for the menu.
   *
   * `expiriesAll` stays on `deco` and must: `expiryRefusal` DISABLES on it, and
   * a disable computed from a selection is the trap where choosing an expiry
   * then narrowing past it kills the only control that could clear it. The
   * gate's fast path reads it too, and its written proof is
   * `matchedPreContract ⊆ deco` — fold that list from the subset and the
   * implication inverts, so an empty list would RELEASE every contract row the
   * gate exists to hold back.
   *
   * So the list an operator picks from is separate from the list the refusal
   * is computed over. This is the first, and it is what the cascade requires:
   * only the expiries left by feed, universe, instrument, segment, timeframe
   * and month. Sorted, because every list on this page is. */
  const expiryOffered = $derived.by(() => {
    const seen = new Set();
    for (const r of matchedPreContract) if (r.expiry !== null) seen.add(r.expiry);
    return [...seen].sort();
  });

  /**
   * WEEKLY OR MONTHLY, THE WEEKDAY, AND ANY HOLIDAY SHIFT (DB-076; D-3337
   * item 5, D-3331), for every expiry of the picked underlying. Measured from
   * its stored expiries and the calendar of record — see `expiryTags`; no rule
   * about which weekday an index expires on is written here.
   */
  const expiryTagMap = $derived.by(() => {
    const own = picked ? instrumentIndex.get(picked)?.rows ?? [] : [];
    const set = new Set();
    for (const r of own) if (r.expiry !== null) set.add(r.expiry);
    const cal = calendar;
    return expiryTags(set, (iso) => dayCode(cal, iso));
  });

  /** How many rows above this rung carry a contract at all — the gate's denominator. */
  const contractHere = $derived(
    matchedPreContract.reduce(
      (/** @type {number} */ a, /** @type {any} */ r) => a + (r.expiry === null ? 0 : 1),
      0
    )
  );

  /**
   * THE GATE, APPLIED. Everything above the strike rung, with contract rows
   * held to the chosen expiry.
   *
   * With `expiry === ''` no contract row can match — there is nothing to match
   * — so every one of them drops and every non-contract row passes. That is
   * the owner's rule expressed as one comparison rather than as a special
   * case, which is why there is no branch here to get wrong.
   */
  const expiryGated = $derived(
    /* NO PASS AT ALL WHEN THE STORE NAMES NO CONTRACT, and that is a proof
       rather than an optimisation: `matchedPreContract` is a subset of `deco`,
       and `expiriesAll` is empty exactly when no row in `deco` carries an
       expiry — so no row in the subset can carry one either, and the filter
       below would keep every one of them. Falling through UNCHANGED is the
       same shape `windowed` and `matchedPreContract` already use, and it keeps
       today's store — indices and equities, no contract anywhere — off a
       20,516-row pass on every recompute. `expiryHeld` is a length difference,
       so it reads 0 here without a branch of its own. */
    expiriesAll.length === 0
      ? matchedPreContract
      : matchedPreContract.filter(
          (/** @type {any} */ r) => r.expiry === null || r.expiry === expiry
        )
  );

  /** How many rows the gate is holding back right now. Counted, never assumed. */
  const expiryHeld = $derived(matchedPreContract.length - expiryGated.length);

  /**
   * WHY THE EXPIRY PANEL CANNOT BE OPENED, or `null` when it can.
   *
   * Two reasons and they are opposites — an unread store is a wait, a store
   * with no contract in it is a fact about the pull — so one greyed control
   * spells whichever it is rather than going dim and silent.
   *
   * NEITHER REASON IS "EXPIRED CONTRACTS ARE FILTERED OUT", AND THE CONTROL
   * DOES NOT FILTER THEM. `expiriesAll` is folded from `deco`, which is
   * `/store.json`'s own rows before any membership join, and `expiryOf` is a
   * suffix test on the name with no comparison to today anywhere in it — so an
   * expired contract that is ON DISK is listed, counted and selectable, and the
   * default universe is `''`, no join at all, which is the one setting that
   * also reaches a stored instrument this feed's master no longer lists.
   * Nothing here is live-only.
   *
   * WHY THE STORE HOLDS NONE TODAY IS A DIFFERENT FACT AND IT IS NAMED, because
   * "this control lists nothing" and "this control hides something" look
   * identical from the outside. Measured, not assumed: `decode_master_row` in
   * `crates/core/src/vendor.rs` declines every FUT/CE/PE row as
   * `Skip::LiveContract`, and `fno_answer` in `crates/api/src/server.rs` answers
   * `POST /pull/fno` with 503 and `audit::Outcome::NotStarted` — its own reason
   * string is quoted below. So nothing in this build can put an expired contract
   * on the disk this page reads; the day one lands, this rung shows it with no
   * change here.
   */
  const expiryRefusal = $derived.by(() => {
    if (expiriesAll.length > 0) return null;
    if (db.deco.length === 0)
      return `nothing has been read for ${feedDisplay(feeds.active)} yet, so no instrument name has been looked at — this is an unread store, not a store without contracts`;
    /* WHAT THE STORE HOLDS, AND WHAT THE FEED SAYS — never a build note. The
       old sentence ended "nothing in this build can fetch one", which stopped
       being true when expired options were imported (DB-G67). */
    const fno = /** @type {any} */ (activeFeed)?.fno;
    return `not one of the ${fmt(db.deco.length)} instrument-month(s) ${feedDisplay(feeds.active)} holds is a contract: no stored name ends “-<YYYY-MM-DD>-FUT” or “-<YYYY-MM-DD>-<strike in paisa>-CE|PE” (crates/core/src/instrument.rs). An expiry is read off a stored name, never guessed, and nothing in this list is compared to today, so an expired contract on disk would be listed here.${fno === 'none' ? ` /feeds.json says ${feedDisplay(feeds.active)} serves no expired F&O (fno: none).` : ''}`;
  });

  /* ---- THE SIDE — the finest rung on the page --------------------------
     TWO STRINGS AND NOT THREE. `OptionSide::as_str` emits `CE` and `PE`; the
     third row on this control is `''`, which is NO JOIN AT ALL and is not a
     value the store has ever held. Picking `Both` narrows nothing and leaves
     every index in the table; picking `CE` keeps only rows whose name ends
     that way, which drops every instrument that is not an option — the same
     honest reading "at these strikes" already has on the strike panel. */

  /** Which sides the store names at all. Empty when it holds no option. */
  const sidesAll = $derived.by(() => {
    const s = new Set();
    for (const r of db.deco) if (r.side !== null) s.add(r.side);
    return [...s].sort(txt);
  });

  /**
   * THE ROWS THE SIDE RUNG COUNTS: everything above it, the strike and
   * moneyness picks included, the side's own pick left out — the facet rule
   * (DB-N09). Counted over `expiryGated` alone, the list read "Both 438 /
   * CE 236 / PE 202" with strike 11,600 chosen, which were the whole expiry's
   * counts; for that strike they are 8 / 6 / 2.
   */
  const sideBase = $derived.by(() => {
    let out = expiryGated;
    if (strikePick.size > 0) {
      out = out.filter((/** @type {any} */ r) => r.strike !== null && strikePick.has(String(r.strike)));
    }
    if (mnyPick.size > 0) {
      out = out.filter((/** @type {any} */ r) => {
        const k = db.rungKeyOf(r);
        return k !== null && mnyPick.has(k);
      });
    }
    return out;
  });

  /** Rows per side, with the gate above it applied — the facet rule, again. */
  /* THE SIDES THIS SELECTION REACHES, sorted — the cascade's ninth rung.
   *
   * `sidesAll` stays on `deco` and feeds `sideRefusal`, which DISABLES; a
   * disable computed from a selection is the trap where choosing a side and
   * narrowing past it kills the control that could clear it. Same split as
   * `expiryOffered` against `expiriesAll`, and for the same reason. */
  const sideOffered = $derived.by(() => {
    const seen = new Set();
    for (const r of sideBase) if (r.side !== null) seen.add(r.side);
    return [...seen].sort();
  });

  /** How many rows the Both row returns — what clicking it actually gives. */
  const mnyFilteredCount = $derived(sideBase.length);

  const sideCount = $derived.by(() => {
    const c = new Map();
    for (const r of sideBase) {
      if (r.side !== null) c.set(r.side, (c.get(r.side) ?? 0) + 1);
    }
    return c;
  });

  /** How many rows the side rung can reach at all, under the gate and the strike. */
  const sidedHere = $derived(
    sideBase.reduce(
      (/** @type {number} */ a, /** @type {any} */ r) => a + (r.side === null ? 0 : 1),
      0
    )
  );

  /**
   * WHY THE SIDE CANNOT BE CHOSEN AT ALL, or `null`.
   *
   * IT IS A FACT ABOUT THE STORE AND NEVER ABOUT THE SELECTION, exactly as
   * `strikeRefusal` is: `sidesAll` is counted from `deco`, the whole store for
   * this feed, so this cannot flip while the reader narrows. That matters
   * because this value DISABLES the control, and a disabled control that is
   * still holding a live narrowing is a filter with no way back — pick `CE`,
   * narrow to a month with no calls in it, and a selection-dependent refusal
   * would grey the only control that could undo it.
   *
   * "This selection reaches no option" is a real and useful thing to say, and
   * it is said — by `sideEmpty` below, on the clause, with the control left
   * open.
   */
  const sideRefusal = $derived.by(() => {
    if (sidesAll.length > 0) return null;
    if (db.deco.length === 0)
      return `nothing has been read for ${feedDisplay(feeds.active)} yet, so no instrument name has been looked at — this is an unread store, not a store without options`;
    return `${feedDisplay(feeds.active)} holds no option. A side is the last two characters of an option's name — crates/core/src/instrument.rs ends one “-CE” or “-PE” — and not one of the ${fmt(db.deco.length)} instrument-month(s) here ends either way. /store.json sends no side field, so a side can only ever be READ OFF A NAME.`;
  });

  /**
   * WHY THERE IS NO OPTION IN THIS SELECTION, or `null` when there is one.
   *
   * A SENTENCE, NOT A DISABLE. The store holds options — `sideRefusal` is
   * `null` — and the rungs above have simply left none here. The commonest
   * cause by far is the expiry gate one cell to the left, and naming that is
   * the difference between a control that looks broken and one that points at
   * the control which would fix it.
   */
  const sideEmpty = $derived.by(() => {
    if (sideRefusal !== null || sidedHere > 0) return null;
    if (!expiry && expiryHeld > 0)
      return `the ${fmt(expiryHeld)} contract row(s) this selection reaches are held back until an expiry is chosen, so there is no option here to take a side of yet — choose an expiry beside this and it fills`;
    return `this selection reaches no option: ${fmt(sideBase.length)} row(s) are here and none of them carries a side`;
  });

  /**
   * The selection, applied. `matched` is what every panel, tile, card and
   * column on this page below here already reads, so the contract rung reaches
   * all of them through one expression and none of them changed.
   *
   * An EMPTY selection narrows nothing, per the block on `strikePick`. A
   * non-empty one keeps only rows that carry the chosen strike or sit on the
   * chosen rung, which drops every row with no contract at all — that is the
   * honest reading of "at these strikes" and it is why the counted line under
   * the rung says how many rows carry a contract in the first place.
   */
  const matched = $derived.by(() => {
    /* THE GATE IS ALREADY IN `expiryGated` AND IT IS NOT OPTIONAL. Starting
       from `matchedPreContract` here and re-applying it would be two spellings
       of one rule, which is how one of them goes stale. */
    let out = expiryGated;
    /* AN EMPTY SIDE NARROWS NOTHING, per the block on `side`. `CE` keeps only
       rows whose name ends `CE`, which drops every index and every future —
       the honest reading of "calls only". */
    if (side) out = out.filter((/** @type {any} */ r) => r.side === side);
    if (strikePick.size > 0) {
      out = out.filter(
        (/** @type {any} */ r) => r.strike !== null && strikePick.has(String(r.strike))
      );
    }
    if (mnyPick.size > 0) {
      out = out.filter((/** @type {any} */ r) => {
        const k = db.rungKeyOf(r);
        return k !== null && mnyPick.has(k);
      });
    }
    return out;
  });

  /** How many rows carry a strike at all — the denominator of the strike rung. */
  const contractRows = $derived(
    expiryGated.reduce(
      (/** @type {number} */ a, /** @type {any} */ r) => a + (r.strike === null ? 0 : 1),
      0
    )
  );
  // OPENS ON THE DATA, NOT ON THE INVENTORY.
  //
  // The page used to open on the coverage view, so the first thing an operator
  // saw was a survey of what he holds rather than the values themselves -- and
  // with a nearly-empty store that survey is a screen of zeros, which reads as
  // a broken page rather than as an empty one. Bars is the table the page is
  // for; coverage is the second question, and it is one click away.
  let view = $state('bars');
  const viewNow = $derived(VIEWS.find((v) => v.key === view) ?? VIEWS[0]);
  let pageSize = $state(50);
  /**
   * EACH VIEW KEEPS ITS OWN PAGE (DB-021). One shared `page` meant Coverage
   * opened on its LAST page after a visit to page 900 of Bars. `page` below is
   * published as the CURRENT view's entry, so every reader and writer of
   * `db.page` is unchanged.
   */
  let pages = $state({ census: 1, bars: 1 });

  /* ======================================================================
     THE DAYS — held span, era, closed days (DB-044, DB-045, DB-N04)
     ====================================================================== */

  /**
   * THE DAYS THIS SELECTION HOLDS, from the CENSUS spans of the rows it
   * matches (`first_ts`/`last_ts`), never from the rows on screen (DB-045).
   * Before a contract is chosen the expiry gate holds every contract row
   * back, and the span is then the rows the rungs above leave.
   */
  const heldDays = $derived.by(() => {
    const own = matched.length > 0 ? matched : expiryGated;
    return heldSpan(own);
  });

  /**
   * The bounds a day field may take: the held span where it is known, else
   * this feed's era (GDFL's archive, Zerodha's window), else unbounded.
   */
  const dayBounds2 = $derived({
    lo: heldDays.lo || era.from,
    hi: heldDays.hi || era.to
  });

  /**
   * WHETHER THE DAY WINDOW NARROWS ANYTHING: the fields differ from the full
   * span they were seeded with. A window the reader typed (or a link carried)
   * before any span was known narrows when it is set at all.
   */
  const dayNarrowing = $derived(
    daySeed.from === '' && daySeed.to === ''
      ? Boolean(fromDay || toDay)
      : (fromDay !== '' && fromDay !== daySeed.from) || (toDay !== '' && toDay !== daySeed.to)
  );

  /** Whether a window is disjoint from what the selection holds. */
  const dayDisjoint = $derived(
    Boolean(heldDays.lo && heldDays.hi) &&
      ((Boolean(toDay) && toDay < heldDays.lo) || (Boolean(fromDay) && fromDay > heldDays.hi))
  );

  /**
   * A closed day by the calendar of record, for the day fields' grid: `true`
   * closed, `false` a session, `null` the calendar does not reach.
   * @param {string} iso
   */
  function dayClosed(iso) {
    const c = dayCode(calendar, iso);
    return c === null ? null : c === '.';
  }

  /**
   * WHAT THE DATE FIELDS SAY ABOUT THEIR BOUNDS (DB-045, DB-046, DB-G89),
   * built from the census span and the feed's era — and saying that the
   * window narrows a contract too.
   */
  const dayReason = $derived.by(() => {
    const held = heldDays.lo && heldDays.hi;
    const span = held
      ? `This selection holds ${dayLabel(heldDays.lo)} to ${dayLabel(heldDays.hi)} (the census's first and last bar); days outside it are struck through because there is no bar there to show.`
      : era.from
        ? `Nothing in this selection is counted yet, so the fields are bounded by ${era.label}, ${dayLabel(era.from)} to ${dayLabel(era.to)}.`
        : 'Nothing in this selection is counted yet, so there is no held range to bound these fields with.';
    const shut = calendar
      ? ` Days the exchange calendar of record marks closed (weekends and holidays, /calendar.json) are struck too and can still be picked.`
      : calWhy
        ? ` Closed days are not marked: ${calWhy}.`
        : '';
    const contract = contractRungs.expiry
      ? ' The window narrows a contract too: only its bars on the chosen days are shown, within its life.'
      : '';
    return `${span}${shut}${contract}`;
  });

  /**
   * WHERE A CHOSEN CONTRACT'S STORED LIFE IS CUT, AND BY WHAT (DB-G67,
   * DB-G68). The archive's first and last days are on GDFL's descriptor
   * (`archive`, D-3338); a contract first stored on the first day may have
   * been listed earlier, and one whose last stored bar comes before its
   * expiry is short of its final sessions. Each case says which cause the
   * page can name, and names none it cannot measure. `''` when nothing is
   * cut or no contract is chosen.
   */
  const lifeNote = $derived.by(() => {
    if (!expiry || matched.length === 0) return '';
    const span = heldSpan(matched);
    const a = /** @type {any} */ (activeFeed)?.archive;
    /** @type {string[]} */
    const out = [];
    if (a?.from && span.lo === a.from) {
      out.push(
        `its first stored bar is on ${dayLabel(a.from)}, the vendor archive's first day, so its life may have begun before the archive`
      );
    }
    if (span.hi && span.hi < expiry) {
      if (a?.to && expiry > a.to) {
        out.push(
          `it expires ${dayLabel(expiry)}, after the vendor archive's last day (${dayLabel(a.to)}), so its final sessions are not in the archive`
        );
      } else {
        out.push(
          `its last stored bar is on ${dayLabel(span.hi)}, before its expiry on ${dayLabel(expiry)}: the store holds no later bar for it — it may not have traded after that day, or the census is behind (the census banner names a lag)`
        );
      }
    }
    return out.length ? `This contract's stored life is cut: ${out.join('; ')}.` : '';
  });

  /* ======================================================================
     THE TIME PICKERS' SESSION GRID (DB-074)
     ====================================================================== */

  /** The rung's length in seconds, or `undefined` for a rung not on the ladder. */
  const rungSecs = $derived(TF_SECS.get(timeframe));
  /** Whether the bounds carry a second (`HH:MM:SS`) — the 1s rung. */
  const secondRung = $derived(Number.isFinite(rungSecs) && /** @type {number} */ (rungSecs) < 60);

  /**
   * The minutes a bar of this rung opens on (09:15 to 15:29 on 1min and 1s,
   * every five minutes on 5min), plus any minute outside the regular session
   * the loaded rows actually hold — never only the minutes on screen.
   */
  const sessionRows = $derived.by(() => {
    const held = new Set();
    for (const b of db.barRows ?? []) {
      const t = timeLabel(b.ts);
      if (t !== '—') held.add(t);
    }
    return sessionGrid(rungSecs, held);
  });

  /**
   * One end of the time window, set from a picked minute (and, on the 1s rung,
   * a second): `HH:MM:SS` on 1s, `HH:MM` otherwise.
   *
   * @param {'from' | 'to'} end
   * @param {string} minute `HH:MM`, or `''` to clear
   * @param {string | null} [second]
   */
  function setTimeBound(end, minute, second = null) {
    const v = minute ? timeBound(minute, rungSecs, end, second) : '';
    if (end === 'from') fromTime = v;
    else toTime = v;
  }

  /* ======================================================================
     THE STRIKES THIS SELECTION REACHES (DB-G98) — for the strike rung's note
     ====================================================================== */
  const strikesHere = $derived.by(() => {
    const seen = new Set();
    for (const r of expiryGated) if (r.strike !== null) seen.add(r.strike);
    const list = [...seen].sort((a, b) => a - b);
    let step = null;
    for (let i = 1; i < list.length; i += 1) {
      const gap = list[i] - list[i - 1];
      if (gap > 0 && (step === null || gap < step)) step = gap;
    }
    return { list, step };
  });
  /* EVERY RUNG COUNTS, INCLUDING THE TWO SILENT ONES. `filtered` drives both
     "of N stored" on the first tile and whether "Reset all filters" is offered
     at all, so a universe or a month window left out here would leave a live
     narrowing with no way back and the tile claiming the whole store. */
  /* THE EXPIRY GATE IS DELIBERATELY NOT IN HERE, AND THAT IS THE ONE
     EXCEPTION. Everything in this list is a narrowing `reset` can undo; the
     gate is not — it is in force at `expiry === ''`, which is the value reset
     restores. Listing it would make the button offer to clear something
     pressing it cannot clear, which is the exact failure the comment above
     warns about, pointed the other way. A CHOSEN expiry IS resettable and is
     in the list; the gate is reported instead, on the rung's own clause, in
     `narrowing`, in `blocked` and in the empty-table panel. */
  /**
   * WHETHER ANYTHING DIFFERS FROM WHAT "Reset all filters" RESTORES (D-3337
   * item 2): the default instrument, Spot, the finest held rung, the whole
   * held span, no time bound, no contract pick, no universe, the window
   * applied. Every rung is always SET now (one instrument, one segment, one
   * rung), so "set" cannot be the test — only "differs from the reset" is.
   * The day window compares against the span it was seeded with
   * (`dayNarrowing`), which is also what the bars grid's manifest check asks.
   */
  const filtered = $derived(
    Boolean(
      (filter && filter !== defaultPick) ||
        (kind && kind !== DEFAULT_SEGMENT) ||
        month ||
        holesOnly ||
        universe ||
        (timeframe && tfAll.length > 0 && timeframe !== tfAll[0][0]) ||
        fromMonth ||
        toMonth ||
        expiry ||
        side ||
        strikePick.size ||
        mnyPick.size ||
        fromTime ||
        toTime ||
        oow ||
        dayNarrowing
    )
  );

  /* MOVED UP FROM THE BOTTOM OF THE SCRIPT, above its first reader. `blocked`
     names the feed in a sentence, and a `$derived` closing over a `const`
     declared below it is the temporal-dead-zone reference the FORMATTING block
     at the top of this file describes: it happens to run because a derived is
     lazy, and it throws the day the evaluation order changes. Same value, same
     expression, declared before it is read. */
  const feedName = $derived(feedDisplay(feeds.active));

  /* ======================================================================
     ONE ANSWER TO "WAS ANYTHING READ, AND IF NOT, WHY NOT"
     ----------------------------------------------------------------------
     Every panel used to decide this for itself, and panels that decide
     separately disagree: the blank state said "this feed has never landed a
     bar" while no feed had been chosen at all, and the summary strip printed a
     coverage of 100.00% over the same empty selection the table below it was
     calling empty.

     `blocked` is `null` when the store was read AND the selection on screen
     holds rows. Otherwise it carries the reason, in two lengths — `tsub` fits
     under a tile, `why` is the sentence. Order matters: the FIRST condition
     that makes the rest unanswerable wins, so the message always names the
     narrowing that actually did it rather than the last one checked.
     ====================================================================== */

  /**
   * The narrowings currently in force, named once for every panel that has to
   * explain an empty result.
   *
   * `text` is the phrase; `key` is the raw store key behind it, for the
   * `title` attribute, and it is absent where there is no key. The month is
   * LABELLED in the phrase and RAW in the key — the display form never becomes
   * the thing compared, per the rule at the top of `$lib/dates.js`.
   */
  /* THE LIST READS IN CASCADE ORDER, coarsest first, because the sentence it
     builds is read as a chain: universe, then the month window, then the text,
     then the segment, then the one month, then the holes toggle. A blocker
     named out of order sends the operator to the wrong control.

     THE MONTH KEYS ARE LABELLED IN `text` AND RAW IN `key`. `key` is what the
     `title` attribute carries, so the store's own spelling is always one hover
     away and the display form never becomes the thing compared. */
  /* AND EVERY ROW CARRIES `k`, WHICH IS THE ONLY THING KEYED ON. The `{#each}`
     that renders this list used to key on `n.text` — a DISPLAY string built out
     of `monthLabel`, a universe's label and the operator's own typed text. Two
     narrowings that happen to render the same phrase would collide and Svelte
     would drop one; a phrase that changes with the label re-creates a node that
     did not change. `k` names the RUNG, is one of eight fixed strings, and can
     never be either. */
  const narrowing = $derived(
    [
      universe
        ? { k: 'universe', text: `universe ${chosenUniverse.label}`, key: chosenUniverse.token ?? '' }
        : null,
      /* THE RUNG IS ITS OWN KEY — `1min` is what the store spells and what the
         control writes, so there is no display form to carry separately. */
      timeframe ? { k: 'timeframe', text: `timeframe ${timeframe}`, key: timeframe } : null,
      fromMonth || toMonth
        ? {
            k: 'window',
            text: `months ${fromMonth ? monthLabel(fromMonth) : 'any'} → ${toMonth ? monthLabel(toMonth) : 'any'}`,
            key: `${fromMonth || '*'} … ${toMonth || '*'}`
          }
        : null,
      /* THE INSTRUMENT, BY ITS EXACT KEY (DB-001). It read "the text NIFTY",
         which was what the old prefix filter did. */
      filter ? { k: 'instrument', text: `instrument ${filter}`, key: filter } : null,
      /* "segment", matching the control's own label. The state is still called
         `kind` because that is the field `deco` parses out of the instrument
         string and renaming it would touch the sort, the facet counts and the
         drawer for no gain — only the word an operator reads changed. */
      kind
        ? { k: 'segment', text: `segment ${SEG_VIEW.find((v) => v.key === kind)?.name ?? kind}`, key: kind }
        : null,
      month ? { k: 'month', text: `month ${monthLabel(month)}`, key: month } : null,
      /* THE EXPIRY IS NAMED WHETHER IT IS A CHOICE OR A GATE, and they are two
         different sentences because they are two different facts. A chosen
         expiry is a filter like any other and reads like one. The GATE is
         reported only while it is actually holding rows back — `expiryHeld`
         is counted, so on a store with no contract in it this line is silent
         rather than announcing a refusal that refuses nothing.

         `k` is a different fixed string for each, so the two can never collide
         in the `{#each}` and one can never be dropped for looking like the
         other. The day is LABELLED in `text` and RAW in `key`, exactly as the
         month is. */
      expiry
        ? { k: 'expiry', text: `expiry ${dayLabel(expiry)}`, key: expiry }
        : expiryHeld > 0
          ? {
              k: 'expiry-gate',
              text: `every future and option, which is held back until an expiry is chosen`
            }
          : null,
      /* THE CONTRACT RUNG IS FINER THAN THE MONTH AND COARSER THAN THE HOLES
         TOGGLE, and it is named in the same shape as every other narrowing —
         the label the panel shows, and the store's own key one hover away. A
         strike is named in rupees because that is what the panel shows; the
         paisa integer behind it is the key. */
      /* EVERY STRIKE, NOT THE FIRST (DB-077). Strike is multi-select now, and
         a chip naming one of two picks hides the other. */
      strikePick.size
        ? {
            k: 'strike',
            text: `${strikePick.size === 1 ? 'strike' : 'strikes'} ${[...strikePick]
              .map(Number)
              .sort((a, b) => a - b)
              .map((p) => strikeText(p))
              .join(', ')}`,
            key: [...strikePick].map(Number).sort((a, b) => a - b).join(' · ')
          }
        : null,
      mnyPick.size ? { k: 'moneyness', text: `moneyness ${[...mnyPick].sort().join(', ')}` } : null,
      side ? { k: 'side', text: `option type ${side}`, key: side } : null,
      /* THE DAY AND TIME WINDOWS ARE NARROWINGS TOO (DB-077), and an empty
         grid under a one-week window used to be explained without them. */
      dayNarrowing
        ? {
            k: 'days',
            text: `days ${fromDay ? dayLabel(fromDay) : 'any'} → ${toDay ? dayLabel(toDay) : 'any'}`,
            key: `${fromDay || '*'} … ${toDay || '*'}`
          }
        : null,
      fromTime || toTime
        ? {
            k: 'times',
            text: `time of day ${fromTime || 'any'} → ${toTime || 'any'} IST`,
            key: `${fromTime || '*'} … ${toTime || '*'}`
          }
        : null,
      oow ? { k: 'oow', text: 'months outside your window shown' } : null,
      holesOnly ? { k: 'holes', text: 'holes only' } : null
    ].filter((n) => n !== null)
  );

  const blocked = $derived.by(() => {
    if (feeds.error)
      return {
        tsub: 'the feed list failed',
        why: `/feeds.json could not be read — ${feeds.error}. Until it answers there is no feed to read a store for, so nothing on this page has been counted.`
      };
    if (!feeds.active)
      return {
        tsub: 'no feed chosen',
        why: 'A bar belongs to the vendor that supplied it, so with no feed selected there is no store to read. Nothing below has been counted — this is an unmade choice, not an empty store.'
      };
    if (db.error)
      return {
        tsub: 'the manifest failed',
        why: `/store.json could not be read — ${db.error}. Nothing was counted, including the zeroes.`
      };
    if (db.loading && db.rows.length === 0)
      return { tsub: 'not read yet', why: 'The store manifest is still being read.' };
    if (db.rows.length === 0)
      return {
        tsub: 'this feed holds nothing',
        why: `${feedName} has never landed a bar in this store, so there is nothing to count.`
      };
    /* THE UNIVERSE GETS ITS OWN BRANCH, ABOVE THE GENERIC ONE, because "none of
       them match universe NIFTY Total Market" is true and useless: the number
       an operator needs is how many of that universe's members the store holds
       AT ALL, and that is a different count from the one the table is showing.
       First blocker wins, so this only fires when the universe alone emptied
       the set — a finer control that empties it is named by the branch below. */
    if (universe && universed.length === 0)
      return {
        tsub: 'nothing stored in this universe',
        /* THE SAME CORRECTION AS `tfRefusal`, AND FOR THE SAME REASON. With
           Everything selected this said "none of them is in Everything", which
           is a contradiction, and pointed at the universe row — the one rung
           that cannot be the cause when it is already at its widest. */
        why:
          universe === ''
            ? `${feedName} holds ${fmt(db.deco.length)} instrument-month(s) and none of them survives the rungs below the universe — which is already at Everything, so it is not the universe narrowing this. Not stored is not the same as not offered: this page reads the store, and what a vendor could have served is not a question /store.json can answer.`
            : `${feedName} holds ${fmt(db.deco.length)} instrument-month(s) and none of them is in ${chosenUniverse.label}. ${
                universeCount.get(universe)
                  ? `${fmt(universeCount.get(universe).held)} of ${fmt(universeCount.get(universe).members)} member(s) are held.`
                  : 'Membership was not counted — see the note beside the universe row.'
              } Not stored is not the same as not offered: this page reads the store, and what a vendor could have served is not a question /store.json can answer.`
      };
    /* THE GATE GETS ITS OWN BRANCH, ABOVE THE GENERIC ONE, for the same
       reason the universe does: "none of them match the current selection" is
       true here and useless. Nothing was filtered out by a choice — the rows
       are being HELD BACK for want of one, the way out is a control the
       operator has not touched yet, and "Clear the filters" is the one thing
       that will NOT open it. First blocker wins, so this fires only when the
       gate alone emptied the set; a finer control that empties it afterwards
       is named by the branch below. */
    if (matched.length === 0 && !expiry && expiryHeld > 0)
      return {
        tsub: 'no expiry chosen',
        why: `Every one of the ${fmt(expiryHeld)} instrument-month(s) this selection reaches is a future or an option, and a contract row is not shown until an expiry is chosen for it. This is a refusal, not an empty store: a bar belongs to ONE contract, so blending ${fmt(expiriesAll.length)} expiries into a single table would show a series no contract ever traded. Choose an expiry on the Contract strip and they appear. Clearing the filters will not release them — the gate is not a filter.`
      };
    if (matched.length === 0)
      return {
        tsub: 'nothing matches the filter',
        // `narrowing` cannot be empty here — with no filter in force `matched`
        // IS `deco`, and `rows.length === 0` returned two branches up — but the
        // fallback is written anyway, because the one sentence that must never
        // come out malformed is the one explaining why a screen is blank.
        why: `The store was read and holds ${fmt(db.deco.length)} instrument-month(s); none of them match ${narrowing.length ? narrowing.map((n) => n.text).join(', ') : 'the current selection'}. The store is not empty — this selection is.`
      };
    return null;
  });

  const stranded = $derived.by(() => {
    const out = [];
    /* AN INSTRUMENT THE RUNGS ABOVE NO LONGER OFFER — the universe moved,
       or a link named one this feed's store does not list. Clearing re-seeds
       the opening instrument. */
    if (filter && db.deco.length > 0 && !instrumentOffered.some((/** @type {any} */ r) => r.key === filter)) {
      out.push({ rung: 'Instrument', value: filter, clear: () => (filter = '') });
    }
    /* `SEG_VIEW`, NOT `kinds`: one vocabulary. Only a value from neither
       vocabulary can strand this rung now. */
    if (kind && !SEG_VIEW.some((s) => s.key === kind)) {
      out.push({
        rung: 'Segment',
        value: kind,
        clear: () => (kind = SEG_VIEW[0].key)
      });
    }
    if (timeframe && !tfAll.some(([t]) => t === timeframe)) {
      /* RE-SEEDS RATHER THAN CLEARING TO NOTHING. `''` used to mean "all rungs"
         and there is no such view any more, so clearing would leave the page
         with no rung at all. It moves to the first rung the store holds — the
         same value the seeding effect would choose. */
      out.push({
        rung: 'Timeframe',
        value: timeframe,
        clear: () => (timeframe = tfAll[0]?.[0] ?? '')
      });
    }
    if (expiry && !expiryOffered.includes(expiry)) {
      out.push({ rung: 'Expiry', value: expiry, clear: () => (expiry = '') });
    }
    if (side && !sideOffered.includes(side)) {
      out.push({ rung: 'Side', value: side, clear: () => (side = '') });
    }
    /* THE STRIKE IS NAMED TOO (DB-020). A strike left over from another
       expiry or segment emptied the grid with no banner and no Clear. */
    if (strikePick.size > 0) {
      const here = new Set(strikesHere.list.map(String));
      const gone = [...strikePick].filter((k) => !here.has(k));
      if (gone.length > 0) {
        out.push({
          rung: gone.length === 1 ? 'Strike' : 'Strikes',
          value: gone.map((k) => strikeText(Number(k))).join(', '),
          clear: () => (strikePick = new Set([...strikePick].filter((k) => here.has(k))))
        });
      }
    }
    if (mnyPick.size > 0 && !contractRungs.money) {
      out.push({ rung: 'Moneyness', value: [...mnyPick].join(', '), clear: () => (mnyPick = new Set()) });
    }
    /* A DAY WINDOW THAT CANNOT MEET THE SELECTION (row 132 of the 9 Oct
       drill-down: an Oct 2024 window carried onto a 2019 contract). */
    if (dayDisjoint) {
      out.push({
        rung: 'Days',
        value: `${fromDay ? dayLabel(fromDay) : 'any'} → ${toDay ? dayLabel(toDay) : 'any'}`,
        clear: () => {
          fromDay = heldDays.lo;
          toDay = heldDays.hi;
          daySeed = { from: heldDays.lo, to: heldDays.hi };
        }
      });
    }
    return out;
  });

  /* EVERY RUNG, OR THE BUTTON LIES. "Reset all filters" that leaves a universe
     or a month window standing puts the operator back on a screen that is
     still narrowed, with the control that says so now reading "unfiltered".
     `filtered` above and this function have to name the same seven pieces of
     state; they are written next to each other so a new rung cannot be added
     to one and forgotten in the other. */
  function reset() {
    filter = '';
    month = '';
    kind = '';
    holesOnly = false;
    universe = '';
    timeframe = '';
    fromMonth = '';
    toMonth = '';
    /* THE DAY WINDOW, WHICH THIS BUTTON USED TO LEAVE BEHIND. The comment
       above says "EVERY RUNG, OR THE BUTTON LIES", and these two were the only
       live, operator-settable rung it skipped: `month`, `holesOnly`,
       `fromMonth` and `toMonth` are all documented dead, while `fromDay` and
       `toDay` genuinely narrow `barWindowed`. Narrow the window to a week,
       press Reset all filters, and every other rung returned to its opening
       value while the week silently stayed.
       EMPTY AND NOT THE SPAN: the seeding effect's guard is exactly
       `fromDay === '' && toDay === ''`, so clearing them re-arms it and the
       store's own floor and ceiling come back on the next reading. Writing the
       span here instead would duplicate that logic and drift from it. */
    fromDay = '';
    toDay = '';
    /* THE TIME HALF OF THE SAME WINDOW, AND THE LANDING WITH IT. `fromTime`
       and `toTime` narrow `barWindowed` exactly as the day pair does, so a
       Reset that left them standing would be the identical lie the block above
       was written about. `goTime` is not a filter, but the mark it left and
       the sentence under it both describe a page this button is moving away
       from, so they go too. */
    fromTime = '';
    toTime = '';
    goTime = '';
    landedRk = '';
    db.jumpWhy = '';
    /* THE OPERATOR'S DEFAULTS, NOT THE ALPHABET'S (D-3337 item 2). The empty
       rungs re-seed: NIFTY where this feed holds it, Spot, the finest held
       rung, the whole held span — and newest first on page one. Reset used to
       land on 360ONE and keep a Date-ascending sort (drill-down row 39). */
    query = '';
    oow = false;
    daySeed = { from: '', to: '' };
    pages = { census: 1, bars: 1 };
    if ('barSortKey' in db) {
      db.barSortKey = 'ts';
      db.barDesc = true;
    }
    clearContract();
  }

  /* THE CONTRACT RUNG'S OWN CLEAR, called by `reset` and by the link in the
     rung. A `new Set()` and never `.clear()`: see the block on `strikePick`.

     IT PUTS `expiry` BACK TO `''`, WHICH CLOSES THE GATE RATHER THAN OPENING
     IT, and that is correct and is the reason the gate is reported separately
     everywhere else. Reset means "every control back to its opening value";
     the expiry's opening value is "none chosen", and none chosen means the
     contract rows are held. A reset that instead left the last expiry standing
     would be a control claiming to be untouched while still narrowing. */
  function clearContract() {
    expiry = '';
    side = '';
    strikePick = new Set();
    mnyPick = new Set();
  }

  Object.defineProperties(db, {
    filter: { get: () => filter, set: (v) => (filter = v) },
    month: { get: () => month, set: (v) => (month = v) },
    kind: { get: () => kind, set: (v) => (kind = v) },
    holesOnly: { get: () => holesOnly, set: (v) => (holesOnly = v) },
    universe: { get: () => universe, set: (v) => (universe = v) },
    fromMonth: { get: () => fromMonth, set: (v) => (fromMonth = v) },
    toMonth: { get: () => toMonth, set: (v) => (toMonth = v) },
    fromDay: { get: () => fromDay, set: (v) => (fromDay = v) },
    toDay: { get: () => toDay, set: (v) => (toDay = v) },
    fromTime: { get: () => fromTime, set: (v) => (fromTime = v) },
    toTime: { get: () => toTime, set: (v) => (toTime = v) },
    goTime: { get: () => goTime, set: (v) => (goTime = v) },
    landedRk: { get: () => landedRk, set: (v) => (landedRk = v) },
    timeframe: { get: () => timeframe, set: (v) => (timeframe = v) },
    drop: { get: () => drop, set: (v) => (drop = v) },
    strikePick: { get: () => strikePick, set: (v) => (strikePick = v) },
    mnyPick: { get: () => mnyPick, set: (v) => (mnyPick = v) },
    expiry: { get: () => expiry, set: (v) => (expiry = v) },
    side: { get: () => side, set: (v) => (side = v) },
    feedDisplay: { get: () => feedDisplay },
    catalogueIsThisFeed: { get: () => catalogueIsThisFeed },
    masterRows: { get: () => masterRows },
    chosenUniverse: { get: () => chosenUniverse },
    tokensHere: { get: () => tokensHere },
    tierState: { get: () => tierState },
    carriesArray: { get: () => carriesArray },
    tierRefusal: { get: () => tierRefusal },
    membershipWhy: { get: () => membershipWhy },
    universeRefusal: { get: () => universeRefusal },
    universeNames: { get: () => universeNames },
    universed: { get: () => universed },
    universeCount: { get: () => universeCount },
    outsideMaster: { get: () => outsideMaster },
    typed: { get: () => typed },
    instrumentRows: { get: () => instrumentRows },
    instrumentKeys: { get: () => instrumentKeys },
    picked: { get: () => picked },
    kinds: { get: () => kinds },
    instrumentOffered: { get: () => instrumentOffered },
    contractRungs: { get: () => contractRungs },
    segOf: { get: () => segOf },
    segmentRows: { get: () => segmentRows },
    segmented: { get: () => segmented },
    tfAll: { get: () => tfAll },
    timeframed: { get: () => timeframed },
    tfRefusal: { get: () => tfRefusal },
    monthsAll: { get: () => monthsAll },
    onWindowDown: { get: () => onWindowDown },
    windowed: { get: () => windowed },
    scoped: { get: () => scoped },
    holed: { get: () => holed },
    matchedPreContract: { get: () => matchedPreContract },
    expiriesAll: { get: () => expiriesAll },
    expiryCount: { get: () => expiryCount },
    expiryOffered: { get: () => expiryOffered },
    contractHere: { get: () => contractHere },
    expiryGated: { get: () => expiryGated },
    expiryHeld: { get: () => expiryHeld },
    expiryRefusal: { get: () => expiryRefusal },
    sidesAll: { get: () => sidesAll },
    sideOffered: { get: () => sideOffered },
    mnyFilteredCount: { get: () => mnyFilteredCount },
    sideCount: { get: () => sideCount },
    sidedHere: { get: () => sidedHere },
    sideRefusal: { get: () => sideRefusal },
    sideEmpty: { get: () => sideEmpty },
    matched: { get: () => matched },
    contractRows: { get: () => contractRows },
    view: { get: () => view, set: (v) => (view = v) },
    viewNow: { get: () => viewNow },
    pageSize: { get: () => pageSize, set: (v) => (pageSize = v) },
    /* THE CURRENT VIEW'S PAGE (DB-021): every reader and writer of `db.page`
       is unchanged, and each view keeps its own. */
    page: { get: () => pages[/** @type {'census' | 'bars'} */ (view)] ?? 1, set: (v) => (pages[/** @type {'census' | 'bars'} */ (view)] = v) },
    pages: { get: () => pages, set: (v) => (pages = v) },
    instrument: { get: () => filter, set: (v) => (filter = v) },
    query: { get: () => query, set: (v) => (query = v) },
    oow: { get: () => oow, set: (v) => (oow = v) },
    oowHidden: { get: () => oowHidden },
    oowMonths: { get: () => oowMonths },
    oowHere: { get: () => oowHere },
    activeFeed: { get: () => activeFeed },
    archiveTo: { get: () => archiveTo },
    era: { get: () => era },
    today: { get: () => today },
    noMaster: { get: () => noMaster },
    instrumentIndex: { get: () => instrumentIndex },
    instrumentMatched: { get: () => instrumentMatched },
    defaultPick: { get: () => defaultPick },
    feedSegments: { get: () => feedSegments },
    expiryTagMap: { get: () => expiryTagMap },
    sideBase: { get: () => sideBase },
    heldDays: { get: () => heldDays },
    dayBounds2: { get: () => dayBounds2 },
    daySeed: { get: () => daySeed, set: (v) => (daySeed = v) },
    dayNarrowing: { get: () => dayNarrowing },
    dayDisjoint: { get: () => dayDisjoint },
    dayClosed: { get: () => dayClosed },
    dayReason: { get: () => dayReason },
    calendar: { get: () => calendar, set: (v) => (calendar = v) },
    calWhy: { get: () => calWhy, set: (v) => (calWhy = v) },
    calFor: { get: () => calFor, set: (v) => (calFor = v) },
    rungSecs: { get: () => rungSecs },
    secondRung: { get: () => secondRung },
    sessionRows: { get: () => sessionRows },
    setTimeBound: { get: () => setTimeBound },
    strikesHere: { get: () => strikesHere },
    lifeNote: { get: () => lifeNote },
    urlWhy: { get: () => urlWhy, set: (v) => (urlWhy = v) },
    linkNotes: { get: () => linkNotes, set: (v) => (linkNotes = v) },
    filtered: { get: () => filtered },
    feedName: { get: () => feedName },
    narrowing: { get: () => narrowing },
    blocked: { get: () => blocked },
    stranded: { get: () => stranded },
    reset: { get: () => reset },
    clearContract: { get: () => clearContract }
  });
}


/**
 * THE SEEDS — the effects that put a value on an empty rung once the store
 * has answered (the instrument, the segment, the timeframe, the day window),
 * the exchange calendar the date fields and the expiry tags read, and the
 * reconciliation a change of feed, instrument or segment triggers.
 *
 * The first three seed effects keep the shape `if (db.<rung> === '' && …)`
 * and read only `db`: `web/tests/database-page-fixture.js` cuts each one out
 * of this file and runs it over plain inputs.
 *
 * @param {any} db the page's shared state object (see the shell)
 */
export function selectionSeeds(db) {
  /** A feed change the link reader made itself, which is not pruned. */
  let quietFeed = /** @type {string | null} */ (null);
  /** A sort and a page a link carried, applied once the selection settles. */
  /** @type {{sort: string, desc: boolean} | null} */
  let linkSort = null;
  /** @type {{view: string, page: number} | null} */
  let linkPage = null;
  /* ======================================================================
     THE SELECTION FROM THE ADDRESS BAR (DB-077; D-3337 item 4)
     ----------------------------------------------------------------------
     Read ONCE, when the feed list has answered (a link's `feed` must be one
     the server offers). Every rung the link names is set before the census
     arrives, so the seeds fill only what it left empty. A value the page
     cannot honour is NAMED in `linkNotes` — never dropped in silence — and a
     rung the store turns out not to hold is named by `stranded` with its
     Clear button, exactly as a rung the reader stranded would be.
     ====================================================================== */
  let linkRead = false;
  $effect(() => {
    if (linkRead || feeds.all.length === 0) return;
    linkRead = true;
    untrack(() => {
      const { sel, refused } = decodeDb(page.url.search);
      /** @type {string[]} */
      const notes = refused.map((r) => `${r.key}=${r.value} was not read: ${r.why}`);
      if (sel.feed) {
        const f = feeds.all.find((x) => x.wire === sel.feed);
        if (!f) notes.push(`feed=${sel.feed} is not a feed this server lists`);
        else if (/** @type {any} */ (f).offered === false) notes.push(`feed=${sel.feed} is excluded by the operator (D-3338) and is not offered`);
        else if (feeds.active !== sel.feed) {
          quietFeed = sel.feed;
          selectFeed(sel.feed);
        }
      }
      const link = sel.key ? linkOfKey(sel.key) : null;
      if (sel.key && link === null) notes.push(`key=${sel.key} is not an instrument key (EXCHANGE-SEGMENT-UNDERLYING[-contract])`);
      const sym = sel.sym ?? link?.sym;
      if (sym) db.filter = sym;
      const exp = sel.exp ?? link?.expiry;
      const strikes = sel.k ?? (link?.strike ? [link.strike] : []);
      const side = sel.side ?? link?.side;
      if (sel.seg) db.kind = sel.seg;
      else if (!link && exp) {
        /* A contract named without a segment is read as one: an option when
           a strike or side is named, else a future, filed by its expiry. */
        const f = feeds.all.find((x) => x.wire === (sel.feed ?? feeds.active));
        db.kind = segmentOf(
          { expiry: exp, instrument: strikes.length || side ? 'contract-CE' : 'contract-FUT' },
          db.today,
          /** @type {any} */ (f)?.archive?.to ?? null
        );
      } else if (link) {
        const f = feeds.all.find((x) => x.wire === (sel.feed ?? feeds.active));
        db.kind =
          link.contract === 'spot'
            ? 'spot'
            : segmentOf(
                { expiry: link.expiry, instrument: /** @type {string} */ (sel.key) },
                db.today,
                /** @type {any} */ (f)?.archive?.to ?? null
              );
      }
      if (sel.tf) db.timeframe = sel.tf;
      if (exp) db.expiry = exp;
      if (strikes.length) db.strikePick = new Set(strikes);
      if (side) db.side = side;
      if (sel.from || sel.to) {
        db.fromDay = sel.from ?? '';
        db.toDay = sel.to ?? '';
        /* A LINKED WINDOW IS THE READER'S, not a seed: it stays when the span
           arrives, and `dayNarrowing` reports it. */
        db.daySeed = { from: '\u0000', to: '\u0000' };
      }
      if (sel.tfrom) db.fromTime = sel.tfrom;
      if (sel.tto) db.toTime = sel.tto;
      if (sel.oow) db.oow = true;
      if (sel.view) db.view = sel.view;
      if (sel.sort || sel.dir) linkSort = { sort: sel.sort ?? 'ts', desc: (sel.dir ?? 'desc') === 'desc' };
      if (sel.page && sel.page > 1) linkPage = { view: sel.view ?? 'bars', page: sel.page };
      db.linkNotes = notes;
    });
  });

  /**
   * EXACTLY ONE SEGMENT, SEEDED — SPOT FIRST (D-3337 item 2).
   *
   * The operator's opening segment is Spot. It is chosen when the instrument
   * holds spot rows; an instrument that holds none (an option-only
   * underlying such as ACC on GDFL) opens on the first segment it does hold,
   * so a pick never lands on an empty table over a store that has its rows.
   * Waits for the decorated census and for the instrument, because a segment
   * held by another instrument is not evidence the selected one has it.
   */
  $effect(() => {
    if (db.kind === '' && db.deco.length > 0 && db.filter !== '' && db.segmentRows.length > 0) {
      const held = db.segmentRows.filter((/** @type {any} */ s) => s.why === undefined);
      db.kind = (held.find((/** @type {any} */ s) => s.key === 'spot') ?? held[0] ?? db.segmentRows[0]).key;
    }
  });

  /**
   * EXACTLY ONE INSTRUMENT, SEEDED — NIFTY FIRST (D-3337 item 2, operator
   * answer 11). `'NIFTY'` here is `DEFAULT_INSTRUMENT` (selection-model.js),
   * written out because this body runs in the seam fixture with `db` alone.
   * Where this feed's store holds no NIFTY, the first offered name with rows
   * is chosen, as before; the alphabet (360ONE) was never the operator's
   * choice.
   */
  $effect(() => {
    if (db.filter === '' && db.deco.length > 0 && db.instrumentOffered.length > 0) {
      const held =
        db.instrumentOffered.find((/** @type {any} */ r) => r.key === 'NIFTY' && r.months > 0) ??
        db.instrumentOffered.find((/** @type {any} */ r) => r.months > 0);
      db.filter = (held ?? db.instrumentOffered[0]).key;
    }
  });

  /**
   * EXACTLY ONE RUNG, ALWAYS, SEEDED FROM THE STORE — THE FINEST HELD.
   *
   * `tfAll` is sorted by bar LENGTH (`tfCmp`), so `tfAll[0]` is the finest
   * rung the selection holds: 1s on GDFL, 1min on Zerodha. A rung the
   * operator picked is never overridden here; a feed change that leaves it
   * unheld clears it (`reconcile` below) and this re-seeds it.
   */
  $effect(() => {
    if (db.timeframe === '' && db.filter !== '' && db.kind !== '' && db.tfAll.length > 0) db.timeframe = db.tfAll[0][0];
  });

  /**
   * THE DAY WINDOW OPENS ON THE SPAN THE CENSUS SAYS THIS SELECTION HOLDS
   * (DB-044, DB-045), and follows it while the reader has not narrowed it.
   *
   * It opened on 01 Jan 2015 (a hard-coded floor) and today, although GDFL
   * starts 2018-09-03 and GDFL 360ONE starts 2023-01. Now it is the first and
   * last bar of the rows the selection matches. When the selection changes —
   * another instrument, an expiry — a window still equal to the old span (or
   * empty) moves to the new one; a window the reader narrowed is his and
   * stays, and `stranded` speaks up if it no longer meets the selection.
   */
  $effect(() => {
    const lo = db.heldDays.lo;
    const hi = db.heldDays.hi;
    /* The fields are read too, so a Reset (or a Clear of both) re-seeds even
       when the selection's span did not move. */
    const from = db.fromDay;
    const to = db.toDay;
    const seed = db.daySeed;
    if (!lo || !hi) return;
    untrack(() => {
      const blank = from === '' && to === '';
      if (!blank && seed.from === lo && seed.to === hi) return;
      const whole = blank || (from === seed.from && to === seed.to);
      db.daySeed = { from: lo, to: hi };
      if (whole) {
        db.fromDay = lo;
        db.toDay = hi;
      }
    });
  });

  /**
   * THE CALENDAR OF RECORD FOR THIS FEED AND RUNG (D-3336), read once per
   * (feed, rung) through one ticketed request: the closed days the date
   * fields strike (DB-N04) and the holidays an expiry shift is measured
   * against (DB-076). A refusal or failure is named in `calWhy`; the fields
   * then strike only out-of-span days and say why closed days are not marked.
   */
  const calReads = createPageRequests();
  $effect(() => {
    const feed = feeds.active;
    const rung = db.timeframe;
    if (!feed || !rung) return;
    const want = `${feed}|${rung}`;
    if (untrack(() => db.calFor) === want) return;
    db.calFor = want;
    db.calendar = null;
    db.calWhy = '';
    calReads.cancel();
    void calReads.run(async (ticket) => {
      try {
        // A CALENDAR STILL BUILDING (202, DB-N19) IS ASKED AGAIN at the
        // server's hint, never read as a calendar with no day string.
        const { response: r, body } = await askCalendar(
          `/calendar.json?feed=${encodeURIComponent(feed)}&rung=${encodeURIComponent(rung)}`,
          ask,
          ticket.signal
        );
        if (!ticket.current()) return;
        const cal = calendarOf(body);
        const reason = typeof body?.reason === 'string' && body.reason ? ` (${body.reason})` : '';
        db.calendar = cal;
        db.calWhy = cal
          ? ''
          : body?.status === 'building'
            ? `/calendar.json is still being built${reason}`
            : r.ok
              ? `/calendar.json answered with no day string${reason}`
              : `/calendar.json answered HTTP ${r.status}`;
      } catch (why) {
        if (ticket.current()) db.calWhy = `/calendar.json could not be read: ${String(/** @type {any} */ (why)?.message ?? why)}`;
      }
    });
  });
  $effect(() => () => calReads.dispose());

  /* ======================================================================
     WHAT A CHANGE DROPS (DB-019, DB-020; D-3337 item 3)
     ----------------------------------------------------------------------
     A FEED change re-seeds every rung the new census lacks: an instrument it
     does not hold, a segment the instrument does not hold, a rung it does
     not store (GDFL stores only 1s, Zerodha no 1s — every switch used to land
     on an empty grid under "Timeframe is set to 1s …"), and every contract
     pick it does not offer; a window wholly outside its span returns to it.
     An INSTRUMENT change drops the contract picks the new underlying does
     not offer (BANKNIFTY kept NIFTY's 02 Jan 2020 11,500 PE). A SEGMENT
     change drops the picks the segment hides — Spot has no expiry, strike or
     side, so leftovers emptied the grid with no control left to clear them.
     Pages return to one on every change (`selectionPaging`).

     Each change is noted as PENDING and applied by one effect once the
     census and the coarser rungs it depends on have settled, in cascade
     order: instrument, then segment, then the rest. Anything else that moves
     a rung (a link, a reset) is not an operator change and is not pruned.
     ====================================================================== */
  /** @type {{feed: boolean, instrument: boolean, segment: boolean} | null} */
  let pending = $state(null);
  let lastFeed = /** @type {string | null} */ (null);
  let lastInstrument = '';
  let lastKind = '';

  $effect(() => {
    const f = feeds.active;
    const ins = db.filter;
    const k = db.kind;
    untrack(() => {
      const feedMoved = lastFeed !== null && f !== null && f !== lastFeed && f !== quietFeed;
      const insMoved = lastInstrument !== '' && ins !== '' && ins !== lastInstrument;
      const segMoved = lastKind !== '' && k !== '' && k !== lastKind;
      if (feedMoved || insMoved || segMoved) {
        pending = {
          feed: Boolean(pending?.feed) || feedMoved,
          instrument: Boolean(pending?.instrument) || insMoved,
          segment: Boolean(pending?.segment) || segMoved
        };
      }
      /* A SEGMENT'S HIDDEN RUNGS GO AT ONCE: Spot has no expiry, strike,
         moneyness or side, and Futures no strike, moneyness or side. */
      if (segMoved) {
        const rungs = contractRungsOf(k);
        if (!rungs.expiry) db.expiry = '';
        if (!rungs.strike && db.strikePick.size) db.strikePick = new Set();
        if (!rungs.money && db.mnyPick.size) db.mnyPick = new Set();
        if (!rungs.side) db.side = '';
      }
      if (f !== null) lastFeed = f;
      if (ins !== '') lastInstrument = ins;
      if (k !== '') lastKind = k;
      if (f === quietFeed) quietFeed = null;
    });
  });

  $effect(() => {
    const p = pending;
    if (p === null || db.deco.length === 0) return;
    /* Read the rungs this pass depends on, so it re-runs as the seeds settle. */
    const ins = db.filter;
    const k = db.kind;
    const tf = db.timeframe;
    const rows = db.segmentRows;
    const tfs = db.tfAll;
    untrack(() => {
      if (p.feed) {
        if (ins !== '' && !db.instrumentIndex.has(ins)) {
          db.filter = '';
          return;
        }
        if (ins === '') return;
      }
      if (p.feed || p.instrument) {
        const here = rows.find((/** @type {any} */ s) => s.key === k);
        if (k !== '' && (here === undefined || here.why !== undefined)) {
          const held = rows.some((/** @type {any} */ s) => s.why === undefined);
          if (held) {
            db.kind = '';
            return;
          }
        }
        if (k === '') return;
      }
      if (p.feed) {
        if (tf !== '' && !tfs.some((/** @type {any} */ [t]) => t === tf)) {
          db.timeframe = '';
          return;
        }
        if (tf === '') return;
        /* A BOUND CARRIED ACROSS RUNGS IS RE-EXPRESSED AT THE NEW RESOLUTION:
           a 1s `HH:MM:SS` bound on a minute rung would compare a `HH:MM`
           label against seconds and drop the minute it names. */
        const secs = TF_SECS.get(tf);
        db.fromTime = rebound(db.fromTime, secs, 'from');
        db.toTime = rebound(db.toTime, secs, 'to');
        /* The window toggle is a feed's own (Zerodha's operator window); a
           feed without one has nothing to show outside it. */
        if (db.oow && !db.activeFeed?.window) db.oow = false;
        if (db.dayDisjoint) {
          db.fromDay = db.heldDays.lo;
          db.toDay = db.heldDays.hi;
          db.daySeed = { from: db.heldDays.lo, to: db.heldDays.hi };
        }
      }
      /* THE NEW SELECTION'S OFFER AT THE KEPT EXPIRY: `expiryGated` is the
         rows the rungs above leave held to the current expiry, so its strikes
         and sides are what that expiry offers under the new underlying. */
      const sides = new Set();
      for (const r of db.expiryGated) if (r.side !== null) sides.add(r.side);
      const kept = keepOffered(
        { expiry: db.expiry, strikes: db.strikePick, side: db.side, money: db.mnyPick },
        {
          rungs: db.contractRungs,
          expiries: db.expiryOffered,
          strikes: db.strikesHere.list.map(String),
          sides
        }
      );
      if (kept.expiry !== db.expiry) db.expiry = kept.expiry;
      if (kept.strikes.size !== db.strikePick.size) db.strikePick = kept.strikes;
      if (kept.side !== db.side) db.side = kept.side;
      if (kept.money.size !== db.mnyPick.size) db.mnyPick = kept.money;
      pending = null;
    });
  });

  /** Hand over the link's sort once, and forget it. */
  function takeLinkSort() {
    const s = linkSort;
    linkSort = null;
    return s;
  }
  /** Hand over the link's page once, and forget it. */
  function takeLinkPage() {
    const p = linkPage;
    linkPage = null;
    return p;
  }
  Object.defineProperties(db, {
    linkPending: { get: () => ({ sort: linkSort, page: linkPage }) },
    takeLinkSort: { get: () => takeLinkSort },
    takeLinkPage: { get: () => takeLinkPage },
    reconciling: { get: () => pending !== null }
  });
}

/**
 * A NARROWING RETURNS THE READER TO PAGE ONE, AND THE ADDRESS BAR FOLLOWS THE
 * SELECTION — the effect that watches every rung that changes which rows
 * exist, the per-view sort resets, a linked page, and the URL mirror.
 *
 * @param {any} db the page's shared state object (see the shell)
 */
export function selectionPaging(db) {
  /* EVERY RUNG THAT CHANGES WHICH ROWS EXIST (DB-021). The timeframe, the
     expiry, the side, the day and time windows, the feed and the window
     toggle were missing, so a page number survived them and landed mid-list.
     Each view keeps its own page; this returns BOTH to one, because both
     views read the same selection. */
  $effect(() => {
    void [
      feeds.active,
      db.filter,
      db.kind,
      db.timeframe,
      db.universe,
      db.month,
      db.holesOnly,
      db.fromMonth,
      db.toMonth,
      db.expiry,
      db.side,
      db.strikePick,
      db.mnyPick,
      db.fromDay,
      db.toDay,
      db.fromTime,
      db.toTime,
      db.oow,
      /* AND THE READ BUDGET, because it changes which FILES were opened and
         therefore which bars exist at all. */
      db.readBudget
    ];
    untrack(() => {
      db.restartAtTop();
      db.pages = { census: 1, bars: 1 };
    });
  });

  /* A SORT RETURNS ITS OWN VIEW TO PAGE ONE: page 12 of a re-sorted run is a
     different twelve rows. The other view's page is not touched. */
  $effect(() => {
    void [db.sortKey, db.desc];
    untrack(() => {
      /* A LINK'S OWN SORT MUST NOT ERASE THE LINK'S OWN PAGE. */
      if (db.linkPending.page) return;
      db.restartAtTop();
      db.pages.census = 1;
    });
  });
  $effect(() => {
    void [db.barSortKey, db.barDesc];
    untrack(() => {
      if (db.linkPending.page) return;
      db.restartAtTop();
      db.pages.bars = 1;
    });
  });

  /** Whether every rung the page seeds has settled on the census. */
  const settled = $derived(
    db.deco.length > 0 &&
      db.filter !== '' &&
      db.kind !== '' &&
      db.timeframe !== '' &&
      !db.reconciling &&
      (db.daySeed.from !== '' || db.matched.length === 0)
  );

  /* A LINKED SORT AND PAGE ARE APPLIED ONCE THE SELECTION HAS SETTLED, after
     the resets above have run for the seeds, so the link's page is the page
     shown rather than the first thing the seeds reset. */
  $effect(() => {
    if (!settled) return;
    untrack(() => {
      const s = db.takeLinkSort();
      if (s) {
        if (db.view === 'census') {
          db.sortKey = s.sort;
          db.desc = s.desc;
        } else {
          db.barSortKey = s.sort;
          db.barDesc = s.desc;
        }
      }
    });
  });
  $effect(() => {
    if (!settled) return;
    void [db.barSortKey, db.barDesc, db.sortKey, db.desc];
    untrack(() => {
      const p = db.takeLinkPage();
      if (p) db.pages[p.view === 'census' ? 'census' : 'bars'] = p.page;
    });
  });

  /**
   * THE ADDRESS BAR, MIRRORED (DB-077). Written with SvelteKit's own
   * `replaceState` once the selection has settled, and only when the
   * encoded selection differs from the address — idempotent, so one
   * selection is one URL and a reload, a bookmark or a shared link restores
   * it. A failure is named in `urlWhy`, never swallowed.
   */
  const sortOpening = $derived(
    db.view === 'census' ? db.sortKey === 'instrument' && !db.desc : db.barSortKey === 'ts' && db.barDesc
  );
  const encoded = $derived(
    encodeDb({
      feed: feeds.active ?? '',
      seg: db.kind,
      sym: db.filter,
      tf: db.timeframe,
      exp: db.expiry,
      k: db.strikePick,
      side: db.side,
      from: db.dayNarrowing ? db.fromDay : '',
      to: db.dayNarrowing ? db.toDay : '',
      tfrom: db.fromTime,
      tto: db.toTime,
      /* A VIEW'S OWN OPENING SORT IS NOT WRITTEN: Bars opens newest first,
         Coverage by instrument ascending. */
      sort: sortOpening ? '' : db.view === 'census' ? db.sortKey : db.barSortKey,
      dir: sortOpening ? undefined : (db.view === 'census' ? db.desc : db.barDesc) ? 'desc' : 'asc',
      page: db.page,
      view: db.view,
      oow: db.oow
    })
  );
  $effect(() => {
    const q = encoded;
    if (!settled || typeof location === 'undefined') return;
    untrack(() => {
      const now = location.search.replace(/^\?/, '');
      if (now === q) return;
      try {
        replaceState(q ? `?${q}` : location.pathname, {});
        db.urlWhy = '';
      } catch (why) {
        db.urlWhy = `the address bar could not be updated: ${String(/** @type {any} */ (why)?.message ?? why)}`;
      }
    });
  });
  Object.defineProperties(db, {
    settled: { get: () => settled },
    encoded: { get: () => encoded }
  });
}

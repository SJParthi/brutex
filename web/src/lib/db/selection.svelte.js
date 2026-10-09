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
import { feeds } from '../feeds.svelte.js';
/* THE FEED'S OWN MASTER, ALREADY ON HAND. `+layout.svelte` calls
   `loadCatalogue(feeds.active)` on every feed change, so the universe rung
   costs this page NO request — it is a join against a list the layout has
   already paid for. It also arrives stamped with the feed it answers for,
   which is the only thing that makes a count under this feed's name
   checkable. See `universeRefusal`. */
import { catalogue } from '../index.svelte.js';
import { UNIVERSES, EVERYTHING, tokensOf, LEGACY_BLIND, censusKeyOf, SEG_VIEW, VIEWS } from './notes-controls.js';
import { fmt, txt, tfCmp } from './notes-labels.js';
import { parseKey } from '../instrument.js';
import { monthLabel, dayLabel } from '../dates.js';
import { strikeText } from './decoration.js';
import { spanFloor } from './bars-plan.js';
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

  /* ---- THE DAY WINDOW, AND IT BELONGS TO SPOT ONLY ---------------------
   *
   * A spot series runs forever, so "show me this week" is a real question and
   * a calendar is the only way to ask it. A CONTRACT is different: it is
   * defined BY its expiry, and its bars run from listing to that expiry and
   * stop. Narrowing a contract by a second date range asks a question the
   * contract has already answered, and a reader who set a window on spot and
   * then chose an expiry would be silently filtering a series whose extent was
   * never theirs to choose.
   *
   * So the pair is applied and drawn only while no expiry is chosen. Not
   * disabled — absent. A control that cannot change the answer is not a
   * control, and the page's other rungs already follow that rule.
   *
   * `YYYY-MM-DD`, the same shape `b.day` carries, so the compare is a string
   * compare and never a Date. Empty means unbounded on that side.
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

  const universed = $derived(
    universeNames === null ? db.deco : db.deco.filter((/** @type {any} */ r) => universeNames.has(r.underlying))
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
  const universeCount = $derived.by(() => {
    const out = new Map();
    if (!catalogueIsThisFeed) return out;
    const stored = new Set();
    for (const r of db.deco) stored.add(r.instrument);
    for (const u of UNIVERSES) {
      if (!u.token || !tokensHere.has(u.token)) continue;
      let members = 0;
      let held = 0;
      for (const r of masterRows()) {
        if (!tokensOf(r).includes(u.token)) continue;
        members += 1;
        if (stored.has(censusKeyOf(r))) held += 1;
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
  const outsideMaster = $derived.by(() => {
    if (!catalogueIsThisFeed) return null;
    const listed = new Set();
    for (const r of masterRows()) listed.add(censusKeyOf(r));
    const names = new Set();
    let months = 0;
    for (const r of db.deco) {
      if (listed.has(r.instrument)) continue;
      months += 1;
      names.add(r.instrument);
    }
    return { months, instruments: names.size };
  });

  const typed = $derived(filter.trim().toUpperCase());

  const instrumentRows = $derived.by(() => {
    const by = new Map();
    for (const it of universed) {
      let a = by.get(it.underlying);
      if (!a)
        by.set(
          it.underlying,
          (a = { key: it.underlying, months: 0, bars: 0, short: 0, segs: new Set(), keys: new Set() })
        );
      a.months += 1;
      a.bars += it.rows;
      a.short += it.short;
      // WHICH SEGMENTS THIS NAME HOLDS, so the Segment rung can offer what is
      // there and refuse what is not BY NAME rather than by absence.
      if (it.segRung) a.segs.add(it.segRung);
      a.keys.add(it.instrument);
    }
    return [...by.values()].sort((a, b) => txt(a.key, b.key));
  });

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
    for (const it of db.textMatched) c.set(it.kind, (c.get(it.kind) ?? 0) + 1);
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
      meta.set(u, { sym: r?.symbol ?? u, kind: r?.kind ? String(r.kind).toLowerCase() : '' });
    }
    /** @param {string} k @param {{months:number,bars:number,short:number}=} h */
    const decorate = (k, h) => ({
      key: k,
      sym: meta.get(k)?.sym ?? k,
      kind: meta.get(k)?.kind ?? '',
      months: h?.months ?? 0,
      bars: h?.bars ?? 0,
      short: h?.short ?? 0
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
  const contractRungs = $derived(
    kind === 'futures'
      ? { expiry: true, strike: false, money: false, side: false }
      : kind === 'options'
        ? { expiry: true, strike: true, money: true, side: true }
        : { expiry: false, strike: false, money: false, side: false }
  );

  /** Which of the three a stored row belongs to, off its own name. */
  /** @param {{expiry: string | null, instrument: string}} r */
  function segOf(r) {
    if (r.expiry === null) return 'spot';
    return /-(CE|PE)$/.test(r.instrument) ? 'options' : 'futures';
  }

  /** /ingest's three, each with what this selection holds of it. */
  const segmentRows = $derived.by(() => {
    const n = new Map();
    for (const r of db.textMatched) {
      const s = segOf(r);
      n.set(s, (n.get(s) ?? 0) + 1);
    }
    return SEG_VIEW.map((s) => {
      const c = n.get(s.key) ?? 0;
      return {
        key: s.key,
        name: s.name,
        /* THE DETAIL IS /ingest's NOTE, which is what that page puts here. The
           held count is this page's own fact and rides the title beside it. */
        detail: s.note,
        title: c > 0 ? `${fmt(c)} instrument-month(s) held. ${s.note}` : `Nothing stored. ${s.note}`,
        /* NOT `disabled`. A segment with nothing stored is a legitimate thing to
           select — the answer is an empty table, which IS the answer to "do I
           hold any of these". Disabling it would refuse the question. */
        why: c === 0 ? 'nothing stored under this segment for the current selection' : undefined
      };
    });
  });

  /** Text and kind: the base the month cards roll up. */
  /* `segOf`, NOT `r.kind`. `kind` now holds one of /ingest's three — spot,
     futures, options — and those are derived from a row's expiry rather than
     read off the store's second field. See `SEG_VIEW`. */
  const segmented = $derived(kind ? db.textMatched.filter((/** @type {any} */ r) => segOf(r) === kind) : db.textMatched);

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
  const timeframed = $derived(
    timeframe ? segmented.filter((/** @type {any} */ r) => r.timeframe === timeframe) : universed
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
    return `not one of the ${fmt(db.deco.length)} instrument-month(s) ${feedDisplay(feeds.active)} holds is named the way this repository names a contract. crates/core/src/instrument.rs ends a future “-<YYYY-MM-DD>-FUT” and an option “-<YYYY-MM-DD>-<strike in paisa>-CE|PE”, and nothing in this store ends either way: it holds indices and equities, whose names carry no expiry at all. /store.json sends no expiry field, so an expiry here can only ever be READ OFF A NAME — never decoded from a spelling this page guessed at. THIS RUNG IS NOT LIVE-ONLY AND NEVER WAS: an expired contract on disk would be listed here, because the list is folded off the stored names and nothing in it is compared to today. There is none because nothing in this build can fetch one — crates/api/src/server.rs answers POST /pull/fno with 503 and the reason “expired F&O has no local-archive path and no HTTP transport in this build”, and crates/core/src/vendor.rs declines every FUT/CE/PE master row as Skip::LiveContract. Nothing is being held back, because there is nothing to hold.`;
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

  /** Rows per side, with the gate above it applied — the facet rule, again. */
  /* THE SIDES THIS SELECTION REACHES, sorted — the cascade's ninth rung.
   *
   * `sidesAll` stays on `deco` and feeds `sideRefusal`, which DISABLES; a
   * disable computed from a selection is the trap where choosing a side and
   * narrowing past it kills the control that could clear it. Same split as
   * `expiryOffered` against `expiriesAll`, and for the same reason. */
  const sideOffered = $derived.by(() => {
    const seen = new Set();
    for (const r of expiryGated) if (r.side !== null) seen.add(r.side);
    return [...seen].sort();
  });

  /** How many rows the Both row returns — what clicking it actually gives. */
  const mnyFilteredCount = $derived(expiryGated.length);

  const sideCount = $derived.by(() => {
    const c = new Map();
    for (const r of expiryGated) {
      if (r.side !== null) c.set(r.side, (c.get(r.side) ?? 0) + 1);
    }
    return c;
  });

  /** How many rows the side rung can reach at all, under the gate. */
  const sidedHere = $derived(
    expiryGated.reduce(
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
    return `this selection reaches no option: ${fmt(expiryGated.length)} row(s) are here and none of them carries a side`;
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
  let page = $state(1);
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
  const filtered = $derived(
    Boolean(
      typed ||
        month ||
        kind ||
        holesOnly ||
        universe ||
        timeframe ||
        fromMonth ||
        toMonth ||
        expiry ||
        side ||
        strikePick.size ||
        mnyPick.size ||
        /* THE DAY WINDOW COUNTS AS A FILTER, and it was the only live one
           missing from this list. `month`, `holesOnly`, `fromMonth` and
           `toMonth` are all documented dead; `fromDay`/`toDay` are the two the
           operator can actually move, and `barWindowed` really does narrow on
           them. Left out, a reader who had cut the window to one week saw no
           "filtered" state and no Reset button — the page said nothing was
           narrowing while a week's worth of rows was all it would show.
           Compared against the seeded span, not against empty: the fields open
           populated at the store's own floor and ceiling, so "set" is not
           "narrowed" — only a value that differs from the full span is.
           `dayWindowNarrows` is that comparison, named once and shared with
           the manifest-disagreement guard, which needs the identical question
           answered the identical way. */
        db.dayWindowNarrows
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
      typed ? { k: 'text', text: `the text “${filter.trim()}”` } : null,
      /* "segment", matching the control's own label. The state is still called
         `kind` because that is the field `deco` parses out of the instrument
         string and renaming it would touch the sort, the facet counts and the
         drawer for no gain — only the word an operator reads changed. */
      kind ? { k: 'segment', text: `segment ${kind}` } : null,
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
      strikePick.size
        ? {
            k: 'strike',
            text:
              `strike ${strikeText(Number([...strikePick][0]))}`,
            key: [...strikePick].join(' · ')
          }
        : null,
      mnyPick.size
        ? {
            k: 'moneyness',
            text:
              `moneyness ${[...mnyPick][0]}`
          }
        : null,
      side ? { k: 'side', text: `option type ${side}`, key: side } : null,
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
    /* `filter && !picked`, AND THE OLD TEST COULD NOT FIRE AT ALL.
       It read `picked && !instrumentKeys.has(picked)`. `instrumentKeys` maps
       UPPER(key) -> key and `picked` is `instrumentKeys.get(typed) ?? ''`, so
       `picked` is only ever truthy when it came OUT of that map — and for an
       all-uppercase store key, which is every key this store produces,
       `has(picked)` is then true and the guard is false. Meanwhile the exact
       condition the branch exists for — `filter` naming an instrument the
       coarser rungs no longer offer — is when the lookup MISSES, and `picked`
       collapses to `''` and short-circuits it. Both arms wrong, in opposite
       directions.

       So the rung most likely to be stranded (see the Instrument gate above,
       where a universe the store holds nothing of used to remove the control
       outright) was the one rung that could never report it, and its Clear
       button was unreachable.

       `filter` is what the operator set and `picked` is what it resolved to;
       set-but-unresolved is precisely stranded. `value` prints what he typed
       rather than the empty string it resolved to. */
    if (filter && !picked) {
      out.push({ rung: 'Instrument', value: filter, clear: () => (filter = '') });
    }
    /* `SEG_VIEW`, NOT `kinds`. `kind` holds one of /ingest's three now — spot,
       futures, options — and `kinds` holds the store's own second field, INDEX /
       CASH / FNO. Comparing across the two vocabularies made this fire on every
       load: "Segment is set to spot, which the rungs above it no longer offer",
       over a rung that was offering exactly that. All three are always offered,
       so the only way to strand this rung now is a value from neither
       vocabulary. */
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
    page: { get: () => page, set: (v) => (page = v) },
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
 * THE SEEDS — the four effects that put a value on an empty rung once the store
 * has answered: the day window, the segment, the instrument and the timeframe.
 *
 * Creates the page's effects that stood at old lines 6613, 6664, 6671 and 6697; the shell calls it
 * in that position of the order.
 *
 * @param {any} db the page's shared state object (see the shell)
 */
export function selectionSeeds(db) {
  /* A SELECTION THE CASCADE HAS SINCE ELIMINATED, NAMED RATHER THAN CLEARED.
   *
   * Every rung offers only what the rungs above it leave, so widening one above
   * can strand a choice below: pick NIFTY, switch the universe to one NIFTY is
   * not in, and the instrument stays selected while its own menu no longer
   * lists it. The table empties — correctly — and NOTHING SAYS WHY. That is the
   * one hole the cascade left, and an empty grid with no reason is the shape §4
   * calls a silent failure.
   *
   * NAMED, NOT CLEARED. Dropping the selection silently is the other half of
   * the same fault: the reader asked for something and the page decided on
   * their behalf that they no longer want it, invisibly. This states the fact
   * and offers the button. */
  /**
   * THE DAY WINDOW OPENS ON THE SPAN THE STORE ACTUALLY HOLDS.
   *
   * Both fields opened on `dd Mon yyyy` while /ingest's opened on real dates,
   * and that was the visible half of "these are not the same control". The
   * other half was a five-pixel height difference, fixed in `DayField`.
   *
   * IT IS /ingest's RULE, NOT ITS VALUE, AND THAT DISTINCTION IS FORCED HERE.
   *
   * On /ingest the window is the ASK and its bounds are the server picker's
   * whole span, so it can open on 01 Jan 2015. Here the two fields are BOUNDED
   * BY WHAT THE STORE HOLDS — `min={dayBounds[0]}` on the From field — because
   * the calendar's job on this page is to strike the days with no bar behind
   * them, which is how a reader sees where the store starts and stops.
   *
   * Seeding this to 01 Jan 2015 was tried and reverted: it puts a value in the
   * field that the field's own `min` refuses, so it renders `aria-invalid` and
   * the control contradicts itself on load. A window that opens refused is worse
   * than one that opens narrow.
   *
   * So it opens on the whole span the store holds — `dayBounds[0]` to
   * `dayBounds[1]`, both measured off the rows, never a date typed into this
   * file. That is the same PRINCIPLE /ingest follows: mapped to what is really
   * there.
   *
   * SEEDED ONCE, NOT PINNED. The guard is that both fields are empty, so this
   * fires when the store first answers and never again — a window the reader
   * narrowed is his, and `stranded` speaks up if a chosen day stops being held
   * rather than this quietly moving him.
   */
  $effect(() => {
    if (db.fromDay === '' && db.toDay === '' && db.spanCeil) {
      db.fromDay = spanFloor;
      db.toDay = db.spanCeil;
    }
  });

  /**
   * EXACTLY ONE SEGMENT AND EXACTLY ONE INSTRUMENT, SEEDED.
   *
   * The `All segments` and `All instruments` head rows are gone — every rung on
   * this page is `single`, so "all of them" was never a member of the set being
   * offered, only the way to leave the rung unanswered. With the rows gone the
   * empty string is no longer a state the page can rest in, so each seeds itself
   * the moment there is something to seed from.
   *
   * ONE INSTRUMENT IS NOT A NARROWING, IT IS A CORRECTION. The bars table has no
   * instrument column — `b.instrument` appears only on a row's `title` — so a
   * table blended across instruments printed OPEN, HIGH, LOW and CLOSE from
   * different price scales with nothing on screen saying which row belonged to
   * which name. Exactly one instrument is the only selection under which those
   * columns mean anything, the same argument that took `All rungs` off Timeframe.
   *
   * PREFER SOMETHING HELD. Both seeds pick a member with rows behind it where
   * one exists, because a page that opens on an empty table over a store that
   * holds data has answered a question nobody asked. It falls back to the first
   * offered member when the store holds none of them — which is honest: the
   * table is empty because the store is.
   */
  /* `deco.length > 0` IS THE GUARD, AND `segmentRows.length > 0` WAS NOT ONE.
     `segmentRows` is `SEG_VIEW.map(...)` — always exactly three rows, whatever
     the store holds — so that test is a constant, not a gate, and this effect
     fired on the very first tick. At mount `feeds.active` is still null and
     `rows` is `[]`, so all three rows carry a `why`, `held` is `undefined`,
     and `kind` was set to `segmentRows[0].key`. Because the outer guard is
     `kind === ''` it then never re-evaluated, and `forgetPreviousFeed`
     deliberately preserves `kind`, so a later feed never re-opened it either.

     Net: "PREFER SOMETHING HELD" — the rule the paragraph above states — was
     dead for this rung. The choice was always made against an empty store and
     always landed on the first row. It is masked today only because spot is
     the one populated segment, so `segmentRows[0]` happens to be right; it
     becomes visible the day the store holds a settled contract and no spot row
     for the selection.

     Waiting for the complete decorated census is what makes `why` mean
     "read, and this segment is empty" rather than "still preparing". Raw rows
     arrive before that asynchronous work finishes; they cannot seed a picker
     whose held counts come from the prepared snapshot. The instrument must
     also settle first: a segment held by another instrument is not evidence
     that the selected one has it. */
  $effect(() => {
    if (db.kind === '' && db.deco.length > 0 && db.filter !== '' && db.segmentRows.length > 0) {
      const held = db.segmentRows.find((/** @type {any} */ s) => s.why === undefined);
      db.kind = (held ?? db.segmentRows[0]).key;
    }
  });

  $effect(() => {
    if (db.filter === '' && db.deco.length > 0 && db.instrumentOffered.length > 0) {
      const held = db.instrumentOffered.find((/** @type {any} */ r) => r.months > 0);
      db.filter = (held ?? db.instrumentOffered[0]).key;
    }
  });

  /**
   * EXACTLY ONE RUNG, ALWAYS, SEEDED FROM THE STORE.
   *
   * `timeframe === ''` used to mean "every rung at once" and there is no such
   * view any more — see the comment where the `All rungs` row used to be. So the
   * empty string is no longer a state this page can rest in: the moment the
   * store answers, the finest rung it holds is selected.
   *
   * FINEST, NOT FIRST BY NAME. `tfAll` is already sorted by `tfCmp`, which
   * orders by BAR LENGTH and not by text — as text these read `15min`, `1day`,
   * `1min`, which is an ordering by first character and not by anything a bar
   * length has. So `tfAll[0]` is the finest rung stored, which is the one the
   * engine sweeps and the one a reader opening this page is most likely to want.
   *
   * IT DOES NOT OVERRIDE A CHOICE. The guard is that `timeframe` is empty, so
   * this fires once per store reading and never again — a rung the operator
   * picked is his, and `stranded` is what speaks up if the store stops holding
   * it rather than this quietly moving him.
   */
  $effect(() => {
    if (db.timeframe === '' && db.filter !== '' && db.kind !== '' && db.tfAll.length > 0) db.timeframe = db.tfAll[0][0];
  });
}

/**
 * A NARROWING RETURNS THE READER TO PAGE ONE — the effect that watches every
 * rung and both grids' sorts and resets the page.
 *
 * Creates the page's effects that stood at old lines 7393; the shell calls it
 * in that position of the order.
 *
 * @param {any} db the page's shared state object (see the shell)
 */
export function selectionPaging(db) {
  $effect(() => {
    /* THE TWO NEW RUNGS ARE IN HERE FOR THE SAME REASON THE OTHERS ARE. A
       universe or a month window changes WHICH rows exist, not merely their
       order, so an offset kept across the change lands the operator halfway
       down a list they have never seen — and the row under the cursor is a
       different row than the one they left. */
    void [
      db.typed,
      db.month,
      db.kind,
      db.holesOnly,
      db.universe,
      db.fromMonth,
      db.toMonth,
      /* THE CONTRACT RUNG IS IN HERE FOR THE SAME REASON THE OTHERS ARE: a
         strike or a rung changes WHICH rows exist, not merely their order. */
      db.strikePick,
      db.mnyPick,
      db.sortKey,
      db.desc,
      /* THE BAR GRID'S OWN SORT IS IN HERE FOR THE SAME REASON THE CENSUS
         GRID'S IS: a re-order changes which rows the FIRST page holds, and
         a reader left on page 12 of a re-sorted run is standing somewhere
         they never chose. */
      db.barSortKey,
      db.barDesc,
      /* AND THE READ BUDGET, because it changes which FILES were opened and
         therefore which bars exist at all. */
      db.readBudget
    ];
    untrack(() => {
      db.restartAtTop();
      /* A NARROWING RETURNS THE READER TO PAGE ONE. Keeping the ordinal
         across a change of WHICH ROWS EXIST is the same defect as keeping
         the scroll offset, one altitude up: page 12 of the old run is a
         different twelve rows in the new one, and very often none at all.
         `pageNow` already clamps, so this is not a correctness fix - it is
         the difference between landing at the top of the answer and landing
         at the end of it. */
      db.page = 1;
    });
  });
}

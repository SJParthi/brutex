<script>
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
   * **O(1) per keystroke.** The filter is a prefix probe into a Map, built once
   * per dataset, exactly as `web/typeahead.js` does it. Never a scan of the
   * universe, and never a request per character.
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
  import { feeds } from '$lib/feeds.svelte.js';
  import Picker from '$lib/Picker.svelte';
  import DayField from '$lib/DayField.svelte';
  /* THE FEED'S OWN MASTER, ALREADY ON HAND. `+layout.svelte` calls
     `loadCatalogue(feeds.active)` on every feed change, so the universe rung
     costs this page NO request — it is a join against a list the layout has
     already paid for. It also arrives stamped with the feed it answers for,
     which is the only thing that makes a count under this feed's name
     checkable. See `universeRefusal`. */
  import { catalogue } from '$lib/index.svelte.js';
  // A STORE KEY IS NOT AN INSTRUMENT NAME. `$lib/instrument.js` takes the
  // contract tail off before the underlying, which is the difference between
  // BANKNIFTY and BANKNIFTY-2026-07-28-4810000-PE.
  import { parseKey, segmentOf } from '$lib/instrument.js';
  /* Display only. `month` stays the raw `YYYY-MM` because it is the filter
     itself — `r.month === month`, `r.month.startsWith(typed)`, the `{#each}`
     keys and the sort comparator all read the key, never the label. See the
     rule at the top of `$lib/dates.js`.

     `stampLabel` renders the one instant this page shows a human — when
     `/store.json` was last read. It names Asia/Kolkata rather than inheriting
     the host's zone, so "as of 15:29" is the same minute for every operator
     reading the same store.

     `dayLabel` renders the DAY WINDOW each stored month actually covers.
     `/store.json` has carried `first_ts` / `last_ts` per row — in MICROseconds
     — since the census gained them, and no page has ever read them. A month
     row that says "2020-03" and nothing else cannot distinguish a month held
     from the 2nd to the 31st from one held on the 2nd alone; the window says
     which, in the `02 Sep 2024` form this product uses everywhere. */
  import { MON, dayLabel, monthLabel, stampLabel, timeLabel } from '$lib/dates.js';
  import { store, survey, syncStore, refreshStore, surveyStores } from '$lib/store.svelte.js';
  // THE COMPLETENESS ARITHMETIC, OUT OF THE MARKUP AND UNDER A TEST. Two
  // defects lived in these expressions — a denominator taken from whichever
  // rung's row arrived first, and a row that was its own denominator reporting
  // `full` — and neither was reachable by anything but loading this page
  // against a store in exactly the right state. `web/tests/completeness.test.js`
  // drives them under `node --test`.
  import { denominators, denomKey, isSole, rollUpMonths } from '$lib/completeness.js';
  import { basisPoints, bpsText, dirOf } from '$lib/bps.js';
  import { exact } from '$lib/money.js';
  // A REQUEST THAT CANNOT END IS A SPINNER THAT LIES. `ask` is `fetch` with a
  // ceiling; see `$lib/ask.js` for why the wrapper exists rather than a signal
  // threaded through every call site.
  import { ask } from '$lib/ask.js';

  /* ======================================================================
     THE SHAPES, NAMED ONCE — imported where they already exist
     ----------------------------------------------------------------------
     `StoreRow` and `Feed` are DECLARED BY THE MODULES THAT FETCH THEM, and
     they are imported here rather than restated. A second spelling of one
     wire shape is the drift this whole file is written against: the day
     `/store.json` gains an eleventh field, the copy that is not the fetcher's
     goes quietly stale and every reader downstream of it believes the stale
     one. `import('…').Name` in a JSDoc type is a compile-time reference and
     emits nothing — it costs the bundle zero bytes.

     `MasterRow` is the exception, and it is one only because no module owns
     `/instruments.json`'s row shape yet: `$lib/index.svelte.js` types
     `catalogue.rows` as `any[]` and `masterRows()` re-states that. It is
     declared HERE, from a live response read off the running server rather
     than guessed, so the two names this page actually joins on —
     `exchange`/`segment`/`symbol`, which is the census key, and `universes`,
     which is the membership token list — are checked at the join and not at
     the crash. The other fields are on the wire and are written down because
     leaving them out would make the typedef read as a claim that they are
     absent.

     The response the shape was read from, `GET /instruments.json?feed=dhan`:

       {"symbol":"NIFTY","key":"NSE-NIFTY","kind":"Index","exchange":"NSE",
        "segment":"INDEX","universe":"index+fno","universes":["index","fno"],
        "bars":8470,"href":"/instruments?q=NSE-NIFTY"}

     `universes` is optional and `universe` is the legacy `+`-joined string
     that predates it — see `tokensOf`, which reads the array when the row has
     one and splits the string when it does not. Both are written optional
     because a master row served by an older API binary carries only the
     second, and the type has to admit the row this page is built to survive.
     ====================================================================== */
  /** @typedef {import('$lib/store.svelte.js').StoreRow} StoreRow */
  /** @typedef {import('$lib/feeds.svelte.js').Feed} Feed */
  /**
   * @typedef {{
   *   symbol: string,
   *   key: string,
   *   kind: string,
   *   exchange: string,
   *   segment: string,
   *   universe?: string,
   *   universes?: string[],
   *   bars: number,
   *   href: string
   * }} MasterRow
   */

  /* ======================================================================
     DATA
     ====================================================================== */
  /* THE ROWS ARE NOT THIS PAGE'S READ ANY MORE.
     ----------------------------------------------------------------------
     `/store.json` was fetched from SIX places across this product on six
     clocks, and TWO of them were on this page. Nothing reconciled them: while a
     pull ran, this page and `/ingest` printed different totals for the same
     disk and each was right about its own snapshot. `$lib/store.svelte.js`
     reads it ONCE per (feed, generation) and folds every shape any page needs
     in one pass; these four names are the same four this file always used,
     pointed at that reading.

     `rows` IS THE RAW ARRAY, exactly what the wire sent — this page reads ten
     fields off a row and derives its own denominators, so it takes the array
     and not the census's normalised view of it. */
  const rows = $derived(store.feed === feeds.active ? store.rows : []);
  const error = $derived(store.error);
  /* A READING THIS FEED HAS NOT ANSWERED FOR YET IS STILL "IN FLIGHT", even in
     the instant between the selection moving and the effect that re-reads it.
     Without the second clause that instant renders as a finished read of an
     empty store — "Nothing stored for Groww" over a store nobody has asked
     about — which is a claim about the disk, not a state of the page. */
  const loading = $derived(
    store.state === 'reading' ||
      (feeds.active != null && store.state !== 'error' && store.feed !== feeds.active)
  );
  /* WHEN THIS FEED'S STORE LAST ANSWERED — 0 when it never has. `store.lastOk`
     is the last SUCCESSFUL read and carries the feed it was for, so a failed
     refresh keeps the minute (under the word "last good") and a feed switch
     drops it, which is what this page's `as of` line has always meant. */
  const fetchedAt = $derived(store.lastOk.feed === feeds.active ? (store.lastOk.at ?? 0) : 0);
  /* Bumped by Refresh; the store grows under a pull. IT IS THE SHARED CLOCK:
     the bump re-reads the census for every page at once, so Refresh here can no
     longer leave `/ingest` holding an older total than this table. */
  const nonce = $derived(store.generation);

  let filter = $state('');
  let month = $state(''); /* '' = every month */
  let kind = $state(''); /* '' = every kind  */
  let holesOnly = $state(false);
  let sortKey = $state('instrument');
  let desc = $state(false);

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

  /* THE WINDOW IS ALWAYS DRAWN, AND IT NEVER TOUCHES THE RUNGS ABOVE IT.
   *
   * An earlier draft hid this pair the moment an expiry was chosen. That was a
   * misreading of "the dates must not be applicable to a contract". The order
   * the owner set is feed, universe, instrument, segment, timeframe, expiry,
   * strike, moneyness, side, FROM DATE, TO DATE — the dates are in the F&O
   * list, so they are present for a contract too.
   *
   * What must not happen is the window narrowing the CONTRACT RUNGS. It cannot:
   * this filters `barRows`, which is downstream of every offer.
   * `expiryOffered` folds `matchedPreContract`; `strikeOffered` and
   * `sideOffered` fold `expiryGated` — all upstream of the bars. So choosing a
   * fortnight can never make an expiry vanish from its own menu, which is the
   * failure that instruction was guarding against.
   *
   * A contract's extent is still its expiry. The window only says which of its
   * days to draw. */
  const dayWindowApplies = true;

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
   * A session is 375 one-minute bars, 09:15–15:29 inclusive.
   *
   * NSE's Closing Auction Session moved the close: from 2026-08-03 the
   * continuous session ends at 15:29. Stated because a completeness figure with
   * an unstated denominator is a number nobody can check.
   */
  const BARS_PER_SESSION = 375;

  /**
   * Bars one session holds **at the rung the row is stored at**.
   *
   * # The defect this removes
   *
   * `BARS_PER_SESSION` is 375 because that is how many ONE-MINUTE bars a
   * session holds. It was applied to every row regardless of rung, and the
   * server compounded it by stamping `"timeframe":"1m"` on every census row —
   * so a complete 1-day month, 21 bars for 21 trading days, was divided by 375
   * and reported as `0.06 sessions observed`, i.e. ~99.7% missing. The store
   * was right; both readers were wrong in the same direction.
   *
   * A DAILY session is ONE bar by definition. That is not an approximation:
   * the rung IS the session for `1day`.
   *
   * A rung this function has no number for returns `null`, and every caller
   * renders a dash rather than a figure — an unknown denominator produces no
   * percentage at all, never a plausible one. The same rule the % change
   * columns already follow.
   *
   * @param {string | undefined} tf a rung name as the census spells it
   * @returns {number | null} `null` means NO RECORDED SESSION SIZE, never zero
   */
  const barsPerSession = (tf) => {
    if (tf === '1min' || tf === '1m') return BARS_PER_SESSION;
    if (tf === '1day') return 1;
    return null;
  };

  /**
   * THE BAR-LENGTH LADDER — a rung's own length in SECONDS.
   *
   * # Why a number and not a list position
   *
   * `store::path::Timeframe` is `{ secs: u32, name: &'static str }`, and the
   * seconds are the field that MEANS something: `1min` is 60 and `1day` is
   * 86,400 because those are the numbers in `crates/store/src/path.rs`, not
   * because of where they sit in an array here. Every value below is copied
   * from a `Timeframe` constant in that file and nothing is interpolated
   * between them — a rung this table has no second count for is not given a
   * guessed one.
   *
   * `1m` IS AN ALIAS AND IS MARKED AS SUCH. It is what the server stamped on
   * every census row before it learned to write the real rung, and it may still
   * be in a cached response, so it gets the length this table already knows
   * rather than falling off the end of the ladder.
   *
   * `1hr` WAS A SECOND ALIAS AND IS GONE. This comment used to read: *"`1hr` is
   * `pull::vendor::Granularity`'s spelling of the rung the store files under
   * `60min`"* — a plain statement that one rung had two names, absorbed here by
   * hand because the two crates disagreed. D-0132 made them one word: the
   * store's, `60min`, which is the one that becomes a path. Nothing emits `1hr`
   * any more, and the server refuses it by name as an unknown rung, so keeping
   * a row for it would map a spelling nothing sends onto a length — a workaround
   * outliving the defect it worked around.
   *
   * # Why this exists at all
   *
   * Sorted as TEXT, the rungs read `15min`, `1day`, `1min`, `30min`, `3min`,
   * `5min`, `60min` — ordered by first character. That is the same defect as a
   * month column ordered by `Apr`, and it is why `$lib/dates.js` opens with the
   * rule it does. A bar-length axis has ONE order and it is the length.
   *
   * @type {ReadonlyMap<string, number>}
   */
  const TF_SECS = new Map([
    ['1s', 1],
    ['1min', 60],
    ['1m', 60],
    // 2min AND 10min WERE MISSING, AND THE STORE HAS BOTH.
    //
    // `store::path::Timeframe::KNOWN` lists SECOND_1, MINUTE_1, MINUTE_2,
    // MINUTE_3, MINUTE_5, MINUTE_10, MINUTE_15, MINUTE_30, MINUTE_60 and
    // DAY_1 — ten rungs — and this table carried seven of them. The two
    // absentees sort to `Infinity` and land at the END of the ladder, after
    // 1day, each labelled "length not recorded". Measured on the live page:
    // the rung menu read 1min, 3min, 5min, 15min, 30min, 60min, 1day, 10min,
    // 2min. Two of the nine rungs the store actually holds were out of order
    // and disclaiming a length the store publishes.
    //
    // The seconds are `Timeframe::MINUTE_2.secs` = 120 and
    // `MINUTE_10.secs` = 600, read from `crates/store/src/path.rs`, not
    // arithmetic done here.
    ['2min', 120],
    ['3min', 180],
    ['5min', 300],
    ['10min', 600],
    ['15min', 900],
    ['30min', 1800],
    ['60min', 3600],
    ['1day', 86400]
  ]);

  /**
   * A rung's position on that ladder.
   *
   * A rung the table has no length for sorts LAST — `Infinity`, which is past
   * every real length — and is never folded into the middle, because a string
   * nobody measured has no measured relation to the rungs either side of it.
   * The control draws it, says `length not recorded`, and lets it be chosen:
   * the store holds those rows and hiding the only way to reach them would be
   * worse than admitting the ladder does not reach that far.
   *
   * @param {string} tf
   */
  const tfSecs = (tf) => TF_SECS.get(tf) ?? Number.POSITIVE_INFINITY;

  /**
   * THREE-WAY, and it is not a formality.
   *
   * `a < b ? -1 : 1` answers "greater" for two EQUAL inputs, in both
   * directions at once, and a comparator that contradicts itself leaves the
   * order of those elements unspecified — the same store could render its
   * rungs in a different order on the next pass. Equal lengths fall through to
   * `txt`, which is itself three-way, so two spellings of one length are
   * ordered and stay ordered.
   *
   * The example this used to give was `1hr` beside `60min`, and D-0132 removed
   * it: they were one rung with two names and there is now one. The rule is
   * kept because `1m` beside `1min` is still a live pair, and because a
   * comparator that only happens to be consistent is not one.
   *
   * @param {string} a
   * @param {string} b
   */
  const tfCmp = (a, b) => {
    const sa = tfSecs(a);
    const sb = tfSecs(b);
    return sa < sb ? -1 : sa > sb ? 1 : txt(a, b);
  };

  /* `tfNote` — the third of these — is NOT here, and its absence is the rule
     the FORMATTING block below states: it calls `fmt`, so it is declared after
     `fmt` and not before it. The two above call nothing. */

  /* ======================================================================
     FORMATTING
     ----------------------------------------------------------------------
     DECLARED HERE, ABOVE EVERY READER. These sat at the bottom of the script
     and the refusal notice — a `$derived` that formats live counts — closed
     over `fmt` from above it. It ran, because a derived is lazy and the const
     is initialised by the time anything reads it, but `svelte-check` called it
     what it is: a temporal-dead-zone reference that works by scheduling
     accident. One reordering of the evaluation and it throws.
     ====================================================================== */
  /* PINNED TO en-IN, LIKE EVERY OTHER PAGE. An unqualified `Intl.NumberFormat()`
     inherits whatever locale the host happens to run, so the same store renders
     8,78,28,617 on one machine and 878,286,617 on another. That makes a
     screenshot unreproducible and a bug report unfalsifiable — and this one
     formatter feeds `fmt`, which is called 36 times across the headline
     counters, the month cards, the receipt prose and every table cell.
     `audit/+page.svelte:145` pins the same way. */
  // AND IT NOW DASHES AN UNKNOWN. `nf.format(NaN)` is the string "NaN" and
  // `format(Infinity)` is "∞" — a non-answer rendered as though it were one, at
  // all 126 of this page's call sites, while `/audit` guarded the same values.
  // `exact` does not round, because a record count is already whole and
  // rounding one would hide a fractional value that should never have arrived.
  const fmt = exact;
  /* FIXED TWO DECIMALS, ALWAYS. Adaptive precision made the digit count change
     from row to row, so the column's decimal point moved and the eye had to
     re-find it on every line. A column of numbers is a column or it is not. */
  /* `null` IS A LEGAL ARGUMENT AND IT RENDERS A DASH. A ratio whose
     denominator is zero is not 100% and not 0% — it does not exist, and
     `(null * 100).toFixed(2)` would print `0.00%`, a measurement, over an
     absence. Every caller that can reach an empty denominator passes `null`
     and gets the dash; the REASON belongs beside it at the call site, which is
     the only place that knows which absence it is. */
  /** @param {number | null | undefined} p a ratio in 0..1, or the absence of one */
  const pctText = (p) => (p === null || p === undefined ? '—' : `${(p * 100).toFixed(2)}%`);
  /**
   * A session count, or a dash when the rung has no recorded session size.
   *
   * `null` reaches here from `sessions()` for a rung `barsPerSession` has no
   * number for. A dash is the honest render: `0.00` would be a measurement and
   * this is an absence. Same rule the % change columns follow.
   *
   * @param {number | null | undefined} d
   */
  const dayText = (d) =>
    d === null || d === undefined ? '—' : Number.isInteger(d) ? String(d) : d.toFixed(2);
  /**
   * When `/store.json` was last read, as a date and an IST minute.
   *
   * A bare `toLocaleTimeString()` printed `15:29:04` in whatever zone the
   * machine happens to sit in and named neither the zone nor the day — so a
   * stamp from yesterday's session was indistinguishable from one taken a
   * minute ago, which is the single fact this line exists to report.
   *
   * `0` means the store has NOT been read, and that is an absence: passing it
   * through would render the epoch, `01 Jan 1970, 05:30`, which is a stamp
   * nobody took.
   *
   * @param {number} t epoch MILLISECONDS, or `0` for "never read"
   */
  const clock = (t) => (t ? stampLabel(t) : '—');

  /**
   * The DAYS one stored month actually covers. `02 Sep 2024 → 30 Sep 2024`.
   *
   * `null` when the row carries no stamps — an older `/store.json` payload, or
   * an entry written before the census recorded them. The caller prints the
   * absence and names it; inventing the month's first and last calendar day
   * would be this page asserting a trading calendar it does not have, and the
   * first and last day of a month are very often not trading days at all.
   *
   * Both ends go through `dayLabel`, so the month is `Sep` and never `Sept`
   * and the zone is IST whatever the reading machine is set to.
   *
   * THE PARAMETER NAMES THE TWO FIELDS IT READS AND NOT THE WHOLE ROW. Every
   * caller passes a decorated census row, but what this function needs is a
   * pair of stamps — so a month card, a drawer line and a grid cell can all
   * reach it without one of them having to be a census row to qualify.
   *
   * @param {{ firstAt: number | null, lastAt: number | null } | null | undefined} it
   * @returns {string | null} `null` when the row carries no stamps at all
   */
  const heldWindow = (it) =>
    it && it.firstAt !== null && it.lastAt !== null
      ? `${dayLabel(it.firstAt)} → ${dayLabel(it.lastAt)}`
      : null;

  /* ---- THE PERCENTAGE COLUMNS ----------------------------------------
     `/store.json` sends INTEGER BASIS POINTS. 125 is +1.25%. It is never a
     float on the wire, because `CLAUDE.md` §7 bans the float for money and a
     ratio derived from money is the same arithmetic — see the handler's own
     comment, which promised this shape for a long time before D-0069 made it
     true. The decimal point is inserted HERE, for display, exactly as it is
     for prices.
     ==================================================================== */

  /**
   * Basis points as a signed percentage with a fixed two decimals.
   *
   * THE DIVISION IS EXACT AND IT IS NOT `bps / 100`. The remainder is taken
   * with `%` — an integer operation — and the quotient is a multiple of 100
   * divided by 100, which IEEE returns exactly for every value inside 2^53.
   * A naive `(bps / 100).toFixed(2)` is a float rounding of a value that has an
   * exact decimal expansion, and it would be the one place on this page where a
   * price-derived number went through a float for no reason at all.
   *
   * TWO DECIMALS ALWAYS, AND THE SIGN IS PART OF THE NUMBER. Adaptive precision
   * moves the decimal point from row to row; a `+` that only appears on losses'
   * mirror image makes a column of gains read as a column of magnitudes. Zero
   * carries no sign because it has no direction — the cell is neutral-coloured
   * and says `0.00%`, which is a real answer and not an unknown.
   *
   * @param {number} bps INTEGER basis points off the wire; 125 is +1.25%
   */
  // `bpsText` and `dirOf` live in `$lib/bps.js` beside `basisPoints`, because
  // the three are one rule: what a ratio rounds to, how it reads, and what
  // colour it earns. The `-0` note there explains why this formatter is safe
  // only while that producer rounds away from zero.

  /** up / down / flat / none — the direction green and red are reserved for.
      `none` is its own word rather than `flat`, because a cell nobody can
      compute and a month that did not move are different facts and must not
      share a colour. */
  /** @param {number | null} bps */


  /**
   * WHY a percentage is a dash. One sentence per server reason code.
   *
   * `CLAUDE.md` §4 — degrade loudly and NAME THE REASON. A single "n/a" over
   * five distinct facts is the fallback that hides a failure: "this equity can
   * never be shown until a threshold is sourced" and "re-ingest this month and
   * it will appear" are opposite instructions, and one dash cannot say both.
   *
   * An unrecognised code is reported as an unrecognised code, WITH the code.
   * Guessing a sentence for it would be this page inventing a fact about the
   * server, which is the exact failure the map exists to prevent.
   */
  const WHY = {
    corporate_action_unverified:
      'REFUSED, not missing. This instrument can split, bonus or consolidate, and no corporate-action threshold is sourced anywhere in this repository — docs/00-charter.md names no verified split-and-bonus feed. An unadjusted 1:5 split reads as an 80% overnight crash that is indistinguishable from a real move, so docs/05-decisions.md D-0018 refuses the window loudly rather than printing it. Indices never split and do show a number. This cell stays a dash until an operator supplies a threshold with a source.',
    not_recorded:
      'The census holds this month and no close was ever read for it — a manifest version-1 entry, or a version-2 one written before its bar file was priced (D-0067). Not zero, and not a price of zero: nobody has looked. Re-ingesting the month records both closes and this fills in.',
    no_earlier_month:
      'This feed holds no bars for the month immediately before this one, so there is no previous change to show. The first month an instrument holds is the ordinary case; a hole in the middle is the other one. It is unknown, never 0.00%.',
    base_not_positive:
      "The month's first close is zero paisa, so the ratio has no base and is undefined. Zero means zero here (CLAUDE.md §7), so this is a real stored price and not a missing one — but nothing can be divided by it.",
    overflow:
      'The move times 10,000 leaves a 64-bit integer, so the server refused it rather than wrapping it into a plausible small number.'
  };

  /**
   * THE PROBE IS WIDENED AT THE LOOKUP, NOT AT THE TABLE. `WHY` keeps the
   * five-key shape it is written in — that is the fact this page holds, and
   * narrowing it to an index signature would let a sixth key be added above
   * and read below with nothing noticing the sentence was never written. The
   * CODE, by contrast, comes off the wire and may be anything at all, which is
   * the entire reason the fallback line exists. So the widening happens
   * exactly where the unknown key is used, and `undefined` in the cast is what
   * makes the `??` a real branch rather than dead code. `barWhyText` does the
   * same for the bar grid's five reasons.
   *
   * `null` is the ordinary case here and not an error: `chg_why` is null on
   * every row that carries a NUMBER, and this sentence is only rendered where
   * the number is absent.
   *
   * @param {string | null} code
   */
  const whyText = (code) =>
    (code === null ? null : /** @type {Record<string, string | undefined>} */ (WHY)[code]) ??
    `The server named a reason this page does not have a sentence for: ${code ?? 'none given'}. That is a mismatch between this page and /store.json, not a fact about the data.`;

  /**
   * What one bar-length rung says about itself under the Timeframe control —
   * its length, and the session size the completeness column divides by.
   *
   * DECLARED HERE AND NOT BESIDE `TF_SECS`, because it calls `fmt` and `fmt`
   * is declared in the block above this line. That is the whole rule the
   * FORMATTING block states, applied rather than quoted.
   *
   * Never a figure this page does not hold: a rung with no recorded
   * bars-per-session says so rather than printing a denominator nobody chose —
   * the same rule `sessions` and the two percentage columns already follow.
   *
   * @param {string} tf
   */
  const tfNote = (tf) => {
    const per = barsPerSession(tf);
    const secs = TF_SECS.get(tf);
    const len = secs === undefined ? 'length not recorded' : `${fmt(secs)}s bars`;
    return per === null ? `${len} · no recorded session size` : `${len} · ${fmt(per)}/session`;
  };

  /**
   * WHICH FEED THE ROWS ON SCREEN BELONG TO.
   *
   * Deliberately a plain `let` and not `$state`: it is compared, never
   * rendered and never derived from, so making it reactive would only add a
   * dependency the fetch effect would then have to `untrack`.
   *
   * # The defect it removes
   *
   * `rows` was left standing while the new feed's request was in flight, so a
   * feed switch put the PREVIOUS feed's counts under the NEW feed's name: the
   * title read `DB · Groww`, every tile still held Dhan's totals, and `as of`
   * still held the minute Dhan was read. Every one of those numbers was
   * counted — from the wrong store. A count that survives the query that
   * produced it is exactly the stale value `CLAUDE.md` §4 bans, and it is
   * worse than a blank because it is plausible.
   *
   * Refresh is the deliberate exception: `nonce` re-runs this effect with the
   * SAME feed, `rows` is kept, and the table stays readable under the
   * "refreshing…" indicator while the same store is re-read.
   *
   * `null` is NOT A FEED and is the only value it holds before the first
   * selection lands, which is why the effect below tests `!f` before it
   * compares: the wire string of a feed is never empty.
   *
   * @type {string | null}
   */
  let loadedFeed = null;

  /* THE SUBSCRIPTION. `syncStore` reads `feeds.active` and the shared
     generation, so a feed change re-reads and any Refresh — this page's, or the
     five-second poll `/ingest` holds while a pull runs — lands here too. The
     staleness this comment block describes is now impossible by construction
     rather than by care: the shared reading CLEARS its value on a feed change
     before the request leaves, and `rows` above refuses to hand back anything
     the reading did not stamp with this feed. */
  $effect(() => syncStore(feeds.active));

  $effect(() => {
    const f = feeds.active;
    if (!f || loadedFeed === f) return;
    loadedFeed = f;
    // A FUNCTION DECLARATION, CALLED FROM ABOVE ITSELF ON PURPOSE. The state
    // it clears is declared further down beside the code that owns it, and a
    // `const` arrow there would be a temporal-dead-zone reference from here —
    // the exact thing the FORMATTING block above says was already found and
    // fixed once on this file. A `function` hoists whole, so this call is
    // valid wherever it appears in the script.
    forgetPreviousFeed();
  });

  // WHERE ELSE THE DATA IS. An empty page that only says "nothing here" makes
  // the operator hunt. This names the feeds that have rows in this store — a
  // fact about the store, and a place to go. It renders no other feed's
  // numbers: one feed is selected and only that one's data is shown.
  //
  // IT IS DROPPED ON A FEED CHANGE, in `forgetPreviousFeed`, for the same
  // reason the counts are. The list excludes whichever feed was active WHEN IT
  // RESOLVED, so carrying it across a switch renders "Rows exist under Groww"
  // while Groww is the selected feed and the panel above it says Groww holds
  // nothing — a live control offering a move the page has already made. It is
  // a small stale value, but it is one produced by a query nobody re-ran.
  //
  // THE SECOND READ OF `/store.json` ON THIS PAGE IS GONE. This was an
  // independent N-request fold answering a question `loadFeeds` had ALREADY
  // asked and folded at boot — "which feed holds anything at all" — on a second
  // clock, with no stamp, and re-fired every time the condition below held.
  // `$lib/store.svelte.js` folds it once, keyed by wire, stamped with the feed
  // list it answered for; `surveyStores` is a no-op when that answer is already
  // in hand for this generation, so the common case costs nothing at all.
  $effect(() => {
    if (loading || error || rows.length || !feeds.all.length) return;
    void nonce; /* a Refresh re-asks it — the store may have gained a feed */
    surveyStores(feeds.all);
  });

  // DERIVED, SO IT CANNOT SURVIVE THE QUESTION THAT PRODUCED IT. The old list
  // excluded whichever feed was active WHEN IT RESOLVED, so carrying it across
  // a switch rendered "Rows exist under Groww" while Groww was selected — a
  // live control offering a move the page had already made. Filtering against
  // the CURRENT selection at read time makes that shape unrepresentable, which
  // is why `forgetPreviousFeed` no longer has to remember to drop it.
  const elsewhere = $derived(
    feeds.all
      .map((f) => ({ feed: f, any: survey.byFeed.get(f.wire)?.any === true }))
      .filter((x) => x.any && x.feed.wire !== feeds.active)
  );

  /* ======================================================================
     THE TRADING CALENDAR, DERIVED FROM THE STORE ITSELF
     ====================================================================== */

  /**
   * How many bars a full month holds is not a constant and cannot be
   * calculated: it is 375 x the number of sessions NSE actually held, and
   * that depends on weekends, gazetted holidays, muhurat sessions and the
   * occasional unscheduled close. This repository does not invent an exchange
   * fact (`CLAUDE.md` section 3 rule 1), so the session count is OBSERVED:
   * for each month, the fullest instrument in the store is the month.
   *
   * O(n) once over the rows, then O(1) per lookup — a Map probe, not a scan,
   * so the 93,776-row census projected in `docs/06-limits.md` section 34
   * costs one pass and not one pass per row.
   *
   * IT IS BUILT FROM THE WHOLE STORE, NEVER FROM THE FILTERED VIEW. The
   * denominator is a fact about what NSE traded; narrowing the table to one
   * kind must not quietly lower the bar the remaining rows are judged against.
   */
  const monthFull = $derived.by(() => {
    // KEYED ON (MONTH, RUNG), NOT ON MONTH.
    //
    // "The fullest instrument in this month" is only a denominator among rows
    // of the SAME bar length. Keyed on the month alone, one 1min row (~8,250
    // bars) beside a 1day row (19) judges every daily row ~8,231 short — 0.23%
    // complete for a month that is actually full. Correct today only because
    // the store happens to hold one rung; that is not a property to rely on.
    return denominators(rows).fullest;
  });

  /**
   * HOW MANY ROWS STAND BEHIND EACH DENOMINATOR — the fact that decides
   * whether "0 short" is a measurement or a tautology.
   *
   * `monthFull` is "the fullest instrument in this (month, rung)". Where that
   * group holds ONE row, the row IS the denominator: it is short of itself by
   * zero, whatever it holds. The store holding `NSE-INDEX-NIFTY 2026-08 1min =
   * 1 bar` and nothing else for that month reported, on the one page built to
   * find holes: `Complete 1 of 1`, `Bars missing 0`, `Coverage 100.00%`, a
   * green `full` chip and a filled meter — over a single bar. The same
   * arithmetic reports a whole store of one bar as complete.
   *
   * A month held at a rung by one instrument is therefore UNVERIFIED, not full:
   * nothing in the store contradicts it and nothing corroborates it either, and
   * `CLAUDE.md` §3 rule 6 says which of those two to print. The same-month
   * many-instruments case was fixed by the D-0089-era rewrite; this is the
   * few-rows case it left behind.
   */
  const monthSupport = $derived(denominators(rows).support);

  /**
   * The denominator key for one row: its month AND its rung.
   *
   * @param {StoreRow} r
   */
  const fullestKey = (r) => denomKey(r.month, r.timeframe);

  /**
   * Whether this row is its own denominator — one row at this (month, rung),
   * so `short === 0` says nothing about the month.
   *
   * @param {StoreRow} r
   */
  const soleDenom = (r) => isSole(monthSupport, r);

  /** What the page says instead of a verdict it cannot support. */
  const SOLE_WHY =
    'This is the only instrument-month stored at this rung for this month, so the denominator is this row itself. ' +
    'Being short of itself by zero is a tautology, not a measurement: nothing here says the month is complete, and nothing says it is not. ' +
    'Pull a second instrument for this month, or read the Days column beside it.';

  /**
   * Bars this instrument-month is short of the fullest one in the same month.
   *
   * # What this used to be, and why it was worse than useless
   *
   * `ceil(rows / 375) * 375 - rows`. That is `(-rows) mod 375`, so it is in
   * [0, 374] for EVERY row, whatever the month really held. It answered "is
   * this row a whole multiple of 375", never "does this row hold the month" —
   * and then the answer was labelled green "full" and counted in "Complete".
   *
   * Measured on the live store, both directions of the error:
   *
   * - `NSE-CASH-SHRIPISTON 2020-07` holds 147 bars against a 8,625-bar month.
   *   It is short 8,478 — 22.6 whole trading days. The old expression said
   *   228, understating the hole by 37x, and the row rendered as a minor gap.
   * - `NSE-INDEX-NIFTY 2020-03` holds 7,774 bars and IS the fullest instrument
   *   in its month — March 2020 held 20.73 sessions, because 2020-03-20 was a
   *   short day. It is complete. The old expression said it was short 101,
   *   inventing a hole in the one row that defines the month.
   *
   * A monitoring page whose entire stated job is "where are the holes"
   * reporting no hole — or a fictional one — is the fallback that hides a
   * failure `CLAUDE.md` section 4 bans, on the one surface built to prevent it.
   *
   * DO NOT "SIMPLIFY" THIS BACK INTO ARITHMETIC ON `rows` ALONE. There is no
   * expression over a single row that can know how many sessions a month had.
   *
   * @param {StoreRow} r
   */
  function shortBy(r) {
    return Math.max(0, (monthFull.get(fullestKey(r)) ?? r.rows) - r.rows);
  }

  /**
   * Whole sessions held, from the bar count AT THIS ROW'S RUNG.
   *
   * Fractional means a partial day. `null` when the rung has no recorded
   * bars-per-session — the caller draws a dash, never a figure computed
   * against a denominator nobody chose.
   *
   * @param {StoreRow} r
   * @returns {number | null}
   */
  function sessions(r) {
    const per = barsPerSession(r.timeframe);
    return per === null ? null : r.rows / per;
  }

  /* ======================================================================
     THE CONTRACT TAIL — the ONE place a stored option names its strike
     ----------------------------------------------------------------------
     `/store.json` sends ten fields per row and NOT ONE OF THEM IS A STRIKE, a
     side, an expiry or a price: `instrument`, `month`, `timeframe`, `rows`,
     `first_ts`, `last_ts`, `chg_bps`, `chg_why`, `prev_chg_bps`,
     `prev_chg_why`. That list is the row writer in `crates/api/src/server.rs`
     and the table in the doc comment above `store_json`, and it is exhaustive.

     So the only place a strike can be read from is the instrument NAME, and
     that is readable only because this repository writes the name in exactly
     one place: `Display for InstrumentKey` in `crates/core/src/instrument.rs`
     ends an option `-{strike.raw()}-{CE|PE}` — the strike in PAISA, then the
     two-letter side every Indian vendor uses.

     THE PARSE IS A SUFFIX TEST AND IT REFUSES EVERYTHING ELSE. It is not a
     guess at a spelling: it recognises exactly the tail this repository emits
     and answers `null` for every other name — which today is every name in the
     store, because indices, equities and futures carry no strike at all. A
     name that is an option under some other spelling is reported as NO
     CONTRACT rather than as a strike this page decoded by pattern-matching.
     `CLAUDE.md` §3 rule 1: no invention.
     ====================================================================== */
  const OPTION_TAIL = /-(\d+)-(CE|PE)$/;

  /**
   * The strike in paisa and the side one census instrument name carries, or
   * `null` when the name does not end the way this repository ends an option's.
   *
   * @param {string} name
   * @returns {{ strike: number, side: 'CE' | 'PE' } | null}
   */
  function contractOf(name) {
    const m = name.match(OPTION_TAIL);
    return m === null
      ? null
      : { strike: Number(m[1]), side: /** @type {'CE' | 'PE'} */ (m[2]) };
  }

  /* ---- THE SAME TAIL, ONE FIELD EARLIER — the expiry -------------------
     SAME SOURCE AND SAME RULE AS THE STRIKE ABOVE IT. `Display for
     InstrumentKey` in `crates/core/src/instrument.rs` writes exactly two
     contract shapes and no others:

         Kind::Future  ->  …-{expiry}-FUT
         Kind::Option  ->  …-{expiry}-{strike.raw()}-{CE|PE}

     and `Display for Expiry`, in the same file, renders that expiry
     `{:04}-{:02}-{:02}` — an ISO calendar day, fixed width, zero padded. The
     two patterns below are those two lines and nothing else.

     IT IS A SUFFIX TEST AND IT REFUSES EVERYTHING ELSE, exactly as
     `OPTION_TAIL` does. `/store.json` sends ten fields per row and not one of
     them is an expiry — the exhaustive list is in the block above
     `OPTION_TAIL` — so the NAME is the only source there is, and a name that
     does not end one of these two ways answers `null`, meaning NO CONTRACT.
     It never answers an expiry decoded out of a spelling this page guessed at.
     `CLAUDE.md` §3 rule 1: no invention.

     THE OPTION PATTERN IS TRIED FIRST AND THAT ORDER MATTERS. `-FUT` cannot
     end an option's name, so the two are disjoint and either order returns the
     same answer today; the option is tried first anyway so that the day a
     third shape is added, the more specific tail is already the one that
     wins. */
  const FUTURE_TAIL = /-(\d{4}-\d{2}-\d{2})-FUT$/;
  const OPTION_EXPIRY_TAIL = /-(\d{4}-\d{2}-\d{2})-\d+-(?:CE|PE)$/;

  /**
   * The expiry one census instrument name carries, as the RAW ISO day, or
   * `null` when the name is not a contract this repository wrote.
   *
   * The returned string is a key and stays one: it is the `{#each}` key, the
   * Set member, the `===` in the gate and the sort key. `dayLabel` is applied
   * at the render site and nowhere near any of those.
   *
   * @param {string} name
   * @returns {string | null}
   */
  function expiryOf(name) {
    const m = name.match(OPTION_EXPIRY_TAIL) ?? name.match(FUTURE_TAIL);
    return m === null ? null : m[1];
  }

  /**
   * A strike in paisa, rendered as rupees, EXACTLY.
   *
   * The same integer arithmetic `bpsText` uses and for the same reason:
   * `CLAUDE.md` §7 bans the float for money, and `p / 100` is a float division
   * of a value that has an exact decimal expansion. The remainder is taken with
   * `%` — an integer operation — and the quotient is a multiple of 100 divided
   * by 100, which IEEE returns exactly inside 2^53.
   *
   * The paisa are dropped only when they are ZERO, which is the ordinary case
   * on an exchange-listed strike. A strike that is not a whole rupee prints its
   * two decimals rather than being rounded into one that was never listed.
   *
   * @param {number} p
   */
  function strikeText(p) {
    const frac = p % 100;
    const whole = (p - frac) / 100;
    return frac === 0 ? fmt(whole) : `${fmt(whole)}.${String(frac).padStart(2, '0')}`;
  }

  /* ======================================================================
     DECORATION — every derived per-row number, computed ONCE
     ----------------------------------------------------------------------
     The comparator runs O(n log n) times. A comparator that called `shortBy`
     would pay two Map probes per comparison, so the short count, the session
     count and the completeness ratio are attached to the row here, in one
     pass, and the sort compares plain numbers.
     ====================================================================== */
  const deco = $derived.by(() =>
    rows.map((r) => {
      const short = shortBy(r);
      // THE CONTRACT IS PARSED ONCE PER ROW PER DATASET, here, beside every
      // other derived per-row number — never inside the chain, the counts, the
      // ladder or the row filter. Those run per paint and per keystroke; this
      // runs when the store is re-read. Same rule `short` and `days` follow.
      const ct = contractOf(r.instrument);
      // THE EXPIRY IS PARSED IN THE SAME PASS AND FOR THE SAME REASON. It is
      // read by the gate, which runs on every keystroke, every scroll frame
      // and every click; parsing it there would put a regex match on the hot
      // path for a value that changes only when the store is re-read.
      const ex = expiryOf(r.instrument);
      const denom = monthFull.get(fullestKey(r)) ?? r.rows;
      // ITS OWN DENOMINATOR, OR A REAL ONE. See `soleDenom` and `SOLE_WHY`.
      const sole = soleDenom(r);
      const cut = r.instrument.lastIndexOf('-');
      const parts = r.instrument.split('-');
      // ONCE PER ROW AT READ TIME, for the reason the line above `expiryOf`
      // gives: this is hoisted off the hot path deliberately.
      const parsed = parseKey(r.instrument);
      // HOISTED, the way `sessions()` above already does it. `barsPerSession`
      // was called TWICE in each of the two expressions below — once to guard
      // and once to divide — so the guard narrowed nothing the checker could
      // follow and the function ran twice per row per render. One call, one
      // value, and the null-check now provably covers the use.
      const per = barsPerSession(r.timeframe);
      return {
        ...r,
        // A SEPARATOR THAT CANNOT OCCUR IN THE DATA, WRITTEN AS AN ESCAPE.
        // This was two literal NUL bytes in the source, which made `grep` and
        // `ripgrep` classify the whole file as binary and return nothing for
        // every pattern — on the file whose completeness bug took an 89-agent
        // sweep to find. The runtime key is byte-identical; the source is now
        // ASCII and searchable.
        key: `${r.instrument}\u0000${r.month}\u0000${r.timeframe}`,
        head: cut > 0 ? r.instrument.slice(0, cut + 1) : '',
        // `sym` IS THE TAIL AND IS NOT THE NAME. On a spot key the two agree;
        // on `NSE-FNO-BANKNIFTY-2026-07-28-4810000-PE` this is `PE`. It stays
        // because the table and the index have always displayed and probed it,
        // and `underlying` beside it is what an INSTRUMENT actually is.
        sym: cut > 0 ? r.instrument.slice(cut + 1) : r.instrument,
        // THE NAME A HUMAN USES, and the key the Instrument rung groups by.
        // Parsed once per row at store-read time, never per keystroke.
        underlying: parsed.underlying ?? r.instrument,
        // WHICH SEGMENT RUNG THIS ROW BELONGS TO — spot, futures or options.
        // `kind` below is the key's own second token and cannot answer it:
        // FNO covers futures and options alike, and only the tail separates
        // them.
        // `expiry`, `strike` and `side` are NOT set here: the row already
        // carries them, parsed from `ct` further down with a fuller account of
        // why each is `null`. Adding a second spelling of the same three fields
        // is how two readers come to disagree about one row -- svelte-check
        // caught it as three duplicate keys, which is the cheap version of that
        // lesson.
        segRung: segmentOf(parsed),
        kind: parts.length > 1 ? parts[1] : '—',
        short,
        days: sessions(r),
        lost: per === null ? null : short / per,
        // NULL, NOT 100.00%. A ratio against itself is 1 for every row that
        // ever existed; `pctText` draws the dash and every meter beside it is
        // suppressed rather than filled — the rule the Coverage tile already
        // keeps: "a bar is a length, and a length is a claim".
        pct: sole ? null : denom > 0 ? r.rows / denom : 1,
        sole,
        denom,
        // THE WIRE IS TOTAL, AND SO IS THIS. `chg_bps` is an integer or it is
        // null, and `chg_why` is the reason for the null — exactly one of the
        // pair on every row. `Number.isInteger` rather than a truthiness test,
        // because 0 basis points is a real answer: a month that closed where it
        // opened, which `!r.chg_bps` would turn into an unknown.
        // THE STRIKE IN PAISA AND THE SIDE, or `null` on both when the name
        // carries no contract. `null` is NOT "strike zero": a zero strike is a
        // price and this is the absence of one, and every reader below tests
        // for `null` rather than for falsity so a hypothetical 0-paisa strike
        // could never be silently dropped the way `!r.strike` would drop it.
        strike: ct === null ? null : ct.strike,
        side: ct === null ? null : ct.side,
        // THE RAW ISO DAY, OR `null` FOR "THIS ROW IS NOT A CONTRACT". `null`
        // is not "expires never" and it is not the empty string: an index and
        // an equity HAVE no expiry, and the gate below reads this field for
        // exactly that distinction — a row with `null` is passed through
        // untouched, a row with a day must match the chosen one.
        expiry: ex,
        chg: Number.isInteger(r.chg_bps) ? r.chg_bps : null,
        chgWhy: r.chg_why ?? null,
        prevChg: Number.isInteger(r.prev_chg_bps) ? r.prev_chg_bps : null,
        prevChgWhy: r.prev_chg_why ?? null,
        // MICROSECONDS ON THE WIRE, MILLISECONDS AT THE CALL SITE. The server
        // sends `first_ts` / `last_ts` unconverted and says so; `$lib/dates.js`
        // takes epoch MILLISECONDS and refuses to sniff the unit, because this
        // repository carries bar times in seconds and response times in
        // milliseconds and a formatter that guessed would render 1970 for half
        // its callers. The `/1000` happens HERE, where the unit is known.
        //
        // `> 0` AND NOT `Number.isFinite`. A zero stamp is not an instant this
        // store ever recorded — it would render `01 Jan 1970, 05:30`, a date
        // nobody took, in a column whose whole job is to say which days are
        // actually held. `null` reaches the renderer and it draws the absence.
        firstAt: typeof r.first_ts === 'number' && r.first_ts > 0 ? r.first_ts / 1000 : null,
        lastAt: typeof r.last_ts === 'number' && r.last_ts > 0 ? r.last_ts / 1000 : null,
        // THREE STATES, NOT TWO. "not full" hides the difference between a
        // late start on one day and two whole trading days that never landed,
        // and the second is the one that needs a re-pull.
        // THE THRESHOLD IS ONE SESSION AT THIS ROW'S RUNG. Against a fixed
        // 375 a daily row could never reach 'gap' — every shortfall is under
        // 375 bars — so two missing trading days read as 'near', the state
        // that means "a late start on one day".
        state:
          sole
            ? 'sole'
            : short === 0
            ? 'full'
            // NO 375 FALLBACK. A rung with no recorded session size has no
              // threshold either, and `?? BARS_PER_SESSION` quietly restored the
              // minute yardstick this function exists to remove. An unknown rung
              // is 'gap' — the state that asks for a re-pull — because a
              // shortfall nobody can size is not a near miss.
              : per === null
                ? 'gap'
                : short < per
                  ? 'near'
                  : 'gap'
      };
    })
  );

  /**
   * ONE DECORATED CENSUS ROW — the shape everything below this line walks.
   *
   * DERIVED FROM `deco` RATHER THAN WRITTEN OUT, and that is the whole point.
   * The decoration above attaches nineteen fields to the ten `/store.json`
   * sends, and a hand-written copy of that list is a second declaration of one
   * shape: the day `lost` or `expiry` changes its meaning, the copy still says
   * the old thing and every reader downstream believes the copy. Reading the
   * element type off the expression that produces it cannot drift, because
   * there is nothing to keep in step.
   *
   * @typedef {(typeof deco)[number]} DecoRow
   */

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
   * THE FEED, WRITTEN FROM THE PANEL'S SCOPE RUNG UNDER THE TOP BAR'S OWN RULE.
   *
   * `feeds.active` is the one selection the whole app reads and this page's own
   * fetch effect already re-reads `/store.json` from it, so this needs no reload
   * of its own — it is the same write the empty-store panel's "Rows exist under"
   * buttons have always made. A feed the list marks not ready is REFUSED rather
   * than hidden: the row is drawn, disabled, and its `title` quotes the server's
   * reason, which is the same test the top bar applies rather than a second
   * opinion about the same feed.
   */

  /**
   * The eight names, in the owner's order, with NO COUNT IN ANY LABEL.
   *
   * `token` is the three-state field the whole rung turns on:
   *   · `null` — no membership source exists. The row is shown and disabled.
   *   · `''`   — Everything: NO JOIN AT ALL, which is not the same as a join
   *              that matches everything. A stored instrument this feed's
   *              master no longer lists — BSE history the charter keeps, a
   *              symbol delisted since the pull — is absent from
   *              `/instruments.json` and would silently vanish under any
   *              membership test. Under Everything it cannot, because nothing
   *              is tested.
   *   · a word — one token out of `universe_label`, matched against the `+`
   *              joined set on the catalogue row.
   */
  /* THE FLOOR IS NAMED, not indexed off the end of the list. `chosenUniverse`
     falls back to it for an unrecognised slug, and a fallback expressed as
     `UNIVERSES.at(-1)` is one that can be `undefined` — every reader would then
     need a guard, and a missing guard on the row that decides whether the
     table is filtered at all is a blank page. A concrete object cannot be
     absent. */
  const EVERYTHING = { key: '', label: 'Everything', token: '' };

  const UNIVERSES = [
    { key: 'n50', label: 'NIFTY 50', token: 'n50' },
    { key: 'n100', label: 'NIFTY 100', token: 'n100' },
    { key: 'n200', label: 'NIFTY 200', token: 'n200' },
    { key: 'n500', label: 'NIFTY 500', token: 'n500' },
    { key: 'ntm', label: 'NIFTY Total Market', token: 'ntm' },
    { key: 'fno', label: 'F&O underlyings', token: 'fno' },
    { key: 'index', label: 'NSE indices', token: 'index' },
    EVERYTHING
  ];

  /**
   * THE TOKENS ONE CATALOGUE ROW CARRIES, out of the field that carries them.
   *
   * `/instruments.json` sends TWO membership fields, and they are not the same
   * fact. `crates/api/src/server.rs:813` emits both on every row:
   *
   *   · `universe`  — a `+`-joined STRING, deliberately frozen at the first
   *                   `LEGACY_UNIVERSE_TOKENS` = 3 entries of `UNIVERSE_TOKENS`
   *                   (`index`, `fno`, `ntm`). It is a compatibility field and
   *                   it will never name a NIFTY tier.
   *   · `universes` — the full ARRAY, `["index","fno","ntm","n500","n200",
   *                   "n100","n50"]` in that declared order. D-0089 / D-0090.
   *
   * So the array is read when the row has one and the legacy string is split
   * when it does not. THAT IS NOT A FALLBACK THAT HIDES A FAILURE: the legacy
   * string is a complete, correct answer for the three tokens it names, and the
   * four tokens it cannot name are REFUSED BY NAME in `tokensHere` below rather
   * than guessed at. Nothing here ever synthesises a tier.
   *
   * The consequence worth stating: this page needs no edit at all on the day
   * the API binary carrying `universes` is running. The four NIFTY rows light
   * up because the field arrived, which is the only evidence that would justify
   * lighting them.
   *
   * @param {any} row
   * @returns {string[]}
   */
  const tokensOf = (row) =>
    Array.isArray(row?.universes)
      ? row.universes.map(String)
      : String(row?.universe ?? '')
          .split('+')
          .filter(Boolean);

  /**
   * The census spells an instrument `EXCHANGE-SEGMENT-SYMBOL`, and that is the
   * ONLY key this join may use.
   *
   * `/instruments.json` also sends a `key`, and it is `EXCHANGE-SYMBOL` — one
   * segment short. Joining on it matches nothing at all, and a join that
   * matches nothing produces a universe holding zero instruments rather than
   * an error: the rung would look like it worked and report "0 of 750 held"
   * over a full store. BUILT FROM THE THREE PARTS, never parsed out of a
   * string, because a symbol may legally contain `-` (`NIFTY-50`, `M&M`).
   * `/markets` proves the same key the same way.
   *
   * @param {MasterRow} row a row of `/instruments.json`, NEVER a census row
   */
  const censusKeyOf = (row) => `${row.exchange}-${row.segment}-${row.symbol}`;

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
   * The four tokens the LEGACY `universe` string is structurally unable to
   * name, whatever the data holds. `LEGACY_UNIVERSE_TOKENS = 3` in
   * `crates/api/src/server.rs` cuts `UNIVERSE_TOKENS` after `index`, `fno` and
   * `ntm`, so a row without a `universes` array cannot report these four even
   * when the instrument is in all of them.
   */
  const LEGACY_BLIND = new Set(['n50', 'n100', 'n200', 'n500']);

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
   * The chosen universe's members as census keys, or `null` for NO JOIN.
   *
   * `null` is returned both for Everything and for every state the refusal
   * above names, so there is ONE expression deciding whether rows are narrowed
   * and one sentence explaining it. Two would drift.
   */
  const universeKeys = $derived.by(() => {
    const token = chosenUniverse.token;
    if (!token || tierState(chosenUniverse) !== 'ready') return null;
    const s = new Set();
    for (const r of masterRows()) {
      if (tokensOf(r).includes(token)) s.add(censusKeyOf(r));
    }
    return s;
  });

  const universed = $derived(
    universeKeys === null ? deco : deco.filter((r) => universeKeys.has(r.instrument))
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
    for (const r of deco) stored.add(r.instrument);
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
    for (const r of deco) {
      if (listed.has(r.instrument)) continue;
      months += 1;
      names.add(r.instrument);
    }
    return { months, instruments: names.size };
  });

  /* ======================================================================
     THE MONTH WINDOW — a range over the row's own unit
     ----------------------------------------------------------------------
     A ROW HERE IS AN INSTRUMENT-MONTH, so the honest range control is a month
     range and not a day picker: a from-day and a to-day would have to be
     rounded to months to be applied, and a control whose value is silently
     coarsened is a control that lies about what it did.

     THERE IS NO `<input type="date">` ON THIS PAGE AND THERE MUST NOT BE. It
     renders in the OS locale — `02/09/2024`, which is two different days
     depending on the reader — and it cannot be restyled. The endpoints are
     chosen from the months the store ACTUALLY HOLDS, through the same one
     dropdown every other control on this page uses, so the label is
     `Sep 2024` (never `Sept`, per `$lib/dates.js`), the value stays raw
     `YYYY-MM`, and the control cannot offer a month that would match nothing.
     ====================================================================== */

  /* ======================================================================
     THE PREFIX INDEX — one Map probe per keystroke
     ----------------------------------------------------------------------
     Same shape as `web/typeahead.js`, for the same reason: the universe is
     bounded, so index it once and probe it forever. Every 1..4-character
     prefix of the full instrument, of its symbol, and of its month maps to the
     rows carrying it. Typing up to four characters is ONE Map probe; beyond
     four it is a filter over one bucket, which is bounded by the largest
     4-prefix bucket and never by the store.
     ====================================================================== */
  const MAX_PREFIX = 4;

  /* THE BUCKET KEY AND THE PROBE KEY ARE ONE FORM, AND THEY WERE NOT.
     `typed` below is `filter.trim().toUpperCase()`, and these buckets were cut
     from the token EXACTLY AS THE STORE SPELLS IT. Two spellings of one key is
     the silent failure this whole file is written against: the day a store
     writes one instrument, one symbol or one month key in anything but upper
     case, its bucket is filed under a prefix no probe can ever ask for — the
     row is in `windowed`, the table below counts it, and the text box simply
     never finds it. Nothing errors and nothing looks wrong. Normalised on BOTH
     sides here, once per token rather than once per prefix, so the index and
     the box cannot spell the same key two ways.

     `it.sym` is the tail the row DISPLAYS, and it stays in the index on
     purpose: it is a substring of `it.instrument`, so typing `NIFTY` at
     `NSE-INDEX-NIFTY` is a real question. What it is not allowed to be is the
     key anything is COMPARED by — `picked` below resolves back to the store's
     own `instrument` spelling, never to what was typed. */
  const index = $derived.by(() => {
    /* THE BUCKET TYPE IS DECLARED, AND IT IS THE ONE ANNOTATION THE WHOLE
       DOWNSTREAM CHAIN HANGS ON. A bare `new Map()` is `Map<any, any>`, so
       `index.get(typed)` is `any`, so `textMatched` is `any`, and `segmented`,
       `timeframed`, `windowed`, `scoped`, `matched` and every reducer and
       comparator over them are `any` in turn — thirty-odd
       `Parameter 'r' implicitly has an 'any' type` reports, every one of them
       raised at a callback that is not where the shape was lost. It is lost
       here, and it is answered here. */
    /** @type {Map<string, DecoRow[]>} */
    const by = new Map();
    for (const it of universed) {
      /** @type {Set<string>} */
      const seen = new Set();
      // `it.underlying` IS IN HERE AND IT IS THE ONE THAT MAKES A CONTRACT
      // REACHABLE BY NAME. Without it the only tokens a contract row offers
      // are its full key -- which starts `NSE-` -- and `it.sym`, which for
      // an option is `CE` or `PE`. Typing BANKNIFTY found the spot series
      // and none of its twenty-six contracts.
      for (const raw of [it.instrument, it.sym, it.underlying, it.month]) {
        const token = raw.toUpperCase();
        for (let n = 1; n <= Math.min(MAX_PREFIX, token.length); n += 1) {
          const k = token.slice(0, n);
          if (seen.has(k)) continue;
          seen.add(k);
          let bucket = by.get(k);
          if (!bucket) by.set(k, (bucket = []));
          bucket.push(it);
        }
      }
    }
    return by;
  });

  const typed = $derived(filter.trim().toUpperCase());

  const textMatched = $derived.by(() => {
    /* `universed`, NOT `deco`. The two coarser rungs above are already applied
       to the index this probes, so an empty text box has to fall through to
       the same set the index was built from — falling back to the whole store
       would make typing WIDEN the selection. */
    if (!typed) return universed;
    if (typed.length <= MAX_PREFIX) return index.get(typed) ?? [];
    const seed = index.get(typed.slice(0, MAX_PREFIX)) ?? [];
    /* THE SAME NORMALISATION AS THE INDEX ABOVE, for the same reason: past four
       characters this is the probe, and a bucket cut one way cannot be filtered
       the other. */
    return seed.filter(
      (r) =>
        r.instrument.toUpperCase().startsWith(typed) ||
        r.sym.toUpperCase().startsWith(typed) ||
        r.underlying.toUpperCase().startsWith(typed) ||
        r.month.toUpperCase().startsWith(typed)
    );
  });

  /* ---- THE INSTRUMENT RUNG, WHICH IS THE TEXT BOX -----------------------
     ONE STATE, TWO ENTRY POINTS. The page has had an instrument filter since
     it was written, and it answers this rung's question faster than a list
     can: a Map probe against a typed prefix. A second, independent "chosen
     instrument" beside it would be two answers to one question, and the table
     would look correct while returning nothing the moment they disagreed.

     So the picker does not own a selection at all. It WRITES the text box, and
     it shows a row as ticked exactly when the text box currently holds that
     row's key. Typing anything else unticks it with no bookkeeping, because
     there is nothing to keep. The Picker is an enumeration of what the coarser
     rungs left; the text box is the state.
     ====================================================================== */
  /* FOLDED BY UNDERLYING, NOT BY STORE KEY, and that one word is the fix.
     Measured on the live store: 27 keys, ONE underlying. Folding by
     `it.instrument` offered BANKNIFTY twenty-seven times -- once as the spot
     series and twenty-six times as its option contracts -- so the rung that
     asks "which instrument" was answering "which contract", and the Segment
     rung beside it was left asking a question that had already been answered.
     `key` stays the field name because everything downstream reads it; what
     changed is what a key IS. */
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
    for (const it of textMatched) c.set(it.kind, (c.get(it.kind) ?? 0) + 1);
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
   * `universeKeys` is the membership join, both already here and both already
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
   * `universeKeys === null` means NO JOIN — Everything, or a universe whose
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
    if (universeKeys === null) return instrumentRows.map((r) => decorate(r.key, r));
    /* THE UNIVERSE'S KEYS ARE STORE KEYS AND THIS RUNG NOW SPEAKS UNDERLYINGS,
       so they are converted and DEDUPLICATED on the way in. Without the dedupe
       a universe naming both a spot key and its contracts would offer the same
       name more than once, which is the defect this whole change removes. */
    const out = [];
    const seen = new Set();
    for (const k of universeKeys) {
      const u = parseKey(k).underlying ?? k;
      if (seen.has(u)) continue;
      seen.add(u);
      out.push(decorate(u, held.get(u)));
    }
    return out.sort((a, b) => txt(a.sym, b.sym) || txt(a.key, b.key));
  });

  /**
   * THE THREE SEGMENTS THE STORE KEYS ON, ALWAYS ALL THREE.
   *
   * `kinds` below counts what is PRESENT, which is the right input for a total
   * and the wrong one for a menu: with one segment stored the rung offered one
   * row, so "does this store hold any CASH?" was a question the control could
   * not be asked. The comment above the rung already states the intended shape —
   * "/ingest draws Segments unconditionally — three rows, two of them refusing
   * with their reason on their face" — and this is the half of it that was
   * missing.
   *
   * The names are the store's own second field, not the mockup's
   * spot/futures/options taxonomy: relabelling would rename three sets into
   * three other sets that do not have the same members.
   */
  const SEGMENT_KEYS = ['INDEX', 'CASH', 'FNO'];

  /**
   * /ingest's THREE SEGMENTS, ON THIS PAGE'S OWN ROWS.
   *
   * The rung offered INDEX / CASH / FNO — the store's own second field — and
   * /ingest offers Spot / Expired futures / Expired options. Same question, two
   * vocabularies, so a reader moving between the pages had to translate.
   *
   * THE FILE ARGUED AGAINST THIS and the objection was real: "relabelling would
   * rename three sets into three other sets that do not have the same members".
   * It is answered rather than ignored — the sets are not relabelled, they are
   * RE-DERIVED from a field every row already carries. `expiry` is what splits
   * them:
   *
   *   Spot            a continuous series — `expiry === null`. INDEX and CASH
   *                   both land here, which is correct: neither settles.
   *   Expired futures a settled contract, `-FUT`.
   *   Expired options a settled contract, `-CE` or `-PE`.
   *
   * So Spot is not "INDEX renamed" — it is every row with no expiry, whatever
   * the store keyed it under, and the membership is computed rather than
   * asserted. The operator's instruction, and the objection's terms are met.
   *
   * The notes are /ingest's own words, including the 503, because the reason an
   * expired contract is absent is the same reason on both pages.
   */
  const SEG_VIEW = [
    { key: 'spot', name: 'Spot', note: 'continuous series — no expiry' },
    {
      key: 'futures',
      name: 'Expired futures',
      note: 'contracts that have already settled · 503 — no transport in this build'
    },
    {
      key: 'options',
      name: 'Expired options',
      note: 'contracts that have already settled · 503 — no transport in this build'
    }
  ];

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
    for (const r of textMatched) {
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
  const segmented = $derived(kind ? textMatched.filter((r) => segOf(r) === kind) : textMatched);

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
    timeframe ? segmented.filter((r) => r.timeframe === timeframe) : universed
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
    if (deco.length === 0)
      return `nothing has been read for ${feedDisplay(feeds.active)} yet, so no row has named a rung — this is an unread store, not a store without bar lengths`;
    return `${feedDisplay(feeds.active)} holds ${fmt(deco.length)} instrument-month(s) and none of them is in ${chosenUniverse.label}, so there is no row here to read a rung off. Widen the universe above and the rungs come back.`;
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
  const scoped = $derived(month ? windowed.filter((r) => r.month === month) : windowed);
  const holed = $derived(scoped.reduce((a, r) => a + (r.short > 0 ? 1 : 0), 0));

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
  const matchedPreContract = $derived(holesOnly ? scoped.filter((r) => r.short > 0) : scoped);

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
    for (const r of deco) if (r.expiry !== null) s.add(r.expiry);
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
    if (deco.length === 0)
      return `nothing has been read for ${feedDisplay(feeds.active)} yet, so no instrument name has been looked at — this is an unread store, not a store without contracts`;
    return `not one of the ${fmt(deco.length)} instrument-month(s) ${feedDisplay(feeds.active)} holds is named the way this repository names a contract. crates/core/src/instrument.rs ends a future “-<YYYY-MM-DD>-FUT” and an option “-<YYYY-MM-DD>-<strike in paisa>-CE|PE”, and nothing in this store ends either way: it holds indices and equities, whose names carry no expiry at all. /store.json sends no expiry field, so an expiry here can only ever be READ OFF A NAME — never decoded from a spelling this page guessed at. THIS RUNG IS NOT LIVE-ONLY AND NEVER WAS: an expired contract on disk would be listed here, because the list is folded off the stored names and nothing in it is compared to today. There is none because nothing in this build can fetch one — crates/api/src/server.rs answers POST /pull/fno with 503 and the reason “expired F&O has no local-archive path and no HTTP transport in this build”, and crates/core/src/vendor.rs declines every FUT/CE/PE master row as Skip::LiveContract. Nothing is being held back, because there is nothing to hold.`;
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
    for (const r of deco) if (r.side !== null) s.add(r.side);
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
    if (deco.length === 0)
      return `nothing has been read for ${feedDisplay(feeds.active)} yet, so no instrument name has been looked at — this is an unread store, not a store without options`;
    return `${feedDisplay(feeds.active)} holds no option. A side is the last two characters of an option's name — crates/core/src/instrument.rs ends one “-CE” or “-PE” — and not one of the ${fmt(deco.length)} instrument-month(s) here ends either way. /store.json sends no side field, so a side can only ever be READ OFF A NAME.`;
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

  /* ======================================================================
     THE CONTRACT RUNG — THE STRIKE CHAIN, AND THE MONEYNESS LADDER OVER IT
     ----------------------------------------------------------------------
     MONEYNESS IS A POSITION ON A LADDER, NOT ONE OF THREE WORDS. A strike is
     in or out of the money BY A DISTANCE: for a call every strike below spot
     is in the money and gets further in as you descend — ITM-1, ITM-2, ITM-3 …
     — and every strike above it gets further out — OTM+1, OTM+2, OTM+3. ATM is
     the single strike nearest spot. Collapsing that to three buckets throws
     away the axis a reader trades on: one word cannot answer for a strike one
     step in and a strike six steps in, which are different instruments.

     THE SIGN IS PART OF THE NAME, and it is what makes the order an order.
     `ITM-10` is DEEPER than `ITM-2` because −10 < −2; compared as text `ITM-10`
     lands before `ITM-2` and `ITM-100` lands between them. `ladderPos` reads
     the offset back off the name as a signed NUMBER, so a two-, three- or
     four-digit offset needs no new rule and no new branch.

     NOTHING HERE CARRIES A DEPTH. There is no `CHAIN = 6`, no cap, no band and
     no bound between the chain and the panel: the rungs are counted off the
     strikes the STORE actually names, so a store holding one strike has one
     rung and says so, and a store holding five hundred has five hundred. A
     rung the current selection holds no bar for stays on the ladder and
     reports `no bars`, which is a finding about the window rather than a
     shorter ladder.

     ONE RENDERER, TWO DATA SOURCES. Both panels are `$lib/Picker.svelte` — the
     same control `/markets` and `/ingest` use and the same one this page
     already uses for Segment and Instrument. Everything that differs between
     strike and moneyness is DATA: what the rows are, what the counts are read
     from, what the button says, what the refusal says. A second lookalike
     control is how two panels start the same and end up two pixels and one
     wording apart, and there is not one here.

     THEY STAY TWO CONTROLS. A rung is a distance and a strike is an absolute
     price, and a reader who wants `52,000` and a reader who wants `ITM-2` are
     asking different questions — one of them survives a move in spot and the
     other does not.
     ====================================================================== */

  /**
   * EVERY STRIKE THE STORE NAMES, ascending, with the sides it holds at it.
   *
   * Built from `deco` — the WHOLE store for this feed — and not from the
   * filtered view, for the same reason `monthFull` is: the chain is what exists
   * on disk, and narrowing the table to one month must not quietly shorten the
   * ladder the remaining rows are placed on. What the narrowing changes is the
   * COUNT beside each rung, which is counted separately and against
   * `matchedPreContract`.
   *
   * One pass over the rows, then O(1) per lookup. Recomputed when the store is
   * re-read, never per keystroke and never per scroll frame.
   */
  const chainAll = $derived.by(() => {
    const by = new Map();
    for (const r of deco) {
      if (r.strike === null) continue;
      let e = by.get(r.strike);
      if (!e) by.set(r.strike, (e = { strike: r.strike, sides: new Set(), months: 0 }));
      e.sides.add(r.side);
      e.months += 1;
    }
    return [...by.values()].sort((a, b) => a.strike - b.strike);
  });

  /** Strike -> its position in the chain. The rung lookup is a probe, not a scan. */
  const chainAt = $derived.by(() => {
    const m = new Map();
    for (let i = 0; i < chainAll.length; i += 1) m.set(chainAll[i].strike, i);
    return m;
  });

  /**
   * THE CHAIN'S OWN GRID, in paisa — counted, never `sym === 'BANKNIFTY' ? 500
   * : 50`. The smallest positive gap between two adjacent listed strikes is a
   * fact about what is on disk; a table of step sizes per symbol would be this
   * page asserting an exchange's contract specification, which
   * `docs/00-charter.md` sources for nothing.
   *
   * `null` under two strikes: one strike has no gap, and a step invented for it
   * would be a number nobody measured.
   */
  const chainStep = $derived.by(() => {
    let step = null;
    for (let i = 1; i < chainAll.length; i += 1) {
      const gap = chainAll[i].strike - chainAll[i - 1].strike;
      if (gap > 0 && (step === null || gap < step)) step = gap;
    }
    return step;
  });

  /**
   * SPOT, IN PAISA — the price the ladder is drawn around.
   *
   * IT IS `null`, ON EVERY STORE THIS PAGE CAN READ, AND THAT IS A FINDING
   * RATHER THAN A STUB. `/store.json` carries ten fields and none is a price:
   * `chg_bps` is a RATIO — this month's first close over its last, in integer
   * basis points — and a ratio has no scale, so no absolute price can be
   * recovered from it. `/instruments.json`, the other endpoint this page holds,
   * carries `symbol`, `key`, `kind`, `exchange`, `segment`, `universe`,
   * `universes`, `bars` and `href`, and none of those is a price either.
   * `/bars.json` does carry prices — and reading one would be a request per
   * contract and about 81 bytes per bar (see the note panel's last line, where
   * that number is measured) to recover a single figure, which is the exact
   * trade D-0067 refused when it put the two closes in the census instead.
   *
   * A DERIVED AND NOT A LITERAL `null`, on purpose. The day a price reaches
   * this page the anchor is a computation over the rows on hand, not an edit to
   * a constant — and `moneyRefusal` beside it has to be able to say "not
   * measured" rather than "impossible".
   *
   * @type {number | null}
   */
  const spotPaisa = $derived.by(() => {
    /* Nothing to read it from. Every field of every endpoint this page calls
       is enumerated above; when one of them becomes a price, it is read here
       and the ladder below lights up with no other change to this file. */
    return null;
  });

  /**
   * WHICH CHAIN ENTRY IS THE MONEY — the index of the strike nearest spot, or
   * `null` when spot is not on hand.
   *
   * Nearest by absolute distance, and the FIRST of two equidistant strikes, so
   * the answer is the same on every read of the same chain. A ladder whose
   * centre moved between two paints would renumber every rung under the
   * reader's selection.
   */
  const atmIndex = $derived.by(() => {
    if (spotPaisa === null || chainAll.length === 0) return null;
    let at = 0;
    for (let i = 1; i < chainAll.length; i += 1) {
      if (Math.abs(chainAll[i].strike - spotPaisa) < Math.abs(chainAll[at].strike - spotPaisa)) {
        at = i;
      }
    }
    return at;
  });

  /**
   * The rung a chain offset sits on FOR ONE SIDE, and the sign is in the name.
   *
   * A call is in the money BELOW spot; a put is in the money ABOVE it. So one
   * strike is `ITM-2` for the CE and `OTM+2` for the PE, and the rung is a
   * function of both — which is exactly why moneyness cannot be a property of
   * the strike control alone, and why these stay two controls.
   *
   * @param {number} k offset from the money, negative below it
   * @param {'CE' | 'PE'} side
   */
  const rungOf = (k, side) =>
    k === 0 ? 'ATM' : ((side === 'CE') === (k < 0) ? 'ITM-' : 'OTM+') + Math.abs(k);

  /* READ BACK OFF THE NAME, so the string on screen, the key in the selection,
     the key in the filter box and the key in the sort are ONE string and cannot
     drift into two. */
  const RUNG = /^(ITM|ATM|OTM)([+-]\d+)?$/;
  /** @param {string} m */
  const famOf = (m) => (m.match(RUNG) ?? [null, m])[1];
  /** The signed offset. `ATM` is 0; `ITM-10` is −10, which is DEEPER than −2. */
  /** @param {string} m */
  const ladderPos = (m) => {
    const p = m.match(RUNG);
    return p && p[2] ? Number(p[2]) : 0;
  };

  /**
   * EVERY RUNG THE CHAIN REACHES, in ladder order: deepest ITM … ITM-1, ATM,
   * OTM+1 … deepest OTM.
   *
   * The union across the sides each strike actually holds, not one rung per
   * strike. A strike is ITM for one side and OTM for the other, so on a chain
   * that runs out of room at one end the union is the wider of the two sides —
   * stating it as the union is true in both cases and "one per strike" is true
   * only on a chain centred exactly on the money.
   *
   * Empty when there is no anchor, which is what disables the control. A rung
   * NAMED without a money to measure from would be this page inventing a
   * distance, and every name on the ladder would be wrong by however far the
   * guess sat from spot.
   */
  const ladderRungs = $derived.by(() => {
    if (atmIndex === null) return [];
    const seen = new Set();
    for (let i = 0; i < chainAll.length; i += 1) {
      for (const side of chainAll[i].sides) seen.add(rungOf(i - atmIndex, side));
    }
    /* SORTED BY SIGNED POSITION, NEVER BY TEXT — see the block header. `txt` is
       the tie-break only, and it can only fire between two names with the same
       offset, which is `ATM` against itself and therefore never. */
    return [...seen].sort((a, b) => ladderPos(a) - ladderPos(b) || txt(a, b));
  });

  /**
   * The rung one stored row sits on, or `null` — no anchor, no contract, or a
   * strike the chain does not list.
   *
   * @param {{ strike: number | null, side: 'CE' | 'PE' | null }} r
   */
  function rungKeyOf(r) {
    if (atmIndex === null || r.strike === null || r.side === null) return null;
    const i = chainAt.get(r.strike);
    return i === undefined ? null : rungOf(i - atmIndex, r.side);
  }

  /* `expiryGated` AND NOT `matchedPreContract`, on both of these. The expiry
     is the rung directly above the strike, so counting a strike over rows the
     gate is holding back would print a bar count the click cannot produce —
     the exact rule the facet block states. With no expiry chosen every strike
     honestly reads `no bars`, because that is what selecting it returns. */

  /** Bars at each strike, over everything the rungs above this one reached. */
  const strikeBars = $derived.by(() => {
    const c = new Map();
    for (const r of expiryGated) {
      if (r.strike !== null) c.set(r.strike, (c.get(r.strike) ?? 0) + r.rows);
    }
    return c;
  });

  /** The same count one level up, per rung. Empty while there is no anchor. */
  const mnyBars = $derived.by(() => {
    const c = new Map();
    if (atmIndex === null) return c;
    for (const r of expiryGated) {
      const k = rungKeyOf(r);
      if (k !== null) c.set(k, (c.get(k) ?? 0) + r.rows);
    }
    return c;
  });

  /**
   * THE COUNT, COUNTED — and the three states are not two.
   *
   * `—` when nothing has been read, because a store nobody read counted
   * nothing. `no bars` when it HAS been read and this row is empty, because
   * that is a finding about the window and not about the control. A number
   * otherwise. `0 bars` would be the third spelling of the second state and
   * reads as a measurement of the first.
   *
   * @param {number} n
   */
  const barsDetail = (n) => (deco.length === 0 ? '—' : n ? `${fmt(n)} bars` : 'no bars');

  /** One row per strike the store names, ascending, priced in rupees. */
  /* THE STRIKES THIS SELECTION ACTUALLY REACHES — the cascade's seventh rung.
   *
   * `chainAll` is the CHAIN: every strike the store lists, in ladder order,
   * folded from `deco`. It has to stay that way — it is the coordinate frame
   * `atmIndex` indexes into and `ladderRungs` measures distance along, and
   * narrowing a frame renumbers every rung under a live moneyness pick.
   *
   * The OFFER is a different question: which of those strikes the rungs above
   * leave. Two values, because they answer two things — the same split
   * `expiryOffered` makes against `expiriesAll`, and for the same reason.
   */
  const strikeOffered = $derived.by(() => {
    const seen = new Set();
    for (const r of expiryGated) if (r.strike !== null) seen.add(r.strike);
    return seen;
  });

  const strikeRows = $derived(
    chainAll
      .filter((c) => strikeOffered.has(c.strike))
      .map((c) => ({
        key: String(c.strike),
        name: strikeText(c.strike),
        detail: barsDetail(strikeBars.get(c.strike) ?? 0)
      }))
  );

  /**
   * The ladder as rows, with the FAMILY rows interleaved so that "all of ITM"
   * and "exactly ITM-2" are the same gesture.
   *
   * A FAMILY OF ONE IS ITS RUNG and is drawn once. Printing both would be the
   * page repeating a control it already drew, one row apart — and ATM is always
   * a family of one, by definition.
   *
   * The family key is prefixed `fam:` so it can never collide with a rung name:
   * `ATM` is both a family and a rung, and one string standing for two rows is
   * how a tick lands on the wrong one.
   */
  /* THE RUNGS THIS SELECTION REACHES. `ladderRungs` is the ladder — the frame —
   * and this is which of its rungs the strike above actually leaves. */
  const mnyOffered = $derived.by(() => {
    const seen = new Set();
    for (const r of expiryGated) {
      if (strikePick.size > 0 && !(r.strike !== null && strikePick.has(String(r.strike)))) continue;
      const k = rungKeyOf(r);
      if (k !== null) seen.add(k);
    }
    return seen;
  });

  const mnyRows = $derived.by(() => {
    const out = [];
    for (const f of ['ITM', 'ATM', 'OTM']) {
      const run = ladderRungs.filter((m) => famOf(m) === f && mnyOffered.has(m));
      if (run.length === 0) continue;
      if (run.length > 1) {
        out.push({
          key: `fam:${f}`,
          name: f,
          detail: `${fmt(run.length)} rungs · ${barsDetail(
            run.reduce((a, m) => a + (mnyBars.get(m) ?? 0), 0)
          )}`
        });
      }
      for (const m of run) out.push({ key: m, name: m, detail: barsDetail(mnyBars.get(m) ?? 0) });
    }
    return out;
  });

  /**
   * The selection as the PANEL sees it: the rungs, plus a family key for every
   * run that is wholly taken.
   *
   * A ROW IS TICKED WHEN EVERY KEY UNDER IT IS, so half a family reads as
   * unticked and ticking it takes the rest — never the other way round.
   */
  const mnySelected = $derived.by(() => {
    const s = new Set(mnyPick);
    for (const f of ['ITM', 'OTM']) {
      const run = ladderRungs.filter((m) => famOf(m) === f);
      if (run.length > 1 && run.every((m) => mnyPick.has(m))) s.add(`fam:${f}`);
    }
    return s;
  });

  /**
   * A family tick, expanded into its whole run — in BOTH directions.
   *
   * The panel hands back a Set; the family keys in it are not selection, they
   * are a gesture. Comparing what the run WAS against what the family key now
   * says is what distinguishes "the reader ticked ITM" from "the reader ticked
   * the last remaining ITM rung, which happens to complete the family", and the
   * second must not then re-add rungs the first would have.
   *
   * @param {Set<string>} next
   */
  function onMnyPick(next) {
    const out = new Set();
    for (const k of next) if (!k.startsWith('fam:')) out.add(k);
    for (const f of ['ITM', 'OTM']) {
      const run = ladderRungs.filter((m) => famOf(m) === f);
      if (run.length < 2) continue;
      const was = run.every((m) => mnyPick.has(m));
      const now = next.has(`fam:${f}`);
      if (now && !was) for (const m of run) out.add(m);
      else if (!now && was) for (const m of run) out.delete(m);
    }
    mnyPick = out;
  }

  /* WHAT EACH BUTTON SAYS. The count is the reader's own selection against what
     this chain supports — never a label and never a constant. An empty
     selection says what it MEANS rather than what it holds: no join at all, the
     same sentence the `Everything` universe row carries. */
  /* SINGLE SELECT, SO THERE IS NO PLURAL ARM.
   *
   * `Picker` has carried a `single` prop all along — it emits `new Set([key])`
   * and never grows — and these two call sites were the only ones on the page
   * that had not switched it on. The instrument picker already had it. So the
   * page offered single select on one control and multi on two, which is the
   * inconsistency rather than a design.
   *
   * The `size > 1` arm is gone rather than left standing: a branch no input can
   * reach is a branch no test can cover and a mutant that deletes it survives.
   * The Set stays as the storage because that is what `Picker` emits; it now
   * holds at most one key. */
  const strikeSummary = $derived(
    chainAll.length === 0
      ? 'No strike stored'
      : strikePick.size === 0
        ? `Any strike · ${fmt(chainAll.length)} listed`
        : `${strikeText(Number([...strikePick][0]))} only`
  );

  const mnySummary = $derived(
    ladderRungs.length === 0
      ? 'No ladder'
      : mnyPick.size === 0
        ? `Any rung · ${fmt(ladderRungs.length)} on the ladder`
        : `${[...mnyPick][0]} only`
  );

  /**
   * WHY THE STRIKE PANEL CANNOT BE OPENED, or `null` when it can.
   *
   * `CLAUDE.md` §4: degrade loudly and NAME THE REASON. "Not yet read" and "the
   * store holds no option" are opposite instructions — one is a wait and the
   * other is a fact about the pull — and one greyed control cannot say both.
   */
  const strikeRefusal = $derived.by(() => {
    if (chainAll.length > 0) return null;
    if (deco.length === 0)
      return `nothing has been read for ${feedDisplay(feeds.active)} yet, so no instrument name has been looked at — this is an unread store, not a store without options`;
    return `not one of the ${fmt(deco.length)} instrument-month(s) ${feedDisplay(feeds.active)} holds is named the way this repository names an option. crates/core/src/instrument.rs ends one “-<strike in paisa>-CE” or “-<strike in paisa>-PE”, and nothing in this store ends either way: it holds indices, equities and futures, whose names carry no strike at all. /store.json sends no strike field, so a strike here can only ever be READ OFF A NAME — never decoded from a spelling this page guessed at.`;
  });

  /**
   * WHY THE LADDER CANNOT BE OPENED, or `null` when it can — and WHICH of the
   * two reasons it is.
   *
   * The second one is permanent on today's wire and it is the more important
   * of the two, because it is the one an operator would otherwise assume away:
   * a chain is not a ladder. Rungs are distances FROM THE MONEY, and nothing
   * this page reads says where the money is.
   */
  /* THE PRICE CLAUSE, WRITTEN ONCE. It is the reason on the control's own
     tooltip AND the reason on the counted line under the row, and two spellings
     of one refusal is how one of them goes stale. */
  const NO_SPOT =
    'a rung is a DISTANCE FROM SPOT, and no endpoint this page reads carries a price. /store.json’s chg_bps is a RATIO and a ratio has no scale, so no absolute price can be recovered from it; /instruments.json carries no price at all; /bars.json does, and reading it would be one request and about 81 bytes per bar per contract to recover a single figure — the exact trade D-0067 refused when it put the two closes in the census instead. Nothing here guesses one: every name on a ladder anchored to a guess would be wrong by however far the guess sat from the money.';

  const moneyRefusal = $derived.by(() => {
    if (ladderRungs.length > 0) return null;
    if (chainAll.length === 0)
      return `there is no chain to place a rung on — ${strikeRefusal ?? 'no strike is stored'}. And there would be no ladder even with one, because ${NO_SPOT}`;
    return `${fmt(chainAll.length)} strike(s) are stored and not one of them can be named ITM or OTM, because ${NO_SPOT} The ladder is derived from this chain and nothing else, so it fills itself in the day a spot price reaches this page — no list of rung names is written down anywhere to be edited.`;
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
        const k = rungKeyOf(r);
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

  /* ======================================================================
     THE TWO PERCENTAGE COLUMNS — WHAT THEY ARE, AND WHY MOST CELLS ARE A DASH
     ----------------------------------------------------------------------
     Every number in this panel is COUNTED FROM THE ROWS ON SCREEN'S OWN
     DATASET, never typed in. The previous version of this block asserted that
     `/store.json` carries no price, which was true when it was written and
     false the day D-0069 landed — a hardcoded claim about the server outliving
     the server. Counting means the panel cannot say "0 refused" over a table
     full of dashes, or the reverse.
     ====================================================================== */

  /** Bytes per bar on `/bars.json`, MEASURED: 668,251 bytes for 8,250 bars.
      A constant because it is a property of the wire format, not of the store;
      everything multiplied by it below is counted from the live data instead
      of typed in, so this panel cannot drift into asserting a row count the
      page beside it contradicts. */
  const BYTES_PER_BAR = 81.0;

  const storeBars = $derived(deco.reduce((a, r) => a + r.rows, 0));
  const kindCount = $derived.by(() => {
    const c = new Map();
    for (const r of deco) c.set(r.kind, (c.get(r.kind) ?? 0) + 1);
    return c;
  });

  /** How many of the whole dataset carry each reason code, per column. */
  const whyCount = $derived.by(() => {
    const chg = new Map();
    const prev = new Map();
    for (const r of deco) {
      if (r.chg === null) chg.set(r.chgWhy, (chg.get(r.chgWhy) ?? 0) + 1);
      if (r.prevChg === null) prev.set(r.prevChgWhy, (prev.get(r.prevChgWhy) ?? 0) + 1);
    }
    return { chg, prev };
  });

  const numbered = $derived(deco.reduce((a, r) => a + (r.chg === null ? 0 : 1), 0));
  const refusedForSplits = $derived(whyCount.chg.get('corporate_action_unverified') ?? 0);

  /**
   * Every reason code present, counted in both columns, most common first.
   *
   * ENUMERATED, NEVER LISTED BY HAND. The version of this panel that named
   * `base_not_positive` and `overflow` and then said "neither appears in this
   * dataset" was rendered over a dataset containing a `base_not_positive` row.
   * A panel about honesty cannot afford a sentence that is not counted, so the
   * sentence became a count.
   */
  const reasonRoll = $derived.by(() => {
    const codes = new Set([...whyCount.chg.keys(), ...whyCount.prev.keys()]);
    return [...codes]
      .map((code) => ({
        code,
        chg: whyCount.chg.get(code) ?? 0,
        prev: whyCount.prev.get(code) ?? 0
      }))
      .sort((a, b) => b.chg + b.prev - (a.chg + a.prev) || txt(a.code, b.code));
  });

  /**
   * Bytes, at the magnitude that reads as a quantity rather than as `0.0`.
   *
   * @param {number} n
   */
  function bytesText(n) {
    if (n >= 1e9) return `${(n / 1e9).toFixed(1)} GB`;
    if (n >= 1e6) return `${(n / 1e6).toFixed(1)} MB`;
    if (n >= 1e3) return `${(n / 1e3).toFixed(1)} kB`;
    return `${Math.round(n)} B`;
  }

  const NOTES = $derived([
    {
      k: 'What the two columns are.',
      v: `% change is the month's OWN first close to its OWN last close. Prev % change is the previous calendar month's same statistic for the same instrument — not the previous bar, because a row here is an instrument-month. Both arrive as integer basis points and the decimal point is inserted for display: 125 on the wire is +1.25% on screen. CLAUDE.md §7 bans the float for money and a ratio derived from money gets the same treatment, so no percentage on this page has ever been a float, at either end.`
    },
    {
      k: 'No corporate-action threshold is sourced. That is why most cells are a dash.',
      v: `${fmt(refusedForSplits)} of ${fmt(deco.length)} rows are refused for this reason, and it is the only one of the five that is PERMANENT — no pull fixes it. docs/00-charter.md names no verified split-and-bonus feed, and docs/05-decisions.md D-0018 both requires a suspected corporate action to be refused loudly and rejects back-adjusting from an unverified source. An unadjusted 1:5 split is a fake 80% overnight crash that no stored price can be told apart from a real move. So every instrument that can be re-based — cash equities, and the F&O contracts NSE adjusts alongside them — is marked rather than numbered, in BOTH columns, whether or not its neighbour is held. Indices never split, and the ${fmt(kindCount.get('INDEX') ?? 0)} INDEX row(s) are the only ones eligible for a number at all.`
    },
    {
      k: 'A month-level field can flag a month. It cannot name the day.',
      v: `D-0018's stated detector is an unexplained OVERNIGHT gap, which is a bar-to-bar test over a month's ~8,250 bars — O(bars) work that cannot live in a census row (D-0067). Even with a threshold in hand, this column could only ever say "a corporate action may fall in this month", never "split on 2024-06-14". Anything stronger would have to be measured somewhere else and is not being claimed here.`
    },
    {
      k: 'Every reason on this dataset, counted.',
      v:
        reasonRoll.length === 0
          ? 'Nothing is refused: every row in this dataset carries a number in both columns.'
          : `${reasonRoll.map((r) => `${r.code}: ${fmt(r.chg)} in % change, ${fmt(r.prev)} in prev`).join(' · ')}. Counted from the rows themselves rather than listed here, so this line cannot claim a code is absent while the table below shows one.`
    },
    {
      k: 'What it cost to make this affordable.',
      v: `Reading the two closes from /bars.json instead would be ${fmt(deco.length)} request(s) and about ${bytesText(storeBars * BYTES_PER_BAR)} moved — measured at ${BYTES_PER_BAR.toFixed(1)} bytes per bar, from 668,251 bytes for 8,250 bars — to extract ${bytesText(deco.length * 16)} of signal. D-0067 put the two closes in the census entry instead, so a row and its neighbour are two hash probes and no file is opened. This page pays one request for the whole table, as it always did.`
    }
  ]);

  let noteOpen = $state(false);

  /* ======================================================================
     SORT
     ----------------------------------------------------------------------
     Codepoint comparison, not `localeCompare`: instruments are `[A-Z0-9-_&]`
     by construction, so collation adds nothing and would make the order a
     function of the operator's locale. Same inputs, same order, everywhere.
     ====================================================================== */
  const COLS = [
    { key: 'instrument', label: 'Instrument', num: false },
    { key: 'month', label: 'Month', num: false },
    { key: 'timeframe', label: 'TF', num: false },
    { key: 'rows', label: 'Bars', num: true },
    { key: 'days', label: 'Sessions', num: true },
    { key: 'short', label: 'Short by', num: true },
    { key: 'pct', label: 'Completeness', num: true },
    // THESE TWO SORT NOW. They used to carry `key: null` with the comment "a
    // control that reorders 20,516 identical blanks is a control that lies
    // about having data" — which was right while they were blank and is wrong
    // now that they hold basis points. The rule the old comment states is
    // unchanged: EVERY COLUMN THAT HOLDS DATA SORTS. These hold data.
    {
      key: 'chg',
      label: '% Change',
      num: true,
      why: "this month's first close to its last close, in basis points"
    },
    {
      key: 'prevChg',
      label: 'Prev % Change',
      num: true,
      why: "the previous month's % change for the same instrument"
    }
  ];

  /**
   * Three-way codepoint comparison, and STRINGS ONLY.
   *
   * Every one of the fourteen call sites passes a string — a month key, an
   * instrument, a rung name, a reason code — and the block above says why:
   * these keys are `[A-Z0-9-_&]` by construction, so `<` on the codepoints is
   * the whole ordering and `localeCompare` would only make it a function of
   * the operator's locale. Declaring the parameters as strings is what keeps
   * that true: a number reaching here would compare numerically on one
   * argument and lexically on the other the moment one side arrived as text,
   * and the caller — `sorted`, at 20,516 rows — is the last place that would
   * show it.
   *
   * @param {string} a
   * @param {string} b
   */
  function txt(a, b) {
    return a < b ? -1 : a > b ? 1 : 0;
  }

  const sorted = $derived.by(() => {
    const k = sortKey;
    const dir = desc ? -1 : 1;
    return [...matched].sort((a, b) => {
      let d = 0;
      // `days` IS ITS OWN FIELD. Both keys used to sort on `a.rows`, so clicking
      // the Sessions header ordered the table by Bars — two different numbers,
      // and a column that sorts by a neighbour is worse than one that does not
      // sort at all, because the arrow says it worked.
      if (k === 'rows') d = a.rows - b.rows;
      // `?? 0` IS THE COERCION JAVASCRIPT ALREADY PERFORMS, WRITTEN DOWN.
      // `days` is `null` for every rung `barsPerSession` has no session size
      // for — 2min, 3min, 5min and the rest of the ladder — and `null - null`
      // is 0 while `5 - null` is 5, because `-` sends `null` to 0 before it
      // subtracts. Spelling it out changes no order and no output; it only
      // stops the expression from reading as though a null could not arrive.
      //
      // IT IS DELIBERATELY NOT THE `?? -1` RULE `pct` USES TWO LINES DOWN.
      // Sorting an unknown session count below every known one would be a
      // different table from the one this page renders today, and changing
      // which rows an operator sees first is not a typing change.
      else if (k === 'days') d = (a.days ?? 0) - (b.days ?? 0);
      else if (k === 'short') d = a.short - b.short;
      // AN UNKNOWN SORTS AS AN UNKNOWN. `a.pct` is null for a row that is its
      // own denominator, and `null - null` is 0 while `0.9 - null` is 0.9 —
      // a comparator that treats "not measured" as zero orders the table by a
      // number nobody computed. Below every real ratio, and stable among
      // themselves.
      else if (k === 'pct') d = (a.pct ?? -1) - (b.pct ?? -1);
      else if (k === 'month') d = txt(a.month, b.month);
      // BY LENGTH, NOT BY SPELLING. `txt` stood here, which ordered the rungs
      // `15min`, `1day`, `1min`, `30min` — by first character, which is the
      // same defect as a month column ordered by `Apr`. `tfCmp` is the ladder
      // the new Timeframe control is drawn from, so the column and the control
      // cannot disagree about what "next rung" means.
      else if (k === 'timeframe') d = tfCmp(a.timeframe, b.timeframe);
      else if (k === 'chg' || k === 'prevChg') {
        // AN UNKNOWN SORTS LAST IN BOTH DIRECTIONS, and the `return` rather
        // than a `d` is what makes that true: folding it into `dir * d` would
        // put every dash at the TOP on one click and the bottom on the next,
        // so "biggest fall first" would open on a screen of dashes. A dash is
        // not a very small number; it is not a number, and it has no place on
        // either end of an ordering of numbers.
        if (a[k] === null && b[k] === null) d = 0;
        else if (a[k] === null) return 1;
        else if (b[k] === null) return -1;
        else d = a[k] - b[k];
      } else d = txt(a.instrument, b.instrument);
      if (d !== 0) return dir * d;
      // Deterministic tie-break, NOT inverted with the direction: two runs of
      // the same sort produce byte-identical order, so equal values never
      // jitter when the direction is flipped and flipped back.
      return txt(a.instrument, b.instrument) || txt(a.month, b.month);
    });
  });

  /**
   * A click on a column heading: same column flips the direction, a new column
   * opens on the end that column is interesting at.
   *
   * @param {string} key a `COLS[].key` — the field name `sorted` switches on
   */
  function head(key) {
    if (sortKey === key) desc = !desc;
    else {
      sortKey = key;
      // Open a numeric column on its interesting end: most bars first, biggest
      // hole first, biggest gain first — but completeness ascending, because
      // the WORST row is the one being looked for.
      desc = key === 'rows' || key === 'days' || key === 'short' || key === 'chg' || key === 'prevChg';
    }
  }

  /**
   * The `aria-sort` value for one heading — the direction stated to a screen
   * reader, from the same two variables the arrow is drawn from.
   *
   * @param {string} key a `COLS[].key`
   */
  function ariaSort(key) {
    return sortKey === key ? (desc ? 'descending' : 'ascending') : 'none';
  }

  /* ======================================================================
     TWO GRIDS OVER ONE STORE - the CENSUS, and the BARS THEMSELVES
     ----------------------------------------------------------------------
     THE CENSUS GRID ANSWERS "WHAT IS HELD" AND CANNOT ANSWER "WHAT IS IN
     IT". One row per instrument-month, counted off `/store.json`, no bar
     file opened: it is the only view that can survey 20,516 rows for a
     hole. It has no open, no close, no volume and no open interest, because
     the census carries none.

     So a SECOND grid is added rather than the first being replaced. One row
     per BAR, read from `/bars.json` - the endpoint `/markets` already
     reads, one request per instrument-month, the path being the index.
     Which grid is showing is a choice the reader makes and the page states
     in words; neither is a default that hides the other.

     WHAT THE BAR GRID WILL NOT DO. Ten of its twenty-four columns have no
     source on this wire, and every one of them says so on its own face
     rather than printing a number. `CLAUDE.md` section 3 rule 1: a Greek
     this page invented would be indistinguishable, on screen, from one a
     vendor sent.
     ====================================================================== */
  const VIEWS = [
    {
      // KEY STAYS `census`. It is an identifier -- it keys `view`, `VIEWS.find`
      // and every comparison in this file -- and renaming an id to match a
      // label is how a rename becomes a bug. Only the words the operator reads
      // change.
      key: 'census',
      // "Census" was internal vocabulary. The operator read the tab and could
      // not tell what the view was FOR. "Coverage" says it: this is the view
      // that answers what you hold and what is missing.
      label: 'Coverage',
      sub: 'what you hold, per instrument-month',
      showing: 'one row per INSTRUMENT-MONTH, counted from /store.json',
      title:
        'What the store HOLDS. One row per instrument-month, off /store.json - bars, sessions, completeness, % change. No bar file is opened, so this is the only view that can survey the whole store for a hole.'
    },
    {
      key: 'bars',
      label: 'Bars',
      sub: 'the stored values, one row per bar',
      showing: 'one row per BAR, read from /bars.json',
      title:
        'What the store CONTAINS. One row per stored bar, off /bars.json - one request per instrument-month, so the read budget decides how many files are opened. Prices are the store’s own paisa integers.'
    }
  ];
  // OPENS ON THE DATA, NOT ON THE INVENTORY.
  //
  // The page used to open on the coverage view, so the first thing an operator
  // saw was a survey of what he holds rather than the values themselves -- and
  // with a nearly-empty store that survey is a screen of zeros, which reads as
  // a broken page rather than as an empty one. Bars is the table the page is
  // for; coverage is the second question, and it is one click away.
  let view = $state('bars');
  const viewNow = $derived(VIEWS.find((v) => v.key === view) ?? VIEWS[0]);

  /* ---------------------------------------------------------------------
     THE PAGER'S STATE, and it governs BOTH grids.

     `page` is a 1-based ordinal and it is CLAMPED AT EVERY READ (`pageNow`)
     rather than written back: a narrowing that shortens the run must not
     leave the reader on page 40 of 3 looking at a blank grid, and a clamp
     that writes state from inside a derived is a loop.
     --------------------------------------------------------------------- */
  const PAGE_SIZES = [25, 50, 100, 250, 1000];
  let pageSize = $state(50);
  let page = $state(1);

  /* ---------------------------------------------------------------------
     WHAT HAS NO SOURCE, SAID ONCE EACH.

     Every sentence below reaches the screen twice - on the disabled sort
     button's title and on every cell in that column - so the reason cannot
     be on one and missing from the other.
     --------------------------------------------------------------------- */
  const NO_PREMKT =
    'a pre-market percentage needs a pre-open session price, and the store’s bar record carries none: docs/02-store-format.md fixes the record at timestamp, open, high, low, close, volume and open interest, and /bars.json returns exactly those seven fields. There is nothing on this wire to divide.';
  const NO_INTRINSIC =
    'intrinsic value is the distance from SPOT to the strike, clipped at zero, and this page has no spot. /bars.json returns THIS contract’s own prices - the premium - never the underlying’s. Subtracting a strike from a premium is not intrinsic value, it is a wrong number.';
  const NO_EXTRINSIC =
    'extrinsic value is premium minus intrinsic, so it is unknown for exactly as long as intrinsic is, and for the same reason.';
  const NO_GREEK =
    'no endpoint this page reads carries a Greek, and none is computed here. Black-Scholes needs a measured risk-free rate and a measured time to expiry; neither is sourced in this repository, and CLAUDE.md section 3 rule 1 forbids inventing one. A number printed in this column would be indistinguishable, on screen, from one a vendor sent.';

  /* ---------------------------------------------------------------------
     THE BAR GRID'S COLUMNS - all twenty-four, in the approved order.

     `none` is the reason a column has no source. A column with `none` set
     renders a NAMED dash in every cell and its sort button is DISABLED
     wearing that same sentence: a sort control over a column of unknowns
     reorders nothing while claiming to have worked.
     --------------------------------------------------------------------- */
  const BAR_COLS = [
    /* THE DAY AND THE MINUTE ARE TWO COLUMNS, because a grid is read DOWN a
       column. One `18 Aug 2026, 15:29` cell put every row's minute at a
       different x — the day in front of it is a different width on the 9th and
       the 18th — so the one field a reader scans a bar table for could not be
       scanned at all. Sorting still keys on `ts`, the instant, so splitting the
       DISPLAY changes no order. */
    {
      key: 'ts',
      label: 'Date',
      w: 116,
      why: 'the bar’s own opening day, IST - the store holds the instant in microseconds'
    },
    {
      key: 'tod',
      label: 'Time',
      w: 74,
      why: 'the bar’s own opening minute, IST - absent on a day rung, where every bar opens at the same one'
    },
    {
      key: 'tf',
      label: 'TF',
      w: 68,
      why: 'the bar length, sorted by DURATION and never by spelling'
    },
    { key: 'o', label: 'Open', w: 104, num: true, why: 'paisa integer, straight from the store' },
    { key: 'h', label: 'High', w: 104, num: true, why: 'paisa integer, straight from the store' },
    { key: 'l', label: 'Low', w: 104, num: true, why: 'paisa integer, straight from the store' },
    { key: 'c', label: 'Close', w: 108, num: true, why: 'paisa integer, straight from the store' },
    {
      key: 'vol',
      label: 'Volume',
      w: 116,
      num: true,
      why: 'contracts or shares traded inside this bar'
    },
    { key: 'premkt', label: 'Pre-mkt %', w: 102, num: true, none: NO_PREMKT },
    {
      key: 'chg',
      label: 'Change %',
      w: 102,
      num: true,
      why: 'this bar’s close against the PREVIOUS BAR’s close in the same month file, in integer basis points'
    },
    {
      key: 'oi',
      label: 'Open interest',
      w: 124,
      num: true,
      why: 'the stored open interest. i64::MIN is the null sentinel and renders as unknown; a zero is a real zero'
    },
    {
      key: 'oichg',
      label: 'OI chg %',
      w: 100,
      num: true,
      why: 'this bar’s open interest against the previous bar’s, in integer basis points'
    },
    {
      key: 'exp',
      label: 'Expiry',
      w: 116,
      why: 'the contract’s expiry day, parsed from the stored name'
    },
    {
      key: 'dte',
      label: 'Days to exp',
      w: 106,
      num: true,
      why: 'whole calendar days from this bar’s IST day to the expiry day - negative after it'
    },
    { key: 'side', label: 'Type', w: 72, why: 'CE or PE, from the stored name' },
    { key: 'strike', label: 'Strike', w: 96, num: true, why: 'paisa integer, from the stored name' },
    /* `num: true` — MEASURED, NOT ASSUMED. Every neighbour on this side of the
       table carries it (`dte`, `strike`, `intr`, `extr`, `iv`, and the four
       Greeks) and this one did not, so its HEADER rendered left-aligned over a
       CELL that rendered right: the only column on the table whose label and
       value face opposite edges. Found by reading the computed styles of all
       twenty-four columns rather than by looking, which is also how the two
       "clipped" columns turned out not to be clipped at all. */
    { key: 'mny', label: 'Moneyness', w: 106, num: true, none: NO_SPOT },
    { key: 'intr', label: 'Intrinsic', w: 100, num: true, none: NO_INTRINSIC },
    { key: 'extr', label: 'Extrinsic', w: 100, num: true, none: NO_EXTRINSIC },
    { key: 'iv', label: 'IV %', w: 86, num: true, none: NO_GREEK },
    { key: 'delta', label: 'Delta', w: 86, num: true, none: NO_GREEK },
    { key: 'gamma', label: 'Gamma', w: 90, num: true, none: NO_GREEK },
    { key: 'theta', label: 'Theta', w: 88, num: true, none: NO_GREEK },
    { key: 'vega', label: 'Vega', w: 86, num: true, none: NO_GREEK },
    { key: 'rho', label: 'Rho', w: 84, num: true, none: NO_GREEK }
  ];
  /** By key, so a cell reaches its own column's reason without an index. */
  const BAR_META = (() => {
    /** @type {Record<string, any>} */
    const m = {};
    for (const c of BAR_COLS) m[c.key] = c;
    return m;
  })();
  /** The natural width of the bar grid - SUMMED from the columns, never typed. */
  /* THE COLUMNS ACTUALLY DRAWN, WHICH IS NOT ALL OF THEM.
   *
   * Ten of the twenty-four have NO SOURCE on this wire — the pre-market
   * percentage, moneyness, intrinsic, extrinsic and the six greeks — and four
   * more describe a CONTRACT: expiry, days-to-expiry, type and strike. A store
   * of indices and equities has neither kind, so fourteen of twenty-four
   * columns were being drawn across every row of every page purely to say
   * "NO SOURCE" or "—". At fifty rows a page that is seven hundred cells of
   * nothing, and it pushed close and volume off the side of the screen.
   *
   * `CLAUDE.md` §4 — degrade loudly, name the reason — is why they were drawn.
   * §4 is about a FAILURE being concealed, and neither of these is one: a feed
   * that never sends a greek has not failed, and a spot index has no expiry to
   * report. The reason is still named, once, in the line under the table
   * rather than seven hundred times inside it.
   *
   * The contract four come back the moment a contract is stored, on the same
   * `expiryRefusal` the Contract strip uses, so the two cannot disagree about
   * whether this store holds one.
   */
  /* WHAT KIND OF THING IS ON SCREEN, which is what decides the columns.
   *
   * A spot index has no expiry and no strike. A FUTURE has an expiry and no
   * strike. An OPTION has both, plus a side. Drawing all four contract columns
   * for a spot store meant four columns of "-" on every row; drawing strike and
   * type for a futures selection would mean two more.
   *
   * Read off the rows the query actually leaves (`scoped`) rather than off the
   * page being viewed, so paging to row 51 cannot change which columns exist.
   * `side` is the option marker because only an option has one; an expiry with
   * no side is a future. Both come from the name, parsed once per row when the
   * store is read — see `contractOf` and `expiryOf`.
   */
  const shape = $derived.by(() => {
    let future = false;
    let option = false;
    for (const r of scoped) {
      if (r.side) option = true;
      else if (r.expiry) future = true;
      if (option && future) break;
    }
    return { future, option, contract: future || option };
  });

  const OPTION_COLS = new Set(['side', 'strike']);
  const FUTURE_COLS = new Set(['exp', 'dte']);
  /**
   * A DAY RUNG HAS NO MINUTE WORTH A COLUMN.
   *
   * Every 1day bar opens at the same instant — the session open — so the Time
   * column would repeat one value down every row and earn none of its width. It
   * is dropped rather than blanked: a column of identical stamps is not
   * information, and a column of dashes is worse, since a dash on this table
   * means "no source" everywhere else.
   *
   * `rungSeconds` rather than a string test on '1day', so anything coarser than
   * a day — were a rung ever added above it — is covered by the same rule
   * instead of needing a second one.
   */
  const dayRung = $derived((TF_SECS.get(timeframe) ?? 0) >= 86400);
  const BAR_SHOWN = $derived(
    BAR_COLS.filter(
      (c) =>
        !c.none &&
        !(dayRung && c.key === 'tod') &&
        !(!shape.contract && FUTURE_COLS.has(c.key)) &&
        !(!shape.option && OPTION_COLS.has(c.key))
    )
  );
  const BAR_HIDDEN = $derived(BAR_COLS.length - BAR_SHOWN.length);

  const BAR_WIDTH = $derived(BAR_SHOWN.reduce((a, c) => a + c.w, 0));

  /** A PARTITION, AND IT SUMS: sourced + unsourced = every column drawn. */
  const BAR_SOURCED = BAR_COLS.filter((c) => !c.none).length;
  const BAR_UNSOURCED = BAR_COLS.length - BAR_SOURCED;

  /* ---------------------------------------------------------------------
     THE READ BUDGET - a CHOICE, not a count the page took for the reader.

     `/bars.json` is one month per request and about 81 bytes per bar on the
     wire, so "read everything that matched" is a request storm on a 206-row
     census. The budget is a control with its cost on its face; `0` means
     every matched instrument-month and the option says so.
     --------------------------------------------------------------------- */
  const READ_BUDGETS = [1, 3, 6, 12, 24, 0];
  /** @param {number} n */
  const budgetLabel = (n) => (n === 0 ? 'Every one' : `${fmt(n)} newest`);
  /* THE DEFAULT IS EVERY MATCHED MONTH, and it was six.
   *
   * Operator's rule, 2026-08-19: the grid shows what the QUERY selected — the
   * chosen feed, universe, segments, timeframes and date range — not a slice of
   * it. A default of six read the six NEWEST months and rendered the rest as
   * "absent rather than empty", so a January-to-August query opened on March
   * and an operator reasonably read that as January never having been pulled.
   * It had been: 7,500 bars, on disk, structurally perfect.
   *
   * A page that answers a narrower question than the one it was asked is worse
   * than a slow one, because nothing on it is wrong — it is just not the
   * answer. The budget stays as a CONTROL for anyone who wants to cap a very
   * wide query; it is no longer the default. */
  let readBudget = $state(0);

  /**
   * The instrument-months the bar grid would read, NEWEST MONTH FIRST.
   *
   * Newest first because a store is read forward: the question an operator
   * opens this grid with is "what landed", and the newest month is where it
   * landed. "Backfill runs oldest-first" is a rule about WRITING and has no
   * bearing on which file a reader opens first.
   */
  const barPlan = $derived.by(() => {
    const all = [...matched].sort(
      (/** @type {any} */ a, /** @type {any} */ b) =>
        txt(b.month, a.month) || txt(a.instrument, b.instrument) || tfCmp(a.timeframe, b.timeframe)
    );
    const take = readBudget === 0 ? all.length : Math.min(readBudget, all.length);
    return { all, read: all.slice(0, take), held: all.length - take };
  });
  /** A PRIMITIVE, so the fetch effect re-runs when the SET changes and not
      on every keystroke that leaves the same set standing. */
  const barPlanKeys = $derived(barPlan.read.map((/** @type {any} */ r) => r.key).join('\n'));

  /* ---------------------------------------------------------------------
     THE READ. One request per instrument-month, cached for as long as the
     feed and the Refresh nonce hold still, so narrowing the query does not
     re-ask for a file already in memory.
     --------------------------------------------------------------------- */
  /** @type {Map<string, any>} */
  const barCache = new Map();
  let barCacheStamp = '';
  /** @type {{ loading: boolean, error: string | null, files: any[] }} */
  let barState = $state({ loading: false, error: null, files: [] });
  let barToken = 0;

  /**
   * One month file, for one series.
   *
   * A FAILURE IS A VALUE HERE, NOT A REJECTION: one unreadable file must not
   * blank the twelve that read, so the grid lists each refusal under its own
   * instrument-month. Only successes are cached - a retry has to be able to
   * succeed.
   *
   * @param {string} feed
   * @param {any} r
   */
  async function readBarFile(feed, r) {
    const hit = barCache.get(r.key);
    if (hit) return hit;
    /* EXCHANGE-SEGMENT-SYMBOL, AND THE SYMBOL KEEPS ITS OWN HYPHENS. The
       store's series name is `Exchange-Segment-Symbol`, and a symbol is
       legally `ABB-III` or `NIFTY-2026-08-27-2500000-CE`. Only the FIRST
       TWO separators are structural; everything after them is the symbol.
       Splitting on the last one instead is how `-CE` becomes a segment. */
    const parts = String(r.instrument).split('-');
    const q = new URLSearchParams({
      feed,
      exchange: parts[0] ?? '',
      segment: parts[1] ?? '',
      symbol: parts.slice(2).join('-'),
      /* THE RUNG THE ROW ITSELF CARRIES. Omitted, the server defaults to
         `1min`, and a daily file is then asked for at a rung that has no
         file - the exact refusal /markets already paid for once. */
      timeframe: r.timeframe,
      month: r.month
    });
    try {
      const res = await ask(`/bars.json?${q}`);
      let body = null;
      try {
        body = await res.json();
      } catch {
        return {
          key: r.key,
          row: r,
          bars: [],
          faults: null,
          error: `HTTP ${res.status}, and the body was not JSON`
        };
      }
      /* THE SERVER'S OWN SENTENCE, not a status code. `/bars.json` refuses
         with `{"error":"...2026-08.bin does not exist, opening it"}`, which
         names the file; "HTTP 400" names nothing and sends the operator to
         the logs. */
      if (body && typeof body === 'object' && !Array.isArray(body) && body.error) {
        return { key: r.key, row: r, bars: [], faults: null, error: String(body.error) };
      }
      if (!res.ok && res.status !== 206) {
        return { key: r.key, row: r, bars: [], faults: null, error: `HTTP ${res.status}` };
      }
      /* A PARTIAL read answers 206 with `{bars, faults}`. Both shapes are
         handled and a faulty record is surfaced, never silently dropped - a
         grid with an unexplained hole is worse than one with a warning. */
      const out = {
        key: r.key,
        row: r,
        bars: Array.isArray(body) ? body : (body?.bars ?? []),
        faults: Array.isArray(body) ? null : (body?.faults ?? null),
        error: null
      };
      barCache.set(r.key, out);
      return out;
    } catch (why) {
      const w = /** @type {any} */ (why);
      return {
        key: r.key,
        row: r,
        bars: [],
        faults: null,
        error: String(w && w.message ? w.message : w)
      };
    }
  }

  $effect(() => {
    const active = view === 'bars';
    const feed = feeds.active;
    const keys = barPlanKeys;
    void nonce;
    /* THE CACHE IS KEYED ON THE FEED AND THE NONCE. Refresh means "read the
       store again", and a cache that survived it would answer the re-read
       out of the copy the re-read was asked to replace. */
    const stampNow = `${feed ?? ''} ${nonce}`;
    if (barCacheStamp !== stampNow) {
      barCache.clear();
      barCacheStamp = stampNow;
    }
    if (!active || !feed || keys === '') {
      barState = { loading: false, error: null, files: [] };
      return;
    }
    const want = untrack(() => barPlan.read);
    const mine = ++barToken;
    let dead = false;
    barState = { loading: true, error: null, files: untrack(() => barState.files) };
    Promise.all(want.map((/** @type {any} */ r) => readBarFile(feed, r)))
      .then((files) => {
        if (dead || mine !== barToken) return;
        barState = { loading: false, error: null, files };
      })
      .catch((why) => {
        if (dead || mine !== barToken) return;
        /* NAMED, NOT SHRUGGED. `readBarFile` returns its failures as values,
           so reaching here at all is this page failing rather than a file. */
        barState = {
          loading: false,
          error: String(why && why.message ? why.message : why),
          files: []
        };
      });
    return () => (dead = true);
  });

  /** Files that answered with a refusal rather than with bars. */
  const barFails = $derived(barState.files.filter((/** @type {any} */ f) => f.error !== null));
  /** Files that answered 206 - read in part, with the part that failed named. */
  const barFaults = $derived(barState.files.filter((/** @type {any} */ f) => f.faults));
  /** Files whose bar count disagrees with the census row that named them. */
  const barDisagree = $derived(
    barState.files.filter(
      (/** @type {any} */ f) => f.error === null && f.bars.length !== f.row.rows
    )
  );

  /* ---------------------------------------------------------------------
     THE BAR ROWS. Built once per read - never per paint and never per
     keystroke: the comparator below runs O(n log n) times and must compare
     plain fields, exactly as the census comparator does.
     --------------------------------------------------------------------- */
  const IST_OFFSET_MS = 19800000; /* +05:30, and India keeps no DST */

  /**
   * The IST calendar day an epoch-millisecond instant falls on, as the
   * `YYYY-MM-DD` KEY form and never a label. It is subtracted from an
   * expiry, so it has to stay comparable.
   * @param {number} ms
   */
  function istDayKey(ms) {
    const d = new Date(ms + IST_OFFSET_MS);
    return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, '0')}-${String(
      d.getUTCDate()
    ).padStart(2, '0')}`;
  }

  /**
   * Whole calendar days between two ISO days, or `null` if either will not
   * parse. Both ends are read at UTC midnight, so no zone offset can round a
   * day off the difference.
   * @param {string} fromIso
   * @param {string} toIso
   */
  function daysBetween(fromIso, toIso) {
    const a = Date.parse(`${fromIso}T00:00:00Z`);
    const b = Date.parse(`${toIso}T00:00:00Z`);
    if (!Number.isFinite(a) || !Number.isFinite(b)) return null;
    return Math.round((b - a) / 86400000);
  }

  /**
   * A price in paisa, with its two decimals and Indian digit grouping.
   *
   * INTEGER ARITHMETIC THROUGHOUT, exactly as `strikeText` does it: a `%`
   * and a subtraction, never a divide by 100 on the way to the screen.
   * Prices are `i64` paisa (`CLAUDE.md` section 7) and a float would round
   * one of them. Unlike `strikeText` the two decimals are ALWAYS printed -
   * an OHLC column holding 24,150 beside 24,150.25 has to align.
   *
   * @param {number | null | undefined} p
   */
  function paisaText(p) {
    if (p === null || p === undefined) return '-';
    const neg = p < 0;
    const a = neg ? -p : p;
    const frac = a % 100;
    const whole = (a - frac) / 100;
    return `${neg ? '−' : ''}${fmt(whole)}.${String(frac).padStart(2, '0')}`;
  }

  /** Why one bar's percentage is not a number. One code, one sentence. */
  const BAR_WHY = {
    first_bar_in_file:
      'this is the first bar in its month file, so the close it would be measured against is in the previous month - and this request read one month. It is not a zero.',
    previous_close_zero:
      'the previous close is zero paisa, and a ratio against zero is not a number.',
    oi_null:
      'open interest on this bar is the store’s null sentinel, i64::MIN - this feed stamps none for this segment. It is NOT zero; a zero here would be a real zero.',
    oi_null_before:
      'the previous bar carries no open interest, so there is nothing to measure this one against.',
    previous_oi_zero: 'the previous open interest is zero, and a ratio against zero is not a number.'
  };
  /** @param {string | null} code */
  const barWhyText = (code) =>
    (code === null ? null : /** @type {any} */ (BAR_WHY)[code]) ??
    'unknown, and this page did not record why - which is a defect here, not a fact about the store.';

  const barRows = $derived.by(() => {
    /** @type {any[]} */
    const out = [];
    for (const f of barState.files) {
      if (f.error !== null) continue;
      const r = f.row;
      const bars = f.bars;
      for (let i = 0; i < bars.length; i++) {
        const b = bars[i];
        const before = i > 0 ? bars[i - 1] : null;
        /* `typeof ... === 'number'` AND NEVER A TRUTHINESS TEST. The server
           writes `null` for `i64::MIN` and the integer for everything else,
           so `0` is a STORED ZERO and `b.oi || null` would erase it. */
        const oi = typeof b.oi === 'number' ? b.oi : null;
        const prevOi = before && typeof before.oi === 'number' ? before.oi : null;
        const ts = b.t * 1000;
        const day = istDayKey(ts);
        const chgWhy =
          before === null ? 'first_bar_in_file' : before.c === 0 ? 'previous_close_zero' : null;
        const oiWhy =
          oi === null
            ? 'oi_null'
            : before === null
              ? 'first_bar_in_file'
              : prevOi === null
                ? 'oi_null_before'
                : prevOi === 0
                  ? 'previous_oi_zero'
                  : null;
        /* THE ENGINE'S ROUNDING AND THE ENGINE'S OVERFLOW VERDICT, BOTH.
           `basisPoints` rounds half away from zero the way
           `server.rs::basis_points` does — `Math.round` rounded a half toward
           +Infinity here, so a gain and its mirror-image loss printed
           different magnitudes in a column that sorts. It returns `null` where
           the engine returns `Unknown::Overflow`, and that null has to become
           a REASON rather than a bare dash: `WHY.overflow` already holds the
           sentence, and a cell nobody can compute must say why. */
        const chgBps = chgWhy === null ? basisPoints(before.c, b.c) : null;
        const oichgBps = oiWhy === null ? basisPoints(prevOi, oi) : null;
        out.push({
          /* THE KEY IS THE SERIES, THE MONTH, THE RUNG AND THE BAR'S OWN
             SECOND. An `{#each}` key: unique across every file on screen and
             stable across a re-sort. */
          rk: `${r.key} ${b.t} ${i}`,
          instrument: r.instrument,
          head: r.head,
          sym: r.sym,
          month: r.month,
          ts,
          day,
          tf: r.timeframe,
          o: b.o,
          h: b.h,
          l: b.l,
          c: b.c,
          vol: b.v,
          chg: chgBps,
          chgWhy: chgWhy ?? (chgBps === null ? 'overflow' : null),
          oi,
          oichg: oichgBps,
          oichgWhy: oiWhy ?? (oichgBps === null ? 'overflow' : null),
          /* THE RAW ISO DAY, which is what the sort compares and what the
             expiry arithmetic reads. `dayLabel` is applied at the text node
             and nowhere else. */
          exp: r.expiry,
          dte: r.expiry === null ? null : daysBetween(day, r.expiry),
          side: r.side,
          strike: r.strike
        });
      }
    }
    return out;
  });

  /** Bars actually read, and what the census claimed for the same files.
      A PARTITION, and both halves are printed under the grid. */
  const barsRead = $derived(barRows.length);
  const barsClaimed = $derived(
    barState.files.reduce(
      (/** @type {number} */ a, /** @type {any} */ f) => a + (f.error === null ? f.row.rows : 0),
      0
    )
  );

  /* ---------------------------------------------------------------------
     THE BAR GRID'S SORT. Three-way on the KEY, never on the formatted text,
     and an unknown sorts LAST IN BOTH DIRECTIONS - a dash is not a very
     small number, it is not a number.
     --------------------------------------------------------------------- */
  let barSortKey = $state('ts');
  let barDesc = $state(true);

  /* THE DAY WINDOW, APPLIED TO BARS AND NOT TO INSTRUMENT-MONTHS.
   *
   * `scoped` and everything above it counts instrument-MONTHS; a day window
   * over those could only ever narrow to whole months. The question is "show me
   * the 12th", so it is answered where days exist, which is here. */
  const barWindowed = $derived.by(() => {
    if (!dayWindowApplies || (!fromDay && !toDay)) return barRows;
    return barRows.filter(
      (b) => (!fromDay || b.day >= fromDay) && (!toDay || b.day <= toDay)
    );
  });

  /**
   * THE DAYS THIS STORE ACTUALLY HOLDS — `[oldest, newest]` as `yyyy-mm-dd`,
   * or `['', '']` when nothing is loaded.
   *
   * # Why the calendar needs this and did not have it
   *
   * /ingest's day grid draws its bounds: days outside what the field may take
   * are struck through, and a paragraph under the grid names the window. /db's
   * grid drew every day plain, because it passed `$lib/DayField.svelte` no
   * `min` and no `max` — so the two calendars were the same COMPONENT showing
   * different amounts of truth.
   *
   * The bound here is a different fact from /ingest's and that is not an
   * inconsistency. /ingest's floor is the FEED's — how far back the vendor
   * answers, a rolling number that moves every day. This one is the STORE's:
   * the oldest and newest day actually on disk for the current selection. One
   * says what can be asked for, the other says what is held, which is the
   * standing difference between the two pages.
   *
   * ONE PASS over the rows already in memory, no sort and no new request —
   * `barRows` is what the table is about to draw anyway.
   */
  /**
   * THE SPAN THE DAY FIELDS MAY REACH — /ingest's, not the store's.
   *
   * `dayBounds` below is the first and last day the LOADED ROWS hold, and
   * handing that to `DayField` as `min`/`max` made the calendar unable to leave
   * the store: with one month on disk the year pad offered exactly one chip,
   * `2026`, where /ingest's offers twelve. A reader could not navigate to 2015
   * to ask whether anything was there, because the control refused the question
   * before it was asked.
   *
   * So the BOUNDS are the whole span — 01 Jan 2015, the floor the server's own
   * picker offers, to today in IST — and `dayBounds` keeps its other job: it is
   * still what the bounds SENTENCE reports, so the reader is still told exactly
   * where the store starts and stops. The two were one value doing two jobs and
   * only one of them wanted the narrow number.
   *
   * `FLOOR_YEAR` is 2015 here for the same reason it is 2015 on /ingest: it is
   * the floor `crates/api/src/calendar.rs` offers, and the two pages must not
   * disagree about how far back a day may be named.
   */
  const FLOOR_YEAR = 2015;
  const spanFloor = `${FLOOR_YEAR}-01-01`;
  const spanCeil = $derived(istDayKey(Date.now()));

  const dayBounds = $derived.by(() => {
    let lo = '';
    let hi = '';
    for (const b of barRows) {
      /* String compare is sound and deliberate: `YYYY-MM-DD` is
         lexicographically ordered, which is the same reason line 216's
         comparison is written this way. No `Date`, so no timezone can move a
         day. */
      if (!b.day) continue;
      if (lo === '' || b.day < lo) lo = b.day;
      if (hi === '' || b.day > hi) hi = b.day;
    }
    return [lo, hi];
  });

  const barSorted = $derived.by(() => {
    const k = barSortKey;
    const dir = barDesc ? -1 : 1;
    return [...barWindowed].sort((a, b) => {
      const A = a[k];
      const B = b[k];
      let d = 0;
      if (A === null || A === undefined) {
        if (B === null || B === undefined) d = 0;
        else return 1;
      } else if (B === null || B === undefined) {
        return -1;
      } else if (k === 'tf') {
        /* BY DURATION, NOT BY SPELLING. `txt` here would order the rungs
           15min, 1day, 1min, 30min - the same defect as a month column
           ordered by `Apr`. */
        d = tfCmp(A, B);
      } else if (typeof A === 'string' || typeof B === 'string') {
        /* `exp` IS THE RAW ISO DAY, so this comparison is chronological. On
           `29 Oct 2026` it would be alphabetical, and wrong. */
        d = txt(String(A), String(B));
      } else {
        /* THREE-WAY, AND NEVER `A - B`. A subtraction hands a comparator a
           magnitude where it asked for a sign, and it is the shape that
           returns a non-zero for two equal values the moment either end
           stops being a small integer. */
        d = A < B ? -1 : A > B ? 1 : 0;
      }
      if (d !== 0) return dir * d;
      /* Deterministic tie-break, NOT inverted with the direction, so two
         runs of one sort produce byte-identical order. */
      return (
        txt(a.instrument, b.instrument) ||
        (a.ts < b.ts ? -1 : a.ts > b.ts ? 1 : 0) ||
        txt(a.rk, b.rk)
      );
    });
  });

  /** @param {string} key */
  function barHead(key) {
    const col = BAR_COLS.find((c) => c.key === key);
    if (!col || col.none) return; /* a column with no source has nothing to order */
    if (barSortKey === key) barDesc = !barDesc;
    else {
      barSortKey = key;
      /* A new column opens on the end that is useful first: newest and
         largest at the top, text A-Z. */
      barDesc = Boolean(col.num) || key === 'ts';
    }
  }
  /** @param {string} key */
  const barAriaSort = (key) => (barSortKey === key ? (barDesc ? 'descending' : 'ascending') : 'none');

  /* ---------------------------------------------------------------------
     PAGING - the same arithmetic for both grids, so the readout under one
     cannot mean something different under the other.
     --------------------------------------------------------------------- */
  const pageTotal = $derived(view === 'bars' ? barSorted.length : sorted.length);
  const pageCount = $derived(Math.max(1, Math.ceil(pageTotal / pageSize)));
  const pageNow = $derived(Math.min(Math.max(1, page), pageCount));
  const pageFrom = $derived(pageTotal === 0 ? 0 : (pageNow - 1) * pageSize + 1);
  const pageTo = $derived(Math.min(pageNow * pageSize, pageTotal));
  const censusPage = $derived(sorted.slice((pageNow - 1) * pageSize, pageNow * pageSize));
  const barPage = $derived(barSorted.slice((pageNow - 1) * pageSize, pageNow * pageSize));

  /** @param {number} n */
  function goPage(n) {
    const want = Number(n);
    page = Math.min(pageCount, Math.max(1, Number.isFinite(want) ? Math.trunc(want) : 1));
  }
  /** Keep the reader's PLACE across a page-size change, not their ordinal:
      the row they were standing on stays on screen.
      @param {number} n */
  function setPageSize(n) {
    const firstRow = (pageNow - 1) * pageSize;
    pageSize = Number(n);
    page = Math.floor(firstRow / pageSize) + 1;
  }

  /**
   * Which page buttons to draw. Always first, last, current and its
   * neighbours; the rest elided - a 1,840-page run must not render 1,840
   * buttons, which is the constant-per-paint rule applied to a control.
   *
   * @param {number} cur
   * @param {number} total
   */
  function pageList(cur, total) {
    /* ONE SHAPE FROM BOTH BRANCHES. The short branch returned bare numbers
       and the long one returned records, so the `{#each}` key expression was
       reading `.k` off a number for every run of seven pages or fewer. */
    if (total <= 7) {
      return Array.from({ length: total }, (_, i) => ({ k: `p${i + 1}`, p: i + 1, gap: false }));
    }
    const want = new Set([1, total, cur, cur - 1, cur + 1]);
    if (cur <= 3) {
      want.add(2);
      want.add(3);
      want.add(4);
    }
    if (cur >= total - 2) {
      want.add(total - 1);
      want.add(total - 2);
      want.add(total - 3);
    }
    const list = [...want]
      .filter((p) => p >= 1 && p <= total)
      .sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
    /** @type {{ k: string, p: number, gap: boolean }[]} */
    const out = [];
    list.forEach((p, i) => {
      if (i && p - list[i - 1] > 1) out.push({ k: `gap${p}`, p, gap: true });
      out.push({ k: `p${p}`, p, gap: false });
    });
    return out;
  }

  /* ======================================================================
     THE SUMMARY, over the MATCHED set — the question actually on screen
     ====================================================================== */
  const total = $derived(matched.reduce((a, r) => a + r.rows, 0));
  /**
   * COMPLETE MEANS COMPARED. A row that is the only one stored at its (month,
   * rung) is not counted here in either direction: it is not complete and it is
   * not short, because nothing in the store can tell which. See `SOLE_WHY`.
   */
  const full = $derived(matched.reduce((a, r) => a + (r.short === 0 && !r.sole ? 1 : 0), 0));
  const unverified = $derived(matched.reduce((a, r) => a + (r.sole ? 1 : 0), 0));
  const gaps = $derived(matched.reduce((a, r) => a + r.short, 0));
  /** The rows a denominator other than themselves stands behind. */
  const comparable = $derived(matched.filter((r) => !r.sole));
  /** Bars in those rows — the numerator Coverage is entitled to use. */
  const judgedBars = $derived(comparable.reduce((a, r) => a + r.rows, 0));

  /**
   * Session-equivalents across the selection, each row converted AT ITS OWN
   * RUNG before summing.
   *
   * A single `total / 375` cannot be right for a store holding two rungs: 403
   * daily bars are 403 sessions, not 1.07. Rows at a rung with no recorded
   * session size contribute nothing rather than a guess, which is why this can
   * read lower than the bar count implies — and why the label beside it says
   * "session-equivalents" rather than "sessions".
   */
  const sessionEquivalents = $derived(
    matched.reduce((a, r) => {
      const per = barsPerSession(r.timeframe);
      return per === null ? a : a + r.rows / per;
    }, 0)
  );

  /** Whole sessions missing, each row at its own rung. Same rule as above. */
  const gapSessions = $derived(
    matched.reduce((a, r) => {
      const per = barsPerSession(r.timeframe);
      return per === null ? a : a + r.short / per;
    }, 0)
  );

  /**
   * How many matched rows sit at a rung with a recorded session size — the
   * denominator BEHIND the two session figures above, counted rather than
   * assumed.
   *
   * Both of those sums skip a row whose rung `barsPerSession` has no number
   * for. When every row on screen is at such a rung the sums are `0`, and `0`
   * is a measurement: it says the selection holds no sessions, when the truth
   * is that nobody can convert the bars it does hold. Counting the convertible
   * rows lets the sub-lines print a dash and name that, which is the same rule
   * `dayText` already enforces one row at a time.
   */
  const ranged = $derived(
    matched.reduce((a, r) => a + (barsPerSession(r.timeframe) === null ? 0 : 1), 0)
  );

  /* WHAT THE COMPARABLE SET WOULD HOLD IF FULL.
     `total + gaps` counted the self-denominated rows into the numerator AND
     gave them a shortfall of zero, so a store holding one bar reported
     100.00% coverage with a full meter. Those rows are now outside both
     halves, and the tile says how many. */
  const owed = $derived(judgedBars + gaps);

  /**
   * Bars held over bars owed — or `null`, which is a THIRD answer and not a
   * bad hundred percent.
   *
   * `owed > 0 ? total / owed : 1` said 100.00% over a selection holding no
   * bars at all, and the meter beside it filled to the end. Every filter that
   * matches nothing reaches that branch, so the one tile an operator uses to
   * judge the store reported perfect coverage of nothing, in the confident
   * shape of a measurement. 0/0 is not one; it is undefined, `pctText` draws
   * the dash, and the tile names which absence it is.
   */
  const coverage = $derived(owed > 0 ? judgedBars / owed : null);
  const monthsIn = $derived.by(() => {
    const s = new Set();
    for (const r of matched) s.add(r.month);
    return [...s].sort();
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
        mnyPick.size
    )
  );

  /**
   * THE NEWEST BAR IN THE SELECTION — counted, and it is the one fact the
   * mockup's removed "newest bar" notice carried that nothing else on this page
   * stated.
   *
   * `/store.json` has sent `last_ts` per row since the census gained it, and
   * until now this page read it only per row (the month cell's tooltip) and per
   * instrument (the drawer's "Days held"). Nothing said how fresh the SET on
   * screen is, which is the question a banner over a table is actually asked.
   * It is in the count line at the foot instead of in a band of its own: a
   * standing notice is a row of chrome, and the same figure beside the row
   * count is read by the same glance.
   *
   * `null` is an absence and not an epoch — a row with no stamp, or no row at
   * all, and the foot names which. Milliseconds, because `deco` already divided
   * the wire's microseconds where the unit was known.
   *
   * @type {number | null}
   */
  const newestBar = $derived(
    matched.reduce(
      (/** @type {number | null} */ a, /** @type {any} */ r) =>
        r.lastAt !== null && (a === null || r.lastAt > a) ? r.lastAt : a,
      /** @type {number | null} */ (null)
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
    if (error)
      return {
        tsub: 'the manifest failed',
        why: `/store.json could not be read — ${error}. Nothing was counted, including the zeroes.`
      };
    if (loading && rows.length === 0)
      return { tsub: 'not read yet', why: 'The store manifest is still being read.' };
    if (rows.length === 0)
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
        why: `${feedName} holds ${fmt(deco.length)} instrument-month(s) and none of them is in ${chosenUniverse.label}. ${
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
        why: `The store was read and holds ${fmt(deco.length)} instrument-month(s); none of them match ${narrowing.length ? narrowing.map((n) => n.text).join(', ') : 'the current selection'}. The store is not empty — this selection is.`
      };
    return null;
  });

  /* ---- the month roll-up ---------------------------------------------
     WHICH MONTHS ARE COMPLETE, WHICH HAVE HOLES, HOW BIG. A hole is almost
     never one instrument — it is a session the pull did not finish — so the
     month is the altitude the answer actually lives at. Each card is also the
     filter for that month, because seeing a hole and then having to type its
     name is a page that shows a problem and hides the way in. */
  /**
   * THE DENOMINATOR IS SUMMED PER ROW, NOT SNAPSHOT FROM THE FIRST ONE.
   *
   * `fullest` was read once, inside `if (!m)` — from whichever row of that
   * month `/store.json` happened to send first — and then multiplied by the
   * row count. `monthFull` is keyed on (month, RUNG) and says why three lines
   * into its own comment; this took a (month, rung) figure and applied it to
   * every rung in the month. Measured on 2021-08 holding NIFTY@1day=23,
   * NIFTY@1min=8,625 and BANKNIFTY@1min=8,625 — a month that is genuinely
   * complete at both rungs — the card read `25,033.33%` with a meter 250 times
   * its own width and the title "every one of the 3 instruments holds all 23
   * bars" if the daily row arrived first, and `66.75%` tagged SHORT if the
   * minute row did. One headline number with two values, decided by row order.
   *
   * `owed` is the sum of each row's OWN (month, rung) denominator, so every row
   * is judged at its own rung and the ratio is the same whatever order the wire
   * used. `fullest` survives only as a display figure and only where the month
   * holds one rung — `sessions` already worked this way.
   */
  const monthCards = $derived.by(() =>
    rollUpMonths(windowed)
      .map((m) => {
        // ONE CALL, ONE VALUE — the hoist `deco` already makes for the same
        // reason. `barsPerSession` was called TWICE below, once to test for
        // `null` and once to divide by, so the test narrowed nothing that
        // could be checked and the second call was free to be a different
        // answer than the one that was tested. It is pure and it is not,
        // today; a guard that only happens to hold is not a guard.
        //
        // `m.tf` is the rung this month is stored at, or `false` once two
        // rungs disagree — see `rollUpMonths`. A month with no single rung has
        // no session size either, and asking for one is the question that
        // returns `null` here.
        const per = m.tf ? barsPerSession(m.tf) : null;
        return {
          ...m,
          // THE FULLEST ROW, AND ONLY WHERE ONE RUNG MAKES THAT A SINGLE
          // NUMBER. `null` draws a dash and the title says the month holds
          // two rungs.
          fullest: m.tf ? (m.n > 0 ? m.owed / m.n : 0) : null,
          // NULL RATHER THAN A NUMBER when the rung is unknown or mixed; the
          // renderer draws a dash. `dayText` owns that rule for every caller.
          sessions: per !== null && m.n > 0 ? m.owed / m.n / per : null
        };
      })
      .sort((a, b) => txt(b.month, a.month))
  ); /* newest first: that is where a pull lands */

  /* A HOLE IS A HOLE, AND AN UNKNOWN IS NOT ONE. `state !== 'full'` counted the
     months that hold too little AND the months nothing can be said about into
     one figure labelled "with holes". They are different answers and the strip
     now prints both. */
  const holeMonths = $derived(
    monthCards.filter((m) => m.state === 'near' || m.state === 'gap').length
  );
  const unprovenMonths = $derived(monthCards.filter((m) => m.state === 'sole').length);

  /* ======================================================================
     VALUE-CHANGE FLASH — THE ONLY GREEN AND RED ON THIS PAGE
     ----------------------------------------------------------------------
     `CLAUDE.md`-adjacent discipline, stated here because it is a rule this
     page now keeps and previously broke: GREEN AND RED MEAN DIRECTION. They
     mark a figure that MOVED between two reads of the store, and the two
     refused percentage columns they are reserved for. Data quality — short,
     near, gap — is a severity, not a direction, and wears the amber warning
     hue at two intensities instead. Red meaning both "the price fell" and
     "bars are missing" is two facts wearing one colour, and at a glance
     neither can be read.
     ====================================================================== */
  /* THE THREE MAPS THIS BLOCK KEEPS, AND WHAT EACH ONE'S VALUE MEANS.
     ----------------------------------------------------------------------
     `flash` is keyed by TILE NAME — `n`, `bars`, `full`, `gaps`, `cov`,
     `months`, the same six `snapshot` builds below — and its `n` is a
     generation counter, NOT a count of anything on screen: it is what re-keys
     the node so the animation restarts, and it is what the timeout compares
     against so a later movement's class is never cleared by an earlier one's
     timer. `dir` is `''` once the animation is over, which is the direction
     ceasing to be stated rather than a seventh direction.

     Declared rather than inferred because `$state({})` is `{}` — a type with
     no keys at all — so every one of the four reads below is
     `expression of type 'any' can't be used to index type '{}'`, and the
     object would silently accept a misspelt tile name for as long as nobody
     looked at the screen.

     `prev` holds the LAST VALUE each tile was seen at, and `null` is one of
     them: Coverage has no ratio when the selection has no denominator, and
     `mark` refuses to call the arrival or departure of an unknown a movement. */
  /** @type {Record<string, { n: number, dir: string }>} */
  let flash = $state({});
  /** @type {Map<string, number | null>} */
  const prev = new Map();
  /** @type {Map<string, ReturnType<typeof setTimeout>>} */
  const timers = new Map();
  let seeded = false;

  /**
   * Record one tile's new value, and tint it if it MOVED.
   *
   * @param {string} name the tile key — one of the six `snapshot` names
   * @param {number | null} v its value now, or `null` for "not measurable"
   */
  function mark(name, v) {
    const p = prev.get(name);
    prev.set(name, v);
    // AN UNKNOWN IS NOT A DIRECTION, IN EITHER POSITION. Coverage reports
    // `null` when the selection has no denominator, and `null > 0.9` is false
    // — so without this guard a figure becoming unknown would tint red and a
    // figure ceasing to be unknown would tint red as well, both stating a
    // movement nobody measured. `null` still lands in `prev` above, so the
    // baseline is current the moment a real number returns.
    if (p === undefined || p === v || p === null || v === null) return;
    const n = (flash[name]?.n ?? 0) + 1;
    flash[name] = { n, dir: v > p ? 'up' : 'down' };
    // DROP THE CLASS ONCE THE ANIMATION IS OVER. `n` is unchanged, so the node
    // is not re-keyed and nothing replays — the direction simply stops being
    // stated. A class that outlives its animation is a class waiting to fire
    // spuriously the next time the node happens to be re-created.
    clearTimeout(timers.get(name));
    timers.set(
      name,
      setTimeout(() => {
        if (flash[name]?.n === n) flash[name] = { n, dir: '' };
      }, 800)
    );
  }

  /* ---- ROW-LEVEL FLASH ------------------------------------------------
     The tiles flashed and the rows did not, which had it backwards: a running
     backfill lands bars in specific instrument-months, and the row is where
     that fact lives. `moved` maps a row key to the direction its bar count
     went between two reads, and it is CLEARED after the animation window so a
     row scrolled into view ten seconds later does not replay a stale tint. */
  let moved = $state(new Map());
  const barsBefore = new Map();
  let barsSeeded = false;
  let movedTimer = 0;

  /**
   * Drop every baseline the flash compares against, and the open drawer.
   *
   * Called from the fetch effect when — and only when — the ACTIVE FEED
   * changed. "This figure moved" is a claim about one store read twice; Dhan's
   * totals against Groww's baseline would tint six tiles and a screenful of
   * rows for a movement that never happened, because the two are not the same
   * instrument universe, the same session handling or the same price scale.
   *
   * Un-seeding rather than clearing-and-comparing is deliberate: the first
   * response from the new feed re-seeds and flashes nothing, which is the same
   * rule the initial page load already follows and for the same reason.
   *
   * The drawer goes too. It describes one instrument's every stored month, and
   * the new feed may not carry that instrument at all — `openRowData` would
   * fall to `null` on its own, but only after the response lands, leaving the
   * other feed's months on screen in between.
   *
   * And `elsewhere` goes, for the reason stated where it is declared: it names
   * the OTHER feeds that hold rows, computed against whichever feed was active
   * when it resolved, so across a switch it can offer the feed already chosen.
   * Its own effect re-runs and re-answers the moment the new store comes back
   * empty; until then the honest render is no list at all.
   *
   * THE CASCADE STATE DELIBERATELY SURVIVES, and that is not an oversight.
   * `filter`, `kind` and `month` have always survived a feed switch — they are
   * the operator's QUESTION, not the answer, and re-asking it by hand on every
   * switch is what makes a comparison impossible. `universe`, `fromMonth` and
   * `toMonth` join them for the same reason, and they cannot go stale
   * unnoticed the way a count can: the universe rung re-counts against the new
   * feed's master and REFUSES to count at all until that master arrives
   * (`universeRefusal`), and a month window is a plain string comparison that
   * is as true of one store as of another.
   */
  function forgetPreviousFeed() {
    seeded = false;
    barsSeeded = false;
    prev.clear();
    barsBefore.clear();
    for (const t of timers.values()) clearTimeout(t);
    timers.clear();
    clearTimeout(movedTimer);
    flash = {};
    moved = new Map();
    openKey = null;
    // `elsewhere` IS NOT DROPPED HERE ANY MORE — it is `$derived` off the
    // shared survey and filtered against the CURRENT selection, so the stale
    // shape this function used to have to remember cannot be built.
  }

  /**
   * THE FLASH MEANS "THE STORE MOVED", NOT "YOU TYPED".
   *
   * The effect depends on `fetchedAt` and NOTHING else — every figure is read
   * inside `untrack`. Two consequences, both deliberate:
   *
   * - Typing in the filter changes all six numbers and flashes none of them.
   *   A tile that tints on every keystroke is a tile whose tint carries no
   *   information, and it would tint over the exact figure being read.
   * - The first response seeds the baseline and flashes nothing. Comparing the
   *   arriving store against the empty page it replaced would flash all six
   *   green on every load, which is the same nothing said louder.
   */
  $effect(() => {
    const at = fetchedAt;
    untrack(() => {
      if (!at) return;
      /**
       * THE PAIRS ARE DECLARED AS PAIRS. Inferred, this array is
       * `(string | number | null)[][]` — every element of every row a union of
       * all three — so the destructured `k` below is as likely to be a number
       * as the tile name it actually always is, and `mark` cannot state what
       * it takes. The tuple says what the rows are: a name, and a figure or
       * the absence of one.
       *
       * @type {[string, number | null][]}
       */
      const snapshot = [
        ['n', matched.length],
        ['bars', total],
        ['full', full],
        ['gaps', gaps],
        /* `null` PASSES THROUGH AS `null`, never as a rounded zero.
           `Math.round(null * 10000)` is 0, which would enter the baseline as a
           coverage of 0.00% and flash red the instant a real ratio returned.
           `mark` refuses to give an unknown a direction. */
        ['cov', coverage === null ? null : Math.round(coverage * 10000)],
        ['months', monthsIn.length]
      ];
      if (!seeded) {
        seeded = true;
        for (const [k, v] of snapshot) prev.set(k, v);
      } else {
        for (const [k, v] of snapshot) mark(k, v);
      }

      /* One pass over the whole store per REFRESH — not per render, not per
         scroll frame, and never per row on screen. */
      const next = new Map();
      for (const it of deco) {
        const was = barsBefore.get(it.key);
        if (barsSeeded && was !== undefined && was !== it.rows) {
          next.set(it.key, it.rows > was ? 'up' : 'down');
        }
        barsBefore.set(it.key, it.rows);
      }
      barsSeeded = true;
      moved = next;
      clearTimeout(movedTimer);
      if (next.size) movedTimer = setTimeout(() => (moved = new Map()), 900);
    });
  });

  /* ======================================================================
     WINDOWING — only the visible slice is in the DOM
     ====================================================================== */
  /* 40px, AND IT IS THE SAME 40 THE `--dbrow` TOKEN SETS. Raised together with
     it: this places row N at `N * ROW`, so a disagreement drifts the rows away
     from the scrollbar one pixel per row and the list is unusable by the
     thousandth. 32 was cramped at the grid's new 14px type. */
  /* A CHANGE IN BASIS POINTS AS A PERCENTAGE OF THE CELL'S WIDTH.
   *
   * Capped at 100 basis points — one percent — because that is where an
   * index minute-bar move stops being ordinary. Everything at or beyond it
   * draws a full bar, so the scale answers "is this one big" rather than
   * "how big", which is the question a column of fifty rows is actually
   * being scanned for. The exact figure is the text, unclipped, right there.
   *
   * `Math.abs` — direction is already carried by `data-dir`, which colours
   * the bar; length carries magnitude alone, so an up and a down of the same
   * size are the same length and are comparable at a glance.
   */
  /* SCALED TO WHAT IS ACTUALLY IN THE SELECTION, NOT TO A GUESS.
   *
   * The first draft fixed full width at 100 basis points -- one percent -- on
   * the reasoning that an index minute-bar move stops being ordinary there.
   * Measured against the real rows: the 90th percentile is 5 bps, the 99th is
   * 8, and the largest on the page is 8. Every bar drew at 5-8% of the cell and
   * the column was blank. A scale nothing reaches is not a scale.
   *
   * Taken from `barRows` -- every row the query matched, not `barPage` -- so
   * turning to page 2 cannot silently change what a full bar means. That
   * stability is the whole reason this is not computed per page.
   *
   * The floor stops a dead-flat selection from magnifying rounding into a
   * full-width bar: with every row at 0 or 1 bps there is nothing to compare
   * and the column stays quiet.
   */
  const CHG_FLOOR_BPS = 10;

  /* ONE SCALE PER RUNG, because a rung is what makes two moves comparable.
   *
   * A single scale over the whole selection was the second draft and it was
   * also wrong: with 1min and 1day rows matched together the daily move — the
   * whole session in one bar — set full width, and every minute row drew at
   * under a tenth of it. The column went blank again for the rung anybody
   * actually scans.
   *
   * A minute bar and a daily bar are not two sizes of the same thing, so they
   * do not share a ruler. Keyed on `tf`, each rung is measured against its own
   * largest move and the bar answers "big for a bar of this length".
   */
  /* VOLUME AND RANGE GET THEIR OWN PER-RUNG SCALES, for the reason the change
   * scale has one: a minute's volume and a day's are not two sizes of the same
   * thing, so they do not share a ruler. Folded over `barRows` and never
   * `barPage`, so turning the page cannot change what a full bar means. */
  const volFullByTf = $derived.by(() => {
    /** @type {Map<string, number>} */
    const top = new Map();
    for (const b of barRows) {
      const v = b.vol ?? 0;
      if (v > (top.get(b.tf) ?? 0)) top.set(b.tf, v);
    }
    return top;
  });

  /** @param {number} v @param {string} tf */
  function volMag(v, tf) {
    const full = volFullByTf.get(tf) ?? 0;
    return full <= 0 ? 0 : Math.round((Math.min(v ?? 0, full) / full) * 100);
  }

  /* THE BAR'S OWN RANGE, high minus low, as a share of the widest range at that
   * rung. This is what the spine down the left edge is scaled by: a doji reads
   * as a short tick and a wide bar as a tall one, so the column answers "where
   * did anything actually happen" before a single figure is read. */
  const rngFullByTf = $derived.by(() => {
    /** @type {Map<string, number>} */
    const top = new Map();
    for (const b of barRows) {
      const r = b.h - b.l;
      if (r > (top.get(b.tf) ?? 0)) top.set(b.tf, r);
    }
    return top;
  });

  /** @param {number} r @param {string} tf */
  function rngMag(r, tf) {
    const full = rngFullByTf.get(tf) ?? 0;
    if (full <= 0) return MIN_SPINE;
    /* SQUARE ROOT, NOT LINEAR. Measured on the real page: against the widest
     * 1-minute range in the month, an ordinary minute is under a fifth of it,
     * so a linear scale put all fifty rows on the floor — distinct values: 1.
     * A range is a length and the eye compares lengths by area, so the square
     * root is the honest curve here: it keeps the ordering exact and spends the
     * height where the rows actually are. The widest bar is still full height. */
    return Math.max(MIN_SPINE, Math.sqrt(Math.min(r, full) / full));
  }

  /* A FLAT BAR STILL GETS A TICK. Zero range is a real state — a minute with
   * one print — and a row with no mark at all reads as a rendering fault. */
  const MIN_SPINE = 0.08;

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
    if (fromDay === '' && toDay === '' && spanCeil) {
      fromDay = spanFloor;
      toDay = spanCeil;
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
  $effect(() => {
    if (kind === '' && segmentRows.length > 0) {
      const held = segmentRows.find((s) => s.why === undefined);
      kind = (held ?? segmentRows[0]).key;
    }
  });

  $effect(() => {
    if (filter === '' && instrumentOffered.length > 0) {
      const held = instrumentOffered.find((r) => r.months > 0);
      filter = (held ?? instrumentOffered[0]).key;
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
    if (timeframe === '' && tfAll.length > 0) timeframe = tfAll[0][0];
  });

  const stranded = $derived.by(() => {
    const out = [];
    if (picked && !instrumentKeys.has(picked)) {
      out.push({ rung: 'Instrument', value: picked, clear: () => (filter = '') });
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

  const chgFullByTf = $derived.by(() => {
    /** @type {Map<string, number>} */
    const top = new Map();
    for (const b of barRows) {
      if (b.chg === null) continue;
      const a = Math.abs(b.chg);
      if (a > (top.get(b.tf) ?? 0)) top.set(b.tf, a);
    }
    for (const [tf, v] of top) top.set(tf, Math.max(v, CHG_FLOOR_BPS));
    return top;
  });

  /** The rungs on screen and their scales, for the header's own explanation. */
  const chgScaleText = $derived(
    [...chgFullByTf].map(([tf, v]) => `${tf} ${fmt(v)} bps`).join(', ')
  );

  /**
   * @param {number} bps
   * @param {string} tf
   */
  function chgMag(bps, tf) {
    const full = chgFullByTf.get(tf) ?? CHG_FLOOR_BPS;
    return Math.round((Math.min(Math.abs(bps), full) / full) * 100);
  }

  const ROW = 40;
  const HEAD = 30; /* the sticky header covers the top of the scroll box */
  const OVER = 6;
  /** @type {HTMLElement | null} */
  let scroller = $state(null);
  let scrollTop = $state(0);
  let viewportH = $state(600);

  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVER));
  const count = $derived(Math.ceil(viewportH / ROW) + OVER * 2);
  /* WINDOWED INSIDE THE PAGE, NOT INSTEAD OF IT. The pager decides WHICH
     rows are reachable; the window decides which of those are in the DOM.
     Dropping the window when the pager landed would have put 1,000 rows of
     nine cells into the document on the largest page size, which is the
     layout cost the window exists to refuse. */
  const slice = $derived(censusPage.slice(first, first + count));

  /* THE SEAM APPEARS ONLY ONCE THERE IS SOMETHING BEHIND IT. A permanent rule
     down the pinned column is a border pretending to be a shadow; the hairline
     is drawn when — and only when — columns are actually sliding under it.

     IT IS A `class:` DIRECTIVE AND NOT `classList.toggle`, WHICH IS NOT A
     STYLE CHOICE. Svelte prunes CSS whose selectors it cannot see used in the
     template, so a class added only from JavaScript compiles to a rule that is
     deleted — the class lands on the node and nothing happens. Measured: the
     element carried `xscrolled` while the `::after` stayed at opacity 0.
     A boolean that flips at most twice per gesture costs nothing anyway. */
  let xscrolled = $state(false);
  function onScroll() {
    if (!scroller) return;
    scrollTop = scroller.scrollTop;
    const x = scroller.scrollLeft > 0;
    if (x !== xscrolled) xscrolled = x;
  }

  /* ======================================================================
     KEYBOARD — the table is navigable without a mouse
     ----------------------------------------------------------------------
     A virtualised list cannot give every row a tab stop: 20,516 tab stops is
     not navigation. The scroll box is the single stop and carries the cursor,
     which is the `aria-activedescendant` pattern — the focused row is always
     scrolled into view, so the id it names is always in the DOM.
     ====================================================================== */
  let cursor = $state(-1);
  /** @type {string | null} */
  let openKey = $state(null); /* the row whose detail drawer is open */
  /** @type {HTMLInputElement | null} */
  let searchEl = $state(null);
  /** @type {HTMLElement | null} */
  let drawerEl = $state(null);

  /* ======================================================================
     THE FIND BOX IS A COMBOBOX, AND IT NOW SAYS SO IN THE DOCUMENT
     ----------------------------------------------------------------------
     The box has always offered a list — that is what a prefix index is for —
     and the document said it was a bare text field. `role="combobox"` with
     `aria-autocomplete="list"`, `aria-expanded` and `aria-controls` is the
     WAI-ARIA pattern for exactly this control, and every one of those
     attributes is a PROMISE: `aria-controls` names a listbox, so a listbox has
     to exist and has to be the thing that opens.

     SO THE LISTBOX IS REAL. It is not a decoration bolted on to justify an
     attribute — it is the same enumeration the instrument `Picker` beside it
     shows, cut to what has been typed, and it reaches something the Picker
     cannot: a MONTH. The box has always matched instrument OR month (that is
     what `index` is built over) and only the instruments were ever listed.

     IT COSTS NO SCAN OF THE STORE. Both lists it reads are already built and
     are bounded by the UNIVERSE rather than by the row count —
     `instrumentRows` is one entry per instrument the window holds and
     `monthsAll` is one per month — so a keystroke walks hundreds of entries,
     never 20,516 rows, and never 93,776. The page's O(1)-per-keystroke
     property is about the ROW probe and is untouched: `textMatched` is still
     one Map probe.

     THE KEYBOARD BELONGS TO THE INPUT, which is the pattern's whole point: the
     options are not tab stops, `aria-activedescendant` names the highlighted
     one, and ↑ ↓ Enter are handled on the field. Escape is handled in
     `onWindowKey` beside the menu's, so ONE place decides what Escape shuts
     first and the two cannot fight over it.
     ====================================================================== */

  /** Is the suggestion listbox showing? Never true with an empty box. */
  let findOpen = $state(false);
  /** Which suggestion is highlighted, or −1 for none. An INDEX, not a key. */
  let findCursor = $state(-1);

  /** A rendering budget, not an authority on what matches. See `findMore`. */
  const FIND_ROWS = 10;

  /** Instrument keys starting with what is typed — bounded by the universe. */
  const findInstruments = $derived(
    typed ? instrumentRows.filter((r) => r.key.toUpperCase().startsWith(typed)) : []
  );
  /** Month keys starting with what is typed. RAW `YYYY-MM` on both sides. */
  const findMonths = $derived(
    typed ? monthsAll.filter(([m]) => m.toUpperCase().startsWith(typed)) : []
  );

  /**
   * The rows the listbox draws — instruments first, then months.
   *
   * `key` is the store's own spelling and it is what a click writes into the
   * box; `name` is the display form and it is written nowhere else. The month
   * rows are the case that makes this matter: `Sep 2024` is not a key the
   * store has ever heard of, so committing the label would filter to nothing
   * while the screen looked right.
   */
  const findRows = $derived.by(() => {
    /** @type {{ key: string, kind: 'instrument' | 'month', name: string, detail: string }[]} */
    const out = [];
    for (const r of findInstruments) {
      if (out.length >= FIND_ROWS) break;
      out.push({
        key: r.key,
        kind: 'instrument',
        name: r.key,
        detail:
          r.short > 0
            ? `${fmt(r.months)} month(s) held · −${fmt(r.short)}`
            : `${fmt(r.months)} month(s) held`
      });
    }
    for (const [m, n] of findMonths) {
      if (out.length >= FIND_ROWS) break;
      out.push({
        key: m,
        kind: 'month',
        /* `?? m` IS THE FALLBACK `monthLabel` ALREADY PROMISES, written where
           the checker can see it: it returns its input verbatim for a key it
           cannot parse, and its signature says `string | null | undefined`
           because it accepts those. `m` is a store key and is always a string,
           so this is a no-op at runtime — and on the day the store writes a
           month key nothing can parse, the raw key is what shows, which is the
           value worth seeing. */
        name: monthLabel(m) ?? m,
        detail: `${fmt(n)} row(s) held`
      });
    }
    return out;
  });

  /* HOW MANY MATCH THAT THE BUDGET DID NOT DRAW. Counted from the full
     filtered lists, so the foot of the menu never claims the ten it drew are
     all there are — a list silently truncated is a list that lies. */
  const findMore = $derived(findInstruments.length + findMonths.length - findRows.length);

  /** The listbox is open only when there is something typed to be a list ABOUT. */
  const findShown = $derived(findOpen && typed !== '');

  /**
   * Commit one suggestion. THE KEY, never the name.
   *
   * The box stays the single piece of state — writing `filter` is the whole
   * commit, exactly as typing it by hand would be — so there is nothing here
   * to keep in step with the instrument `Picker` beside it, which ticks a row
   * precisely when the box holds that row's key.
   *
   * @param {{ key: string }} row
   */
  function commitFind(row) {
    filter = row.key;
    findOpen = false;
    findCursor = -1;
    searchEl?.focus();
  }

  /**
   * ↑ ↓ move the highlight, Enter commits it. Nothing else is intercepted, so
   * every ordinary editing key still edits.
   *
   * ESCAPE IS NOT HERE. `onWindowKey` owns it, above the row drawer and below
   * the calendar, so the order in which the three things on this page close is
   * decided in ONE place instead of racing between a field handler and a
   * window handler that both fire for the same press.
   *
   * @param {KeyboardEvent} e
   */
  function onFindKey(e) {
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    const n = findRows.length;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      if (!findShown) {
        findOpen = true;
        return;
      }
      if (n === 0) return;
      const d = e.key === 'ArrowDown' ? 1 : -1;
      /* WRAPS, and the `+ n` is what makes it wrap in BOTH directions: `-1 %
         n` is `-1` in JavaScript, not `n - 1`, so ↑ from the first row would
         land on nothing and the highlight would vanish rather than move. */
      findCursor = findCursor < 0 ? (d > 0 ? 0 : n - 1) : (findCursor + d + n) % n;
      e.preventDefault();
      return;
    }
    if (e.key === 'Enter' && findShown && findCursor >= 0 && findCursor < n) {
      commitFind(findRows[findCursor]);
      e.preventDefault();
    }
  }

  /* ======================================================================
     EXPORT — the matched set, written by the browser, out of memory
     ----------------------------------------------------------------------
     NO REQUEST, AND NO FILE IN THIS REPOSITORY.

     It writes exactly the rows `sorted` already holds — the same set the count
     above the button reports, in the order on screen rather than in the
     window's own slice — so it costs one pass over an array that is already
     built. It adds no endpoint, no round trip and nothing to the wire: every
     field it emits was decoded from `/store.json` when the store was read.

     THE BYTES GO TO THE BROWSER'S DOWNLOAD AND NOWHERE ELSE. `CLAUDE.md` §2
     lists the tracked extensions and `.csv` is not one of them, so a `.csv`
     written into this tree would be CI gate 1 failing the build. A `Blob` and
     an object URL never touch the filesystem this repository is checked out
     on, and the URL is revoked in the same statement pair it is created in.

     EVERY CELL IS THE WIRE FORM. `month` goes out `2024-09` and not `Sep
     2024`, `expiry` goes out `2026-10-29`, `strike` goes out in PAISA and
     `timeframe` goes out `1min` — the store's own spellings, which is what
     makes the file joinable against the store. Only the HEADER row is words.
     That is the rule at the top of `$lib/dates.js` applied to a file instead of
     to a screen: a display form that becomes a key is a key that matches
     nothing.

     A PERCENTAGE GOES OUT AS THE INTEGER BASIS POINTS IT ARRIVED AS, beside
     its reason code. Inserting the decimal point would make it a float, which
     `CLAUDE.md` §7 bans for money and this page has never done at either end;
     and an unknown goes out EMPTY beside its `chg_why`, never as `0`, because
     zero is a real month that closed where it opened.
     ====================================================================== */

  /**
   * The columns, in the store's own order, with the wire value for each.
   *
   * `null` becomes the empty field and never `null`, `—`, or a zero: an
   * absence in a data file is an empty cell, and any of the other three would
   * be read back as a value.
   *
   * @type {{ label: string, v: (r: any) => string }[]}
   */
  const CSV_COLS = [
    { label: 'instrument', v: (r) => r.instrument },
    { label: 'month', v: (r) => r.month },
    { label: 'timeframe', v: (r) => r.timeframe },
    { label: 'rows', v: (r) => String(r.rows) },
    { label: 'fullest_in_month', v: (r) => String(r.denom) },
    { label: 'short_by', v: (r) => String(r.short) },
    { label: 'first_ts_micros', v: (r) => (r.firstAt === null ? '' : String(r.firstAt * 1000)) },
    { label: 'last_ts_micros', v: (r) => (r.lastAt === null ? '' : String(r.lastAt * 1000)) },
    { label: 'chg_bps', v: (r) => (r.chg === null ? '' : String(r.chg)) },
    { label: 'chg_why', v: (r) => r.chgWhy ?? '' },
    { label: 'prev_chg_bps', v: (r) => (r.prevChg === null ? '' : String(r.prevChg)) },
    { label: 'prev_chg_why', v: (r) => r.prevChgWhy ?? '' },
    { label: 'expiry', v: (r) => r.expiry ?? '' },
    { label: 'strike_paisa', v: (r) => (r.strike === null ? '' : String(r.strike)) },
    { label: 'side', v: (r) => r.side ?? '' }
  ];

  /* THE BAR GRID EXPORTS BARS, NOT THE CENSUS. Writing instrument-months out
     of a screen showing bars would be a file that does not match the grid it
     was taken from, which is worse than no button. Every value is the store's
     own wire form: paisa integers, raw ISO expiries, integer basis points,
     an EMPTY FIELD for an unknown and never a zero.

     THE TEN UNSOURCED COLUMNS ARE NOT IN THIS FILE. A column of empty fields
     under a `delta` heading is a promise that the number could arrive; the
     reason it cannot is on the grid, where it can be read. */
  const BAR_CSV_COLS = [
    { label: 'instrument', v: (/** @type {any} */ r) => r.instrument },
    { label: 'month', v: (/** @type {any} */ r) => r.month },
    { label: 'timeframe', v: (/** @type {any} */ r) => r.tf },
    { label: 'ts_micros', v: (/** @type {any} */ r) => String(r.ts * 1000) },
    { label: 'ist_day', v: (/** @type {any} */ r) => r.day },
    { label: 'open_paisa', v: (/** @type {any} */ r) => String(r.o) },
    { label: 'high_paisa', v: (/** @type {any} */ r) => String(r.h) },
    { label: 'low_paisa', v: (/** @type {any} */ r) => String(r.l) },
    { label: 'close_paisa', v: (/** @type {any} */ r) => String(r.c) },
    { label: 'volume', v: (/** @type {any} */ r) => String(r.vol) },
    { label: 'chg_bps', v: (/** @type {any} */ r) => (r.chg === null ? '' : String(r.chg)) },
    { label: 'chg_why', v: (/** @type {any} */ r) => r.chgWhy ?? '' },
    { label: 'open_interest', v: (/** @type {any} */ r) => (r.oi === null ? '' : String(r.oi)) },
    { label: 'oi_chg_bps', v: (/** @type {any} */ r) => (r.oichg === null ? '' : String(r.oichg)) },
    { label: 'oi_chg_why', v: (/** @type {any} */ r) => r.oichgWhy ?? '' },
    { label: 'expiry', v: (/** @type {any} */ r) => r.exp ?? '' },
    { label: 'days_to_expiry', v: (/** @type {any} */ r) => (r.dte === null ? '' : String(r.dte)) },
    { label: 'side', v: (/** @type {any} */ r) => r.side ?? '' },
    { label: 'strike_paisa', v: (/** @type {any} */ r) => (r.strike === null ? '' : String(r.strike)) }
  ];

  /**
   * RFC 4180 quoting, and the test is on the four characters that break a
   * field rather than on a guess about which ones might. A quote inside a
   * quoted field is doubled — that is the standard's own escape and not a
   * backslash, which spreadsheets read as a literal backslash.
   *
   * @param {string} s
   */
  function csvQuote(s) {
    return /["\r\n,]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  }

  /**
   * @param {any[]} out
   * @returns {string}
   */
  function csvText(out) {
    /* THE COLUMN SET FOLLOWS THE GRID ON SCREEN, from one read of `view`, so
       the header row and the records under it cannot come from two different
       questions. */
    const cols = view === 'bars' ? BAR_CSV_COLS : CSV_COLS;
    const lines = [cols.map((c) => csvQuote(c.label)).join(',')];
    for (const r of out) lines.push(cols.map((c) => csvQuote(c.v(r))).join(','));
    /* CRLF, AND A TRAILING ONE. RFC 4180 §2: records end with CRLF, and the
       last record may. Excel on Windows is the reader that cares. */
    return `${lines.join('\r\n')}\r\n`;
  }

  /**
   * The file name, built only from KEYS.
   *
   * The feed's WIRE string, the rung's own spelling and the raw first and last
   * `YYYY-MM` in the selection — never a display label, so two exports of the
   * same selection are the same name on every machine whatever locale it runs.
   */
  const csvName = $derived(
    `brutex-db-${view}_${feeds.active ?? 'no-feed'}_${timeframe || 'all-rungs'}_${
      monthsIn[0] ?? 'none'
    }_${monthsIn[monthsIn.length - 1] ?? 'none'}.csv`
  );

  /** What Export writes: the WHOLE matched set of the grid on screen, in the
      order that grid is sorted in — never the page, and never the window. */
  const csvRows = $derived(view === 'bars' ? barSorted : sorted);

  function exportCsv() {
    /* THE BUTTON IS DISABLED IN THIS STATE AND SAYS WHY ON ITS OWN FACE; this
       is the second guard, for the keyboard path and for the day the disabled
       test and this one drift. Writing a header row with no records under it
       would be a file that looks like an answer. */
    if (csvRows.length === 0) return;
    const url = URL.createObjectURL(
      new Blob([csvText(csvRows)], { type: 'text/csv;charset=utf-8' })
    );
    const a = document.createElement('a');
    a.href = url;
    a.download = csvName;
    a.click();
    URL.revokeObjectURL(url);
  }

  /**
   * Put the keyboard cursor on row `next` of the current page, clamped, and
   * scroll it into view.
   *
   * @param {number} next a row index into `censusPage`, in or out of range
   */
  function moveTo(next) {
    const n = censusPage.length;
    if (n === 0) return;
    cursor = Math.max(0, Math.min(n - 1, next));
    // Written straight to the DOM, never back into `scrollTop` state, so the
    // cursor moving and the list scrolling cannot feed each other.
    const el = scroller;
    if (!el) return;
    const top = HEAD + cursor * ROW;
    if (top < el.scrollTop + HEAD) el.scrollTop = cursor * ROW;
    else if (top + ROW > el.scrollTop + viewportH) el.scrollTop = top + ROW - viewportH;
  }

  /**
   * Move the cursor by `d` rows. From no cursor at all it enters the list at
   * the end the movement came from — down enters at the top, up at the bottom.
   *
   * @param {number} d rows to move, signed
   */
  function step(d) {
    moveTo(cursor < 0 ? (d > 0 ? 0 : censusPage.length - 1) : cursor + d);
  }

  /**
   * Open the drawer on one census row.
   *
   * `undefined` IS A REAL ARGUMENT AND IT OPENS NOTHING. The Enter/Space path
   * passes `censusPage[cursor]`, and an index into a windowed page can be past
   * its end for the instant between a filter narrowing the list and the cursor
   * being reset — which is exactly why the guard is here rather than at the
   * one call site that can produce it.
   *
   * @param {DecoRow | undefined} it
   */
  function openRow(it) {
    if (it) openKey = it.key;
  }

  /** @param {KeyboardEvent} e */
  function onKey(e) {
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    const page = Math.max(1, Math.floor(viewportH / ROW) - 1);
    switch (e.key) {
      case 'ArrowDown':
        step(1);
        break;
      case 'ArrowUp':
        step(-1);
        break;
      case 'PageDown':
        step(page);
        break;
      case 'PageUp':
        step(-page);
        break;
      case 'Home':
        moveTo(0);
        break;
      case 'End':
        moveTo(censusPage.length - 1);
        break;
      case 'Enter':
      case ' ':
        if (cursor >= 0) openRow(censusPage[cursor]);
        else return;
        break;
      case 'Escape':
        if (openKey) openKey = null;
        else if (cursor >= 0) cursor = -1;
        else return;
        break;
      default:
        return;
    }
    e.preventDefault();
  }

  /* `/` FOCUSES THE FILTER, from anywhere on the page that is not already a
     text field — the shortcut every terminal and every code host uses. */
  /** @param {KeyboardEvent} e */
  function onWindowKey(e) {
    const t = e.target;
    const typing =
      t instanceof HTMLElement &&
      (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable);
    /* AN OPEN MENU OWNS ESCAPE, for the same reason the calendar owns the
       keyboard above it: a menu left on screen after the key that dismisses
       things has been pressed is a control the reader has to click to be rid
       of. It is tested before `/` so a menu cannot be left standing while the
       focus is thrown to the filter box. */
    if (drop !== null && e.key === 'Escape') {
      drop = null;
      return;
    }
    /* AND THE FIND BOX'S OWN LISTBOX SHUTS BEFORE THE ROW DRAWER DOES, for the
       same reason: Escape dismisses the NEAREST thing, and the nearest thing
       to a reader typing in the box is the list under it. Handled here rather
       than on the field so that one function decides the order — a field
       handler and this one both fire for the same press, and two of them
       deciding would close both at once. */
    if (findOpen && e.key === 'Escape') {
      findOpen = false;
      findCursor = -1;
      return;
    }
    if (e.key === '/' && !typing && !e.metaKey && !e.ctrlKey && !e.altKey) {
      e.preventDefault();
      searchEl?.focus();
      searchEl?.select();
    } else if (e.key === 'Escape' && openKey) {
      openKey = null;
      scroller?.focus();
    }
  }

  /* ---- what the drawer shows -----------------------------------------
     EVERY MONTH THIS INSTRUMENT HOLDS, which is the drill-through the study
     asked for at the altitude this data actually supports. Built once as a
     Map, so opening a row is a probe and not a scan of 20,516 rows. */
  const byInstrument = $derived.by(() => {
    /* SAME REASON AS THE PREFIX INDEX ABOVE: an unannotated `new Map()` is
       `Map<any, any>`, and every month the drawer renders off `openMonths`
       would then be a shape nothing checks — on the one panel that reads nine
       fields off each row. */
    /** @type {Map<string, DecoRow[]>} */
    const by = new Map();
    for (const it of deco) {
      let a = by.get(it.instrument);
      if (!a) by.set(it.instrument, (a = []));
      a.push(it);
    }
    for (const a of by.values()) a.sort((x, y) => txt(y.month, x.month));
    return by;
  });

  const openRowData = $derived(openKey ? (deco.find((r) => r.key === openKey) ?? null) : null);
  const openMonths = $derived(openRowData ? (byInstrument.get(openRowData.instrument) ?? []) : []);
  const openTotals = $derived.by(() => {
    let bars = 0;
    let missing = 0;
    let complete = 0;
    let unproven = 0;
    for (const m of openMonths) {
      bars += m.rows;
      missing += m.short;
      if (m.sole) unproven += 1;
      else if (m.short === 0) complete += 1;
    }
    return { bars, missing, complete, unproven, n: openMonths.length };
  });

  /* THE CAST IS THE ONE FACT `querySelector` CANNOT KNOW AND THIS FILE CAN.
     `querySelector` answers `Element`, which is the interface an `<svg>` node
     and an XML node also satisfy, and `Element` has no `focus` — only
     `HTMLElement` does. `.dclose` is the drawer's own close control, declared
     forty lines into the markup below as a `<button>`, so it is an
     `HTMLButtonElement` and always has been; the type is narrowed here rather
     than the call being made conditional, because a conditional would turn
     "this selector matched nothing" — which would mean the drawer rendered
     without its close button, a real defect — into a keyboard trap that
     reports nothing. `?.` still guards the ordinary case: the effect can run
     in the frame before the drawer's children exist. */
  $effect(() => {
    if (openKey && drawerEl)
      /** @type {HTMLElement | null} */ (drawerEl.querySelector('.dclose'))?.focus();
  });

  /* A NEW VIEW STARTS AT THE TOP. Keeping the old offset after a filter
     change lands the operator in the middle of a list they have not seen. */
  let stamp = $state(0);
  /* ONE PLACE THAT PUTS THE GRID BACK AT ITS TOP, called by both effects
     below. It was written out twice for one edit and that is exactly how the
     two copies drift — one of them gaining a reset the other never got. */
  function restartAtTop() {
    stamp += 1;
    if (scroller) scroller.scrollTop = 0;
    scrollTop = 0;
    cursor = -1;
  }
  $effect(() => {
    /* THE TWO NEW RUNGS ARE IN HERE FOR THE SAME REASON THE OTHERS ARE. A
       universe or a month window changes WHICH rows exist, not merely their
       order, so an offset kept across the change lands the operator halfway
       down a list they have never seen — and the row under the cursor is a
       different row than the one they left. */
    void [
      typed,
      month,
      kind,
      holesOnly,
      universe,
      fromMonth,
      toMonth,
      /* THE CONTRACT RUNG IS IN HERE FOR THE SAME REASON THE OTHERS ARE: a
         strike or a rung changes WHICH rows exist, not merely their order. */
      strikePick,
      mnyPick,
      sortKey,
      desc,
      /* THE BAR GRID'S OWN SORT IS IN HERE FOR THE SAME REASON THE CENSUS
         GRID'S IS: a re-order changes which rows the FIRST page holds, and
         a reader left on page 12 of a re-sorted run is standing somewhere
         they never chose. */
      barSortKey,
      barDesc,
      /* AND THE READ BUDGET, because it changes which FILES were opened and
         therefore which bars exist at all. */
      readBudget
    ];
    untrack(() => {
      restartAtTop();
      /* A NARROWING RETURNS THE READER TO PAGE ONE. Keeping the ordinal
         across a change of WHICH ROWS EXIST is the same defect as keeping
         the scroll offset, one altitude up: page 12 of the old run is a
         different twelve rows in the new one, and very often none at all.
         `pageNow` already clamps, so this is not a correctness fix - it is
         the difference between landing at the top of the answer and landing
         at the end of it. */
      page = 1;
    });
  });

  /* MOVING BETWEEN PAGES, BETWEEN PAGE SIZES OR BETWEEN THE TWO GRIDS
     STARTS AT THE TOP TOO - and it is a SEPARATE effect, because this one
     must not write `page`. An effect that both reads `page` and resets it
     to 1 is a control that cannot leave its first page. */
  $effect(() => {
    void [page, pageSize, view];
    untrack(restartAtTop);
  });
  $effect(() => {
    void rows;
    untrack(() => (stamp += 1));
  });

  /* Rows animate in when the SET changes, and never while scrolling — a list
     that re-animates under the scrollbar is a list that cannot be read. */
  let entering = $state(false);
  $effect(() => {
    void stamp;
    entering = true;
    const t = setTimeout(() => (entering = false), 420);
    return () => clearTimeout(t);
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

  const SKELETON = Array.from({ length: 24 }, (_, i) => i);
</script>

<svelte:window onkeydown={onWindowKey} onpointerdown={onWindowDown} />

<div class="pane db">
  <div class="pane-head">
    <span class="pane-title">DB · {feedName}</span>

    <span class="spacer"></span>

    <!-- FRESHNESS IS A CLAIM, AND A FAILED READ CANNOT MAKE IT.
         This line sits ABOVE the error panel and outside it, so it renders on
         every state including the failed one. `fetchedAt` is cleared on a FEED
         CHANGE and nowhere else — the `catch` deliberately leaves it, because
         "when did this feed's store last answer" is the one fact a failed
         refresh must not destroy. What it must NOT do is state that fact as
         CURRENT: `as of 15:29` over `The store manifest could not be read` is
         the page reporting a freshness it does not have, which is exactly the
         fallback that hides a failure `CLAUDE.md` §4 bans — and it is worse
         than a blank because the minute is real, just not this read's.

         So the stamp survives and the WORD changes. `last good` names it as
         the previous SUCCESSFUL read; when there has never been one there is
         no minute to name and the line says that instead of printing the
         epoch. `clock` already refuses `0`. -->
    <span
      class="asof"
      title={error
        ? 'The last SUCCESSFUL read of /store.json for this feed. The read below it failed — nothing on this page was counted from it.'
        : 'When /store.json was last read for this feed'}
    >
      {#if loading && rows.length}
        <span class="dot acc live"></span> refreshing…
      {:else if error}
        <span class="stale"
          >{#if fetchedAt}last good: {clock(fetchedAt)}{:else}— never read successfully{/if}</span
        >
      {:else}
        as of {clock(fetchedAt)}
      {/if}
    </span>
    <button
      class="btn ghost sm"
      type="button"
      onclick={() => refreshStore()}
      disabled={loading}
      title="Re-read /store.json. The store grows while a pull runs."
    >
      Refresh
    </button>
  </div>

  {#if error}
    <!-- LOUD, AND NAMED. The actual failure, and the way to retry it. -->
    <div class="blank fade-in" role="alert">
      <h2>The store manifest could not be read</h2>
      <p class="err">{error}</p>
      <p>
        This page shows nothing rather than guessing. Until <code>/store.json</code> answers, no
        count on this page would be trustworthy — including a zero.
      </p>
      <button class="btn primary" onclick={() => refreshStore()}>Try again</button>
      <!-- THE FEED RUNG SURVIVES THE FAILURE, and this is the state that
           most needs it: one feed's store is unreadable and the operator's
           next move is another feed. Without it the only way out of a failed
           read would be the browser's back button. -->
      <section class="strip solo" aria-label="Query">{@render feedRung()}</section>
    </div>
  {:else if !feeds.active}
    <!-- AN UNMADE CHOICE IS NOT AN EMPTY STORE, and this branch exists because
         the page could not tell them apart. With no feed selected the fetch
         effect returns before it asks for anything, so `rows` stays empty and
         the next branch down said "Nothing stored for —" and "this feed has
         never landed a bar" — a definite claim about a feed nobody had picked,
         over a store nobody had read. The sentence comes from `blocked`, so it
         cannot drift from what the tiles would say. -->
    <div class="blank fade-in">
      <h2>No feed chosen</h2>
      <p>{blocked?.why ?? 'No feed is selected, so no store has been read.'}</p>
      <!-- THE CONTROL THAT ANSWERS THE HEADING, on the panel that states the
           problem. The shortcut buttons below are a different offer — they
           name the feeds that HOLD rows — and neither replaces the other: a
           feed with nothing in it is still a feed an operator may want to
           select before pulling into it. -->
      <section class="strip solo" aria-label="Query">{@render feedRung()}</section>
      {#if elsewhere.length}
        <div class="alt">
          <span class="lbl">Rows exist under</span>
          {#each elsewhere as e (e.feed.wire)}
            <button class="utab" onclick={() => (feeds.active = e.feed.wire)}>
              {e.feed.display}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {:else if loading && rows.length === 0}
    <!-- SKELETONS, SHAPED LIKE WHAT IS COMING. A spinner claims "something is
         happening"; a skeleton claims "a summary, a month band and a table go
         here", which is the useful half of the claim.

         IT IS ON THE SAME BOARD AS THE CONTENT. A skeleton laid out to
         different metrics than the thing it stands in for is a placeholder
         that has to be re-read once the data lands, and the reflow when it
         does is the exact jolt the skeleton existed to prevent. -->
    <div class="board">
    <!-- THE SKELETON IS THE PANEL, TRACK FOR TRACK. It stands in for the two
         control STRIPS and the anchor beneath them, which is what actually
         lands — a placeholder laid out to different metrics than the thing it
         replaces has to be re-read once the data arrives, and the reflow when
         it does is the exact jolt the skeleton existed to prevent. Same
         `.segs`, same `.strip` of `.cell` rungs, same `.anchor` at the foot. -->
    <div class="segs" aria-hidden="true">
      {#each [0, 1, 2] as i (i)}
        <span class="seg"><span class="skel line" style="width:{54 + i * 9}px"></span></span>
      {/each}
    </div>
    <!-- THE FIRST CELL IS REAL WHILE THE REST ARE PLACEHOLDERS, and the count
         is unchanged: six cells, one of them the feed rung and five standing
         in. A read that is taking a long time is a state in which the operator
         may well want a different feed, and a skeleton cannot be pressed.

         SO THE SECTION IS NOT `aria-hidden` ANY MORE — it holds a live control
         — and the attribute moved onto the five placeholder cells, which is
         where it belonged: a `.skel` line has nothing to announce. -->
    <section class="strip" aria-label="Query">
      <span class="lead">Query</span>
      {@render feedRung()}
      {#each [0, 1, 2, 3, 4] as i (i)}
        <div class="field" aria-hidden="true">
          <span class="skel line" style="width:{44 + ((i * 11) % 26)}px"></span>
          <span class="skel" style="width:{96 + ((i * 17) % 40)}px;height:18px;border-radius:5px"
          ></span>
          <span class="skel line" style="width:{56 + ((i * 9) % 24)}px"></span>
        </div>
      {/each}
    </section>
    <section class="strip sub" aria-hidden="true">
      <span class="lead">Contract</span>
      {#each [0, 1] as i (i)}
        <div class="field mcell">
          <span class="skel line" style="width:{40 + i * 14}px"></span>
          <span class="skel" style="width:{110 + i * 20}px;height:18px;border-radius:5px"></span>
          <span class="skel line" style="width:{64 + i * 10}px"></span>
        </div>
      {/each}
    </section>
    <div class="anchor" aria-hidden="true">
      <span class="skel line" style="width:150px;height:19px"></span>
      <span class="px"><span class="skel line" style="width:78px;height:19px"></span></span>
      <p class="note quiet"><span class="skel line" style="width:64%"></span></p>
    </div>
    <div class="tbl">
      <div class="tbl-scroll">
        <div class="tbl-inner">
          <div class="thead">
            {#each COLS as c, i (i)}
              <div class="th" class:numh={c.num} class:inst={i === 0}>{c.label}</div>
            {/each}
          </div>
          <div>
            {#each SKELETON as i (i)}
              <div class="trow skelrow" class:odd={i % 2 === 1}>
                <span class="cell inst"><span class="skel line" style="width:{52 + ((i * 7) % 30)}%"></span></span>
                <span class="cell"><span class="skel line" style="width:80%"></span></span>
                <span class="cell"><span class="skel line" style="width:60%"></span></span>
                <span class="cell"><span class="skel line" style="width:74%"></span></span>
                <span class="cell"><span class="skel line" style="width:52%"></span></span>
                <span class="cell"><span class="skel line" style="width:66%"></span></span>
                <span class="cell"><span class="skel line" style="width:88%"></span></span>
                <span class="cell"><span class="skel line" style="width:50%"></span></span>
                <span class="cell"><span class="skel line" style="width:50%"></span></span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
    <p class="loadnote" role="status">Reading <code>/store.json?feed={feeds.active ?? ''}</code>…</p>
    </div>
  {:else if rows.length === 0}
    <div class="blank fade-in">
      <h2>Nothing stored for {feedName}</h2>
      <p>This feed has never landed a bar in this store.</p>
      <!-- THE RUNG, ON THE PANEL THAT NAMES THE FEED IT IS ABOUT. The
           shortcuts below name only the feeds that HOLD rows; this names
           every feed the server lists, including the empty ones and the
           refused ones with the server's reason on them. -->
      <section class="strip solo" aria-label="Query">{@render feedRung()}</section>
      {#if elsewhere.length}
        <div class="alt">
          <span class="lbl">Rows exist under</span>
          {#each elsewhere as e (e.feed.wire)}
            <button class="utab" onclick={() => (feeds.active = e.feed.wire)}>
              {e.feed.display}
            </button>
          {/each}
        </div>
      {/if}
      <a class="cta" href="/ingest">Pull data on Ingest →</a>
    </div>
  {:else}
    <!-- ==================================================================
         THE BOARD. Every band below is a PANEL on a dark field rather than a
         full-bleed row against a hairline, which is the whole difference
         between this and a settings page: the eye finds a group by its edge,
         and the gaps between the groups are what make a dense row inside one
         readable. It is a flex column with `min-height: 0` for exactly one
         reason — the table below is `flex: 1` and windows itself against the
         height it is given, and a column that will not shrink hands it the
         whole document instead.
         ================================================================== -->
    <div class="board">

    <!-- ==================================================================
         THE QUERY STRIP.

         WHAT STOOD HERE AND DOES NOT ANY MORE: a row of six summary tiles, a
         universe chip rail with a prose paragraph under it, a band of month
         cards, and — most recently — a `.sel` panel of `.pk` rungs copied out
         of /ingest. That copy was the mistake being undone: the owner asked for
         /ingest to have its OWN design, and two pages wearing one vocabulary is
         how a change to either becomes a change to both.

         EVERY RUNG IS NOW A `.cell` IN A `.strip`, which is the vocabulary the
         approved /db design speaks: a `.lead` naming the strip, a caption, the
         control, and one clipped `.count` clause under it carrying what the
         control cannot fit. Nothing the deleted blocks stated is lost — every
         figure the tiles carried is in the anchor's counted line, every
         sentence the paragraphs carried is the clause under the control it is
         about, with the whole of it on that control's `title`.

         READING ORDER IS THE CASCADE: feed → universe → month window → month →
         instrument → text → holes, then the contract strip below. Every control
         is authored after the one that narrows it, so tab order follows the
         markup for free.
         ================================================================== -->
    <section class="strip" aria-label="Query">
      <span class="lead">Query</span>

      <!-- THE FEED IS THE PAGE'S SCOPE AND IT IS CHOSEN HERE — the strip's
           first cell, because every rung after it is this feed's answer. One
           control for one value, on this route and in this bar: the shell's
           `FEED_OWNED` list carries `/db`, so the top bar draws no picker
           here. See the snippet at the foot of this file for why it is a
           snippet and not markup in place. -->
      {@render feedRung()}

      <!-- THE UNIVERSE RUNG — the cascade's second step. `tierRefusal` is
           unchanged and still whole on the row it belongs to, and a refusal is
           DRAWN AND DISABLED rather than hidden. -->
      <div class="field">
        <span class="lab" title="Which membership to hold this store against — the cascade's second rung."
          >Universe</span
        >
        <!-- THE SAME SEARCHABLE `Picker` AS EVERY OTHER RUNG. It was a
             hand-rolled menu with no filter box; with 200+ NIFTY Total Market
             members behind some universes, a list you can only scroll is a list
             you cannot use. `single`, like everything on this page. -->
        <Picker
          single
          filter
          label="universes"
          summary={chosenUniverse.label}
          rows={UNIVERSES.map((u) => ({
            key: u.key,
            name: u.label,
            detail: u.key
              ? universeCount.get(u.key)
                ? `${fmt(universeCount.get(u.key).held)} of ${fmt(universeCount.get(u.key).members)} held`
                : 'not counted'
              : `${fmt(deco.length)} held`,
            why: u.key ? undefined : 'The only setting that reaches stored data whose instrument this feed\u2019s master no longer lists.'
          }))}
          selected={new Set([universe])}
          onchange={(/** @type {Set<string>} */ sel) => (universe = [...sel][0] ?? '')}
        />
        <!-- THE COUNTED CLAUSE THAT WAS A PARAGRAPH. Every figure comes from the
             rows and the master on hand, and the one sentence that is not a
             figure is a refusal naming why a figure is absent. It clips to one
             line; the whole of it is on the button's `title` above and in this
             node for anything that reads the document rather than looks at it.
             `CLAUDE.md` §4: degrade loudly and name the reason. -->
        <!-- ══ THE COUNTED CLAUSE GOES; THE TWO SENTENCES THAT ARE NOT COUNTS
             STAY ══
             `N held · no membership list` drew on every load under a button
             reading `Everything`, and the picked-universe branch counted members
             held against members known — both facts, neither of them news, and
             both now on the control's `title`.
             WHAT STAYS IS WHAT A COUNT CANNOT SAY. `universeRefusal` is a
             refusal, and `outsideMaster` names rows that ARE held and that no
             membership universe can reach — an operator who does not read that
             sentence concludes the store is smaller than it is. Both draw only
             when true.
             THE `{#if}` IS OUTSIDE THE SPAN, not inside it. A `.note` carries
             its marker as `::before`, so an emptied one is not invisible — it is
             a bare `·` under a control. /ingest learned this the hard way. -->
        {#if universeRefusal || (outsideMaster && outsideMaster.months > 0)}
          <span class="note" class:warn={Boolean(universeRefusal)}>
            {#if universeRefusal}
              {chosenUniverse.label} is not narrowing this table — {universeRefusal}
            {/if}
            {#if outsideMaster && outsideMaster.months > 0}
              {fmt(outsideMaster.months)} stored instrument-month(s) across {fmt(
                outsideMaster.instruments
              )} instrument(s) are not listed by {feedName}'s master — they are held, no membership
              universe can reach them, and Everything is where they show.
            {/if}
          </span>
        {/if}
      </div>

      <!-- THE INSTRUMENT RUNG. It WRITES THE TEXT BOX beside it and owns no
           state of its own — a row is ticked exactly when the box holds that
           row's key, so typing over a pick unticks it with nothing to
           synchronise and the two can never disagree.

           HIDDEN, NOT DISABLED, WHEN THERE IS NOTHING TO ENUMERATE. Its parent
           rungs can leave it with no members at all; an empty dropdown is a
           control offering a choice that does not exist. The text box stays
           either way — it is the `/` target and it can reach a month as well as
           an instrument.

           ONE `Picker`, THE SHARED COMPONENT, unchanged: it draws its own
           `.picker` root, its own filter box and its own tick per row, so this
           rung cannot drift from the same control on /markets and /ingest. -->
      {#if instrumentRows.length > 0}
        <div class="field">
          <span class="lab" title="Held, not offered — every name here is a name this store can prove it has."
            >Instrument</span
          >
          <Picker
            single
            filter
            label="instruments"
            summary={picked
              ? (instrumentOffered.find((r) => r.key === picked)?.sym ?? picked)
              : typed
                ? `Typed · ${fmt(instrumentRows.length)} held`
                : `${fmt(instrumentOffered.length)} offered`}
            rows={[
              /* NO "ALL INSTRUMENTS" ROW, for the reason `All rungs` went from
                 Timeframe: this rung is `single`, so "all of them" is not a
                 member of the set it offers. One name is chosen at all times and
                 the head row was the only way to leave it unchosen. */
              ...instrumentOffered.map((r) => ({
                key: r.key,
                name: r.sym,
                /* BACKWARD-LOOKING, ALWAYS. "held" is what /store.json can
                   prove; what the vendor could serve is /ingest's question and
                   no endpoint this page calls can answer it. */
                /* THE KIND, WHICH IS WHAT /ingest PUTS HERE — `equity`, `index`.
                   The held count is a fact about THIS page and belongs on the
                   row's title beside it, not in the column /ingest fills with
                   the instrument's type. */
                detail: r.kind || (r.months === 0 ? 'nothing stored' : 'in this master'),
                title:
                  r.months === 0
                    ? `${r.sym} — ${r.key}. Nothing stored for it at this selection.`
                    : `${r.sym} — ${r.key}. ${fmt(r.months)} month(s) held${r.short > 0 ? `, ${fmt(r.short)} short` : ''}.`
              }))
            ]}
            selected={new Set([picked])}
            onchange={(/** @type {Set<string>} */ sel) => (filter = [...sel][0] ?? '')}
          />
          <!-- RESTATEMENT, BOTH WAYS. `1 instrument(s) held here` under a
               button reading `All · 1 held`, and the picked branch reprinted the
               name the button already shows. Neither fact is lost: the `h1.sym`
               below already carries both on its own title, in fuller words. -->
        </div>
      {/if}

      <!-- THE SEARCH BOX IS GONE. It was the instrument rung's text input, not a
           rung of its own, and the Instrument picker beside it has its own filter
           box — two search fields for one question. Both wrote `filter`, so the
           picker keeps the state and `typed`/`textMatched` are unchanged; what is
           lost is the `/` shortcut and prefix matching against a MONTH, which the
           month rung no longer exists to use. -->

    <!-- ==================================================================
         THE SEGMENT TABLIST — `.segs`, AND IT IS ABOVE WHAT IT FILTERS.

         A tablist belongs over the set it narrows, which is where the approved
         design puts it, so it is the first thing under the pane head and ahead
         of the query strip. The FEED still outranks it: this whole branch only
         renders once a store has answered, so there is never a tab here
         counting a feed nobody chose.

         IT WAS A `Picker`, AND WHAT IT LOST IS ITS OWN RESTATEMENT. The clause
         under it read "INDEX · CASH · FNO — showing INDEX" and "3 segment(s)
         held": the enumeration and the count of an enumeration that is now
         drawn, one tab per segment, with each tab's own counted total on its
         face. A line naming what the row beside it already draws is the page
         repeating a control it has already drawn.

         "SEGMENT" IS THE STORE'S OWN SECOND FIELD — INDEX, CASH, FNO — and NOT
         the mockup's spot/futures/options taxonomy. Relabelling it would rename
         three sets into three other sets that do not have the same members.
         ================================================================== -->
      <!-- ALWAYS DRAWN, DISABLED WHEN THERE IS NOTHING TO CHOOSE.
           It was `{#if kinds.length > 1}`, on the reasoning that one segment is
           not a choice between segments. True, and it is the wrong conclusion
           twice over. /ingest draws Segments unconditionally — three rows, two
           of them refusing with their reason on their face — so a reader
           comparing the two strips found a rung on one page and a gap on the
           other. And it is this file's own rule besides: a rung that cannot be
           used is DRAWN AND DISABLED, never hidden, because "why is there no
           segment control" is exactly the question `CLAUDE.md` §4 says must be
           answered out loud rather than by an absence. The single segment the
           store holds is now stated rather than implied. -->
      <!-- NOT DISABLED, AND /ingest IS THE PRECEDENT. Its segments menu carries
           no `disabled` at all: it draws all three rows, `tuck`s the two it
           cannot serve, and lets the operator open them. This cell greyed itself
           out whenever the store held fewer than two segments — which is the
           ordinary case — so the control that answers "what segments are in
           here" was dead exactly when somebody would ask it. A live control
           holding one row answers the question; a grey one refuses it. -->
      <div class="field">
        <span class="lab" title="The store's own second field — INDEX, CASH, FNO.">Segment</span>
        <!-- A PICKER, NOT A TAB ROW. It was the only rung drawn as tabs, which
             made it look like a different kind of thing from the eight around
             it; it writes the same `kind` the cascade reads. Two or three
             options today, and the filter box costs nothing — nine rungs that
             behave nine ways is nine things to learn. -->
        <!-- THE FACE COUNTS SEGMENTS, NOT ROWS. Every other rung's "All ·" names
             how many OPTIONS its menu holds — 1 instrument, 9 rungs — and this
             one printed `textMatched.length`, the row count, so it read
             "All · 9" beside a store that holds exactly one segment. -->
        <Picker
          single
          filter
          label="segments"
          title={kinds.length < 2
            ? `${kinds.length === 1 ? kinds[0][0] : 'Nothing'} is the only segment this store holds, so there is nothing to narrow. The store's own second field — INDEX, CASH, FNO.`
            : `${fmt(segmented.length)} instrument-month(s) across ${fmt(kinds.length)} segment(s). The store's own second field — INDEX, CASH, FNO.`}
          summary={kind
            ? (SEG_VIEW.find((s) => s.key === kind)?.name ?? kind)
            : `${fmt(SEG_VIEW.length)} offered`}
          rows={segmentRows}
          selected={new Set([kind])}
          onchange={(/** @type {Set<string>} */ sel) => (kind = [...sel][0] ?? '')}
        />
        <!-- BOTH BRANCHES RESTATED THE FACE. `INDEX — the only segment this
             store holds` under a button reading `All · 1`, and it wrapped to
             two lines, so this was the one cell that made the strip's row
             heights ragged. On the control's `title` now. The cell already
             carries `.off` when there is nothing to choose, which is the
             visible refusal. -->
      </div>

      <!-- THE BAR-LENGTH RUNG — the cascade's third step, and the control the
           approved design draws as `<select id="tf">` beside the others.

           IT IS A `.picker` AND NOT A `<select>`, like every other rung in
           this strip. The design's `<select>` is shorthand for "a dropdown
           goes here"; a native one cannot carry a per-row count, cannot carry
           a per-row `title`, and cannot be searched — and the counted row is
           the whole reason this page's menus exist.

           EVERY RUNG THE STORE HOLDS IS OFFERED AND NO OTHER. The bar-length
           ladder in `TF_SECS` is an ORDERING, not a list of choices: offering
           `5min` because the ladder names it, on a store that holds no
           five-minute bar, would be a control promising a set that does not
           exist. The rows are counted off the rows. -->
      <!-- ALWAYS DRAWN, DISABLED WHEN EMPTY — the same correction as Segment
           above, for the same two reasons. `tfRefusal` already existed to say
           WHY there is no rung to pick, and gating the whole cell on
           `tfAll.length > 0` deleted the control that was carrying that
           sentence, so the one state the refusal was written for was the one
           state the reader never saw it in. The `Picker` is already
           `disabled={Boolean(tfRefusal)}`. -->
      <div class="field" class:off={Boolean(tfRefusal)} title={tfRefusal ?? undefined}>
        <span class="lab" title="Bar length — the rung each row is stored at.">Timeframe</span>
        <!-- SEARCHABLE, LIKE THE REST. Nine rungs today and ten in
             `store::path::Timeframe::KNOWN`; the list is short now and the
             filter costs nothing, and consistency across the nine rungs is
             worth more than saving a filter box on one of them. -->
        <Picker
          single
          filter
          label="rungs"
          disabled={Boolean(tfRefusal)}
          title={tfRefusal
            ? `No rung stored — ${tfRefusal}`
            : `${timeframe ? `${fmt(timeframed.length)} of ${fmt(universed.length)} instrument-month(s) at ${timeframe}. ${tfNote(timeframe)}` : `${fmt(tfAll.length)} rung(s) held, ${fmt(segmented.length)} instrument-month(s) across them`} Only rungs this store actually holds are listed — the list is counted off the rows, never off a fixed ladder.`}
          summary={tfRefusal ? 'No rung stored' : timeframe || 'Pick a rung'}
          rows={[
            /* NO "ALL RUNGS" ROW. A table mixing rungs is a table whose columns
               stop meaning one thing: this file already records that "a month
               holding both 1min and 1day rows has no single session size", so
               `All · 9` produced a session count, a bars-per-session and a
               change-% that belonged to no rung in particular. Exactly one rung
               is selected at all times — seeded from the store by the effect
               beside `stranded`. */
            ...tfAll.map(([tf, n]) => ({
              key: tf,
              name: tf,
              detail: `${fmt(n)} held`,
              title: tfNote(tf)
            }))
          ]}
          selected={new Set([timeframe])}
          onchange={(/** @type {Set<string>} */ sel) => (timeframe = [...sel][0] ?? '')}
        />
        <!-- ══ THE COUNTS GO; THE REFUSAL STAYS ══
             `1 rung(s) held` drew under a button reading `All · 1`, and the
             picked branch counted rows at a rung the button already names. Both
             are on the control's `title` now — the same cut /ingest made to its
             five captions, for the same reason.
             `tfRefusal` is NOT a count and does not move: it is the §4 case and
             draws only when there is a refusal to name. The `{#if}` is OUTSIDE
             the span, because a `.note` carries its marker as `::before` and an
             emptied one paints a bare `·` under the control. -->
        {#if tfRefusal}
          <span class="note warn">No bar length — {tfRefusal}</span>
        {/if}
      </div>
      <!-- ==================================================================
           THE DAY WINDOW — SPOT ONLY, AND ABSENT RATHER THAN DISABLED.
           A spot series runs forever, so "show me the 12th" is a real question.
           A CONTRACT is defined by its expiry and its bars stop there, so a
           second date range asks something the contract has already answered.
           Gated on `dayWindowApplies`, which is simply "no expiry chosen".
           ================================================================== -->
      <!-- THE TWO ENDS ARE ONE RUNG, AND THE RUNG TAKES THE WHOLE ROW.
           As two independent grid items they were placed like any other pair,
           so at five tracks From landed in column 5 and To wrapped to column 1
           of the next row — the two ends of one window at opposite corners of
           the panel, with three unrelated rungs between them. /ingest draws
           the identical control and answers this with `.pk.wide2`: the day
           window is `grid-column: 1 / -1` and holds both ends in a `.dates`
           flex row, capped so a pair of date fields cannot spread across
           1,200px and read as the panel's most important control when it is
           its last one. Same rule, same reason, same numbers. -->
      <div class="field wide">
        <!-- "DAYS TO SHOW" IS GONE AND ONLY THE HEADING WAS — the same cut
             /ingest made to "Days to pull". It stacked a group label over two
             fields that already carry their own, FROM DATE and TO DATE, and no
             other control on this strip has that extra level.
             The sentence it held moves to `.dates`, the element that IS the
             window, rather than being deleted with the heading that carried
             it. -->
        <!-- THE PRODUCT'S OWN CALENDAR, AND THIS FILE HAS DEMANDED ONE IN
             WRITING FOR LONGER THAN IT HAD ONE. Twenty lines below, the month
             window's comment reads "The product's OWN calendar, never the
             platform's ... There is not one on this page and there must never
             be" — and directly above it sat two `<input type="date">`.

             `<input type="date">` renders in the OS locale, so `02/09/2024` is
             the 2nd of September to one reader and the 9th of February to
             another, and no rule on this page can reach its text or its popup
             to say which. `dd Mon yyyy` cannot be misread in any locale, and it
             is what `$lib/dates.js` already renders every other date in this
             product as — so until now the DISPLAYED day and the EDITABLE day
             disagreed on the same screen.

             `$lib/DayField.svelte` is that control and it was written for
             exactly this: its own header names /db's platform input as the
             defect it exists to remove. It had no importer anywhere in the
             tree. One component, both pages, and the two can no longer drift —
             the same argument `Picker` settles for every other rung here. -->
        <div
          class="dates"
          title="The window in days. Spot only — a contract's window is its expiry, so these are ignored once an expiry is chosen."
        >
          <DayField
            label="From date"
            value={fromDay}
            min={spanFloor}
            max={toDay || spanCeil}
            onchange={(/** @type {string} */ d) => (fromDay = d)}
            boundsReason={dayBounds[0]
              ? `This store holds ${dayLabel(dayBounds[0])} to ${dayLabel(dayBounds[1])} for the current selection; days outside it are struck through because there is no bar there to show. Only bars on or after this day — spot only, since a contract's window is its expiry.`
              : 'No bar is loaded, so there is no held range to bound this field with. Only bars on or after this day.'}
          />
          <DayField
            label="To date"
            value={toDay}
            min={fromDay || spanFloor}
            max={spanCeil}
            onchange={(/** @type {string} */ d) => (toDay = d)}
            boundsReason={dayBounds[1]
              ? `This store holds ${dayLabel(dayBounds[0])} to ${dayLabel(dayBounds[1])} for the current selection; days outside it are struck through because there is no bar there to show. Only bars on or before this day — spot only, since a contract's window is its expiry.`
              : 'No bar is loaded, so there is no held range to bound this field with. Only bars on or before this day.'}
          />
        </div>
      </div>
      



      <!-- THE MONTH WINDOW — two ends, so it is TWO `.cell`s, exactly as the
           design draws a From and a To. The product's OWN calendar, never the
           platform's: `<input type="date">` renders in the OS locale, so
           `02/09/2024` is two different days depending on who is reading it,
           and no rule in this file can reach either its text or its popup.
           There is not one on this page and there must never be.

           IT WORKS IN WHOLE MONTHS, and that is the unit of the data rather
           than a simplification — a row here IS an instrument-month.

           IT RENDERS ONLY WHEN THERE IS A WINDOW TO CHOOSE. One month is not a
           range, and a control whose every setting produces the same table is a
           control that has nothing to say. -->
      <!-- THE FROM/TO RANGE IS GONE. ONE MONTH CONTROL, SINGLE SELECT.
           ==================================================================
           MONTH was secretly TWO controls: this from/to range, and the single
           month picker below. A range is a selection of many months, which is
           a multi-select — and every control on this page is single select.
           Two controls for one rung also meant the rung was applied at two
           different depths of the cascade, which is the one thing the prefix
           rule cannot express.

           `fromMonth` and `toMonth` stay declared and stay empty, so
           `windowed` is `timeframed` by definition and the cascade collapses
           to one month step. The calendar snippet and its handlers are dead
           and are removed in the commit that follows this one, separately,
           because deleting ~250 lines across thirty-three sites in the same
           edit as a behaviour change is how a page stops rendering with
           nothing to point at.
           ================================================================== -->

      <!-- MONTH IS NOT ONE OF THE RUNGS.

           The nine are feed, universe, instrument, segment, timeframe,
           expiry, strike, moneyness, side. Month was never among them, and it
           was the most confused rung on the page besides: two controls for one
           thing, a from/to range that was a multi-select on a single-select
           page, and a single-month picker beneath it.

           `month` stays declared and permanently empty, so `scoped` is
           `windowed` by definition and the cascade runs feed -> universe ->
           instrument -> segment -> timeframe and straight on to the contract
           rungs. Every month the selection reaches is in the table; the row
           count and the pager are what say how many. -->



      <!-- THE HOLES-ONLY TOGGLE IS GONE. It was a tenth rung on a page whose
           rungs are nine, and it filtered on a property of the DATA rather
           than on an identity, which is not what this strip is for.
           `holesOnly` stays declared and permanently false, so
           `matchedPreContract` is `scoped` by definition. -->

    </section>

    <!-- ==================================================================
         THE CONTRACT STRIP. Authored after the query strip because a strike is
         finer than a segment and a rung is finer than a strike.

         IT IS ALWAYS OPEN AND ITS CELLS GO `.off` INSTEAD. A strip that
         collapsed when there is no chain would take the REFUSAL with it, and
         "why is there no strike control" is exactly the question `CLAUDE.md` §4
         says must be answered out loud rather than by an absence.

         BOTH PANELS ARE `Picker` — one renderer, two data sources. The filter
         box, the checkbox on every row, Select-all-shown and the right-aligned
         detail column all come from it and cannot drift away from the other
         pages.

         THE REASON IS ON THE CELL AND ALSO UNDER IT, and the second is the one
         that matters: `Picker` takes no `title`, and a refusal that lives only
         on a tooltip is loud to a pointer and silent to everything else.
         `undefined` rather than `''` so a rung with nothing to explain carries
         no attribute at all.
         ================================================================== -->
    <!-- ==================================================================
         DRAWN ONLY WHEN A CONTRACT EXISTS TO CHOOSE.

         This strip used to render unconditionally, so a store holding nothing
         but indices and equities — which is every store this build can
         produce — showed FOUR controls reading "No contract stored", "No
         strike stored", "No ladder" and "No option stored", above a paragraph
         explaining why each was dead. Four dead controls and their obituary,
         on every page load, for a rung that cannot exist here.

         `CLAUDE.md` §4 says degrade LOUDLY and name the reason, and that is
         why this was built that way. But §4 is about a FAILURE being hidden.
         "This store has no futures" is not a failure — it is the normal and
         permanent state of a spot-index store, and `expiryRefusal`'s own text
         says so: nothing in this build can fetch a contract at all. Drawing
         four disabled controls to announce a non-event is not loudness, it is
         noise, and it pushed the actual query controls off the first screen.

         The reason is not lost. The moment `expiriesAll` holds anything the
         strip returns with every refusal it ever had, and the one-line summary
         under the table still counts what is stored.

         ------------------------------------------------------------------
         THE GATE ABOVE IS WITHDRAWN, ON THE OWNER'S INSTRUCTION, AND THE
         ARGUMENT FOR IT IS LEFT STANDING BECAUSE IT WAS NOT A BAD ONE.

         `{#if !expiryRefusal}` collapsed this whole strip whenever no contract
         could be reached — which, on today's wire, is always. The reasoning
         above is sound as far as it goes: four permanently dead controls
         announcing a non-event is noise, not loudness, and the strip did cost
         the query rungs their place on the first screen.

         Two things overrule it.

         The owner asked for these four by name and in order — feed, universe,
         instruments, segments, timeframes, expiry, strike, moneyness, side —
         and asked for each to carry the same view, the same dropdown and the
         same search as every rung above it. A control that is absent has no
         view to be the same as. That is a decision about what this page IS,
         and it is the owner's to make, not this file's.

         And the consistency argument runs the other way now. /ingest draws its
         two contract segments unconditionally, both refusing, both with the
         reason on their face; the two pages are meant to read as one product.
         The cost the comment measured is also mostly gone: the strip is a
         wrapping grid since D-0153, so four rungs are one row, not four.

         What made this cheap to reverse is that nothing was deleted when it
         was hidden. Every cell still carries its own `.off` and its own
         refusal text, exactly as the `.strip.sub` rule below still describes
         ("it is ALWAYS DRAWN ... a strip that collapsed would take the two
         REFUSALS with it") — a comment that had been true of the CSS and false
         of the markup ever since the gate went in. It is true of both again.
         ================================================================== -->
    <!-- ══ THE WHOLE STRIP FOLDS WHEN THERE IS NO CONTRACT TO NARROW ══

         The comment above argues this strip must be DRAWN because "/ingest
         draws its two contract segments unconditionally, both refusing, both
         with the reason on their face; the two pages are meant to read as one
         product." That reasoning is right and its premise is out of date:
         /ingest passes `tuck` to both its universe and its segments menus, so
         the rows it cannot serve sit behind ONE counted line reading "2 rows
         this feed cannot serve — show why". Folding here is what restores the
         consistency that comment was reaching for, not what breaks it.

         AND NOTHING IS HIDDEN, which is the distinction §4 turns on. The
         section is not gated away: it is drawn, its count is on the line, its
         reason is on the line, and one click opens all four cells with every
         refusal intact. That is `Picker`'s own `tuck` argument applied to a
         strip — "a list whose first screenful is entirely unusable teaches the
         operator no more than a hidden row does. It teaches less: it reads as a
         broken build."

         WHY IT IS EMPTY IS A FACT ABOUT THE BUILD, NOT ABOUT THE STORE, and
         `expiryRefusal` already says so in full: POST /pull/fno answers 503,
         "expired F&O has no local-archive path and no HTTP transport in this
         build", and every FUT/CE/PE master row is declined as
         `Skip::LiveContract`. No pull an operator can start will populate these
         four controls, which is exactly why four of them standing open with
         eight lines of amber text was reading as a broken page.

         OPEN WHENEVER THERE IS SOMETHING TO CHOOSE. `open={!expiryRefusal}` —
         the fold is the exception, not the default, so a store that does hold
         contracts is unchanged from before this commit. -->
    <!-- ══ ALWAYS DRAWN. WHAT VARIES IS WHICH RUNGS ARE IN IT ══
         It folded behind one line, and the fold carried its OWN `Contract` lead
         while the section inside kept another — so the page printed the word
         twice, which read as a stray empty box beside the heading.
         It does not fold now. The section is always open and each rung is drawn
         only when the chosen segment actually HAS it. That is better than the
         fold: a control that does not exist for a thing is not the same as a
         control disabled for it. See `contractRungs`. -->
    <section class="strip sub" aria-label="Contract">
      <span class="lead">Contract</span>
      {#if !contractRungs.expiry}
        <span class="note quiet"
          >Spot is a continuous series — no expiry, strike, ladder or side to narrow. Choose Expired
          futures or Expired options above and the rungs those contracts have appear here.</span
        >
      {/if}

      <!-- THE EXPIRY, AND IT IS THE STRIP'S FIRST CELL BECAUSE IT IS ITS
           COARSEST RUNG: a strike belongs to a contract, and a contract is an
           expiry. The approved design draws it first for the same reason.

           IT IS THE ONE CONTROL HERE THAT REFUSES BEFORE IT IS TOUCHED. The
           owner's rule is that a future or an option without the chosen expiry
           shows nothing, so `''` holds every contract row back rather than
           passing them all. The clause under it is therefore never decoration:
           it is the count of what is being withheld and the reason, which is
           the whole of `CLAUDE.md` §4 on one line.

           DISABLED ONLY WHEN THERE IS NOTHING TO CHOOSE, and it says which of
           two things that is on its own face — an unread store, or a store
           that holds no contract. A dead control with no reason on it is the
           thing §4 forbids.

           THE REASON IS SAID THREE TIMES AND THAT IS NOT REDUNDANCY. A
           DISABLED button does not fire pointer events in every browser, so
           its own `title` is the one tooltip that may never appear — the cell
           carries it too, and the clause under the control carries it in the
           document where anything that reads rather than looks will find it.
           `undefined` rather than `''` so a rung with nothing to explain
           carries no attribute at all. -->
      {#if contractRungs.expiry}
        <div class="field" class:off={Boolean(expiryRefusal)} title={expiryRefusal ?? undefined}>
          <span class="lab" title="Which contract — futures and options show nothing without one.">Expiry</span>
          <!-- THE SAME `Picker` THE OTHER THREE USE, so this control is
               searchable like them. It was 77 lines of hand-rolled dropdown with
               no filter box, which is why an expiry could only be found by
               scrolling. `single` because every control on this page is single
               select; `filter` because that is the whole point of the change. -->
          <Picker
            single
            filter
            label="expiries"
            disabled={Boolean(expiryRefusal)}
            summary={expiryRefusal
              ? 'No contract stored'
              : expiry
                ? dayLabel(expiry)
                : `None chosen · ${fmt(expiryOffered.length)} here`}
            rows={[
              {
                /* THE UNCHOSEN ROW IS DRAWN AND IT DOES NOT SAY "ALL". It is the
                   state that HOLDS CONTRACTS BACK — a bar belongs to one
                   contract, and two expiries under one heading would be a series
                   no contract ever traded. */
                key: '',
                name: 'None chosen',
                detail: `${fmt(contractHere)} contract row(s) held back`,
                why:
                  contractHere > 0
                    ? 'This is a refusal, not an empty store. Indices and equities carry no expiry and are unaffected — they are in the table below already.'
                    : undefined
              },
              ...expiryOffered.map((d) => ({
                key: d,
                name: dayLabel(d),
                detail: `${fmt(expiryCount.get(d) ?? 0)} instrument-month(s)`,
                title: `Contracts expiring ${dayLabel(d)} — the store's own key for it is ${d}.`
              }))
            ]}
            selected={new Set(expiry ? [expiry] : [])}
            onchange={(/** @type {Set<string>} */ sel) => (expiry = [...sel][0] ?? '')}
          />
          <span class="note" class:warn={Boolean(expiryRefusal) || expiryHeld > 0}>
            {#if expiryRefusal}
              No contract — {expiryRefusal}
            {:else if expiry}
              {fmt(expiryCount.get(expiry) ?? 0)} row(s) on this contract{#if expiryHeld > 0}
                · {fmt(expiryHeld)} on another expiry are not shown{/if}
            {:else if expiryHeld > 0}
              {fmt(expiryHeld)} future/option row(s) HELD BACK until an expiry is chosen — a bar
              belongs to one contract, so nothing here blends two
            {:else}
              {fmt(expiriesAll.length)} expiry/expiries stored · no contract row is in this selection,
              so nothing is being held back
            {/if}
          </span>
        </div>
      {/if}

      {#if contractRungs.strike}
        <div class="field mcell" class:off={Boolean(strikeRefusal)} title={strikeRefusal ?? undefined}>
          <span class="lab" title="An absolute price on the chain.">Strike</span>
          <Picker
            single
            filter
            label="strikes"
            disabled={chainAll.length === 0}
            summary={strikeSummary}
            rows={strikeRows}
            selected={strikePick}
            onchange={(/** @type {Set<string>} */ sel) => (strikePick = new Set(sel))}
          />
          <span class="note" class:warn={Boolean(strikeRefusal)}>
            {#if strikeRefusal}
              No strike chain — {strikeRefusal}
            {:else}
              {fmt(chainAll.length)} strike(s) · {strikeText(chainAll[0].strike)} to {strikeText(
                chainAll[chainAll.length - 1].strike
              )}{#if chainStep !== null}
                · grid {strikeText(chainStep)} apart{/if}<!--
                WHY EVERY ROW IN THE PANEL READS `no bars` WHILE THE GATE IS
                SHUT. The counts here are taken over `expiryGated`, so with no
                expiry chosen they are all zero — correctly, because that is what
                clicking one returns. Left unsaid, a panel of zeroes over a chain
                the store demonstrably holds reads as a broken count.
                -->{#if !expiry && expiryHeld > 0}
                · every count here is zero until an expiry is chosen — {fmt(expiryHeld)} contract
                row(s) are held back{/if}
            {/if}
          </span>
        </div>
      {/if}

      <!-- MONEYNESS IS A SEPARATE CONTROL AND STAYS ONE. A strike is an
           absolute price and a rung is a distance from spot; the same tick
           means two different queries and neither is a view of the other. -->
      {#if contractRungs.money}
        <div class="field mcell" class:off={Boolean(moneyRefusal)} title={moneyRefusal ?? undefined}>
          <span class="lab"
            title="A distance measured along that chain — the same 35,000 is ITM-2 for a call and OTM+2 for a put, which is why it is its own control."
            >Moneyness</span
          >
          <Picker
            single
            filter
            label="rungs"
            disabled={moneyRefusal !== null}
            summary={mnySummary}
            rows={mnyRows}
            selected={mnySelected}
            onchange={onMnyPick}
          />
          <!-- THE REFUSAL IS SAID ONCE. When there is no chain the ladder's
               reason STARTS with the chain's, so the clause narrows to the part
               that is still true after a chain arrives. -->
          <span class="note" class:warn={Boolean(strikeRefusal || moneyRefusal)}>
            {#if strikeRefusal}
              No ladder — a rung is a distance measured along a chain, so there is nothing to place
              one on. There would be none with a chain either: {NO_SPOT}
            {:else if moneyRefusal}
              The ladder is not drawn — {moneyRefusal}
            {:else}
              {fmt(ladderRungs.length)} rung(s), {ladderRungs[0]} to {ladderRungs[
                ladderRungs.length - 1
              ]} — every position this chain reaches on both sides of the money
            {/if}
          </span>
        </div>
      {/if}

      <!-- OPTION TYPE — the strip's last and finest cell, where the approved
           design draws it. Two values the store can hold and one that means no
           join: `CE`, `PE`, and `Both`, which is `''`.

           `Both` IS NOT "EVERY OPTION". It is the absence of a side filter, so
           it leaves every index, equity and future in the table — the same
           distinction the strike panel's empty Set carries, and the reason
           picking `CE` legitimately drops everything that is not a call.

           DISABLED ONLY ON A FACT ABOUT THE STORE — no option in it at all —
           and NEVER on a fact about the selection. Greying it because the
           current rungs left no option would be a filter with no way back: pick
           `CE`, narrow to a month with no calls, and the one control that could
           undo it is the one that just went dead. "This selection reaches no
           option" is said instead, on the clause, with the control still open
           and the expiry gate named as the usual cause. -->
      {#if contractRungs.side}
        <div
          class="field"
          class:off={Boolean(sideRefusal || sideEmpty)}
          title={sideRefusal ?? sideEmpty ?? undefined}
        >
          <span class="lab" title="Which side of the contract — calls or puts.">Side</span>
          <!-- SEARCHABLE AND SINGLE, LIKE EVERY OTHER RUNG. Two options today,
               and the consistency is the point: nine rungs that behave nine ways
               is nine things to learn. -->
          <Picker
            single
            filter
            label="sides"
            disabled={Boolean(sideRefusal)}
            summary={sideRefusal ? 'No option stored' : side ? side : `Both \u00b7 ${fmt(sideOffered.length)}`}
            rows={[
              { key: '', name: 'Both', detail: `${fmt(mnyFilteredCount)} row(s)` },
              ...sideOffered.map((s) => ({
                key: s,
                name: s,
                detail: `${fmt(sideCount.get(s) ?? 0)} row(s)`,
                title: s === 'CE' ? 'Calls.' : 'Puts.'
              }))
            ]}
            selected={new Set([side])}
            onchange={(/** @type {Set<string>} */ sel) => (side = [...sel][0] ?? '')}
          />
          <span class="note" class:warn={Boolean(sideRefusal || sideEmpty)}>
            {#if sideRefusal}
              No side — {sideRefusal}
            {:else if sideEmpty}
              No option here — {sideEmpty}
            {:else if side}
              {fmt(sideCount.get(side) ?? 0)} of {fmt(sidedHere)} option row(s) here are {side} — every
              row without a side is out of the table too
            {:else}
              {fmt(sidedHere)} option row(s) here · both sides, which narrows nothing
            {/if}
          </span>
        </div>
      {/if}
    </section>


    <!-- ==================================================================
         THE ANCHOR. What this query is looking at, the one headline figure,
         the two presses that are actions rather than filters, and the counted
         line under all three.

         THERE IS NO PRICE HERE AND NO SPARKLINE, and neither is an omission:
         `/store.json` serves this page no price at all — `chg_bps` is a RATIO
         and a ratio has no scale — so a `.px` carrying a traded number would be
         a figure this page invented. What `.px` carries instead is the one
         number the whole strip above exists to move: how many instrument-months
         the selection matched, with its own measured up/down flash, and how
         many the store holds behind it. It is lifted OUT of the counted line
         rather than repeated in it.
         ================================================================== -->
    <div class="anchor">
      <h1
        class="sym"
        title={picked
          ? `${picked} — one instrument of the ${fmt(instrumentRows.length)} the rungs above leave, in the store's own spelling.`
          : `${feedName}'s whole store at this selection. No single instrument is named: the Find box holds ${typed ? `“${filter.trim()}”, which is not one of the ${fmt(instrumentRows.length)} instrument keys on offer` : 'nothing'}.`}
      >
        {picked || feedName}
      </h1>

      <!-- THE COUNT CHIP AND THE "% change — what these are" HELP LINK ARE GONE.
           Neither is a rung and neither is a column: the first restates a number
           the row count under the grid already gives, and the second is help text
           on a page whose columns say what they are. -->

      <!-- THE TWO PRESSES THAT ARE ACTIONS RATHER THAN FILTERS. The count is ON
           the button: "% change — what these are" invites a click nobody makes,
           and "2,206 refused" is the fact an operator wants explained the moment
           they see a column of dashes. -->
      <span class="actions">
        <!-- ==============================================================
             THE SECOND VIEW WAS UNREACHABLE, AND THE COMMENT ABOVE `view`
             SAID IT WAS "ONE CLICK AWAY".

             `view` was declared `$state('bars')` and never assigned again --
             the only writes to it were gone, so `VIEWS` carried two entries,
             every `view === 'census'` branch was dead markup, and the coverage
             survey was reachable by editing the source. Its own title says
             what that cost: it "is the only view that can survey the whole
             store for a hole", and the page holding a hole survey nobody can
             open is worse than not having one, because the code reads as
             though the capability is there.

             THE CONTROL IS NOT INVENTED HERE. `.segs`, `.seg`,
             `.seg[aria-selected='true']` and `.seg .c` were all still in the
             stylesheet below, orphaned -- the compiler flagged three of them as
             unused selectors, which is how the original shape was recovered
             rather than guessed. It is a TABLIST with a count on each tab, and
             that CSS's own comment says why: the clause that used to sit under
             the control said how many segments were held, and the tabs are that
             enumeration. Restoring the markup the styles were written for also
             clears the three warnings.
             ============================================================== -->
        <span class="segs" role="tablist" aria-label="View">
          {#each VIEWS as v (v.key)}
            <button
              class="seg"
              type="button"
              role="tab"
              aria-selected={view === v.key}
              tabindex={view === v.key ? 0 : -1}
              title={v.title}
              onclick={() => (view = v.key)}
              >{v.label}<span class="c"
                >{fmt(v.key === 'bars' ? barSorted.length : sorted.length)}</span
              ></button
            >
          {/each}
        </span>
        {#if filtered}
          <button class="btn ghost sm" type="button" onclick={reset}>Reset all filters</button>
        {/if}
        <!-- EXPORT WRITES THE MATCHED SET, NOT THE WINDOW. Only ~30 rows exist
             in the DOM at any moment; the file is every row the selection
             matched, in the order the table is sorted in.

             DISABLED WITH THE REASON ON ITS FACE rather than sitting inert: a
             primary press that does nothing and says nothing is the silent
             failure `CLAUDE.md` §4 names, and "nothing matched" and "no feed
             chosen" are different sentences that a grey button cannot tell
             apart on its own. -->
        <button
          class="btn ghost sm"
          type="button"
          disabled={csvRows.length === 0}
          title={csvRows.length === 0
            ? view === 'bars'
              ? 'Nothing to export — no bar has been read into this grid, so there are no records to write.'
              : `Nothing to export — ${blocked?.why ?? 'no instrument-month matches this selection'}`
            : view === 'bars'
              ? `Write the ${fmt(csvRows.length)} bar(s) on this grid as CSV, named ${csvName} — the whole matched set, not the page. Built in the browser from rows already in memory: no request is made, and nothing is written into this repository. Every cell is the store's own wire form — microsecond stamps, prices and strikes in paisa, percentages as integer basis points, an EMPTY field for an unknown and never a zero.`
              : `Write the ${fmt(csvRows.length)} matched instrument-month(s) as CSV, named ${csvName}. Built in the browser from rows already in memory: no request is made, and nothing is written into this repository — the bytes go to your downloads. Every cell is the store's own wire form — raw YYYY-MM months, raw ISO expiries, strikes in paisa, percentages as the integer basis points they arrived as — so the file joins against the store. Only the header row is words.`}
          onclick={exportCsv}>Export CSV</button
        >
      </span>

      <!-- ==============================================================
           THE COUNTED LINE, AND IT IS WHAT THE SIX TILES WERE. Bars,
           complete, bars missing, coverage and months — the same figures, the
           same sub-clauses, the same `{#key}`-driven flash on each of them, in
           one line under the anchor instead of six boxes above the fold. The
           sixth, instrument-months, is the `.px` above: promoted, not dropped.

           A COUNTED ZERO STAYS A ZERO. The store WAS read, so "0
           instrument-months match this filter" is a measurement and printing a
           dash for it would hide a fact the page holds. What may never be
           printed is a figure nothing produced — a ratio with no denominator,
           or a session count at a rung nobody has a session size for. Those
           render the em dash and `blocked.tsub` names which absence it is.
           ============================================================== -->
      <!-- ==================================================================
           THE NUMBERS ARE THE HEADLINE; THE CLAUSES ARE ON HOVER.

           Every figure here carried a trailing `.u` clause, and read end to end
           the line came out as three hundred and nineteen characters of running
           prose: "BARS 8,470 · 20 session-equivalents COMPLETE 0 of 9 · 0
           short · 9 not comparable BARS MISSING 0 · 0 whole sessions, vs the
           fullest instrument each month COVERAGE — every row here is the only
           one stored at its (month, rung), so each is its own denominator and
           there is no ratio MONTHS 1 Aug 2026 → Aug 2026 · 0 with holes".

           Five numbers matter and they were buried in it. `terse` hides the
           clauses and keeps every one of them on the title, so the qualifier is
           one hover away rather than in the way of the figure it qualifies.
           Nothing is deleted — the markup, the counts and the reasons are all
           still built; only their default visibility changed.
           ================================================================== -->
      <!-- THE STATS LINE IS GONE. BARS / COMPLETE / BARS MISSING / COVERAGE /
           MONTHS was a fifth band of numbers about the numbers below it. The
           row count under the grid is the one figure a reader needs, and the
           grid itself is the rest. -->

      <span class="sr-only" aria-live="polite"
        >{viewNow.showing}. {fmt(pageTotal)} row(s) matched, showing {fmt(pageFrom)} to {fmt(
          pageTo
        )} on page {fmt(pageNow)} of {fmt(pageCount)}.</span
      >

      <!-- ================================================================
           THE READ BUDGET SAYS SO, AND CAN BE LIFTED.

           `barPlan` has always computed `held` -- how many matched
           instrument-months the budget did NOT open -- and nothing rendered
           it. `readBudget` is 6 and had no control, so a query matching sixty
           files drew rows from six and the grid looked like the whole answer.
           Rows from the other fifty-four were not empty, they were ABSENT,
           and absence with no notice is the failure CLAUDE.md §4 forbids: it
           neither degrades loudly nor refuses.

           The button sets the budget to 0, which `barPlan` already reads as
           "no ceiling" -- the escape existed in the arithmetic and had no way
           in. It is a press rather than the default because opening sixty
           files is a real cost the operator should choose, not one a page
           takes on their behalf.
           ================================================================ -->
      {#if view === 'bars' && barPlan.held > 0}
        <p class="budget" role="status">
          Read <b>{fmt(barPlan.read.length)}</b> of {fmt(barPlan.all.length)} matched
          instrument-month(s). Rows from the other <b>{fmt(barPlan.held)}</b> are absent rather than
          empty, so this grid is a sample of the match and not the whole of it.
          <button
            class="btn ghost sm"
            type="button"
            title="Open every matched instrument-month. One request per file, so {fmt(
              barPlan.held
            )} more file(s) are read."
            onclick={() => (readBudget = 0)}>Read all {fmt(barPlan.all.length)}</button
          >
        </p>
      {/if}
    </div>

    {#if noteOpen}
      <!-- ================================================================
           THE REFUSAL, IN FULL. `CLAUDE.md` §4: degrade loudly and name the
           reason. A column silently missing reads as forgotten; a column that
           states the endpoint, the cost, the decision that blocks it and the
           one server change that would unblock it reads as blocked — and the
           next person does not have to re-derive any of it.
           ================================================================ -->
      <section
        class="why panel-in"
        id="db-blocked"
        aria-label="What the percentage columns are, and why most cells are a dash"
      >
        <div class="whyhead">
          <span class="tag info">{fmt(numbered)} of {fmt(deco.length)} rows show a number</span>
          <h3>
            <b>% Change</b> and <b>Prev % Change</b> — integer basis points, or a dash that says
            which of five things it is.
          </h3>
          <button class="link" onclick={() => (noteOpen = false)}>close</button>
        </div>
        <ol class="whylist">
          {#each NOTES as b (b.k)}
            <li><b>{b.k}</b> <span>{b.v}</span></li>
          {/each}
        </ol>
        <p class="whyfoot">
          A dash is never a zero and never "nothing happened" — hover any one of them for the reason
          that cell in particular is unknown. Green and red mean DIRECTION and nothing else: they
          appear on a percentage and on a figure that moved between two reads of the store. A missing
          bar is a severity, not a direction, and wears amber.
        </p>
      </section>
    {/if}

      <!-- THE VIEW BAR IS GONE. The Coverage/Bars tabs, the "Showing one row
           per BAR" sentence, the read-budget picker and the "6 read + 3 over
           budget" clause were four pieces of chrome between the rungs and the
           grid, and none of them is a rung or a column. `view` stays pinned to
           `bars`, which is the grid this page is for. -->


    <!-- ==================================================================
         THE CENSUS TABLE. Windowed: only the visible slice exists in the DOM.
         ================================================================== -->
    {#if view === 'census'}
    <div class="tbl">
      <!-- The scroll region is FOCUSABLE ON PURPOSE and carries the row
           cursor. A keyboard operator has no other way to reach 20,516 rows —
           giving each row a tab stop is not navigation — so the box is the
           single stop and `aria-activedescendant` names the row inside it.
           Only Firefox focuses an overflow container by itself, so without the
           explicit stop PageDown does nothing in Chrome and WCAG 2.1.1 is
           unmet on the page's only content. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        class="tbl-scroll"
        class:xscrolled
        bind:this={scroller}
        bind:clientHeight={viewportH}
        onscroll={onScroll}
        onkeydown={onKey}
        tabindex="0"
        role="grid"
        aria-rowcount={censusPage.length}
        aria-activedescendant={cursor >= 0 ? `dbrow-${cursor}` : undefined}
        aria-label="Stored instrument-months, {fmt(censusPage.length)} row(s) on page {fmt(
          pageNow
        )} of {fmt(pageCount)}. Arrow keys move, Enter opens."
      >
        <!-- `presentation` because this div exists for `min-width` and for the
             spacer's positioning context, and for nothing an assistive reader
             needs. Left with an implicit role it would sit between `grid` and
             its `row`s and break the ownership chain. -->
        <div class="tbl-inner" role="presentation">
          <div class="thead" role="row">
            {#each COLS as c, ci (ci)}
              <div
                class="th"
                class:numh={c.num}
                class:inst={ci === 0}
                role="columnheader"
                aria-sort={ariaSort(c.key)}
              >
                <button
                  class="sortbtn"
                  type="button"
                  onclick={() => head(c.key)}
                  title={c.why ? `${c.label} — ${c.why}. Click to sort.` : `Sort by ${c.label}`}
                >
                  <span>{c.label}</span>
                  <span class="caret" class:on={sortKey === c.key} class:desc aria-hidden="true"
                  ></span>
                </button>
              </div>
            {/each}
          </div>

          {#if sorted.length === 0}
            <div class="tnone">
              <h3>No row matches</h3>
              <!-- THE NARROWINGS ARE LISTED FROM `narrowing`, THE SAME DERIVED
                   THE TILES CONSULT. This used to be a hand-glued chain of
                   `{#if}`s inserting its own commas, so the panel that explains
                   an empty table and the strip that reports it were two
                   independent spellings of one fact and could drift apart on
                   any edit. The emphasis and the raw-key tooltip both survive;
                   only the glue is gone. -->
              <p>
                {feedName} holds {fmt(deco.length)} instrument-months, and none of them match
                {#each narrowing as n, i (n.k)}{i > 0 ? ', ' : ''}<b
                    title={n.key ? `the store's own key for it is ${n.key}` : undefined}
                    >{n.text}</b
                  >{/each}.
              </p>
              <!-- THE GATE IS NOT A FILTER AND THE BUTTON BELOW WILL NOT OPEN
                   IT, so it is said here in full rather than left to be
                   discovered by pressing the button and watching nothing
                   change. `CLAUDE.md` §4: degrade loudly and name the reason —
                   and name the way out, which is a control, not this button. -->
              {#if expiryHeld > 0}
                <p class="tgate">
                  {fmt(expiryHeld)} of them are futures or options and are being <b>held back</b>,
                  not filtered:
                  {#if expiry}
                    they are on a contract other than {dayLabel(expiry)}. Choose theirs on the
                    Contract strip above.
                  {:else}
                    no expiry is chosen. A bar belongs to <b>one contract</b>, so this page will not
                    put two expiries under one heading — choose an expiry on the Contract strip and
                    they appear.
                  {/if}
                  Clearing the filters does not release them; only choosing an expiry does.
                </p>
              {/if}
              <button class="btn" onclick={reset}>Clear the filters</button>
            </div>
          {:else}
            <!-- The spacer is the FULL height of THE PAGE, so the scrollbar
                 tells the truth about 1,000 rows while 30 of them exist. Rows
                 are absolutely positioned at their own offset — no accumulated
                 rounding down a long list. It was the full height of the whole
                 sorted set before the pager landed; leaving it there would have
                 drawn a scrollbar for 20,516 rows over a page holding 50. -->
            <div class="tspace" role="rowgroup" style="height:{censusPage.length * ROW}px">
              {#key stamp}
                {#each slice as it, i (it.key)}
                  {@const n = first + i}
                  <!-- svelte-ignore a11y_click_events_have_key_events -->
                  <!-- svelte-ignore a11y_interactive_supports_focus -->
                  <div
                    class="trow"
                    class:odd={n % 2 === 1}
                    class:row-in={entering}
                    class:cur={cursor === n}
                    class:open={openKey === it.key}
                    data-state={it.state}
                    id="dbrow-{n}"
                    role="row"
                    aria-rowindex={n + 1}
                    aria-selected={cursor === n}
                    onclick={() => {
                      cursor = n;
                      openRow(it);
                    }}
                    style="top:{n * ROW}px; animation-delay:{Math.min(i * 8, 180)}ms"
                  >
                    <span class="cell inst" role="gridcell">
                      <span class="pip" aria-hidden="true"></span>
                      <span class="ihead">{it.head}</span><span class="isym">{it.sym}</span>
                    </span>
                    <!-- THE RAW KEY LEADS THE TOOLTIP AND THE DAY WINDOW
                         FOLLOWS IT. `2020-03` is the store's own spelling and
                         has to stay one hover away and copyable; the window
                         after it is the fact `first_ts` / `last_ts` have been
                         on the wire for and no page has read — it tells a month
                         held from the 2nd to the 31st from one held on the 2nd
                         alone, which no bar count can. -->
                    <span
                      class="cell mono"
                      role="gridcell"
                      title={heldWindow(it)
                        ? `${monthLabel(it.month)} · held ${heldWindow(it)} — the store's own key for it is ${it.month}`
                        : `${monthLabel(it.month)} — the store's own key for it is ${it.month}`}
                      >{monthLabel(it.month)}</span
                    >
                    <span class="cell tf" role="gridcell">{it.timeframe}</span>
                    <span
                      class="cell num"
                      role="gridcell"
                      class:flash-up={moved.get(it.key) === 'up'}
                      class:flash-down={moved.get(it.key) === 'down'}>{fmt(it.rows)}</span
                    >
                    <span class="cell num dimnum" role="gridcell">{dayText(it.days)}</span>
                    <span class="cell num short" role="gridcell">
                      <!-- A TAUTOLOGY IS NOT A VERDICT. `short === 0` against a
                           denominator that is this row itself printed the green
                           word `full` over a month holding one bar. -->
                      {#if it.sole}
                        <span class="unproven" title={SOLE_WHY}>unverified</span>
                      {:else if it.short === 0}
                        <span class="okword">full</span>
                      {:else}
                        <span class="missnum">−{fmt(it.short)}</span>
                        <!-- THE NULL TEST IS WRITTEN OUT, NOT RELIED ON.
                             `lost` is `null` for a rung with no recorded
                             session size, and `null >= 1` is false — so the
                             days-lost chip was already suppressed for those
                             rows and still is. Stating the absence is what
                             lets `Math.floor` below be read as taking a
                             number, which is the only thing it can take. -->
                        {#if it.lost !== null && it.lost >= 1}
                          <span class="lost">{Math.floor(it.lost)}d</span>
                        {/if}
                      {/if}
                    </span>
                    <span class="cell comp" role="gridcell">
                      <!-- NO METER FOR A RATIO THAT DOES NOT EXIST — the rule
                           the Coverage tile already keeps. An empty track reads
                           as 0% and a full one as complete; both are claims. -->
                      {#if it.pct !== null}
                        <span class="meter" aria-hidden="true"
                          ><span class="fill" style="width:{it.pct * 100}%"></span></span
                        >
                      {/if}
                      <span class="cpct" title={it.pct === null ? SOLE_WHY : undefined}
                        >{pctText(it.pct)}</span
                      >
                    </span>
                    <!-- THE TWO PERCENTAGES. A number carries its sign, two
                         decimals and its direction's colour; an unknown is a
                         dash that NAMES ITSELF on hover and never a 0.00%.
                         `title` for the pointer, `aria-label` for everything
                         else — the reason has to reach both or the loud
                         degradation is only loud to one of them. -->
                    <span class="cell num pc" role="gridcell" data-dir={dirOf(it.chg)}>
                      {#if it.chg === null}
                        <span class="unk" title={whyText(it.chgWhy)} aria-label={whyText(it.chgWhy)}
                          >—</span
                        >
                      {:else}
                        {bpsText(it.chg)}
                      {/if}
                    </span>
                    <span class="cell num pc" role="gridcell" data-dir={dirOf(it.prevChg)}>
                      {#if it.prevChg === null}
                        <span
                          class="unk"
                          title={whyText(it.prevChgWhy)}
                          aria-label={whyText(it.prevChgWhy)}>—</span
                        >
                      {:else}
                        {bpsText(it.prevChg)}
                      {/if}
                    </span>
                  </div>
                {/each}
              {/key}
            </div>
          {/if}
        </div>
      </div>

      {#if openRowData}
        <!-- ==============================================================
             THE DRILL-THROUGH, at the altitude the data supports. Not a
             chart — this store serves no price to this page — but every
             month this instrument holds, which is the question a row raises.
             ============================================================== -->
        <!-- A `div`, NOT an `aside`. `role="dialog"` on a complementary
             landmark is the a11y contradiction `svelte-check` names: this is a
             transient overlay the operator dismisses, not a standing region of
             the page. `aria-modal="false"` because it traps nothing — the
             table behind it stays readable and reachable. -->
        <div
          class="drawer panel-in"
          bind:this={drawerEl}
          role="dialog"
          aria-modal="false"
          aria-label="{openRowData.instrument} — every stored month"
        >
          <div class="dhead">
            <div class="dtitle">
              <span class="ihead">{openRowData.head}</span><span class="isym"
                >{openRowData.sym}</span
              >
            </div>
            <button class="link dclose" onclick={() => (openKey = null)}>close ·<kbd class="kbd">Esc</kbd></button>
          </div>

          <div class="dstats">
            <div class="dstat">
              <span class="k">Months held</span><span class="v">{fmt(openTotals.n)}</span>
            </div>
            <div class="dstat">
              <span class="k">Complete</span><span class="v"
                >{fmt(openTotals.complete)}{#if openTotals.unproven > 0}<span
                    class="unproven"
                    title={SOLE_WHY}>&nbsp;+{fmt(openTotals.unproven)} unverified</span
                  >{/if}</span
              >
            </div>
            <div class="dstat">
              <span class="k">Bars</span><span class="v">{fmt(openTotals.bars)}</span>
            </div>
            <div class="dstat">
              <span class="k">Short by</span
              ><span class="v" class:risk={openTotals.missing > 0}>{fmt(openTotals.missing)}</span>
            </div>
          </div>

          <!-- WHICH DAYS THE OPENED MONTH ACTUALLY COVERS. This is the one
               place `/store.json`'s `first_ts` / `last_ts` are shown at full
               size, and this drawer is the right place for them: the owner
               named ONE instrument, and the per-instrument detail this page
               already has is the surface that answers about it. A second OHLC
               table would be `/markets` written twice, and `/store.json` serves
               no price to build one from.

               DEGRADES LOUDLY. A row without stamps says so and names the
               payload as the reason; it does not print the month's first and
               last calendar day, which are very often not trading days. -->
          <p class="dspan">
            <span class="k">Days held</span>
            {#if heldWindow(openRowData)}
              <span class="v" title="first_ts → last_ts from /store.json, microseconds, rendered IST"
                >{heldWindow(openRowData)}</span
              >
              <span class="n"
                >in <span
                  title={`${monthLabel(openRowData.month)} — the store's own key for it is ${openRowData.month}`}
                  >{monthLabel(openRowData.month)}</span
                ></span
              >
            {:else}
              <span class="v">—</span>
              <span class="n"
                >/store.json carried no first/last stamp for this row, so the days it covers are
                unknown — not the whole month, and not one day</span
              >
            {/if}
          </p>

          <div class="dlist">
            {#each openMonths as m (m.key)}
              <div class="drow" data-state={m.state} class:this={m.key === openKey}>
                <span
                  class="dm"
                  title={heldWindow(m)
                    ? `${monthLabel(m.month)} · held ${heldWindow(m)} — the store's own key for it is ${m.month}`
                    : `${monthLabel(m.month)} — the store's own key for it is ${m.month}`}
                  >{monthLabel(m.month)}</span
                >
                {#if m.pct !== null}
                  <span class="meter" aria-hidden="true"
                    ><span class="fill" style="width:{m.pct * 100}%"></span></span
                  >
                {/if}
                <span class="dpct" title={m.pct === null ? SOLE_WHY : undefined}
                  >{pctText(m.pct)}</span
                >
                <span class="dshort" title={m.sole ? SOLE_WHY : undefined}
                  >{#if m.sole}unverified{:else if m.short === 0}full{:else}−{fmt(m.short)}{/if}</span
                >
              </div>
            {/each}
          </div>

          <p class="dfoot">
            Each month is measured against the fullest instrument stored for that month —
            <span
              title={`${monthLabel(openRowData.month)} — the store's own key for it is ${openRowData.month}`}
              >{monthLabel(openRowData.month)}</span
            >
            {#if openRowData.sole}
              held {fmt(openRowData.denom)} bars at its fullest — and this row IS that fullest, and
              the only one stored at this rung for that month. The denominator is this row, so no
              share of the month can be stated for it. {SOLE_WHY}
            {:else}
              held {fmt(openRowData.denom)} bars at its fullest, so this row's {fmt(
                openRowData.rows
              )} is {pctText(openRowData.pct)} of the month.
            {/if}
          </p>
        </div>
      {/if}
    </div>
    {:else}
      <!-- ==================================================================
           THE BAR GRID — ONE ROW PER BAR, twenty-four columns, read from
           `/bars.json`.

           A REAL `<table>`, and the census grid beside it is still a grid of
           divs. The two are not inconsistent by accident: the census grid is
           WINDOWED — rows absolutely positioned inside a spacer — and a
           `<tbody>` cannot be windowed that way without lying to the row
           model. This grid is PAGED instead, so the whole page is in the
           document and the browser's own column algorithm can do the work
           that the census grid has to do with `grid-template-columns`.

           TEN OF THE TWENTY-FOUR COLUMNS HAVE NO SOURCE ON THIS WIRE and
           every one of them says so, on the header and in every cell.
           `CLAUDE.md` §4: degrade loudly and name the reason.
           ================================================================== -->
      {#if stranded.length > 0}
      <p class="stranded" role="status">
        {#each stranded as st (st.rung)}
          <span class="sitem">
            <b>{st.rung}</b> is set to <b>{st.value}</b>, which the rungs above it no longer offer —
            so no row can match.
            <button class="btn ghost sm" type="button" onclick={st.clear}>Clear {st.rung}</button>
          </span>
        {/each}
      </p>
    {/if}

    <div class="tbl">
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div
          class="tbl-scroll bscroll"
          tabindex="0"
          role="region"
          aria-label="Stored bars, {fmt(barPage.length)} on this page of {fmt(
            pageTotal
          )}. Sorted by {BAR_META[barSortKey]?.label ?? barSortKey}."
        >
          <table
            class="bgrid lean"
            class:noday={dayRung}
            class:nofuture={!shape.contract}
            class:nooption={!shape.option}
            style="min-width:{BAR_WIDTH}px"
          >
            <colgroup>
              {#each BAR_COLS as c (c.key)}
                <col style="width:{c.w}px" />
              {/each}
            </colgroup>
            <thead>
              <tr>
                {#each BAR_COLS as c (c.key)}
                  <th
                    scope="col"
                    class:numh={c.num}
                    class:deadh={Boolean(c.none)}
                    aria-sort={c.none ? undefined : barAriaSort(c.key)}
                  >
                    <!-- A COLUMN WITH NO SOURCE DOES NOT PRETEND TO SORT. The
                         button is still here, still reachable and still
                         labelled — it is DISABLED and it carries the reason on
                         its own title, which is the difference between a
                         control that is missing and one that is refused. -->
                    <button
                      class="sortbtn"
                      type="button"
                      disabled={Boolean(c.none)}
                      title={c.none
                        ? `${c.label} — no source, so there is nothing to order: ${c.none}`
                        : c.key === 'chg'
                          ? `${c.label} — ${c.why}. The bar behind each figure is that figure's size against the largest move in this selection, measured PER RUNG so a minute bar is never judged against a daily one (${chgScaleText}). Scaled across every matched row rather than this page, so turning the page does not change what a full bar means. Click to sort; click again to reverse.`
                          : `${c.label} — ${c.why}. Click to sort; click again to reverse.`}
                      onclick={() => barHead(c.key)}
                    >
                      <span>{c.label}</span>
                      <span
                        class="caret"
                        class:on={barSortKey === c.key && !c.none}
                        class:desc={barDesc}
                        aria-hidden="true"
                      ></span>
                    </button>
                  </th>
                {/each}
              </tr>
            </thead>

            <tbody>
              {#if barState.error}
                <tr class="bstate">
                  <td colspan={BAR_COLS.length}>
                    <div class="bnone" role="alert">
                      <h3>The bar files could not be read</h3>
                      <p class="err">{barState.error}</p>
                      <p>
                        Nothing is drawn rather than a plausible blank: a grid with no rows and no
                        reason reads as an empty store, and this is a failed read.
                      </p>
                      <button class="btn" onclick={() => refreshStore()}>Try again</button>
                    </div>
                  </td>
                </tr>
              {:else if barPlan.all.length === 0}
                <tr class="bstate">
                  <td colspan={BAR_COLS.length}>
                    <div class="bnone">
                      <h3>No instrument-month matches this query</h3>
                      <p>
                        A bar lives in an instrument-month file, so with none matched there is no
                        file to open — this grid has made no request. {feedName} holds {fmt(
                          deco.length
                        )} instrument-month(s) in total.
                      </p>
                      <button class="btn" onclick={reset}>Clear the filters</button>
                    </div>
                  </td>
                </tr>
              {:else if barState.loading && barPage.length === 0}
                {#each SKELETON.slice(0, 12) as i (i)}
                  <tr class="brow skelrow">
                    {#each BAR_COLS as c, ci (c.key)}
                      <td class:bn={c.num}
                        ><span
                          class="skel line"
                          style="width:{46 + ((i * 13 + ci * 7) % 44)}%;animation-delay:{i * 60}ms"
                        ></span></td
                      >
                    {/each}
                  </tr>
                {/each}
              {:else if barPage.length === 0}
                <tr class="bstate">
                  <td colspan={BAR_COLS.length}>
                    <div class="bnone">
                      <h3>No bar came back</h3>
                      <p>
                        {fmt(barPlan.read.length)} instrument-month(s) were read and
                        <b>{fmt(barsRead)}</b> bar(s) arrived.
                        {#if barFails.length}
                          {fmt(barFails.length)} of those file(s) refused — each refusal is named
                          under the grid.
                        {:else}
                          The file(s) opened and held no record. The census says they hold {fmt(
                            barsClaimed
                          )}, so this is a disagreement between the manifest and the file, not an
                          empty query.
                        {/if}
                      </p>
                      <button class="btn" onclick={() => refreshStore()}>Read again</button>
                    </div>
                  </td>
                </tr>
              {:else}
                {#key stamp}
                  {#each barPage as b, i (b.rk)}
                    <tr
                      class="brow"
                      class:odd={i % 2 === 1}
                      class:row-in={entering}
                      data-dir={b.c > b.o ? 'up' : b.c < b.o ? 'down' : 'flat'}
                      style="--rng:{rngMag(b.h - b.l, b.tf)}"
                    >
                      <!-- DATE / TIME. The stamp is IST and says so; the rung
                           and the instrument that produced the bar are on the
                           title, because a grid that mixes two files must be
                           able to say which row came from which. -->
                      <td
                        class="bt"
                        title="{stampLabel(b.ts)} IST · {b.tf} bar · {b.instrument} · month file {b.month}"
                        >{dayLabel(b.ts)}</td
                      >
                      <!-- ALWAYS RENDERED, HIDDEN BY CLASS. This grid hides a
                           column with positional CSS on `.bgrid`, so header and
                           body must agree cell-for-cell; an `{#if}` here dropped
                           the body's cell while the header kept its own and the
                           two rows slid apart by one. -->
                      <td class="bt" title="{stampLabel(b.ts)} IST — the bar's opening minute"
                        >{timeLabel(b.ts)}</td
                      >
                      <td class="bt">{b.tf}</td>
                      <td class="bn" title="{fmt(b.o)} paisa, as stored">{paisaText(b.o)}</td>
                      <td class="bn" title="{fmt(b.h)} paisa, as stored">{paisaText(b.h)}</td>
                      <td class="bn" title="{fmt(b.l)} paisa, as stored">{paisaText(b.l)}</td>
                      <td class="bn" title="{fmt(b.c)} paisa, as stored">{paisaText(b.c)}</td>
                      <td class="bn bvol" style="--vmag:{volMag(b.vol, b.tf)}%">{fmt(b.vol)}</td>

                      <!-- PRE-MARKET %: no source on this wire. -->
                      <td class="bn na"
                        ><span
                          class="unk"
                          title={BAR_META.premkt.none}
                          aria-label="Pre-mkt %: no source — {BAR_META.premkt.none}">no source</span
                        ></td
                      >

                      <!-- THE NUMBER, AND THE SAME NUMBER AS A LENGTH.
                           A column of ±0.05% reads as a column of identical
                           text; the bar behind it is the only thing that makes
                           an outlier findable by eye at fifty rows a page. It
                           is drawn from the SAME `b.chg` the text renders and
                           carries no scale of its own — see `--mag`. -->
                      <td
                        class="bn bpc"
                        data-dir={dirOf(b.chg)}
                        style={b.chg === null ? undefined : `--mag:${chgMag(b.chg, b.tf)}%`}
                      >
                        {#if b.chg === null}
                          <span
                            class="unk"
                            title={barWhyText(b.chgWhy)}
                            aria-label={barWhyText(b.chgWhy)}>—</span
                          >
                        {:else}
                          {bpsText(b.chg)}
                        {/if}
                      </td>

                      <!-- OPEN INTEREST. `i64::MIN` is the null sentinel and it
                           renders as an unknown that names itself. A ZERO IS A
                           ZERO and renders as one — the two are different facts
                           and this column has to keep them apart. -->
                      <td class="bn">
                        {#if b.oi === null}
                          <span
                            class="unk"
                            title={barWhyText('oi_null')}
                            aria-label={barWhyText('oi_null')}>unknown</span
                          >
                        {:else}
                          <span
                            title={b.oi === 0
                              ? 'zero, and stored as zero — not the null sentinel'
                              : 'open interest, as stored'}>{fmt(b.oi)}</span
                          >
                        {/if}
                      </td>

                      <td class="bn bpc" data-dir={dirOf(b.oichg)}>
                        {#if b.oichg === null}
                          <span
                            class="unk"
                            title={barWhyText(b.oichgWhy)}
                            aria-label={barWhyText(b.oichgWhy)}>—</span
                          >
                        {:else}
                          {bpsText(b.oichg)}
                        {/if}
                      </td>

                      <!-- EXPIRY. The row keeps the raw ISO day — that is what
                           the sort compares and what the day arithmetic reads —
                           and it becomes `29 Oct 2026` here and only here. -->
                      <td class="bt">
                        {#if b.exp === null}
                          <span
                            class="unk"
                            title="this instrument is not a contract — an index and an equity have no expiry. It is not an unknown date."
                            >—</span
                          >
                        {:else}
                          <span title="the store's own key for it is {b.exp}">{dayLabel(b.exp)}</span
                          >
                        {/if}
                      </td>
                      <td class="bn">
                        {#if b.dte === null}
                          <span
                            class="unk"
                            title="no expiry on this instrument, so there is no distance to one."
                            >—</span
                          >
                        {:else}
                          <span
                            title={b.dte < 0
                              ? `${fmt(-b.dte)} day(s) AFTER expiry — this bar is from the contract's life, read out of a month file that outlives it`
                              : 'whole calendar days from this bar to expiry'}>{fmt(b.dte)}d</span
                          >
                        {/if}
                      </td>
                      <td class="bt">
                        {#if b.side === null}
                          <span class="unk" title="not an option — this instrument has no side.">—</span>
                        {:else}
                          <span class="chip {b.side === 'CE' ? 'ce' : 'pe'}">{b.side}</span>
                        {/if}
                      </td>
                      <td class="bn">
                        {#if b.strike === null}
                          <span class="unk" title="not an option — this instrument carries no strike."
                            >—</span
                          >
                        {:else}
                          <span title="{fmt(b.strike)} paisa, as stored">{strikeText(b.strike)}</span>
                        {/if}
                      </td>

                      <!-- MONEYNESS, INTRINSIC, EXTRINSIC and the five Greeks
                           plus IV: no source on this wire, and each says which
                           source it would need. -->
                      <td class="na"
                        ><span
                          class="unk"
                          title={BAR_META.mny.none}
                          aria-label="Moneyness: no source — {BAR_META.mny.none}">no source</span
                        ></td
                      >
                      <td class="bn na"
                        ><span
                          class="unk"
                          title={BAR_META.intr.none}
                          aria-label="Intrinsic: no source — {BAR_META.intr.none}">no source</span
                        ></td
                      >
                      <td class="bn na"
                        ><span
                          class="unk"
                          title={BAR_META.extr.none}
                          aria-label="Extrinsic: no source — {BAR_META.extr.none}">no source</span
                        ></td
                      >
                      <td class="bn na"
                        ><span class="unk" title={NO_GREEK} aria-label="IV %: no source — {NO_GREEK}"
                          >no source</span
                        ></td
                      >
                      <td class="bn na"
                        ><span class="unk" title={NO_GREEK} aria-label="Delta: no source — {NO_GREEK}"
                          >no source</span
                        ></td
                      >
                      <td class="bn na"
                        ><span class="unk" title={NO_GREEK} aria-label="Gamma: no source — {NO_GREEK}"
                          >no source</span
                        ></td
                      >
                      <td class="bn na"
                        ><span class="unk" title={NO_GREEK} aria-label="Theta: no source — {NO_GREEK}"
                          >no source</span
                        ></td
                      >
                      <td class="bn na"
                        ><span class="unk" title={NO_GREEK} aria-label="Vega: no source — {NO_GREEK}"
                          >no source</span
                        ></td
                      >
                      <td class="bn na"
                        ><span class="unk" title={NO_GREEK} aria-label="Rho: no source — {NO_GREEK}"
                          >no source</span
                        ></td
                      >
                    </tr>
                  {/each}
                {/key}
              {/if}
            </tbody>
          </table>
        </div>

        <!-- EVERY REFUSAL, EVERY PARTIAL READ AND EVERY DISAGREEMENT, NAMED.
             None of these is allowed to be a shorter grid. -->
        {#if barFails.length || barFaults.length || barDisagree.length || barsRead > 0}
          <div class="bnotes">
            {#if barsRead > 0}
      <!-- THE COLUMNS NOTE IS GONE. "10 of 24 columns drawn, the rest have
           nothing to put in them" is a sentence about the page rather than
           about the store, and the columns that are drawn are self-evident. -->

            {/if}
            {#each barDisagree as f (f.key)}
              <p class="bnote warn">
                <b>{f.row.instrument} {f.row.month} {f.row.timeframe}</b> — the file returned {fmt(
                  f.bars.length
                )} bar(s) and the census manifest claims {fmt(f.row.rows)}. The grid shows what the
                FILE returned; the difference is a fact about the store, not a rendering choice.
              </p>
            {/each}
            {#each barFaults as f (f.key)}
              <p class="bnote warn">
                <b>{f.row.instrument} {f.row.month} {f.row.timeframe}</b> — read in part: {f.faults}
              </p>
            {/each}
            {#each barFails as f (f.key)}
              <p class="bnote err">
                <b>{f.row.instrument} {f.row.month} {f.row.timeframe}</b> — {f.error}
              </p>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- ==================================================================
         THE PAGER — how you reach the rows that are not on screen.

         IT GOVERNS BOTH GRIDS, so `1–50 of 8,250 rows` means the same thing
         under either one and the arithmetic is written once. `pageNow` is a
         CLAMP and not a write-back: a narrowing that shortens the run
         cannot strand the reader on a page that no longer exists.

         EVERY DISABLED BUTTON SAYS WHY, on its own title. A greyed control
         with no reason is the silent degradation §4 bans; "already on the
         first page" is one word longer and answers the question.
         ================================================================== -->
    <div class="pgbar" role="group" aria-label="Paging">
      <span class="pgof">
        {#if pageTotal === 0}
          <b>0</b> rows — {view === 'bars'
            ? 'no bar was read, so there is no page to turn'
            : 'nothing matched, so there is no page to turn'}
        {:else}
          <b>{fmt(pageFrom)}–{fmt(pageTo)}</b> of <b>{fmt(pageTotal)}</b> rows · page
          <b>{fmt(pageNow)}</b> of <b>{fmt(pageCount)}</b>
        {/if}
      </span>

      <!-- THE LAST OS-DRAWN MENU IN THIS PRODUCT. `<select>` styles its closed
           face and NOT its open list -- that popup belongs to the operating
           system -- so this one control kept rendering the OS blue highlight
           under a page that draws everything else itself. Same fix as both
           calendars: `Picker`, single, with each row's own sentence carried
           across as its `why` rather than an `<option title>` no reader hovers. -->
      <div class="pgsize">
        <span>Rows per page</span>
        <Picker
          single
          label="page sizes"
          summary={fmt(pageSize)}
          title="How many rows this page holds. The reader's place is kept across a change: the row you were standing on stays on screen."
          rows={PAGE_SIZES.map((n) => ({
            key: String(n),
            name: fmt(n),
            detail: n >= 1000 ? `${fmt(n * BAR_COLS.length)} cells` : undefined,
            why:
              n >= 1000
                ? `${fmt(n)} rows in one document. On the bar grid that is ${fmt(n * BAR_COLS.length)} cells — legible, and slow to lay out.`
                : undefined
          }))}
          selected={new Set([String(pageSize)])}
          onchange={(/** @type {Set<string>} */ sel) => {
            const n = Number([...sel][0]);
            if (Number.isFinite(n) && n > 0) setPageSize(n);
          }}
        />
      </div>

      <label class="pgjump">
        <span>Go to page</span>
        <input
          type="number"
          min="1"
          max={pageCount}
          value={pageNow}
          disabled={pageTotal === 0}
          title={pageTotal === 0
            ? 'There is no page to go to: nothing is on this grid.'
            : `Any page from 1 to ${fmt(pageCount)}. Out-of-range is clamped, never refused.`}
          onchange={(e) => goPage(Number(e.currentTarget.value))}
        />
        <span class="pgof2">of {fmt(pageCount)}</span>
      </label>

      <span class="pgnav">
        <button
          class="pg"
          type="button"
          disabled={pageNow === 1}
          title={pageNow === 1 ? 'Already on the first page' : 'First page'}
          aria-label="First page"
          onclick={() => goPage(1)}>«</button
        >
        <button
          class="pg"
          type="button"
          disabled={pageNow === 1}
          title={pageNow === 1 ? 'Already on the first page' : `Page ${fmt(pageNow - 1)}`}
          aria-label="Previous page"
          onclick={() => goPage(pageNow - 1)}>‹</button
        >
        {#each pageList(pageNow, pageCount) as item (item.k)}
          {#if item.gap}
            <!-- ELIDED, AND IT IS NOT A CONTROL. A 1,840-page run must not
                 render 1,840 buttons; the jump box above reaches the pages
                 this row does not draw, which is why the elision is honest. -->
            <span class="pg gap" aria-hidden="true">…</span>
          {:else}
            <button
              class="pg"
              type="button"
              aria-current={item.p === pageNow ? 'page' : undefined}
              title="Page {fmt(item.p)} of {fmt(pageCount)}"
              onclick={() => goPage(item.p)}>{fmt(item.p)}</button
            >
          {/if}
        {/each}
        <button
          class="pg"
          type="button"
          disabled={pageNow === pageCount}
          title={pageNow === pageCount
            ? pageCount === 1
              ? 'There is only one page'
              : 'Already on the last page'
            : `Page ${fmt(pageNow + 1)}`}
          aria-label="Next page"
          onclick={() => goPage(pageNow + 1)}>›</button
        >
        <button
          class="pg"
          type="button"
          disabled={pageNow === pageCount}
          title={pageNow === pageCount
            ? pageCount === 1
              ? 'There is only one page'
              : 'Already on the last page'
            : `Last page, ${fmt(pageCount)}`}
          aria-label="Last page"
          onclick={() => goPage(pageCount)}>»</button
        >
      </span>
    </div>

    <!-- THE STANDING BAR UNDER THE PAGER. It is not a second pager and it
         draws no page buttons: the pager above owns the ordinals. What this
         carries is what the pager cannot — how much of the page is actually
         in the document, the newest instant in the whole matched set, and
         the keys that move a reader through it.

         IT GAINED THE NEWEST BAR, which is the one fact the standing "newest
         bar" notice carried that nothing else on this page stated: `last_ts`
         was read per row (the month cell's tooltip) and per instrument (the
         drawer's "Days held") and never for the SET on screen, which is the
         question a banner over a table is actually asked. Counted from the
         rows, so it narrows with them; an absence is named and never rendered
         as an epoch. -->
    <p class="pager">
      <!-- THE ROW COUNT, AND NOTHING ELSE. "N in the DOM" is a fact about the
           windowing, not about the store; "newest bar" repeats a value the
           first row of the grid already shows; the keyboard legend is a help
           text on a page whose shortcuts are one key each. -->
      <span class="of"><b>{fmt(pageTotal)}</b> row(s)</span>
    </p>
    </div>
  {/if}
</div>

<!-- ======================================================================
     THE FEED RUNG — FIRST IN THE STRIP, AND A CONTROL RATHER THAN A READOUT.
     ----------------------------------------------------------------------
     WHAT STOOD HERE AND DOES NOT ANY MORE: `.cell.scopeline`, a caption over
     a `.feedname` span of plain text. It was drawn to `.mnyb`'s exact metrics
     — same size, same weight, same mono family — so it sat in a row of six
     dropdowns looking like the seventh and doing nothing, and the one real
     selector was two rows away in the top bar. A face that looks like a
     control and refuses the press is worse than no control at all.

     SO THE CONTROL IS HERE AND THE TOP BAR YIELDS. `/db` is on the shell's
     `FEED_OWNED` list as of the same commit that added this rung, so the
     count of feed pickers on this route is exactly one, in the place the
     approved design draws it: the FIRST cell of the query strip, because the
     feed is this page's whole scope and every rung after it is that feed's
     answer.

     A SNIPPET RATHER THAN MARKUP IN PLACE, because the strip is not the only
     place it has to appear. The failed read, the unchosen feed, the loading
     store and the empty store are four states with no strip in them, and each
     is exactly the state in which an operator most needs another feed. A
     control that vanishes with its data is the dead end this snippet exists
     to make impossible.

     IT IS `.picker`/`.mnyb`/`.menu`/`.opt` AND NOT A `Picker`, for the reason
     Universe, Month and Timeframe are not: a `Picker` row is a checkbox, the
     feed is a SELECTION — exactly one, never a set — and a feed the server
     refuses needs a row that is DRAWN, DEAD, and carrying `/feeds.json`'s own
     reason. A tick cannot say why it is unavailable.
     ====================================================================== -->
{#snippet feedRung()}
  <div class="field">
    <!-- THE CAPTION IS THE CAPTION. What the italic sub-clause used to say —
         that this is the page's whole scope, and where it is chosen — is on
         the control's `title`, one hover from the thing it is about, and the
         clause below states the same in the document. -->
    <span class="lab">Broker feed</span>
    <!-- THE SAME `Picker` /ingest'S FEED RUNG NOW USES, AND THAT SYMMETRY IS
         THE WHOLE POINT OF THIS CHANGE.

         BOTH pages hand-rolled this one control and no other. Every other rung
         here was already `Picker`; /ingest's were `.dd`/`.ddm`. So the feed
         menu was the last place the two pages drew one question two ways — a
         plain ✓ list with no filter here, a different plain ✓ list with no
         filter there — and it is the menu, not the label, that a reader
         actually looks at. Unifying the class names in 673e7da made the labels
         match and left this untouched, which is exactly why the pages still
         did not look alike afterwards.

         EVERY BEHAVIOUR IS KEPT AND NONE RE-IMPLEMENTED:

         * A refused feed is DRAWN AND DISABLED with `/feeds.json`'s own reason,
           never omitted — `CLAUDE.md` §4, and `Picker`'s `disabled` + `why` is
           the shape every other rung on this page already uses.
         * The per-feed survey count keeps its four states — unavailable, not
           read, N held, not surveyed — as the row's `detail`, so a dash is
           still a dash and never a zero.
         * The control is NEVER disabled, which is what keeps it reachable when
           the list is empty; the empty case is stated below rather than behind
           a dead press.

         SINGLE, because this page shows one store at a time and the operator
         asked for exactly that: every rung on /db is `single`. -->
    <Picker
      single
      filter
      label="feeds"
      title={`${feedName}. THE PAGE'S WHOLE SCOPE: every count on this page is this feed's store — every row read from /store.json?feed=${feeds.active ?? ''} and every membership count from /instruments.json?feed=${feeds.active ?? ''} — and no page in this product puts one feed's numbers beside another's, because the two are not the same instrument universe, the same session handling or the same price scale. A bar belongs to the vendor that supplied it, so changing this changes the store, not the view of one.${feeds.error ? ` The feed list itself could not be read: ${feeds.error}. Nothing below this line has been scoped to anything.` : ''}`}
      summary={feeds.error
        ? 'Feed list unread'
        : feeds.all.length === 0
          ? 'No feeds'
          : feeds.active
            ? feedName
            : 'Select a feed'}
      rows={feeds.all.map((f) => {
        const ready = f.ready === true;
        const held = survey.byFeed.get(f.wire);
        return {
          key: f.wire,
          name: f.display,
          detail: !ready
            ? 'unavailable'
            : held?.error
              ? 'not read'
              : held
                ? `${fmt(held.cells)} held`
                : 'not surveyed',
          disabled: !ready,
          why: ready
            ? undefined
            : (f.why ??
              'the server marked this feed not ready and stated no reason, which is itself the thing to fix.'),
          title: ready
            ? `Selects ${f.display}. Every count and every row below becomes this feed's, read again from /store.json?feed=${f.wire} and /instruments.json?feed=${f.wire} — nothing is deleted, nothing is merged with the feed you are leaving, and no figure on this page ever puts the two side by side.${
                held
                  ? held.error
                    ? ` Its store could not be read on the last survey: ${held.error}.`
                    : ` The last survey read ${fmt(held.cells)} instrument-month(s) and ${fmt(held.bars)} bar(s) under it.`
                  : ' Its store has not been surveyed in this session, so the count beside it is a dash rather than a zero.'
              }`
            : `${f.display} is refused by the server, and this is /feeds.json's own reason rather than a paraphrase of it: ${f.why ?? 'no reason stated.'}`
        };
      })}
      selected={new Set([feeds.active])}
      onchange={(/** @type {Set<string>} */ sel) => {
        const next = [...sel][0];
        if (next) feeds.active = next;
      }}
    />
    <!-- THE STATES BEFORE A STORE ANSWERS ARE NAMED RATHER THAN COUNTED: a
         "0 held" under a feed nobody chose, or under a read that failed, is a
         claim about a disk nobody reached. -->
    <!-- `N held · read <clock>` IS GONE. The pane head above already carries
         `as of <clock>` beside a Refresh button, so this restated the one fact
         on screen twice, and the held count is the face of every rung below it.
         Every branch that remains is a refusal or an in-flight state.
         THE `{#if}` WRAPS THE SPAN. Left inside it, the normal state — no error,
         a feed chosen, not loading — rendered an EMPTY `.note`, and a `.note`
         carries its marker as `::before`, so "empty" paints a bare `·` under the
         control. Emptying the element and keeping the element is how a cut
         leaves litter. -->
    {#if feeds.error || error || !feeds.active || (loading && rows.length === 0)}
      <span class="note" class:warn={Boolean(error) || Boolean(feeds.error) || !feeds.active}>
        {#if feeds.error}
          the feed list could not be read, so nothing below is scoped to anything
        {:else if error}
          the store could not be read for this feed — every count on this page is this feed's store,
          so none is shown
        {:else if !feeds.active}
          no feed is chosen, so no store has been read — this control is the page's whole scope
        {:else}
          reading this feed's store — every count below is this feed's store
        {/if}
      </span>
    {/if}
  </div>
{/snippet}

<!-- ======================================================================
     ONE END OF THE MONTH WINDOW — the field, the ▦, and the calendar.
     ----------------------------------------------------------------------
     ONE SNIPPET, RENDERED TWICE. From and To differ in three things and only
     three: which state they write, which side the panel hangs off, and which
     end of the store they open at when they have no bound yet. Writing the
     panel out twice is how the two ends drift into two controls.

     `value` is the raw `YYYY-MM` and it is never anything else. `monthLabel`
     is applied at the three places text reaches a screen — the closed field,
     the grid's own `title`, and the footer — and nowhere near the `{#each}`
     key, the `aria-selected` comparison or the assignment.
     ====================================================================== -->

<style>
  /* THE HAND-ROLLED FEED MENU IS GONE, and with it the last place this page
     drew a control of its own. `.mnyb`, `.menu`, `.opt` and their tick/name/
     count children styled ONE dropdown — the broker feed — while every other
     rung here was already `$lib/Picker.svelte`. /ingest had a second, different
     hand-rolled copy of the same question. That divergence, not the class
     names, is why the two pages still looked unalike after their labels were
     unified: a reader looks at the menu. Both feed rungs are `Picker` now, so
     these rules have nothing left to match. */

  /* Everything here is expressed through `theme.css` tokens. No literal colour,
     no literal font, no second theme system — and every `transition` and
     `animation` sits inside the reduced-motion guard, the same discipline the
     theme file holds itself to. */

  .db {
    /* 40px, AND IT IS THE SAME 40 THE WINDOWING ARITHMETIC USES. `ROW = 40` in
       the script places row N at `N * 40px`; if these two ever disagree the
       rows drift away from the scrollbar a pixel per row and the list is
       unusable by the thousandth. Neither number moves without the other. */
    --dbrow: 40px;
  }

  /* ---- THE BOARD ------------------------------------------------------
     The dark field the panels sit on. The wash is two very wide radial tints
     mixed OUT OF THE THEME'S OWN ACCENTS rather than typed in — a literal
     colour here would be a second theme the toggle cannot reach, which is what
     `theme.css` lines 20-27 forbid, and it would be the wrong colour in one of
     the two modes by construction. At 5% and 4% it is a gradient the eye reads
     as depth and never as a hue. */
  .board {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    padding: var(--s5);
    background-color: var(--bg);
    background-image:
      radial-gradient(
        1100px 620px at 6% 0%,
        color-mix(in srgb, var(--acc) 5%, transparent),
        transparent 60%
      ),
      radial-gradient(
        900px 520px at 96% 0%,
        color-mix(in srgb, var(--info) 4%, transparent),
        transparent 58%
      );
  }

  /* ====================================================================
     THE CONTROL VOCABULARY — `.segs`, `.strip`, `.lead`, `.cell`, `.picker`,
     `.mnyb`, `.menu`, `.opt`, `.anchor`, `.sym`, `.px`, `.note`, `.pager`.

     WHAT THIS BLOCK REPLACES: `.sel`, `.pickers`, `.pk`, `.plbl`, `.pknote`,
     `.dd`, `.ddb`, `.ddm`, `.ddr`, `.ddnone`, `.dates`, `.dlbl`, `.ask` and
     `.foot` — /ingest's control panel, copied wholesale into /db. That copy is
     the mistake being undone. The two pages ask the same SHAPE of question and
     were told to answer it in their own voices; one vocabulary across both is
     how an edit to either silently becomes an edit to both, and it is why a
     grid of 248px boxes ended up standing in for a row of strip cells.

     NOTHING BELOW IS A LITERAL. Every colour, size, radius and space is a
     `theme.css` token, so both themes are one definition and the page has no
     second theme system the toggle cannot reach. The three tokens the old
     panel used to alias locally — `--raise`, `--panel2`, `--accdeep` — are
     still NOT aliased here: `theme.css` defines them, which is what lets
     `$lib/Picker.svelte` theme its own menus, and a local copy is the copy
     that goes stale.
     ==================================================================== */

  /* ---- THE SEGMENT TABLIST -------------------------------------------
     A tablist, sized to its own content, above what it filters. It scrolls
     rather than wraps: three tabs never need it, and a fourth segment landing
     in the store must not push the strip below it down a row. */
  .segs {
    flex: none;
    display: flex;
    gap: var(--s2);
    width: max-content;
    max-width: 100%;
    padding: var(--s1);
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r3);
    overflow-x: auto;
  }
  .seg {
    appearance: none;
    border: 0;
    background: transparent;
    color: var(--dim);
    font: inherit;
    font-size: var(--fs-base);
    font-weight: var(--w-semi);
    padding: var(--s3) var(--s6);
    border-radius: var(--r2);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: var(--s4);
    white-space: nowrap;
  }
  .seg:hover {
    color: var(--ink);
  }
  .seg:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .seg[aria-selected='true'] {
    color: var(--on-acc);
    background: var(--acc);
  }
  /* COUNTED, PER SEGMENT, AND IT IS THE WHOLE REASON THIS IS A TABLIST AND NOT
     A ROW OF WORDS. The clause that used to sit under the control said how many
     segments were held; the tabs are that enumeration, and each one carries its
     own total. */
  .seg .c {
    font-family: var(--mono);
    font-size: var(--fs-mini);
    font-variant-numeric: tabular-nums;
    color: var(--faint);
    background: var(--panel-2);
    padding: 1px var(--s4);
    border-radius: var(--r-full);
  }
  .seg[aria-selected='true'] .c {
    background: color-mix(in srgb, var(--ink-hi) 22%, transparent);
    color: var(--on-acc);
  }

  /* ---- THE STRIPS ----------------------------------------------------
     One row of controls on one raised board. `overflow: visible` is
     load-bearing and not an oversight: every menu and every calendar in the
     strip hangs BELOW its own cell, and a clipped popup is a control that opens
     into nothing.

     ONE ROW, AND THE `nowrap` IS THE WHOLE FIX FOR A BROKEN BASELINE.
     ----------------------------------------------------------------------
     This was `flex-wrap: wrap` over cells with a 232px flex BASIS, and nine
     rungs at 232px cannot share a line at any width this pane is ever given —
     so the strip broke into two flex lines. A wrapped flex line is stretched
     to ITS OWN tallest cell, not the container's, so the second line began
     wherever the first one ended: two captions, two heights, and a row that
     reads as an accident rather than a design. Nothing was wrong with any
     individual rung — there were simply more of them than the basis allowed.

     `nowrap` here plus `flex: 1 1 0` on the cell below is what makes ONE line
     unconditional: a zero basis means the sum of the hypothetical sizes is
     zero, which is never greater than the line, so there is no width at which
     the browser may wrap. Every cell then takes an equal share of what is
     left, which is the same statement as "one baseline and one height" —
     every caption starts at the same y, every control sits 2px under it, and
     every clause closes the cell on the same line.

     WHAT PAYS FOR IT IS CLIPPING, WHICH THIS STRIP ALREADY CHOSE. The
     caption, the `.mnyb` face, `Picker`'s face and the `.note` clause are all
     `nowrap`/ellipsis with the whole text on a `title` and in the document —
     that was the rule before this change and it is why narrowing a cell costs
     no fact. */
  /* THE STRIP IS /INGEST'S `.pickers` GRID, AND THAT IS THE WHOLE POINT.
   *
   * This used to be a single unbroken horizontal BAR: `display: flex`,
   * `flex-wrap: nowrap`, every rung an equal `flex: 1 1 0` share divided by
   * hairlines, with a vertical `.lead` down the left. It was a different
   * object from the control strip on /ingest, and the owner asked for one
   * object on both pages — same view, same dropdown, same search, same data.
   *
   * An earlier comment in this file recorded the opposite instruction ("the
   * owner asked for /ingest to have its OWN design, and two pages wearing one
   * vocabulary is how a change to either becomes a change to both"). That
   * decision is SUPERSEDED, on the record, by D-0153. The hazard it named is
   * real and is answered rather than ignored: the shared thing is
   * `$lib/Picker.svelte`, one component, so a change to the look lands on both
   * pages ON PURPOSE instead of drifting. What is NOT shared is either page's
   * content — the rungs, their refusals and their cascade are still this
   * file's own.
   *
   * Every metric below is /ingest's `.pickers`, copied because it is the same
   * rule and not because it is a nice number: `auto-fit` + a 248px floor is
   * what lets nine rungs wrap onto as many rows as the window allows instead
   * of nine cells fighting over one line. The bar could not wrap, so at nine
   * rungs each one got a ninth of the width and every caption clipped
   * mid-word. */
  .strip {
    /* `flex: none` IS ABOUT THE STRIP AS AN ITEM, NOT AS A CONTAINER, and the
       two roles are easy to confuse into a bug. `.board` is a flex COLUMN and
       this is one of its children; the default `flex-shrink: 1` lets a child
       be compressed below its content when the column overflows, which is what
       a tall table under it guarantees. Dropped by mistake in the grid rewrite
       below, and measured: the strip reported `height: 124px` against a
       `scrollHeight` of 409px — Timeframe and the day window were cut off at
       the panel's edge with nothing to scroll them into view. */
    flex: none;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(248px, 1fr));
    /* ---- RESET A ROW TEMPLATE THIS FILE DID NOT WRITE ----------------------
     * `theme.css` §8 also styles `.strip`, and it declares
     * `grid-template-rows: var(--lab-h) var(--ctl-h) auto` for a SUBGRID
     * vocabulary: three named rows — label, control, note — that every
     * `.field` child spans with `grid-template-rows: subgrid`, so a two-line
     * refusal under one control cannot push that control off the baseline its
     * neighbours sit on.
     *
     * That rule was always matching this element. It was INERT only because
     * this file said `display: flex`, and a row template does nothing to a
     * flex container. Switching to `display: grid` woke it, and the symptom
     * was measured before it was understood: the row holding the rungs
     * resolved to `--ctl-h` (42px) while the rungs in it were 88–104px, so
     * every one of them overflowed its own track and the day window below
     * drew straight through the feed rung's note.
     *
     * `none` rather than adoption, and the reason is wrapping. The shared
     * vocabulary is built for ONE row of fields — three explicit tracks, every
     * field pinned to `grid-row: span 3`. This strip has six rungs plus a
     * full-width day window and has to wrap to a second band at any window
     * narrower than six tracks; pinned rows cannot do that, and a dense flow of
     * repeating three-row bands is a much larger thing than the alignment it
     * would buy. /ingest reaches the same look with plain auto rows and a flex
     * column per rung, so that is what this takes. */
    grid-template-rows: none;
    gap: var(--s6) var(--s5);
    align-items: start;
    min-width: 0;
    overflow: visible;
    padding: var(--s6);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: linear-gradient(180deg, var(--panel-2), var(--panel));
    box-shadow: var(--e1);
  }
  /* THE ONE-CELL STRIP THE BLANK STATES DRAW. The feed rung is the only
     control on a page that has no table yet, so it is given a width rather
     than the whole panel: `flex: 1 1 0` would stretch one cell across a
     centred empty state, and a lone dropdown 900px wide reads as a mistake. */
  .strip.solo {
    width: max-content;
    max-width: 100%;
    margin: var(--s6) auto 0;
    /* `.blank` CENTRES ITS PROSE AND THIS IS NOT PROSE. Without this the
       caption, the face and the clause would each centre inside the cell,
       which is the one place on this page a control would not line up with
       the control it is the same control as. */
    text-align: left;
  }
  /* ONE TRACK'S WORTH, AS A WIDTH RATHER THAN A FLEX BASIS. `flex: 0 1 300px`
     described a flex item and this is a grid item now, so it did nothing at
     all — the lone feed rung stretched to `max-content`, which on a blank page
     is the width of its longest caption. /ingest solves the identical problem
     with `.blankpick { max-width: 320px }`; this is that rule. */
  .strip.solo .field {
    width: 300px;
    max-width: 100%;
  }
  /* THE CONTRACT STRIP IS THE QUIETER OF THE TWO, and it is ALWAYS DRAWN. The
     approved design collapses it when the segment carries no contract; here it
     may not, because on today's wire there is never a chain — `/store.json`
     names no strike and carries no price — and a strip that collapsed would
     take the two REFUSALS with it. "Why is there no strike control" is exactly
     the question `CLAUDE.md` §4 says must be answered out loud rather than by
     an absence, so the strip stays and its cells go `.off`. */
  /* THE FOLD'S CSS IS GONE WITH THE FOLD — six rules, removed BY NAME rather
     than as a line range. A first attempt cut from the fold's comment to the
     next selector and took the seven ported `:global(.pmenu …)` rules with it,
     which un-hid every radio and collapsed the bar table's own container. That
     is the hazard `CLAUDE.md`-adjacent notes keep restating: a CSS range is not
     a rule, and only the rule's own text bounds it. */
  /* ══ THE SAME SELECTION /ingest DRAWS, PORTED VERBATIM ══

     `Picker` is shared, so both pages already had the same MARKUP. What they
     did not share is the page-scoped treatment: /ingest carries nine
     `.field :global(.pmenu …)` rules and this file carried none, so the two
     strips rendered the same component two different ways — /ingest with the
     row carrying its own state, /db still with a gutter of hollow radio rings
     down the left edge of every menu.

     THE ROW IS THE CONTROL. Every rung on this page is `single`, so every menu
     was a column of rings with exactly one filled — a second mark saying what
     the tinted, railed row already says, and the thing that read as dated. The
     input is moved OUT OF THE FLOW, not removed: same type, same :checked, same
     change event, same tab order, so the keyboard and the reader are untouched.

     THE CHECKBOX RULES COME TOO, even though nothing here is multi-select
     today. They cost nothing while unused and they mean the two pages cannot
     drift the next time a menu on either one gains a second tick.

     Kept as page rules rather than pushed into `Picker`: the component is
     shared with /markets and /autopilot, and this is the boundary /ingest
     already established. */
  .field :global(.pmenu .plist input[type='radio']) {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    padding: 0;
    opacity: 0;
    pointer-events: none;
  }
  /* `order: 1` KEEPS THE CHECK ON THE FIRST LINE. `.pwhy` is `flex: 0 0 100%`
     so it claims a row of its own, and the check is an `::after` — last in
     source order, which lands it on a THIRD line under the sentence, alone at
     the left. Ordering the sentence after it puts the mark back beside the name
     with no pixel offset to go stale. */
  .field :global(.pmenu .plist label:has(input[type='radio']) .pwhy) {
    padding-left: 0;
    order: 1;
  }
  /* THE CHECK, DRAWN RATHER THAN TYPED — two borders on a rotated box, the
     technique `Picker` already uses for its own tick, so it cannot come out as
     a missing glyph. */
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
  /* The input carried the focus ring and the input is out of the flow, so the
     row takes it — inset, so it reads as the row being focused. */
  .field :global(.pmenu .plist label:has(input[type='radio']:focus-visible)) {
    outline: 2px solid var(--acc);
    outline-offset: -2px;
  }
  /* A row that cannot be chosen shows no check: it is struck through and
     dimmed, which `Picker` already does. */
  .field :global(.pmenu .plist label.pdis)::after {
    content: none;
  }
  .field :global(.pmenu .plist input[type='checkbox']) {
    width: 14px;
    height: 14px;
    /* 4px, not Picker's 6. On a 14px square a 6px radius is nearly half the
       side and the box renders as a ROUNDEL — the one shape a checkbox may not
       have, because round reads as radio and radio means the others untick. */
    border-radius: 4px;
  }
  .field :global(.pmenu .plist input[type='checkbox']:checked)::after {
    width: 3.5px;
    height: 7px;
    margin-top: -1px;
  }

  .strip.sub {
    background: var(--panel);
  }
  /* THE STRIP'S NAME IS A HEADING ABOVE IT, NOT A GUTTER BESIDE IT. As a flex
     child with a right hairline it was a tenth column competing with the nine
     rungs for the one line. `1 / -1` takes the full track count whatever
     `auto-fit` resolves to — the same rule /ingest uses for `.pk.wide`, and
     the reason it is `-1` rather than `span 9` is that the last line is
     wherever the window says it is. */
  .lead {
    grid-column: 1 / -1;
    font-family: var(--mono);
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }

  /* ---- A RUNG ---------------------------------------------------------
     Caption, control, and one clipped clause. The clause CLIPS RATHER THAN
     WRAPS: this is where a fact that used to be a paragraph now lives, and a
     clause that can grow to three lines would reflow the whole strip the moment
     a universe answered with a longer refusal. The text is WHOLE in the
     document, so anything reading rather than looking gets all of it, and the
     full sentence is also on the control's own `title`. */
  /* THE CELL IS THE SAME THREE ROWS EVERYWHERE — caption, control, clause —
     and that is what makes "one height" true of the CONTENT and not only of
     the boxes. `align-items: stretch` has always given the boxes one height;
     it cannot give the text one, and the two month fields carried no clause,
     so their content stopped one line short and left a hole under the field
     in a row that was otherwise closed. They have a counted clause now. */
  /* A RUNG IS /INGEST'S `.pk` — a three-row flex column and nothing else.
     The hairline dividers and the cell padding both belonged to the bar: a
     divider separates cells sharing one box, and there is no shared box now,
     just tracks with a real gap between them. Keeping either would draw the
     bar's furniture around a grid that is not a bar. */
  .strip .field {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    min-width: 0;
  }
  /* NO CELL-LEVEL FOCUS UNDERLINE. It was the bar's signal — one 2px rule
     under a group of controls sharing a hairline box — and under a grid cell
     it draws across the gap below, pointing at whatever rung wrapped onto the
     next row. Every control in here already states its own focus: `.pbtn`,
     which is the signal /ingest shows and is the one a keyboard reader
     actually needs — per control, not per group. */
  /* A RUNG THAT CANNOT BE USED SAYS SO ON ITS CAPTION AND ITS CONTROL, NEVER ON
     ITS CLAUSE. The clause is the refusal; dimming it would be the page
     whispering the one sentence that has to be read. */
  .strip .field.off > span:first-of-type {
    opacity: 0.55;
  }
  /* THE CAPTION IS /INGEST'S `.plbl`, WHICH IS THE SHARED `.lab` IN
     `theme.css` §8. Four properties differed and every one of them was
     visible: this drew the sans face at `--w-bold` in `--dim` with the default
     line-height, /ingest draws MONO at `--w-semi` in `--faint` on `--lab-h`.
     Two faces for one label is most of why the two strips did not read as the
     same control. `--lab-h` is the height the grid reserves for a label row,
     so it is also what keeps every rung's control on one baseline. */
  .strip .field > span:first-of-type {
    font-family: var(--mono);
    font-size: var(--fs-micro);
    font-weight: var(--w-semi);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    line-height: var(--lab-h);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* THERE IS NO `> span:first-of-type i` RULE ANY MORE, and the node it drew
     is gone rather than hidden. Every caption in this strip carried an italic
     sub-clause — "which membership to hold this store against", "bar length —
     the rung each row is stored at" — set inside a `nowrap`/ellipsis label.
     At a full-width basis it merely looked busy; at an EQUAL SHARE of one
     unbroken row it clips mid-word, and a caption that reads "UNIVERSE which
     membe…" is a label the page spent its width failing to say. Each clause
     moved onto the caption's own `title`, whole, which is where a gloss on a
     word belongs — and the substantive fact was never in the gloss anyway: it
     is the `.note` clause under the control. A rule left standing for a node
     nothing builds is how the sub-clause finds its way back. */
  /* THE CLAUSE IS /INGEST'S `.pknote`: MONO at `--fs-micro`, and it opens with
     a `· ` the way every caption in that strip does. The bullet is generated
     rather than typed into each of the nine strings, so a rung added later
     cannot forget it. Two lines are kept from the bar-era rule below — see
     the measurement there; /ingest reaches the same place with `.pknote.wrap`
     on the notes that need it. */
  .strip .field > .note {
    margin: 0;
    font-family: var(--mono);
    font-size: var(--fs-micro);
    color: var(--faint);
    min-width: 0;
    /* `max-width` IS THE ONE THAT MAKES THE ELLIPSIS FIRE.
     *
     * `overflow` and `text-overflow` were already here and did nothing: with
     * no upper bound the caption simply grew past its cell — FIND's measured
     * 372px inside a 239px cell — and clipped at its OWN 372, which is not a
     * clip at all. It printed straight over the ROWS cell to its right, so
     * "a prefix of up to 4 characters, one Map probe per keystroke" sat on top
     * of "0 of 9 row(s) here are short" and neither could be read.
     *
     * The full sentence is on the `title`, so nothing is lost to the pointer
     * or to a screen reader; what is lost is one caption overwriting another. */
    max-width: 100%;
    /* TWO LINES, NOT ONE CLIPPED ONE.
     *
     * Measured: these captions need 156px to 581px and the cell gives 143px, so
     * every one of the six was cut mid-word -- "9 rung(s) held · 1min 1 · 3m…",
     * "every count below is this fe…". A caption that stops before its first
     * fact is not a shorter caption, it is a blank one that costs a line.
     *
     * Clamped at two so a long one cannot push the strip open, and the whole
     * sentence is still on the `title`. The leading count -- which is the fact
     * these carry -- now fits in every one of them. */
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    white-space: normal;
    line-height: 1.35;
  }
  .strip .field > .note::before {
    content: '· ';
    color: var(--faint);
  }
  .strip .field > .note.warn {
    color: var(--warn);
  }
  .strip .field > .note.warn::before {
    color: var(--warn);
  }
  /* THE FIND BOX AND THE TWO CONTRACT RUNGS ASK FOR MORE OF THE ROW, and they
     are the only three that do: the box holds a typed instrument key and the
     two `Picker`s hold a summary like "412 of 1,204 strikes". Every other cell
     is a label and a short face.

     A LARGER `flex-grow` AND STILL A ZERO BASIS. The basis is what decides
     whether the row may break; the grow is what decides how the one row is
     divided. Asking for more of the line therefore costs nothing here — a
     wider Find box cannot push a rung onto a second baseline, which is
     exactly what a 264px basis used to do. */
  /* A GRID ITEM HAS NO FLEX BASIS, so `flex: 1.5 1 0` and `flex: 1.3 1 0` were
     inert the moment `.strip` stopped being a flex container. They asked for a
     larger share of ONE line, and the grid's answer to "this rung needs more
     room" is a whole track — `span 2` where two tracks exist, and nothing
     where they do not,
  .strip .field.mcell {
    min-width: 0;
  }

  /* ---- THE DAY WINDOW, WHICH IS /INGEST'S `.pk.wide` AND `.dates` --------
     `1 / -1` rather than `span 2` is the rule /ingest states and the reason is
     worth keeping: a spanning item is placed on the first line that FITS it,
     so `span 2` let the window finish a row of rungs whenever an odd count
     left a gap — and the row it finished was the row it then broke. `-1` is
     whatever the last line happens to be, so it is right at every track count,
     including one, where `span 2` grows an implicit second column and pushes
     the panel past its own box. */
  .strip .field.wide {
    grid-column: 1 / -1;
  }
  .strip .field.wide .dates {
    max-width: 560px;
  }
  .dates {
    display: flex;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: var(--s5);
  }

  /* THE SHARED `Picker` NOW WEARS ITS OWN FACE, AND DELETING A RULE IS THE
   * WHOLE OF THE FIX.
   *
   * What stood here stripped `$lib/Picker.svelte`'s `.pbtn` back to a 13px
   * borderless inline face — `border: 0`, `padding: 0 20px 0 0`, a 19px line
   * box, the caret pulled in to 6px. Its reasoning was sound for the strip it
   * was written against: that strip's other rungs were hand-rolled `.mnyb`
   * faces at those metrics, and a bordered box among them would have read as
   * two kinds of control in one bar.
   *
   * The premise is gone. Every rung in this strip is a `Picker` now, so there
   * is nothing left to match DOWN to — and Picker's untouched `.pbtn` is
   * already, property for property, /ingest's `.ddb`: 15px semibold mono,
   * 11px/14px padding, 9px radius, the caret at 17px/12px. /ingest says so in
   * its own comment ("every measurement below is Picker's, so the five read as
   * one control"). The two pages were drawing the SAME COMPONENT and this file
   * was the only reason they did not look it.
   *
   * The one rule kept is /ingest's, and for /ingest's stated reason: `.pbtn`
   * does not clip, so a long summary wraps to a second line and makes that ONE
   * rung taller than its neighbours — the row broken by a string rather than
   * by a rule. Picker is shared and is not edited from here, so the clip is
   * applied from this page, to Pickers inside this strip only. */
  .strip .field :global(.pbtn) {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* SHOWN AND REFUSED, which is right on a page that has one search and wrong in a
     row of borderless faces, so the box is taken off HERE — three class names
     deep, which is what beats a single-class theme rule without touching it. */
  /* THE DAY AND MONTH FIELDS TAKE /INGEST'S BOX, for the same reason the
     Picker face does: this de-styled them to the bar's borderless 19px line, radius and face, which is what makes a row of
     dropdowns and a pair of dates read as one strip. The focus outline is
     restored with them: suppressing it was only defensible while the CELL drew
     a focus underline for the whole group{
    width: 100%;
    max-width: none;
    font-size: var(--fs-base);
    padding: 11px 13px;
    border-radius: 9px;
  }
  /* The month field is the same face at the same height — it is a button rather
     than a text box, for the reason stated at the snippet, but nothing about
     that should be visible in the row. */
  /* ---- THE ANCHOR -----------------------------------------------------
     What the query is looking at, the one headline figure, the two presses that
     are actions rather than filters, and the counted line under all three.

     `.px` CARRIES NO PRICE, AND THAT IS A FINDING RATHER THAN A GAP.
     `/store.json` serves this page no price at all — `chg_bps` is a RATIO, and a
     ratio has no scale — so a traded number here would be one this page
     invented. What it carries instead is the number the whole strip exists to
     move, with its own measured up/down flash. */
  .anchor {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s3) var(--s5);
    padding: 0 var(--s2);
  }
  .sym {
    margin: 0;
    min-width: 0;
    font-family: var(--mono);
    font-size: var(--fs-xl);
    font-weight: var(--w-heavy);
    letter-spacing: -0.03em;
    color: var(--ink-hi);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .px {
    margin-left: auto;
    display: flex;
    align-items: baseline;
    gap: var(--s4);
    min-width: 0;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--s4);
    flex-wrap: wrap;
  }
  /* Not an error and not a success: a statement about what the grid above IS.
     `--warn` would overstate it, since nothing has gone wrong -- so it takes
     the muted voice and earns its attention from the bold figures in it. */
  .budget {
    display: flex;
    align-items: center;
    gap: var(--s3);
    flex-wrap: wrap;
    margin: var(--s3) 0 0;
    color: var(--dim);
    font-size: var(--fs-mini);
  }

  /* ---- THE COUNTED LINE, AND IT IS WHAT THE SIX TILES WERE ------------
     Bars, complete, bars missing, coverage and months — the same figures with
     the same sub-clauses and the same flash, on one rule under the anchor
     instead of in six boxes above the fold. The sixth is the `.px` above. */
  .anchor .note {
    width: 100%;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--s2) var(--s4);
    margin: 0;
    padding-top: var(--s4);
    border-top: 1px solid var(--line);
    font-size: var(--fs-xs);
    color: var(--faint);
  }

  .asof {
    display: inline-flex;
    align-items: center;
    gap: var(--s3);
    font-size: var(--fs-xs);
    color: var(--faint);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  /* A STAMP THAT IS NOT CURRENT WEARS THE SEVERITY HUE, not the faint one.
     `--faint` is the colour of an incidental fact; `last good:` is a warning
     that the newest read failed, and at a glance the two must not look the
     same. Amber and not `--down`, for the reason stated under `.risk`: red on
     this page means a price fell, and a stale read is a severity, not a
     direction. */
  .asof .stale {
    color: var(--warn);
  }

  .btn.sm {
    padding: var(--s2) var(--s4);
    font-size: var(--fs-xs);
  }

  .loadnote {
    padding: 0 var(--s2);
    color: var(--faint);
    font-size: var(--fs-xs);
    flex: none;
  }

  .blank .err {
    font-family: var(--mono);
    font-size: var(--fs-sm);
    color: var(--down);
    background: var(--down-soft);
    border: 1px solid var(--down);
    border-radius: var(--r2);
    padding: var(--s4) var(--s5);
    margin-bottom: var(--s5);
    word-break: break-word;
  }

  code {
    font-family: var(--mono);
    font-size: 0.94em;
    color: var(--ink-2);
  }

  /* ---- SEMANTIC COLOUR, STATED ONCE ----------------------------------
     `--up` and `--down` appear on this page in exactly two places: the tick
     flash, which marks a figure that MOVED between two reads, and the two
     percentage columns — which were RESERVING them and now use them.
     Everything about DATA QUALITY — short, near, gap — is a severity and uses
     `--warn` at two intensities. `.risk` is that hue with a name, so the
     intent is legible at the call site and cannot be mistaken for a direction.

     GREEN IS UP AND RED IS DOWN, which is the NSE convention every Indian
     broker and TradingView's India locale use — not the Japanese or mainland
     Chinese inversion. Stated because it is a convention and not a fact, and
     because a page that silently picked the other one would be unreadable to
     the only operator this is built for. */
  .risk {
    color: var(--warn);
  }

  /* ---- the completeness meter ---------------------------------------
     COLOUR IS THE SECOND SIGNAL, NEVER THE ONLY ONE. The track is hatched, so
     the missing portion has a TEXTURE and survives a greyscale print, a
     projector and every form of colour blindness; the fill is solid. The pip
     beside it is a third encoding — filled, half, hollow — because a 6px bar
     at a glance is a length and not a state. */
  .meter {
    position: relative;
    display: block;
    height: 6px;
    border-radius: var(--r-full);
    background-color: var(--well);
    background-image: repeating-linear-gradient(
      135deg,
      transparent 0 3px,
      var(--line-hard) 3px 4px
    );
    overflow: hidden;
    flex: none;
  }
  .meter .fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    border-radius: var(--r-full);
    background: var(--n7);
  }
  [data-state='full'] .meter .fill{
    background: var(--n7);
  }
  [data-state='near'] .meter .fill{
    background: color-mix(in srgb, var(--warn) 55%, var(--n7));
  }
  [data-state='gap'] .meter .fill{
    background: var(--warn);
  }

  .pip {
    width: 9px;
    height: 9px;
    flex: none;
    border-radius: 2px;
    border: 1.5px solid var(--n7);
    background: var(--n7);
  }
  [data-state='near'] .pip {
    border-color: var(--warn);
    background: linear-gradient(90deg, var(--warn) 0 50%, transparent 50% 100%);
  }
  [data-state='gap'] .pip {
    border-color: var(--warn);
    background: transparent;
  }

  /* ---- the contract rung ----------------------------------------------
     THE SAME BOX AS THE FACET BAR AND THE UNIVERSE RUNG, declared with the same
     tokens and no second spacing vocabulary — `.strip` already carries the
     frame, the gradient and the `overflow: visible` the panels hang out of, and
     this adds only the one thing that differs.

     ------------------------------------------------------------------------
     THE CHECKBOX PAINT BUG THE MOCKUP CARRIED CANNOT OCCUR HERE, and this is
     the record of the check rather than an assumption.

     In the mockup the ladder lived inside `<div class="cell mcell">`, so the
     stylesheet's own `.cell select,.cell input` reached every input in the
     panel and set `appearance:none`, `background:transparent` and `border:0`.
     The rule below it re-declared `accent-color`, `width`, `height` and
     `cursor` at the same specificity and won — so every rung carried a real,
     correctly sized, fully clickable checkbox made of transparent pixels with
     no border and no tick. Present, placed, unpainted.

     Three facts make that impossible on this page, not a control cell. The controls live in
          `.qcell`, and neither selector has an `input` or `select` descendant
          rule at all.
       3. THE CHECKBOX IS NOT IN THIS FILE'S SCOPE. It is drawn by
          `$lib/Picker.svelte`, whose `.plist input` sets `accent-color`,
          `width`, `height` and `cursor` and never touches `appearance`,
          `background` or `border`. Svelte scopes styles per component, so no
          rule written here can reach it — which is the structural reason, not
          a lucky ordering.

     The one GLOBAL rule that does reach it is `theme.css`'s `* { box-sizing;
     margin: 0; padding: 0 }`. It changes a checkbox's box and not its paint:
     `appearance` is untouched, so the UA still draws the control. Nothing is
     re-declared here to "fix" that, because a rule written against a bug this
     page does not have is a rule nobody can later tell from one that matters.
     ---------------------------------------------------------------------- */
  /* ---- THE MONTH WINDOW'S OWN CONTROL ---------------------------------
     A value that reads as a value and a button that opens the calendar. The
     value is monospace and tabular for the same reason every figure on this
     page is: `Sep 2024` and `Mar 2020` are the same width, so the two ends of
     the window line up and the arrow between them stays centred. */
  /* `.dcell` WENT WITH THE MARKUP IT POSITIONED. It was the last rule in this
     block still matching anything{
    display: flex;
    align-items: center;
    gap: var(--s3);
  }
  /* AN UNSET BOUND LOOKS UNSET. `Any earlier` at the ink weight reads as a
     month called "Any earlier"; at the faint weight it reads as the absence it
     is{
    color: var(--faint);
    font-weight: var(--w-mid);
  }

  /* THE CALENDAR. Anchored to the CELL rather than to the field{
    position: absolute;
    left: var(--s4);
    top: calc(100% + var(--s3));
    z-index: 24;
    width: 244px;
    background: var(--panel);
    border: 1px solid var(--line-hard);
    border-radius: var(--r4);
    padding: var(--s4);
    box-shadow: var(--e3);
  }
  /* A MONTH THE STORE HOLDS NOTHING FOR IS STRUCK THROUGH AND REFUSES THE
     CLICK. It keeps its slot: the shape of a backfill's hole is the most
     useful thing this panel can show while a bound is being chosen, and it is
     invisible the moment the empty months are simply left out. Amber{
    color: var(--faint);
    opacity: 0.5;
    cursor: not-allowed;
    text-decoration: line-through;
    text-decoration-thickness: 1px;
    text-decoration-color: color-mix(in srgb, var(--warn) 70%, transparent);
  }
  /* THE BOUND IS ON SCREEN — as the month, in this product's own form. It used
     to be the RAW key here, on the argument that the panel writing the string
     is the right place to read it back; the string it printed was `2024-09`,
     which is the store's spelling standing in for a date on a page where every
     other month reads `Sep 2024`. The key did not go anywhere: the field above
     and each of the twelve month buttons name it in their own `title`{
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--s3);
    margin-top: var(--s4);
    padding-top: var(--s3);
    border-top: 1px solid var(--line-soft);
    font-size: var(--fs-mini);
    color: var(--faint);
    line-height: 1.5;
  }

  /* ---- the refusal notice --------------------------------------------- */
  .why {
    flex: none;
    padding: var(--s5) var(--s6);
    border: 1px solid color-mix(in srgb, var(--info) 32%, var(--line));
    border-radius: var(--r4);
    background: var(--panel);
    box-shadow:
      inset 3px 0 0 var(--info),
      var(--e1);
  }
  .whyhead {
    display: flex;
    align-items: center;
    gap: var(--s4);
    flex-wrap: wrap;
    margin-bottom: var(--s4);
  }
  .whyhead h3 {
    font-size: var(--fs-md);
    font-weight: var(--w-semi);
    flex: 1;
    min-width: 12rem;
  }
  .whyhead h3 b {
    font-family: var(--mono);
    color: var(--ink-hi);
  }
  .whylist {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(19rem, 1fr));
    gap: var(--s4) var(--s6);
    list-style: none;
    counter-reset: why;
  }
  .whylist li {
    counter-increment: why;
    font-size: var(--fs-sm);
    line-height: 1.55;
    color: var(--dim);
    padding-left: var(--s6);
    position: relative;
  }
  .whylist li::before {
    content: counter(why);
    position: absolute;
    left: 0;
    top: 1px;
    font-family: var(--mono);
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    color: var(--info);
  }
  .whylist li b {
    color: var(--ink);
    font-weight: var(--w-semi);
  }
  .whyfoot {
    margin-top: var(--s5);
    padding-top: var(--s4);
    border-top: 1px solid var(--line-soft);
    font-size: var(--fs-xs);
    color: var(--faint);
    line-height: 1.6;
  }

  /* ---- the windowed table --------------------------------------------
     A grid of elements rather than a `<table>`, for one reason: windowing
     needs an EXACT row height, and a table row's height is negotiated between
     collapsed borders, cell padding and line-height. Here it is
     `var(--dbrow)`, stated once, and the arithmetic that places row N at
     `N * 32px` cannot drift from what the browser draws. ARIA carries the
     grid semantics that the markup no longer states. */
  /* THE TABLE IS THE LAST AND LARGEST PANEL. `overflow: hidden` is what keeps
     the sticky header and the drawer inside the rounded corners; the scrolling
     happens one level in, on `.tbl-scroll`, so nothing about the windowing
     changes. */
  .tbl {
    flex: 1;
    /* A FLOOR, NOT `0`. `min-height: 0` lets this flex child give up ALL of its
       height when the siblings above it want more, and `.board` is a
       fixed-height flex column — so on a short viewport the strip, the contract
       section, the anchor and the pager between them took 709 of 727 pixels and
       this box was left with NINETEEN. The table inside was 2,030px tall and
       fully rendered; fifty rows sat in the DOM and none of them was on screen,
       under a pager cheerfully reporting "1–50 of 4,500".
       Measured, and it is why a folded Contract strip appeared to fix a table
       that was never broken. `min-height: 0` is still what stops a flex child
       from refusing to shrink at all, so the floor replaces it rather than
       removing it: shrink, yes — to nothing, no. Past the floor the page
       scrolls, which is the honest outcome. */
    min-height: 220px;
    display: flex;
    flex-direction: column;
    position: relative; /* the drawer's containing block */
    border: 1px solid var(--line);
    border-radius: var(--r4);
    overflow: hidden;
    background: var(--panel);
    box-shadow: var(--e2);
  }
  .tbl-scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
    overscroll-behavior: contain;
  }
  .tbl-scroll:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  /* 1044px WAS THE OLD SUM AND IT IS RE-DERIVED, NOT NUDGED. The two
     percentage columns went from 96px of dead ground each to 116px of signed
     six-to-nine-character numbers under a `Prev % Change` heading, so the
     table's natural width is 224 + 92 + 48 + 104 + 86 + 128 + 150 + 116 + 116
     = 1064. Below that the table scrolls horizontally and the instrument
     column pins, exactly as before. */
  .tbl-inner {
    min-width: 1064px;
    position: relative;
  }

  /* THE PADDING LIVES IN THE CELLS, NOT ON THE ROW. A sticky column pins to
     the scroll box's left edge; with padding on the row the first cell starts
     12px in and, once pinned, leaves a 12px gutter for the other columns to
     scroll through. Moving the padding inward means the sticky cell's own box
     reaches the edge and the seam is clean. */
  .thead,
  .trow {
    display: grid;
    grid-template-columns:
      minmax(224px, 2.2fr) 92px 48px 104px 86px 128px
      minmax(150px, 1fr) 116px 116px;
    align-items: center;
  }
  .th,
  .trow .cell {
    padding-right: var(--s5);
  }
  .th.inst,
  .trow .cell.inst {
    padding-left: var(--s5);
  }

  /* 30px, AND IT IS THE `HEAD` THE CURSOR ARITHMETIC USES. `moveTo` subtracts
     this height to keep the focused row clear of the sticky header; the two
     numbers are one measurement written in two languages and neither moves
     alone. */
  .thead {
    position: sticky;
    top: 0;
    z-index: 3;
    height: 30px;
    background: linear-gradient(180deg, var(--panel-2), var(--bg-2));
    border-bottom: 1px solid var(--line);
  }
  .th {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    min-width: 0;
  }
  .th.numh {
    text-align: right;
  }
  /* IDENTITY DOES NOT LEAVE THE SCREEN. Below ~1044px the table scrolls
     horizontally — measured, not theoretical — and without this the
     instrument name is the first thing to go, leaving nine columns of numbers
     belonging to nothing. */
  .th.inst,
  .trow .cell.inst {
    position: sticky;
    left: 0;
    z-index: 2;
    background: inherit;
  }
  /* THE PINNED HEADER CELL REPAINTS THE HEAD'S OWN GRADIENT, not a flat
     approximation of it. Both boxes are 30px tall and the gradient runs top to
     bottom, so a 224px slice and a 1064px one are the same pixels — a flat
     colour here would show as a seam down the pinned column the moment the
     table scrolls sideways. */
  .th.inst {
    background: linear-gradient(180deg, var(--panel-2), var(--bg-2));
  }
  /* THE SEAM IS AN INHERITED CUSTOM PROPERTY, NOT A DESCENDANT OVERRIDE.
     Driving `opacity` from `.tbl-scroll.xscrolled .cell.inst::after` put the
     switch and the thing switched in different rules whose cascade had to be
     reasoned about — and it lost silently: the class was on the element, the
     compiled rule matched it, and the pseudo-element stayed at 0. A custom
     property inherits down to the pseudo-element unconditionally, so there is
     one declaration site and nothing to out-specify. */
  .tbl-scroll {
    --seam: transparent;
  }
  .tbl-scroll.xscrolled {
    --seam: var(--line-hard);
  }
  .th.inst::after,
  .trow .cell.inst::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    right: 0;
    width: 1px;
    background: var(--seam);
  }
  .sortbtn {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
    width: 100%;
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    color: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    cursor: pointer;
    white-space: nowrap;
  }
  /* THE LABEL STAYS FLUSH RIGHT. The caret holds its 7px whether it is lit or
     not — reserving the space is what stops the header jumping sideways when
     a column is sorted — so on a numeric column it goes to the LEFT of the
     label, and the header text lands on the same pixel as the digits under
     it. Right-aligning the whole button instead pushed every numeric heading
     11px off its column. */
  .th.numh .sortbtn {
    justify-content: flex-end;
    flex-direction: row-reverse;
  }
  .sortbtn:hover {
    color: var(--ink);
  }
  .sortbtn:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
    border-radius: var(--r1);
  }
  /* THE DIRECTION IS ON SCREEN. A dimmed caret on every column says "this
     sorts"; the lit one says which way, and `aria-sort` says the same thing
     to a reader that cannot see it. */
  .caret {
    width: 0;
    height: 0;
    border-left: 3.5px solid transparent;
    border-right: 3.5px solid transparent;
    border-bottom: 5px solid currentColor;
    opacity: 0;
    flex: none;
  }
  .sortbtn:hover .caret {
    opacity: 0.35;
  }
  .caret.on {
    opacity: 1;
    color: var(--acc);
  }
  .caret.on.desc {
    transform: rotate(180deg);
  }

  .tspace {
    position: relative;
  }
  .trow {
    position: absolute;
    left: 0;
    right: 0;
    height: var(--dbrow);
    border-bottom: 1px solid var(--line-soft);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    cursor: pointer;
    /* OPAQUE ON PURPOSE. `.cell.inst` inherits this background to sit over the
       columns sliding beneath it; a translucent row would let them show
       through the pinned column. */
    background: var(--bg-2);
  }
  .trow.odd {
    background: var(--panel-2);
  }
  .trow:hover {
    background: color-mix(in srgb, var(--acc) 7%, var(--bg-2));
  }
  /* OPAQUE, NOT `--acc-soft`. The soft tokens are alpha colours, and the
     pinned instrument cell inherits this background to cover the columns
     sliding beneath it — a translucent cursor row let the Month column show
     straight through the instrument name. Measured at scrollLeft 224. */
  .trow.cur {
    background: color-mix(in srgb, var(--acc) 14%, var(--bg-2));
    box-shadow: inset 0 0 0 1px var(--acc);
  }
  .trow.open {
    background: color-mix(in srgb, var(--acc) 14%, var(--bg-2));
  }
  /* A WHOLE SESSION MISSING GETS AN EDGE, AND IT TRAVELS WITH THE PINNED
     COLUMN. Rows with a partial-day gap get a lighter one: the marker has to
     rank, or it means nothing. Amber at two intensities — never red, which on
     this page means a number went down. */
  .trow .cell.inst {
    box-shadow: inset 3px 0 0 transparent;
  }
  .trow[data-state='near'] .cell.inst {
    box-shadow: inset 3px 0 0 color-mix(in srgb, var(--warn) 45%, transparent);
  }
  .trow[data-state='gap'] .cell.inst {
    box-shadow: inset 3px 0 0 var(--warn);
  }

  .trow .cell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* EVERY NUMERIC COLUMN IS TABULAR AND RIGHT-ALIGNED — no exceptions, so a
     digit in one row sits over the same digit in the next. */
  .trow .cell.num,
  .th.numh {
    font-variant-numeric: tabular-nums;
  }
  .trow .cell.num {
    font-family: var(--mono);
    text-align: right;
  }
  .trow .cell.mono,
  .trow .cell.tf {
    font-family: var(--mono);
    color: var(--dim);
  }
  .trow .cell.dimnum {
    color: var(--dim);
  }

  /* ---- THE TWO PERCENTAGE COLUMNS ------------------------------------
     Right-aligned and monospaced through `.cell.num`, so every glyph — the
     sign included — occupies one cell of the same grid and the decimal point
     of `+1.25%` lands on the decimal point of `-12.50%` in the row below.
     Two decimals always, from `bpsText`; nothing here is adaptive. */
  .trow .cell.pc[data-dir='up'] {
    color: var(--up);
  }
  .trow .cell.pc[data-dir='down'] {
    color: var(--down);
  }
  /* A FLAT MONTH IS NOT A DIRECTION. It is a real 0.00% and it reads as one:
     dimmed, so a screen of them does not compete with the moves. */
  .trow .cell.pc[data-dir='flat'] {
    color: var(--dim);
  }
  /* THE DASH IS AN INVITATION, NOT A SHRUG. The dotted underline and the help
     cursor say there is a reason behind it — a bare em-dash in a numeric
     column reads as "zero, rendered lazily", which is the one thing it must
     never be mistaken for. The reason itself is on `title` and `aria-label`.
     COLOUR IS NOT THE ONLY SIGNAL: the dash's shape and its underline carry
     the state with no hue at all, which is what makes the column legible in
     greyscale and under every form of colour blindness. */
  .unk {
    color: var(--faint);
    cursor: help;
    border-bottom: 1px dotted currentColor;
    padding-bottom: 1px;
  }
  .inst {
    display: flex;
    align-items: center;
    gap: var(--s3);
  }
  /* The store's own key stays whole and copyable; the part that repeats on
     every row is dimmed so the eye lands on the symbol. Rendered as text
     nodes, never markup — `M&M` is a legal symbol. */
  .ihead {
    color: var(--faint);
    font-family: var(--mono);
    font-size: var(--fs-xs);
  }
  .isym {
    font-family: var(--mono);
    font-weight: 650;
    color: var(--ink);
  }

  .short {
    display: flex;
    align-items: baseline;
    justify-content: flex-end;
    gap: var(--s3);
  }
  .okword {
    color: var(--dim);
    font-family: var(--sans);
    font-size: var(--fs-xs);
    letter-spacing: 0.04em;
  }
  /* NOT A SEVERITY AND NOT A PASS. `unverified` is the absence of a
     comparison, so it wears neither the amber of a hole nor the plain weight
     of `full`: it is dimmed and italic, and it carries its reason on hover. */
  .unproven {
    color: var(--dim);
    font-family: var(--sans);
    font-size: var(--fs-xs);
    font-style: italic;
    letter-spacing: 0.04em;
  }
  .missnum {
    color: var(--warn);
  }
  .lost {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    padding: 1px 4px;
    border-radius: 3px;
    background: var(--warn-soft);
    color: var(--warn);
  }
  .comp {
    display: flex;
    align-items: center;
    gap: var(--s4);
  }
  .comp .meter {
    flex: 1;
    min-width: 40px;
  }
  .cpct {
    font-family: var(--mono);
    font-size: var(--fs-xs);
    color: var(--dim);
    width: 4.6rem;
    text-align: right;
    flex: none;
  }
  [data-state='near'] .cpct,
  [data-state='gap'] .cpct {
    color: var(--warn);
  }

  .skelrow {
    position: static;
    align-items: center;
    background: none;
    cursor: default;
  }

  .tnone {
    padding: var(--s8) var(--s6);
    text-align: center;
  }
  .tnone h3 {
    font-size: var(--fs-md);
    margin-bottom: var(--s3);
  }
  .tnone p {
    color: var(--dim);
    font-size: var(--fs-sm);
    margin-bottom: var(--s5);
    line-height: 1.6;
  }
  /* THE GATE'S OWN LINE, AND IT IS THE ONE MESSAGE ON THIS PANEL THAT IS NOT
     GREY. Everything else here explains a filter the button below undoes; this
     explains rows the button CANNOT release, so it is drawn as the warning it
     is rather than as another line of the same paragraph. Boxed and bounded so
     a long sentence cannot run the width of a 1560px table. */
  .tnone .tgate {
    max-width: 46rem;
    margin: 0 auto var(--s5);
    padding: var(--s4) var(--s5);
    border: 1px solid color-mix(in srgb, var(--warn) 45%, var(--line));
    border-radius: var(--r2);
    background: color-mix(in srgb, var(--warn) 8%, transparent);
    color: var(--warn);
    font-size: var(--fs-sm);
    line-height: 1.6;
  }
  .tnone .tgate b {
    font-weight: var(--w-bold);
  }

  /* ---- the row drill-through ------------------------------------------ */
  .drawer {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: min(24rem, 92%);
    z-index: 5;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border-left: 1px solid var(--line-hard);
    box-shadow: var(--e3);
  }
  .dhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
    padding: var(--s4) var(--s5);
    border-bottom: 1px solid var(--line);
    background: var(--bg-2);
    flex: none;
  }
  .dtitle {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--fs-md);
  }
  .dclose {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
    white-space: nowrap;
  }
  .dstats {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1px;
    background: var(--line);
    border-bottom: 1px solid var(--line);
    flex: none;
  }
  .dstat {
    background: var(--panel);
    padding: var(--s4) var(--s5);
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .dstat .k {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }
  .dstat .v {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-md);
    font-weight: var(--w-bold);
  }
  /* ---- the day window, under the four counters ------------------------
     ONE LINE, NOT A FIFTH TILE. `.dstats` is a four-column grid and a fifth
     counter would leave three columns of empty beside it; and the window is a
     RANGE rather than a count, so it does not read as one of them anyway. */
  .dspan {
    flex: none;
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--s3);
    padding: var(--s3) var(--s5);
    border-bottom: 1px solid var(--line);
    background: var(--panel);
  }
  .dspan .k {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }
  .dspan .v {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-xs);
    color: var(--ink);
  }
  .dspan .n {
    font-size: var(--fs-mini);
    color: var(--faint);
    line-height: 1.5;
  }

  .dlist {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .drow {
    display: grid;
    grid-template-columns: 4.6rem 1fr 4.6rem 5.4rem;
    align-items: center;
    gap: var(--s4);
    padding: 0 var(--s5);
    height: 28px;
    border-bottom: 1px solid var(--line-soft);
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
  }
  .drow.this {
    background: var(--acc-soft);
    box-shadow: inset 3px 0 0 var(--acc);
  }
  .dm,
  .dpct,
  .dshort {
    font-family: var(--mono);
  }
  .dpct,
  .dshort {
    text-align: right;
    color: var(--dim);
  }
  [data-state='near'] .dshort,
  [data-state='gap'] .dshort {
    color: var(--warn);
  }
  .dfoot {
    flex: none;
    padding: var(--s4) var(--s5);
    border-top: 1px solid var(--line);
    background: var(--bg-2);
    color: var(--faint);
    font-size: var(--fs-mini);
    line-height: 1.55;
  }

  /* ---- THE PAGER ------------------------------------------------------
     A BAR UNDER THE TABLE AND NOT A LINE ON THE BOARD, which is the one place
     this page follows the approved design rather than the rule it used to hold:
     the design puts the pager inside the grid's own frame, and a footer that
     belongs to the table has to look like it does. It carries no page buttons —
     there are no pages: the table is WINDOWED, the scrollbar is the full height
     of the sorted set, and a row of numbered buttons would be a control with
     nothing to point at. Everything a pager's `.of` says is here: where you are
     standing in the run, how much of it is real right now, how fresh it is, and
     the keys that move you through it. */
  .pager {
    flex: none;
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--s2) var(--s5);
    margin: 0;
    padding: var(--s4) var(--s5);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: var(--panel-2);
  }
  .pager .of {
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .pager .of b {
    color: var(--ink-2);
    font-family: var(--mono);
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
  }
  .dclose .kbd {
    margin: 0 1px;
  }

  /* ---- MOTION. Every transition on this page is in here, so a reader who
     asked for less motion gets a completely static one. */
  @media (prefers-reduced-motion: no-preference) {
    /* THE CARET IS NOT IN HERE, DELIBERATELY. It was, and its opacity was
       animated from 0 to 1 — which meant the sort indicator was a
       *transition* rather than a *state*, and a transition can be left
       unfinished: with the tab backgrounded mid-animation the compositor
       stopped advancing it and the lit caret stayed on the previously sorted
       column while `aria-sort` had already moved. The page then said one
       thing to a screen reader and the opposite to an eye. A state indicator
       has to be right on the frame it changes, so it changes on that frame. */
    .sortbtn{
      transition:
        background-color var(--d-hover) var(--ease-out),
        border-color var(--d-hover) var(--ease-out),
        color var(--d-hover) var(--ease-out),
        opacity var(--d-hover) var(--ease-out),
        box-shadow var(--d-state) var(--ease-out);
    }
    /* THE CELL'S UNDERLINE WIPES IN FROM NOTHING. It is a focus signal, so it
       is a transition on a transform and never on the outline itself — the
       ring the control inside owns has to be right on the frame it lands. */
    .strip .field::after {
      transition: transform var(--d-state) var(--ease-out);
    }
    /* A ROW ANIMATES ITS HOVER TINT AND NOTHING ELSE. Its `box-shadow` is the
       severity spine — a state, by the same argument as the caret — so it is
       left out of the list rather than faded between two meanings. */
    .trow {
      transition: background-color var(--d-hover) var(--ease-out);
    }
    /* ROW ENTRY IS AN OPACITY FADE AND NOT A SLIDE, and that is load-bearing
       rather than taste: `theme.css`'s `.row-in` translates on the Y axis, and
       a transform on the row makes the row the containing block for its own
       `position: sticky` cell — the pinned instrument column would come
       unpinned for the 220ms the animation runs, every time the set changed. */
    .trow.row-in {
      animation: db-row-in var(--d-enter) var(--ease-out) backwards;
    }
    .th.inst::after,
    .trow .cell.inst::after {
      transition: background-color var(--d-state) var(--ease-out);
    }
    /* The bar GROWS to its new value. A pull that lands 375 bars moves every
       meter it touched, which is the difference between a page that reports
       and a page that is watched. */
    .meter .fill {
      transition:
        width var(--d-state) var(--ease-out),
        background-color var(--d-state) var(--ease-out);
    }
  }

  @keyframes db-row-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  /* ====================================================================
     THE VIEW SWITCH, THE BAR GRID AND THE PAGER
     ==================================================================== */

  /* WHICH GRID IS SHOWING, AND THE SENTENCE THAT SAYS SO, ON ONE ROW. The
     tab and the sentence are one derived value read twice{
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s3) var(--s5);
  }
  .views {
    display: inline-flex;
    gap: var(--s2);
    padding: 3px;
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: var(--panel-2);
  }
  .pgsize select {
    appearance: none;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    color: var(--ink);
    font: inherit;
    font-family: var(--mono);
    font-size: var(--fs-sm);
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    padding: var(--s3) var(--s6) var(--s3) var(--s4);
    cursor: pointer;
    background-image: linear-gradient(45deg, transparent 50%, var(--acc) 50%),
      linear-gradient(135deg, var(--acc) 50%, transparent 50%);
    background-position: calc(100% - 14px) 55%, calc(100% - 9px) 55%;
    background-size: 5px 5px, 5px 5px;
    background-repeat: no-repeat;
  }
  .pgsize select:focus-visible,
  .pgjump input:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  /* THE PARTITION THAT SUMS. read + over budget = matched{
    font-size: var(--fs-xs);
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }

  /* ---- THE BAR GRID. A real table: paged, not windowed, so the browser's
     own column algorithm can do the work. */
  .bscroll {
    overflow: auto;
  }
  .bgrid {
    border-collapse: separate;
    border-spacing: 0;
    width: 100%;
    table-layout: fixed;
    /* 14px, UP FROM 12.5. The body sets 16px and this grid was rendering the
       only numbers on the page two and a half pixels under it, in a mono face
       whose figures are already narrower than the sans around them. Measured
       before changing anything: header 11.5px, cells 12.5px, row 32px. */
    font-size: 14px;
    font-variant-numeric: tabular-nums;
    /* Figures line up in a column whatever their digits -- a 1 takes the width
       of a 9. Already implied by the mono face; stated so a future face change
       cannot quietly break the column. */
    font-feature-settings: 'tnum' 1;
  }
  .bgrid thead th {
    position: sticky;
    top: 0;
    z-index: 3;
    height: 30px;
    padding: 0 var(--s4);
    text-align: left;
    background: linear-gradient(180deg, var(--panel-2), var(--bg-2));
    border-bottom: 1px solid var(--line);
    /* THE ONE MEASURED GAP IN THIS GRID, and it is depth rather than type. The
       header is already opaque and already sticky -- measured `rgb(26,32,48)`
       -- so a row scrolling under it is correctly clipped and nothing bleeds
       through. What it had no cue for was that the clipping is a LAYER:
       `box-shadow` measured `none`, so the join between the pinned header and a
       half-scrolled row read as a cut through the row rather than as the header
       sitting over it. One hairline says which is in front. */
    box-shadow: 0 6px 12px -8px rgb(0 0 0 / 0.55);
    /* THE HEADER IS MONO BECAUSE THE COLUMN UNDER IT IS.
     *
     * The alignment was already exact -- both the header text and the cell text
     * ended on the same pixel, measured. It LOOKED wrong because the header was
     * a 11.5px sans and the figures a 12.5px mono, and two right-aligned runs
     * in different faces at different sizes do not read as one edge. Matching
     * the family and closing the size gap fixes the appearance, which is the
     * thing that was actually broken. */
    font-family: var(--mono);
    font-size: 12px;
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--dim);
    white-space: nowrap;
  }
  .bgrid thead th.numh {
    text-align: right;
    /* The header's figures sit on the same grid as the column's. */
    font-variant-numeric: tabular-nums;
  }

  /* ------------------------------------------------------------------
     THE HEADING LANDS ON THE SAME PIXEL AS THE DIGITS UNDER IT.

     Every heading in this grid is a `<button class="sortbtn">`, and that button
     is `display: inline-flex; width: 100%`. `text-align: right` on the `th`
     therefore reaches the BUTTON and stops: the label is a flex item, and a
     flex item is placed by `justify-content`, which defaulted to the start. So
     every numeric heading sat at the LEFT of a column whose data is at the
     RIGHT -- measured across the ten drawn columns, gaps of 34px to 92px, on
     every one.

     A rule for exactly this already existed and had never once applied. It was
     written `.th.numh .sortbtn`, which needs a class literally named `th`; the
     census grid above builds its headings as `<div class="th numh">` and gets
     it, while this grid builds real `<th class="numh">` elements and never
     matched. The comment over that rule promises "the header text lands on the
     same pixel as the digits under it" -- a promise the selector could not
     keep, in a file where the promise had been read as done.

     Measured after: gap 0 on all ten.
     ------------------------------------------------------------------ */
  .bgrid thead th.numh .sortbtn {
    justify-content: flex-end;
  }
  .bgrid thead th:not(.numh) .sortbtn {
    justify-content: flex-start;
  }

  /* AND THE CARET MOVES TO THE OTHER SIDE OF THE LABEL.
   *
   * The button is `<span>Open</span><span class="caret">`: a 7px caret plus the
   * flex `gap`. Reserved whether or not the column is the sorted one, which is
   * what stops the heading jumping sideways when you click it -- so it cannot
   * simply be removed. Left after the label it holds the heading 11px short of
   * the column's right edge while the digits sit flush against it, which is a
   * gap the eye reads on all eight numeric columns at once.
   *
   * `order: -1` puts it before the label, so the label itself ends on the
   * right edge -- the same pixel the digits end on. This is what the older
   * `.th.numh .sortbtn` comment described and what that selector never
   * delivered. */
  .bgrid thead th.numh .caret {
    order: -1;
  }

  /* ------------------------------------------------------------------
     THE FOURTEEN COLUMNS THAT SAY NOTHING ARE NOT DRAWN.

     Ten have NO SOURCE on this wire -- pre-market %, moneyness, intrinsic,
     extrinsic and the six greeks -- and four describe a CONTRACT: expiry,
     days-to-expiry, type and strike. A store of indices and equities has
     neither kind, so fourteen of twenty-four columns were rendered on every
     row of every page purely to read "NO SOURCE" or "-". At fifty rows a page
     that is seven hundred cells of nothing, and close and volume were pushed
     off the side of the screen behind them.

     HIDDEN BY POSITION, NOT BY FILTERING THE COLUMN LIST. The body cells are
     hand-authored in column order rather than looped, so filtering the header
     and the colgroup alone would have shifted every value one column left --
     a far worse bug than the one being fixed, and a silent one. `nth-child`
     over the fixed order keeps header, colgroup and body in lockstep by
     construction.

     `CLAUDE.md` §4 -- degrade loudly, name the reason -- is why they were
     drawn in the first place. §4 is about a FAILURE being concealed, and
     neither of these is one: a feed that never sends a greek has not failed,
     and a spot index has no expiry to report. The reason is still named once,
     in the line under the table, instead of seven hundred times inside it.
     ------------------------------------------------------------------ */
  /* 9 AND 17+, BOTH +1 FROM WHAT THEY WERE — `Time` was inserted at position 2
     and every index after it moved. The comment above is exactly right that
     `nth-child` keeps header, colgroup and body in lockstep; what it cannot do
     is keep THESE NUMBERS in step with the column list itself, which is two
     thousand lines away and has no link to them. Three separate rules were
     silently pointing one column short until the table was rendered and read. */
  .bgrid.lean > colgroup > col:nth-child(9),
  .bgrid.lean > thead > tr > th:nth-child(9),
  .bgrid.lean > tbody > tr > td:nth-child(9),
  .bgrid.lean > colgroup > col:nth-child(n + 17),
  .bgrid.lean > thead > tr > th:nth-child(n + 17),
  .bgrid.lean > tbody > tr > td:nth-child(n + 17) {
    display: none;
  }

  /* TIME (2) IS ABSENT ON A DAY RUNG. Every 1day bar opens at the same instant,
     so the column would repeat one value down every row and earn none of its
     width. Hidden rather than blanked: a column of identical stamps is not
     information, and a column of dashes is worse, because a dash means "no
     source" everywhere else on this table. */
  .bgrid.lean.noday > colgroup > col:nth-child(2),
  .bgrid.lean.noday > thead > tr > th:nth-child(2),
  .bgrid.lean.noday > tbody > tr > td:nth-child(2) {
    display: none;
  }

  /* EXPIRY (13) AND DAYS-TO-EXPIRY (14) belong to anything with a contract, so
     they appear for a future and for an option and for nothing else.

     THESE INDICES MOVED +1 WHEN `Time` WAS INSERTED AT POSITION 2, and that is
     the hazard of hiding by position: the numbers here are a silent duplicate of
     the column ORDER two thousand lines up, and nothing checks them against it.
     Adding a column at the front shifted every rule below and broke both
     contract hidings with no error anywhere — caught by rendering the table, not
     by a gate. */
  .bgrid.lean.nofuture > colgroup > col:nth-child(13),
  .bgrid.lean.nofuture > colgroup > col:nth-child(14),
  .bgrid.lean.nofuture > thead > tr > th:nth-child(13),
  .bgrid.lean.nofuture > thead > tr > th:nth-child(14),
  .bgrid.lean.nofuture > tbody > tr > td:nth-child(13),
  .bgrid.lean.nofuture > tbody > tr > td:nth-child(14) {
    display: none;
  }

  /* TYPE (15) AND STRIKE (16) belong to an OPTION only. A future has an expiry
     and no strike, so a futures selection keeps 13 and 14 and loses these two.
     That is the whole difference between the two contract shapes, and it is
     the reason these are two rules rather than one range. Also +1 — see above. */
  .bgrid.lean.nooption > colgroup > col:nth-child(15),
  .bgrid.lean.nooption > colgroup > col:nth-child(16),
  .bgrid.lean.nooption > thead > tr > th:nth-child(15),
  .bgrid.lean.nooption > thead > tr > th:nth-child(16),
  .bgrid.lean.nooption > tbody > tr > td:nth-child(15),
  .bgrid.lean.nooption > tbody > tr > td:nth-child(16) {
    display: none;
  }

  /* A COLUMN WITH NO SOURCE IS DIMMER AND ITS BUTTON IS REFUSED, not absent.
     `not-allowed` plus the reason on `title` is the pair `CLAUDE.md` §4 asks
     for: the control is visible, it does not work, and it says why. */
  .bgrid thead th.deadh {
    color: var(--faint);
    opacity: 0.62;
  }
  .bgrid thead th .sortbtn:disabled {
    cursor: not-allowed;
  }
  .brow > td {
    padding: 0 var(--s4);
    height: var(--dbrow);
    border-bottom: 1px solid var(--line-soft);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    background: var(--bg-2);
  }
  .brow.odd > td {
    background: var(--panel-2);
  }
  /* OPAQUE, AND MIXED OUT OF THE ACCENT the same way the census grid's
     cursor row is. `--d-hover` was written here first and it is a DURATION
     token, 110ms — a background that resolves to a time is dropped at
     computed-value time and the row simply never lit. */
  .brow:hover > td {
    background: color-mix(in srgb, var(--acc) 7%, var(--bg-2));
  }
  /* THE HOVER LANDS INSTEAD OF SNAPPING. Background ONLY -- never `all`, which
     would animate the row's geometry and reflow a 50-row grid on a mouse move.
     Inside `no-preference` because an operator who asked for stillness gets a
     grid that highlights instantly and never moves. */
  @media (prefers-reduced-motion: no-preference) {
    .brow > td {
      transition: background-color var(--d-hover) var(--ease-out);
    }
  }
  .bt {
    font-family: var(--mono);
    color: var(--ink-2);
  }
  .bn {
    text-align: right;
    font-family: var(--mono);
    color: var(--ink);
  }
  /* THE UNSOURCED CELLS ARE LEGIBLE AND UNMISTAKABLE. They read "no source",
     never a dash that could pass for a lazy zero, and the reason is on the
     title and the aria-label both. */
  .na {
    text-align: right;
  }
  .na .unk {
    font-family: var(--sans);
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    border-bottom-style: dotted;
  }
  /* THE BAR IS A BACKGROUND, NOT AN ELEMENT.
   *
   * One gradient on the cell that already exists, so there is no extra node
   * per row and nothing to keep in sync with the text. It grows from the
   * RIGHT, under the right-aligned figure, so the number stays the thing the
   * eye lands on and the bar is read second.
   *
   * `--mag` is set inline per cell and defaults to 0 here, so a cell that
   * carries no change — the nulls, and every column that is not this one —
   * draws nothing at all rather than a zero-width artefact.
   *
   * `prefers-reduced-motion` is honoured below: the width is a paint, not an
   * animation, but the transition on it is.
   */
  /* ---------------------------------------------------------------------
     THE CHANGE COLUMN IS A SIGNED SILHOUETTE.

     It was a bar growing from the right, coloured up or down. Colour alone is a
     poor encoding: it dies in greyscale and it dies for a red-green dichromat,
     and it gave the reader no shape to scan. Now the axis is the cell's centre
     and the bar grows RIGHT for up and LEFT for down, so direction is carried
     by POSITION and the column resolves into a distribution down fifty rows.

     THE TRANSITION HERE HAD NEVER RUN. It named `background-image`, which is a
     discrete property — no engine interpolates two gradients. `--mag` is now a
     registered property (theme.css) so the number the gradient is built from is
     what animates, and it does.
     --------------------------------------------------------------------- */
  /* THE STRANDED CLAUSE. Warm, not red: nothing is broken — a choice has simply
     been overtaken by one above it. */
  .stranded {
    margin: 0 0 var(--s3);
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s3) var(--s4);
    border: 1px solid var(--warn);
    border-left-width: 3px;
    border-radius: var(--r3);
    background: color-mix(in srgb, var(--warn) 8%, transparent);
    font-size: var(--fs-xs);
  }
  .stranded .sitem {
    display: flex;
    align-items: center;
    gap: var(--s3);
    flex-wrap: wrap;
  }

  .bpc {
    --mag: 0%;
    position: relative;
    background-image: linear-gradient(
      color-mix(in srgb, currentColor 20%, transparent) 0 100%,
      color-mix(in srgb, currentColor 20%, transparent)
    );
    background-repeat: no-repeat;
    /* A 3px band on the cell's floor, not a full-height wash: at 40px rows a
       block fights the figure it is meant to annotate. */
    background-size: var(--mag) 3px;
  }
  .bpc[data-dir='up'] {
    background-position: 50% bottom;
  }
  .bpc[data-dir='down'] {
    background-position: calc(50% - var(--mag)) bottom;
  }

  /* VOLUME, THE SAME IDEA AND DELIBERATELY NEUTRAL. Length reads as size; it
     must never read as direction, so this one has no `data-dir` and takes
     `--dim` rather than the up/down pair. Grows from the right, under the
     figure, so the number stays what the eye lands on. */
  .bvol {
    --vmag: 0%;
    background-image: linear-gradient(
      color-mix(in srgb, var(--dim) 24%, transparent) 0 100%,
      color-mix(in srgb, var(--dim) 24%, transparent)
    );
    background-repeat: no-repeat;
    background-size: var(--vmag) 3px;
    background-position: right bottom;
  }

  /* ---------------------------------------------------------------------
     THE DIRECTION SPINE — one tick per row, down the left edge.

     Height is the bar's own RANGE against the widest range at its rung, so the
     left margin becomes a profile of where the session actually moved. Colour
     is close against open. It lands inside the 8px padding the first cell
     already has, so it displaces nothing, and `overflow: hidden` on the cell
     clips it to the row.
     --------------------------------------------------------------------- */
  .brow > td:first-child {
    position: relative;
  }
  .brow > td:first-child::before {
    content: '';
    position: absolute;
    inset: 0 auto 0 0;
    width: 3px;
    background: currentColor;
    transform: scaleY(var(--rng, 0.18));
    transform-origin: center;
  }
  .brow[data-dir='up'] > td:first-child {
    color: color-mix(in srgb, var(--up) 58%, transparent);
  }
  .brow[data-dir='down'] > td:first-child {
    color: color-mix(in srgb, var(--down) 58%, transparent);
  }
  .brow[data-dir='flat'] > td:first-child {
    color: color-mix(in srgb, var(--dim) 45%, transparent);
  }

  /* O / H / L ARE CONTEXT; CLOSE IS THE FIGURE. Four prices at one weight makes
     a reader parse all four to find the one they came for. */
  .bgrid > tbody > tr > td:nth-child(3),
  .bgrid > tbody > tr > td:nth-child(4),
  .bgrid > tbody > tr > td:nth-child(5) {
    color: var(--ink-2);
  }

  @media (prefers-reduced-motion: no-preference) {
    /* `--mag`, `--vmag` and `--rng` are registered, so these interpolate. The
       rule this replaces named `background-image` and never ran. */
    .bpc,
    .bvol {
      transition:
        --mag 160ms var(--ease-out),
        --vmag 160ms var(--ease-out);
    }
    .brow > td:first-child::before {
      transition: transform 160ms var(--ease-out);
    }
  }
  .bpc[data-dir='up'] {
    color: var(--up);
  }
  .bpc[data-dir='down'] {
    color: var(--down);
  }
  .bpc[data-dir='flat'] {
    color: var(--dim);
  }
  .chip {
    display: inline-block;
    padding: 1px 6px;
    border-radius: var(--r1);
    font-family: var(--mono);
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
  }
  .chip.ce {
    color: var(--up);
    background: color-mix(in srgb, var(--up) 14%, transparent);
  }
  .chip.pe {
    color: var(--down);
    background: color-mix(in srgb, var(--down) 14%, transparent);
  }
  .bstate > td {
    padding: 0;
  }
  .bnone {
    padding: var(--s8) var(--s6);
    text-align: center;
  }
  .bnone h3 {
    font-size: var(--fs-md);
    margin-bottom: var(--s3);
  }
  .bnone p {
    color: var(--dim);
    font-size: var(--fs-sm);
    margin: 0 auto var(--s5);
    max-width: 74ch;
    line-height: 1.6;
    white-space: normal;
  }
  .bnone p.err {
    color: var(--down);
    font-family: var(--mono);
  }
  .bnotes {
    flex: none;
    border-top: 1px solid var(--line);
    background: var(--panel-2);
    padding: var(--s4) var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    max-height: 168px;
    overflow-y: auto;
  }
  .bnote {
    margin: 0;
    font-size: var(--fs-xs);
    line-height: 1.55;
    color: var(--dim);
  }
  .bnote b {
    font-family: var(--mono);
    color: var(--ink-2);
    font-weight: var(--w-semi);
  }
  .bnote.warn {
    color: var(--warn);
  }
  .bnote.err {
    color: var(--down);
  }

  /* ---- THE PAGER ---- */
  .pgbar {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s3) var(--s5);
    padding: var(--s4) var(--s5);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: var(--panel-2);
  }
  .pgof,
  .pgof2 {
    font-size: var(--fs-xs);
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .pgof b {
    color: var(--ink-2);
    font-family: var(--mono);
    font-weight: var(--w-semi);
  }
  .pgsize,
  .pgjump {
    display: inline-flex;
    align-items: center;
    gap: var(--s3);
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  /* BIG ENOUGH TO TYPE IN. A 46px box with 11px digits is a control that
     technically accepts a page number; this one is legible at arm's length
     and its own spinner is not the only way to reach page 37. */
  .pgjump input {
    width: 92px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    color: var(--ink);
    font: inherit;
    font-family: var(--mono);
    font-size: 15px;
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    padding: 10px 12px;
    text-align: right;
  }
  .pgjump input:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .pgnav {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
    margin-left: auto;
  }
  .pg {
    appearance: none;
    min-width: 34px;
    height: 32px;
    padding: 0 var(--s3);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    background: var(--panel);
    color: var(--dim);
    font: inherit;
    font-family: var(--mono);
    font-size: var(--fs-sm);
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .pg:hover:not(:disabled) {
    color: var(--ink);
    border-color: var(--dim);
  }
  .pg:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .pg:disabled {
    opacity: 0.34;
    cursor: not-allowed;
  }
  .pg[aria-current='page'] {
    background: var(--acc);
    border-color: var(--acc);
    color: var(--on-acc);
  }
  .pg.gap {
    border: 0;
    background: none;
    cursor: default;
    color: var(--faint);
    min-width: 18px;
    padding: 0;
  }

</style>

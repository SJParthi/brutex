<script>
  /**
   * MARKETS — the front door, and the only page an operator opens cold.
   *
   * ---------------------------------------------------------------------------
   * THE SHAPE, AND WHY IT IS THIS SHAPE
   * ---------------------------------------------------------------------------
   *
   * A `.helm` across the top and a `.deck` of three `.bay`s under it.
   *
   * The helm is THE STORE ADDRESS, AS A CONTROL. It reads left to right as the
   * path the bytes actually live at:
   *
   *     feed / exchange / segment / symbol / rung  →  timeframe · window
   *
   * Everything before the arrow is READ FROM DISK. Everything after it is
   * ARITHMETIC DONE IN THIS BROWSER, and it is violet for that reason and no
   * other. Each segment is a `.knob` whose FACE carries its current value, so
   * the address is legible without opening anything.
   *
   * The deck's three bays answer the three questions, in the order they are
   * asked: WHICH SERIES · WHAT IT LOOKS LIKE · WHAT IS ON DISK.
   *
   * ---------------------------------------------------------------------------
   * THREE DEFECTS THIS LAYOUT EXISTS TO KILL
   * ---------------------------------------------------------------------------
   *
   * 1. A PINNED MONTH THAT EXISTS AT ANOTHER RUNG DREW AN EMPTY CHART, with no
   *    overlay and no reason. `/store.json` has carried a real `timeframe` per
   *    row since D-0055 — `1min`, `1day` — and this page ignored it: it asked
   *    `/bars.json` without a `timeframe` parameter, which defaults to `1min`,
   *    for months filed under `1day/`. The rung is now a CONTROL on the helm,
   *    read off the census and never assumed; every read names it; a pin is a
   *    `{month, rung}` PAIR so it can never name a month at a rung the store
   *    does not hold; and the month label in the ladder is not clickable at a
   *    rung that does not hold it — its chip is, and the chip switches both.
   *
   * 2. "ITS MASTER WAS READ AND HOLDS NOTHING" WAS ASSERTED WHERE IT CANNOT BE
   *    KNOWN. An empty list and an unanswered request look identical from here.
   *    They are told apart by whether a read of THIS feed's master has
   *    completed, that read is QUOTED with the moment this page observed it,
   *    and what is not known is printed as not known.
   *
   * 3. `bars: 0` AND AN UNREADABLE COUNT RENDERED THE SAME EM DASH. They are
   *    different facts. A measured zero is a ghost `0` — the master answered
   *    and reports nothing on disk. A `bars` field that is absent or not a
   *    whole number is `unread` in amber: the count is UNKNOWN, and no number
   *    is printed for it anywhere on this page.
   *
   * ---------------------------------------------------------------------------
   * THE RULES THIS FILE IS WRITTEN AGAINST
   * ---------------------------------------------------------------------------
   *
   * ONE FEED, CHOSEN ONCE, IN THE TOP BAR. This page WRITES `feeds.active`
   * zero times and there is no second picker on it. The first knob shows the
   * choice and says where it is made; it does not offer to make it again.
   *
   * O(1) PER KEYSTROKE. `search()` is a prefix probe into a Map built once at
   * load — see `$lib/index.svelte.js`. Filtering and ordering run over the
   * MATCHES, never over the universe.
   *
   * ONLY THE VISIBLE SLICE IS IN THE DOM. 30px rows, a spacer carrying the
   * full height so the scrollbar tells the truth.
   *
   * PAISA ARE INTEGERS UNTIL THE LAST POSSIBLE MOMENT — `CLAUDE.md` §7. The
   * crosshair reads bars back out of a Map keyed on the timestamp rather than
   * off the chart, so no displayed number has been through a float.
   *
   * KEY FORM EVERYWHERE A KEY IS USED. `YYYY-MM` in each-block keys, Map keys,
   * comparisons and request parameters; `monthLabel()` only at a text node.
   *
   * (The block name is spelled in prose ON PURPOSE. Typed as a mustache it
   * counts as a real each-block opening from inside this comment, and left
   * this file at 16 opens against 15 closes. The same trap catches a TAG name
   * typed in angle brackets in a comment -- and that one was not theoretical
   * either: a literal style-element tag, a few lines below this, is what made
   * `svelte-check` report "script was left open" against the last line of this
   * file for as long as anyone had looked.
   *
   * THAT WAS NEVER ONE ERROR. `svelte-check` runs `svelte2tsx`, not the
   * compiler -- the compiler parses this file without complaint, which is why
   * it has built and run the whole time -- and svelte2tsx reads that tag,
   * inside this script's own comment, as a real element open and never
   * recovers. The single parse failure it then reports SUPPRESSES TYPE
   * CHECKING FOR THE WHOLE FILE. Taking the angle brackets out moved this file
   * from 1 reported error to 173: it had not been checked at all, and the
   * clean report was the loudest thing in it.
   *
   * So: no tag name in angle brackets, and no block in mustaches, anywhere in
   * this file's comments. It costs the entire file's type coverage and the
   * loss is invisible.)
   *
   * MONTHS FROM A LITERAL THREE-LETTER TABLE — `$lib/dates.js`. `Intl` under
   * `en-IN` emits `Sept`, four letters where the other eleven are three, and a
   * mono column stops aligning the moment September appears in it.
   *
   * EVERY COMPARATOR IS THREE-WAY. Returning `1` for equal has reappeared in
   * this repository three times; `cmp` below is the only ordering primitive.
   *
   * NOTHING DEGRADES SILENTLY — §4. Every empty list says why. Every disabled
   * control says why, on its face or its title. Every empty canvas carries an
   * overlay naming its cause; there is no branch that draws nothing and says
   * nothing. A stale value is never shown as current.
   *
   * NO COLOUR IS NAMED HERE. Every value in the stylesheet below is a
   * `theme.css` token, so the theme toggle reaches all of it. A literal in a
   * page-level style block is a second theme the toggle cannot reach.
   */
  import { catalogue, loadCatalogue, search } from '$lib/index.svelte.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { monthLabel, stampLabel } from '$lib/dates.js';
  // `group`, `rupee` AND THE LOCALE ITSELF LIVE IN `$lib/money.js` so a test can
  // drive them. Their claim -- "8,78,28,617 and never 87,828,617" -- is a
  // property of the RUNTIME's ICU data and not of this code: a build without the
  // full set falls back to `en-US` grouping and prints the wrong shape
  // confidently, in the digit grouping a reader uses to judge magnitude.
  import { group, rupee, LOC } from '$lib/money.js';
  // A REQUEST THAT CANNOT END IS A SPINNER THAT LIES. `ask` is `fetch` with a
  // ceiling; see `$lib/ask.js` for why the wrapper exists rather than a signal
  // threaded through every call site.
  import { ask } from '$lib/ask.js';
  import {
    store,
    syncStore,
    refreshStore,
    RUNG_SECONDS,
    MONTH_KEY,
    rungSeconds as rungSec
  } from '$lib/store.svelte.js';

  /* ======================================================================
     PRIMITIVES
     ====================================================================== */
  const IST_OFFSET = 19800; // +05:30 in seconds. Used for BUCKETING only.

  /**
   * The IST calendar day a UTC second falls in, as an integer.
   * @param {number} t a UTC epoch second
   */
  const istDay = (t) => Math.floor((t + IST_OFFSET) / 86400);

  /**
   * THE ONLY ORDERING PRIMITIVE. Three-way, always.
   *
   * A comparator that returns `1` for equal is not an ordering: `sort` is only
   * required to be stable, and a comparator that lies about ties makes the row
   * order depend on the engine's partition choices. That defect has landed in
   * this repository three separate times, so there is now exactly one place
   * that can get it right.
   *
   * BOTH SIDES ARE THE SAME `T` ON PURPOSE. `@param {any}` would silence the
   * checker and also permit `cmp(1, 'a')` — a relational compare between a
   * number and a string, which JavaScript answers by coercing rather than by
   * refusing, and which yields an ordering nobody intended. Binding one type
   * variable across both parameters makes that call a compile error while
   * still sorting numbers and strings alike.
   *
   * @template {string | number} T
   * @param {T} a
   * @param {T} b
   * @returns {-1 | 0 | 1}
   */
  const cmp = (a, b) => (a < b ? -1 : a > b ? 1 : 0);

  /** @param {unknown} s */
  const cap = (s) => String(s ?? '').charAt(0).toUpperCase() + String(s ?? '').slice(1);

  /**
   * A clock stamp for a read this page OBSERVED. Minutes and seconds, IST.
   *
   * `stampLabel` from `$lib/dates.js` is the day-and-minute form and is right
   * for a bar; a read that happened forty seconds ago wants its seconds.
   */
  const fClock = new Intl.DateTimeFormat('en-GB', {
    timeZone: 'Asia/Kolkata',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hourCycle: 'h23'
  });

  /* ======================================================================
     THE RUNGS ON DISK — the store's own list, not a second copy of it.
     ----------------------------------------------------------------------
     `store::path::Timeframe::KNOWN` files under these seven directory names
     and `/store.json` puts the row's OWN rung on the wire. A rung this table
     cannot map is not guessed at: the series reports `norung` and nothing is
     drawn, because drawing it would mean guessing how many seconds a bar
     covers and a wrong guess draws a convincing chart of the wrong buckets.
     ====================================================================== */
  // THE TABLE ITSELF LIVES IN `$lib/store.svelte.js`, beside the fold that
  // sorts a series by it. Two copies of "how many seconds is a 15min bar" is
  // two answers waiting to differ; the census that orders rungs and the page
  // that draws them now read one.

  /* ======================================================================
     THE DERIVED LADDER — N rungs at runtime.
     ----------------------------------------------------------------------
     The ladder is a LIST. Adding a rung appends a number and touches no code
     path, because aggregation takes a bucket width in seconds and does not
     care which widths were anticipated. Anything FINER than the rung being
     read is refused where the typing happens, not only in a tooltip: it
     cannot be derived from bars that do not contain it.
     ====================================================================== */
  const LADDER_MINUTES = [1, 2, 3, 5, 15, 30, 60];
  /* MINUTES, AND THE TYPE SAYS SO. A bare `$state([])` infers `never[]` under
     `strict`, which makes every later `.sort(cmp)` and `rungEntry(m)` on it
     unprovable — and, worse, would accept a string pushed in from a query
     parameter, which is precisely how a rung width becomes `"15m" * 60`. */
  let extraMinutes = $state(/** @type {number[]} */ ([]));
  /** @param {number} m a rung width in minutes */
  const rungEntry = (m) => ({ id: m >= 60 && m % 60 === 0 ? `${m / 60}h` : `${m}m`, seconds: m * 60 });
  const timeframes = $derived.by(() => {
    const mins = [...new Set([...LADDER_MINUTES, ...extraMinutes])].sort(cmp);
    return [...mins.map(rungEntry), { id: '1D', seconds: 86400 }];
  });

  /** `months` is how many to READ; `sessions` is how many to SHOW, or 0 for all. */
  const RANGES = [
    { id: '1D', months: 1, sessions: 1 },
    { id: '5D', months: 1, sessions: 5 },
    { id: '1M', months: 1, sessions: 0 },
    { id: '3M', months: 3, sessions: 0 },
    { id: 'MAX', months: Infinity, sessions: 0 }
  ];

  /* ======================================================================
     THE FEED — read here, chosen elsewhere.
     ====================================================================== */
  /** @param {string | null | undefined} wire */
  const feedLabel = (wire) => feeds.all.find((f) => f.wire === wire)?.display ?? wire ?? 'no feed';

  /* ======================================================================
     THE STORE CENSUS — which months exist, AT WHICH RUNG, for THIS feed.
     ----------------------------------------------------------------------
     `/store.json` answers (instrument, month, timeframe, rows) for the whole
     store. Fetched ONCE per feed and folded into a Map keyed by
     `EXCHANGE-SEGMENT-SYMBOL` — the spelling `census::Series` writes — so
     asking "what do I hold for this instrument" is one probe.

     EVERY FIELD IS CHECKED, AND A ROW THAT FAILS IS KEPT RATHER THAN DROPPED.
     A census row whose `rows` is absent, negative or not a whole number is a
     row whose count is UNKNOWN. Dropping it would make it indistinguishable
     from a month that does not exist; coercing it to zero would print a
     measurement nobody took. It goes into `censusBad` with the reason, and
     every total computed from that series says so.
     ====================================================================== */
  /**
   * THE SPELLING `census::Series` WRITES, built from a master row.
   *
   * A missing part interpolates as the string `undefined`, which would key a
   * lookup that silently matches nothing rather than announcing that the row
   * was unusable. `??` puts an EMPTY segment there instead: `NSE--NIFTY` still
   * fails to match, and it fails visibly, in a key a reader can recognise.
   *
   * @param {import('$lib/index.svelte.js').MasterRow} row
   */
  const seriesKey = (row) =>
    `${row.exchange ?? ''}-${row.segment ?? ''}-${row.symbol ?? ''}`;

  /* ----------------------------------------------------------------------
     THE FOLD IS NOT THIS PAGE'S ANY MORE.
     ----------------------------------------------------------------------
     This block used to own a fetch, a token, five pieces of flat state, a
     field validator and a sort — and it was ONE OF SIX such blocks reading
     `/store.json` across the product, each on its own clock, none reconciled
     against the others. During a pull this page and `/db` could show different
     totals for the same disk and both were "correct" as of their own snapshot.
     `$lib/store.svelte.js` reads it ONCE per (feed, generation), folds every
     shape any page needs in ONE pass, and stamps the answer with the feed it
     answered for. What is left here are the names this page already used,
     pointed at that one reading.

     `syncStore` is the subscription: it reads `feeds.active` and the shared
     generation, so a feed change re-reads and a Refresh anywhere — including
     the poll `/ingest` holds while a pull runs — arrives here too.
     ---------------------------------------------------------------------- */
  $effect(() => syncStore(feeds.active));

  // 'none' | 'wait' | 'ready' | 'error', in this page's own words. `store.feed`
  // is checked, not assumed: a reading that answered for another feed is not
  // this page's census however recently it landed.
  const censusState = $derived(
    store.state === 'error'
      ? 'error'
      : store.state === 'ready' && store.feed === feeds.active
        ? 'ready'
        : store.state === 'reading' || (store.state === 'ready' && store.feed !== feeds.active)
          ? 'wait'
          : 'none'
  );
  const censusError = $derived(store.error);
  /* THE EMPTY FALLBACK IS TYPED LIKE THE REAL ONE.
     `store.byInstrument` is `Map<string, StoreCell[]>`, but a bare `new Map()`
     beside it is `Map<any, any>` — so the union of the two arms is a map whose
     `.get()` answers `any`, and every callback that walks a month list
     downstream took an implicitly-`any` parameter. One untyped empty
     collection at the top of a derivation erased the types of a dozen readers
     of it, which is the compounding `noImplicitAny` exists to stop. */
  const censusMonths = $derived(
    censusState === 'ready'
      ? store.byInstrument
      : /** @type {typeof store.byInstrument} */ (new Map())
  );
  const censusBad = $derived(
    censusState === 'ready'
      ? store.badByInstrument
      : /** @type {typeof store.badByInstrument} */ (new Map())
  );
  // THE STAMP IS THE CURRENT VALUE'S OR IT IS NOTHING. `store.at` is already
  // cleared on a failed read, so this cannot print a minute over a refusal.
  const censusStamp = $derived(censusState === 'ready' ? store.at : null);

  /* ======================================================================
     THE MASTER, AND THE MOMENT THIS PAGE SAW IT ANSWER.
     ----------------------------------------------------------------------
     `catalogue.ready` is the only evidence that a read COMPLETED, and the
     difference between "answered with no rows" and "never answered" is the
     whole of defect 2 above. The stamp is what this browser OBSERVED — it is
     labelled that way and is never dressed up as a server timestamp.
     ====================================================================== */
  /** @type {number | null} */
  let masterStamp = $state(null);
  /** @type {string | null} */
  let masterStampFeed = $state(null);
  $effect(() => {
    const ready = catalogue.ready;
    const f = catalogue.feed;
    if (ready && f) {
      masterStamp = Date.now();
      masterStampFeed = f;
    }
  });

  // COUNTING ONE BROKER'S MASTER UNDER ANOTHER BROKER'S NAME IS THE §4 FAILURE
  // this whole page exists to avoid. Nothing below prints a number unless this
  // holds.
  const masterIsThisFeed = $derived(catalogue.ready && catalogue.feed === feeds.active);
  const rows = $derived(catalogue.rows);
  const observedAt = $derived(masterStampFeed === feeds.active && masterStamp != null ? masterStamp : null);

  /* ======================================================================
     THE FIRST HALF OF THE ONE LIST — what stops every bay on the page.
     ----------------------------------------------------------------------
     Order matters: the first thing that makes the rest unanswerable wins, so
     the message names the narrowing that did it. Three bays read this one
     function, so they can differ about what to SHOW and can never differ
     about WHAT HAPPENED.
     ====================================================================== */
  /* THE SAME SHAPE AS `chartReason`, BECAUSE IT IS ONE OF ITS ANSWERS.
     `chartReason` opens with `if (blocked) return blocked;`, so these five
     branches are already part of that union whether or not they say so, and
     inferring `kind` as `string` here is what made the declared nine
     unassignable. Two names for one shape is the drift where a kind added to
     one is unhandled by the other. */
  const blocked = $derived.by(/** @returns {ChartReason | null} */ () => {
    if (feeds.error)
      return {
        kind: 'error',
        tsub: 'the feed list failed',
        why: `/feeds.json could not be read — ${feeds.error}. With no list of feeds there is no feed to scope a read to.`
      };
    // TWO DIFFERENT UNANSWERED ROOTS, AND THEY ARE NOT THE SAME SENTENCE. A
    // feed list still in flight has not OFFERED a choice yet; saying "no feed
    // chosen" of it blames the operator for a request that has not landed.
    if (!feeds.active && feeds.all.length === 0)
      return {
        kind: 'wait',
        tsub: 'reading the feed list',
        why: 'Reading /feeds.json. Until it answers there is no feed to choose, so there is nothing yet to scope a master or a census to — this is a read in flight, not an unmade choice.'
      };
    if (!feeds.active)
      return {
        kind: 'wait',
        tsub: 'no feed chosen',
        why: 'A bar belongs to the vendor that supplied it, so with no feed selected there is no master to list and no store to chart. This is an unmade choice, not an empty store — it is made in the top bar.'
      };
    if (catalogue.error)
      return { kind: 'error', tsub: 'the master failed', why: `/instruments.json could not be read — ${catalogue.error}` };
    if (catalogue.ready && catalogue.feed !== feeds.active)
      return {
        kind: 'wait',
        tsub: 'reading the master',
        why: `Reading ${feedLabel(feeds.active)}'s instrument list — the names on hand are still ${feedLabel(catalogue.feed)}'s, and nothing on this page is counted from another feed's master.`
      };
    if (!catalogue.ready)
      return { kind: 'wait', tsub: 'reading the master', why: `Reading ${feedLabel(feeds.active)}'s instrument list…` };
    return null;
  });

  /** Why no count can be printed. Never a bare refusal. */
  const countRefusal = $derived.by(() => {
    if (feeds.error) return 'the feed list could not be read';
    if (!feeds.active) return 'no feed is chosen, so there is no master to count';
    if (catalogue.error) return '/instruments.json could not be read';
    if (!catalogue.ready) return 'the instrument list has not been read yet';
    return `the list on hand answers for ${feedLabel(catalogue.feed)}, not for ${feedLabel(feeds.active)}`;
  });

  /* ======================================================================
     THE ROSTER — search, facets, sort, a windowed list.
     ====================================================================== */
  let typed = $state('');
  let universe = $state('all');
  let segment = $state('');
  let heldOnly = $state(false);
  let unreadOnly = $state(false);
  let sortKey = $state('bars');
  let sortDir = $state(-1);
  /** @type {HTMLInputElement | null} */
  let searchEl = $state(null);
  /** @type {'sym' | 'base' | 'tf' | 'uni' | null} */
  let openTray = $state(null);

  const UNIVERSES = [
    { id: 'n50', label: 'NIFTY 50', token: 'n50' },
    { id: 'n100', label: 'NIFTY 100', token: 'n100' },
    { id: 'n200', label: 'NIFTY 200', token: 'n200' },
    { id: 'n500', label: 'NIFTY 500', token: 'n500' },
    { id: 'ntm', label: 'NIFTY Total Market', token: 'ntm' },
    { id: 'fno', label: 'F&O underlyings', token: 'fno' },
    { id: 'index', label: 'NSE indices', token: 'index' },
    { id: 'all', label: 'Everything', token: '*' }
  ];
  // The four NIFTY tiers exist ONLY in the `universes` ARRAY. The frozen
  // `universe` STRING is `index`, `fno`, `ntm`, those joined by `+`, or
  // `other` — it has never carried a tier and never will. See D-0089/D-0090.
  const TIERSET = new Set(['n50', 'n100', 'n200', 'n500']);

  /**
   * THE MEMBERSHIP, AS A SET, AND ONLY IF THE ROWS CARRY ONE.
   *
   * An API binary that predates D-0089 emits no `universes` array. The four
   * tiers are then UNANSWERABLE from these rows — and they are refused in
   * place, with the reason, rather than synthesised from an alphabetical
   * order that would look exactly like a real answer.
   */
  /** @param {import('$lib/index.svelte.js').MasterRow} row */
  const uniTokens = (row) =>
    Array.isArray(row.universes)
      ? row.universes
      : String(row.universe ?? '')
          .split('+')
          .filter((t) => t && t !== 'other');
  const tiersAnswerable = $derived(rows.length === 0 || rows.some((r) => Array.isArray(r.universes)));

  /**
   * @param {import('$lib/index.svelte.js').MasterRow} row
   * @param {string | null | undefined} token
   */
  function inUniverse(row, token) {
    if (token === '*') return true;
    if (token == null) return false;
    if (TIERSET.has(token) && !tiersAnswerable) return false;
    return uniTokens(row).includes(token);
  }

  /**
   * FOUR ANSWERS FOR ONE COLUMN, AND THEY NEVER LOOK ALIKE.
   *
   *   count   — a measurement.
   *   zero    — a DIFFERENT measurement: the master answered and reports
   *             nothing on disk. This instrument has never been pulled.
   *   unread  — NOT a measurement. The `bars` field is absent or is not a
   *             whole number, so the count is unknown and no number will be
   *             printed for it anywhere.
   *   nomaster— nothing about this row has been measured at all, because the
   *             master on hand is not this feed's.
   */
  /** @param {import('$lib/index.svelte.js').MasterRow | null | undefined} row */
  const barsOf = (row) =>
    Number.isInteger(row?.bars) && /** @type {number} */ (row?.bars) >= 0
      ? /** @type {number} */ (row?.bars)
      : null;
  /** @param {import('$lib/index.svelte.js').MasterRow} row */
  function barState(row) {
    if (!masterIsThisFeed) return { kind: 'nomaster', wait: !catalogue.error, why: countRefusal };
    const n = barsOf(row);
    if (n === null) return { kind: 'unread', why: `this row's \`bars\` field is ${JSON.stringify(row?.bars)}` };
    if (n === 0) return { kind: 'zero' };
    return { kind: 'count', n };
  }

  const hits = $derived.by(() => {
    // READ THE CATALOGUE EXPLICITLY. `search()` probes a plain Map that is not
    // reactive, so with a non-empty query this derived would never learn that
    // the universe was rebuilt under it.
    void catalogue.rows.length;
    void catalogue.ready;

    const spec = UNIVERSES.find((u) => u.id === universe) ?? null;
    const token = spec?.token ?? null;
    const matched = search(typed).filter(
      (row) =>
        inUniverse(row, token) &&
        (segment === '' || row.segment === segment) &&
        (!heldOnly || (barsOf(row) ?? 0) > 0) &&
        (!unreadOnly || barState(row).kind === 'unread')
    );
    const dir = sortDir;
    // The header refuses the bars sort with no master; this is the same
    // refusal, said in the order the rows come out in.
    const key = sortKey === 'bars' && !masterIsThisFeed ? 'symbol' : sortKey;
    // Ordering runs over the MATCHES, never over the universe.
    //
    // THE TIE-BREAK COERCES ONCE, HERE. `symbol` comes off the wire and is
    // typed as possibly absent, and `cmp` binds one type across both sides so
    // it cannot be handed `string | undefined`. Naming the empty string as the
    // stand-in makes a row with no symbol sort FIRST and sort STABLY, which is
    // what an ordering primitive that refuses to lie about ties requires; the
    // alternative — letting `undefined` through — makes every comparison
    // against it return 0, and a comparator that calls unequal rows equal is
    // the exact defect the header of `cmp` says has landed here three times.
    const name = (/** @type {import('$lib/index.svelte.js').MasterRow} */ r) => r.symbol ?? '';
    return [...matched].sort((a, b) => {
      if (key === 'bars') return cmp(barsOf(a) ?? 0, barsOf(b) ?? 0) * dir || cmp(name(a), name(b));
      if (key === 'segment')
        return cmp(String(a.segment), String(b.segment)) * dir || cmp(name(a), name(b));
      return cmp(name(a), name(b)) * dir;
    });
  });

  /**
   * THE PARTITION, AND IT SUMS. Every listed row is in exactly one bucket.
   * Counted over the WHOLE master and only when the master changes — never
   * per keystroke. A count that moved as you typed would be answering a
   * different question from the one on the control.
   */
  const counts = $derived.by(() => {
    const uni = Object.fromEntries(UNIVERSES.map((u) => [u.id, 0]));
    const seg = new Map();
    let held = 0;
    let zero = 0;
    let unread = 0;
    for (const row of rows) {
      for (const u of UNIVERSES) if (inUniverse(row, u.token)) uni[u.id] += 1;
      seg.set(row.segment, (seg.get(row.segment) ?? 0) + 1);
      const st = barState(row);
      if (st.kind === 'count') held += 1;
      else if (st.kind === 'zero') zero += 1;
      else if (st.kind === 'unread') unread += 1;
    }
    return { uni, seg, held, zero, unread };
  });
  /* A ROW WITH NO SEGMENT CONTRIBUTES NO SEGMENT. Coercing it to `''` would
     put a nameless entry in the segment filter that matches nothing an
     operator can see; dropping it leaves the filter listing only segments
     that exist, which is what the control claims to list. */
  const segments = $derived(
    [...new Set(rows.map((r) => r.segment).filter((s) => typeof s === 'string'))].sort(cmp)
  );

  const narrowing = $derived(
    [
      typed.trim() ? `the text “${typed.trim()}”` : null,
      universe !== 'all' ? `universe ${UNIVERSES.find((u) => u.id === universe)?.label ?? universe}` : null,
      segment ? `segment ${segment}` : null,
      heldOnly ? 'held only' : null,
      unreadOnly ? 'unreadable count only' : null
    ].filter(Boolean)
  );
  function clearNarrowing() {
    typed = '';
    universe = 'all';
    segment = '';
    heldOnly = false;
    unreadOnly = false;
    scrollTop = 0;
    if (scroller) scroller.scrollTop = 0;
  }

  function sortBy(key) {
    if (sortKey === key) sortDir = -sortDir;
    else {
      sortKey = key;
      sortDir = key === 'bars' ? -1 : 1; // biggest first for a count, A→Z for a name
    }
  }
  const arrow = (key) => (sortKey !== key ? '' : sortDir === 1 ? '▲' : '▼');

  /* ---- the virtual window ---------------------------------------------- */
  const ROW = 30;
  const OVERSCAN = 6;
  /** @type {HTMLElement | null} */
  let scroller = $state(null);
  let scrollTop = $state(0);
  let viewportH = $state(600);
  const firstRow = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN));
  const windowLen = $derived(Math.ceil(viewportH / ROW) + OVERSCAN * 2);
  const visible = $derived(hits.slice(firstRow, firstRow + windowLen));

  // A NEW ANSWER STARTS AT THE TOP AND WITH NO CURSOR. Leaving the cursor
  // where it was would put "what Enter opens" on a row nobody is looking at.
  $effect(() => {
    void typed;
    void universe;
    void segment;
    void heldOnly;
    void unreadOnly;
    void sortKey;
    void sortDir;
    void catalogue.rows.length;
    cursor = -1;
    if (scroller) scroller.scrollTop = 0;
    scrollTop = 0;
  });

  /* ======================================================================
     SELECTION AND KEYBOARD
     ----------------------------------------------------------------------
     Only the KEY is state. The row is derived, so when the feed changes and
     the new feed does not list this instrument, `picked` becomes null and the
     page says so by name rather than charting a stale row.
     ====================================================================== */
  /** @type {string | null} */
  let pickedKey = $state(null);
  let cursor = $state(-1);

  const byKey = $derived(new Map(rows.map((r) => [r.key, r])));
  const picked = $derived(pickedKey ? (byKey.get(pickedKey) ?? null) : null);

  // A COLD PAGE SHOWS SOMETHING. `didPick` is a plain variable, not state, so
  // this runs once and never fights an operator's own choice.
  let didPick = false;
  $effect(() => {
    const all = catalogue.rows;
    if (didPick || all.length === 0) return;
    didPick = true;
    const held = all.filter((r) => (barsOf(r) ?? 0) > 0);
    /* A ROW THAT CANNOT NAME ITSELF IS NOT A DEFAULT SELECTION. `key` comes
       off the wire, so `?? null` here means "nothing was picked" rather than
       picking a row whose key is the string `undefined` — which every lookup
       downstream would then fail to match, silently, and for a reason no
       message would name. */
    pickedKey = (held.find((r) => r.key === 'NSE-NIFTY') ?? held[0] ?? all[0]).key ?? null;
  });

  function pick(row) {
    if (!row) return;
    pickedKey = row.key;
    pin = null;
    hover = null;
    openTray = null;
  }

  /**
   * @param {number} delta rows to move by, signed
   * @param {number | null} [absolute] a row to jump to instead. The default is
   *        `null` and NOT `undefined`, so inference read the parameter as
   *        `null` alone and rejected every caller that passed a number.
   */
  function moveCursor(delta, absolute = null) {
    const n = hits.length;
    if (n === 0) return;
    const next = absolute != null ? absolute : cursor < 0 ? (delta > 0 ? 0 : n - 1) : cursor + delta;
    cursor = Math.max(0, Math.min(n - 1, next));
  }

  // Keep the cursor on screen. Writes to the DOM, never to reactive state.
  $effect(() => {
    const at = cursor;
    const el = scroller;
    if (!el || at < 0) return;
    const top = at * ROW;
    if (top < el.scrollTop) el.scrollTop = top;
    else if (top + ROW > el.scrollTop + el.clientHeight) el.scrollTop = top + ROW - el.clientHeight;
  });

  function onSearchKey(e) {
    const page = Math.max(1, Math.floor(viewportH / ROW) - 1);
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        moveCursor(1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        moveCursor(-1);
        break;
      case 'PageDown':
        e.preventDefault();
        moveCursor(page);
        break;
      case 'PageUp':
        e.preventDefault();
        moveCursor(-page);
        break;
      case 'Home':
        if (typed === '') {
          e.preventDefault();
          moveCursor(0, 0);
        }
        break;
      case 'End':
        if (typed === '') {
          e.preventDefault();
          moveCursor(0, hits.length - 1);
        }
        break;
      case 'Enter':
        e.preventDefault();
        pick(cursor >= 0 ? hits[cursor] : hits[0]);
        break;
      case 'Escape':
        if (typed !== '') {
          e.preventDefault();
          typed = '';
          cursor = -1;
        }
        break;
    }
  }

  // `/` focuses the search from anywhere — never while typing into something.
  function onWindowKey(e) {
    if (e.key === 'Escape' && openTray) {
      openTray = null;
      return;
    }
    if (e.key !== '/' || e.metaKey || e.ctrlKey || e.altKey) return;
    const el = document.activeElement;
    const tag = el?.tagName;
    /* `instanceof`, NOT A CAST. `document.activeElement` is an `Element`, and
       `isContentEditable` is declared on `HTMLElement` — an SVG element in
       focus has no such property, so asserting one would be claiming a field
       that genuinely is not there rather than narrowing to the case where it
       is. The runtime test is the same test the type wants. */
    const editable = el instanceof HTMLElement && el.isContentEditable;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || editable) return;
    e.preventDefault();
    searchEl?.focus();
    searchEl?.select();
  }

  /* ======================================================================
     WHAT IS HELD FOR THE PICKED INSTRUMENT
     ====================================================================== */
  /**
   * A CENSUS KEY BACK TO AN INSTRUMENT KEY, PARSED AT THE FIRST TWO HYPHENS
   * AND NOWHERE ELSE.
   *
   * `census::Series` spells `EXCHANGE-SEGMENT-SYMBOL`; `instrument::Key`
   * spells `EXCHANGE-SYMBOL`. The two hyphen-free tokens are the exchange and
   * the segment, so cutting at the first two hyphens and keeping ALL of the
   * rest is exact. A `split('-')` is not: `BAJAJ-AUTO` is a real NSE symbol
   * and splitting it would file it under a segment called `BAJAJ`.
   */
  function instrumentKeyOf(censusKey) {
    const a = censusKey.indexOf('-');
    if (a < 0) return null;
    const b = censusKey.indexOf('-', a + 1);
    if (b < 0) return null;
    return `${censusKey.slice(0, a)}-${censusKey.slice(b + 1)}`;
  }
  const censusByInstrument = $derived.by(() => {
    const out = new Map();
    for (const [k, months] of censusMonths) {
      const ik = instrumentKeyOf(k);
      if (ik === null) continue;
      let list = out.get(ik);
      if (!list) out.set(ik, (list = []));
      list.push(...months);
    }
    return out;
  });
  /**
   * HELD UNDER THIS FEED AND ABSENT FROM ITS MASTER.
   *
   * The store does not forget: the BSE history the charter keeps is on disk
   * and no master carries it, and a name a vendor has since dropped is the
   * other shape of the same fact. Neither is reachable from the roster, so
   * both are COUNTED in the footer rather than quietly disappearing. Compared
   * on the census's own key spelling, so nothing is parsed to do it.
   */
  const storedNotListed = $derived.by(() => {
    if (censusState !== 'ready' || !masterIsThisFeed) return [];
    const listed = new Set(rows.map((r) => seriesKey(r)));
    return [...censusMonths.keys()].filter((k) => !listed.has(k)).sort(cmp);
  });

  const heldMonths = $derived(picked ? (censusMonths.get(seriesKey(picked)) ?? []) : []);
  const badMonths = $derived(picked ? (censusBad.get(seriesKey(picked)) ?? []) : []);
  const censusTotal = $derived(heldMonths.reduce((a, m) => a + m.rows, 0));

  /**
   * Every rung this build can map a bar length to, finest first.
   *
   * THE LENGTH IS RESOLVED ONCE PER RUNG, NOT ONCE PER COMPARISON. The sort
   * called `rungSec` inside the comparator, so a set of n rungs resolved it
   * O(n log n) times to order n values — and the comparator was handed
   * `number | null` besides, because the filter above it narrows the ARRAY and
   * not the return of a second call. Pairing each rung with its length, then
   * dropping the unmappable ones with a predicate that narrows, gives the
   * comparator two numbers and calls the resolver n times.
   */
  const storedRungs = $derived(
    [...new Set(heldMonths.map((m) => m.timeframe))]
      .map((t) => ({ t, s: rungSec(t) }))
      .filter(/** @returns {x is { t: string, s: number }} */ (x) => x.s !== null)
      .sort((a, b) => cmp(a.s, b.s))
      .map((x) => x.t)
  );
  /** Rungs on disk this build has no bar length for. Named, never guessed at. */
  const unmappedRungs = $derived([...new Set(heldMonths.map((m) => m.timeframe))].filter((t) => rungSec(t) === null));

  // THE RUNG BEING READ IS A PREFERENCE VALIDATED AGAINST THE CENSUS, not a
  // stored fact. An effect that reads the rung and writes the rung depends on
  // its own output; a derived cannot. Selecting a new instrument therefore
  // cannot leave a rung on the address bar that no file on disk answers to.
  /** @type {string | null} */
  let basePref = $state(null);
  /* THE NULL PREFERENCE IS NOT A MEMBER, AND IT NEVER WAS. `storedRungs` is
     `string[]` now that its length lookup is typed, so `.includes(null)` is
     rejected rather than quietly answered `false` — which is the answer it was
     always giving. Saying it in the guard costs nothing and stops the
     un-chosen case from riding on a coincidence of `Array.includes`. */
  const base = $derived(
    basePref !== null && storedRungs.includes(basePref) ? basePref : (storedRungs[0] ?? null)
  );

  /* THE FLOOR IS READ ONCE AND GUARDED. `base` being non-null does not make
     `rungSec(base)` non-null — that is a second question about the same value,
     and comparing `t.seconds >= null` coerces to `>= 0`, which offers EVERY
     rung as derivable from a base whose length this build cannot map. It is
     unreachable today because `storedRungs` drops unmappable rungs before
     `base` can name one, but the arm that would be wrong is the one that
     silently offers everything. */
  const offered = $derived.by(() => {
    if (base === null) return [];
    const floor = rungSec(base);
    if (floor === null) return [];
    return timeframes.filter((t) => t.seconds >= floor);
  });
  let tfPref = $state('5m');
  const tf = $derived(offered.some((t) => t.id === tfPref) ? tfPref : (offered[0]?.id ?? null));
  const tfSeconds = $derived(offered.find((t) => t.id === tf)?.seconds ?? null);
  const asStored = $derived(base !== null && tfSeconds === rungSec(base));

  let range = $state('1M');
  /** @type {{ month: string, tf: string } | null} */
  let pin = $state(null); // {month, tf} — always both, never a bare month

  const monthsAtBase = $derived(base === null ? [] : heldMonths.filter((m) => m.timeframe === base));

  /**
   * THE PIN'S OWN HEALTH, NAMED. `elsewhere` is the state the old page drew
   * as a blank canvas: a pinned month that exists — at another rung.
   */
  const pinState = $derived.by(() => {
    /* READ ONCE INTO A LOCAL, and not only for the checker's benefit. `pin` is
       reactive state, so each `pin.month` below was a separate read: the
       narrowing from the guard does not survive into the two closures — which
       is what `svelte-check` reported — and in principle three reads of a
       mutable value need not agree with one another. One read, one value, and
       the guard then covers every use of it. */
    const p = pin;
    if (!p) return null;
    if (monthsAtBase.some((m) => m.month === p.month)) return { ok: true };
    return { ok: false, elsewhere: heldMonths.filter((m) => m.month === p.month) };
  });

  // WHICH MONTHS TO ACTUALLY READ. `/bars.json` is one month per request — the
  // path is the index — so the range decides how many requests.
  const wantMonths = $derived.by(() => {
    if (monthsAtBase.length === 0) return [];
    /* READ ONCE, THEN USE THE READING. `pin` is `$state`, so a narrowing on it
       does not survive into a callback — the checker is right that a mutable
       binding can change between the test and the use, and refusing to carry
       the narrowing is not pedantry. Taking one observation into a `const`
       makes the whole derivation agree about which month is pinned, which is
       the property this wanted anyway. */
    const at = pin;
    if (at) return monthsAtBase.filter((m) => m.month === at.month).map((m) => m.month);
    const spec = RANGES.find((r) => r.id === range) ?? RANGES[2];
    const all = monthsAtBase.map((m) => m.month);
    return spec.months === Infinity ? all : all.slice(-spec.months);
  });
  /** Records the census says those month files hold, at the rung being read. */
  const wantRecords = $derived(
    monthsAtBase.filter((m) => wantMonths.includes(m.month)).reduce((a, m) => a + m.rows, 0)
  );

  function pinMonth(month, rung) {
    basePref = rung;
    pin = pin && pin.month === month && pin.tf === rung ? null : { month, tf: rung };
    hover = null;
  }

  /* ---- the add-a-rung box ---------------------------------------------- */
  let addRung = $state('');
  const rungRefusal = $derived.by(() => {
    if (addRung === '' || addRung == null) return null;
    const n = Number(addRung);
    if (!Number.isInteger(n) || n < 1 || n > 1440) return `${addRung} is not a whole number of minutes between 1 and 1440.`;
    if (base === null) return 'No rung on disk can be read here, so nothing can be derived from one.';
    /* THE SAME SECOND QUESTION AS `offered`. `base` names a rung; whether this
       build can map that rung to a bar length is not settled by it being
       named. Refusing here is the honest arm: a rung whose length is unknown
       cannot be shown to be coarser than the one being typed, and `n * 60 <
       null` would coerce to `< 0` and let every width through. */
    const floor = rungSec(base);
    if (floor === null)
      return `This build cannot map ${base} to a bar length, so it cannot say whether ${n}m is finer than it.`;
    if (n * 60 < floor) return `${n}m is finer than the ${base} bars being read — it cannot be derived from them.`;
    if (timeframes.some((t) => t.seconds === n * 60)) return `${n}m is already on the ladder.`;
    return null;
  });
  const addable = $derived(addRung !== '' && rungRefusal === null);
  function doAdd() {
    if (!addable) return;
    const n = Number(addRung);
    extraMinutes = [...extraMinutes, n];
    tfPref = rungEntry(n).id;
    addRung = '';
    openTray = null;
  }

  /* ======================================================================
     THE BARS
     ====================================================================== */
  let barsLoading = $state(false);
  /** @type {string | null} */
  let barsError = $state(null);
  /** @type {string | null} */
  let barsFaults = $state(null);
  /**
   * ONE BAR, AND EVERY FIELD IS REQUIRED — WHICH IS NOT THE CHOICE MADE FOR
   * `MasterRow`, ON PURPOSE.
   *
   * A master row with no `bars` field means "the count is unknown", and the
   * page has a way to say so. A BAR with no `h` means the payload is broken,
   * and there is nothing truthful to draw from it: `?? 0` would put a zero
   * price into a high-low comparison and a zero into a volume sum, and those
   * are not absences, they are wrong measurements that look like right ones.
   * §7 is explicit that prices are integers and never invented.
   *
   * So the type is required and `readMonth` REFUSES a bar that does not meet
   * it, by name, rather than defaulting its way past one.
   *
   * @typedef {{ t: number, o: number, h: number, l: number, c: number, v: number }} Bar
   */

  /** @type {Bar[]} */
  let rawBars = $state([]);
  let barsToken = 0;
  // A RETRY HAS TO CHANGE SOMETHING. Re-assigning `range` to the value it
  // already holds is not a re-read — the effect's dependencies are unchanged
  // and nothing runs, so the button would look like a control and be furniture.
  let barsRetry = $state(0);

  /**
   * @param {import('$lib/index.svelte.js').MasterRow} row
   * @param {string} month
   * @param {string} feed
   * @param {string} rung
   * @returns {Promise<{ bars: Bar[], faults: string | null }>}
   */
  async function readMonth(row, month, feed, rung) {
    // A HALF-KNOWN PLACE IS NOT A PLACE, and this is the same refusal
    // `$lib/place.js` states in those words. The three parts below are the
    // store path — the path IS the index — and a row that cannot name one of
    // them would build `…&segment=undefined&…` and ask the bar route for it:
    // a request made of a value nobody supplied, answered with a 4xx that
    // reads as a missing file rather than as a missing field.
    const { exchange, segment, symbol } = row;
    if (typeof exchange !== 'string' || typeof segment !== 'string' || typeof symbol !== 'string')
      throw new Error(
        `${month}: this master row does not name an exchange, a segment and a symbol, so there is no file to ask for.`
      );
    // THE RUNG IS PART OF THE REQUEST. Omitting it defaults the server to
    // `1min` — which is how this page asked for `…/1min/2021-08.bin` for a
    // series filed under `1day/` and then reported "that read failed" about a
    // file that was never supposed to exist.
    const q = new URLSearchParams({
      feed,
      exchange,
      segment,
      symbol,
      timeframe: rung,
      month
    });
    const r = await ask(`/bars.json?${q}`);
    let body = null;
    try {
      body = await r.json();
    } catch {
      throw new Error(`${month}: HTTP ${r.status}, and the body was not JSON`);
    }
    // THE SERVER'S OWN SENTENCE, not a status code. It names the file, which
    // is worth ten "HTTP 400"s.
    if (body && typeof body === 'object' && !Array.isArray(body) && body.error) throw new Error(`${month}: ${body.error}`);
    if (!r.ok && r.status !== 206) throw new Error(`${month}: HTTP ${r.status}`);
    const bars = Array.isArray(body) ? body : (body?.bars ?? []);
    return { bars, faults: Array.isArray(body) ? null : (body?.faults ?? null) };
  }

  $effect(() => {
    const row = picked;
    const feed = feeds.active;
    const rung = base;
    const months = wantMonths;
    void barsRetry; // READ, NOT USED — it is what makes "Retry the read" a read
    const mine = ++barsToken;

    if (!row || !feed || !rung || months.length === 0) {
      barsLoading = false;
      barsError = null;
      barsFaults = null;
      rawBars = [];
      return;
    }
    barsLoading = true;
    barsError = null;
    Promise.all(months.map((m) => readMonth(row, m, feed, rung)))
      .then((parts) => {
        if (mine !== barsToken) return; // a newer selection won; drop this answer
        rawBars = parts.flatMap((p) => p.bars).sort((a, b) => cmp(a.t, b.t));
        barsFaults = parts.map((p) => p.faults).filter(Boolean).join('; ') || null;
        barsError = null;
        barsLoading = false;
      })
      .catch((why) => {
        if (mine !== barsToken) return;
        rawBars = [];
        barsFaults = null;
        barsError = String(why?.message ?? why);
        barsLoading = false;
      });
  });

  /* ---- aggregation -----------------------------------------------------
     Done on the paisa integers: min, max and last are exact on integers and
     lossy on floats. Buckets are aligned to IST, so an hourly candle breaks
     on the hour an Indian trader sees rather than 30 minutes off it. */
  /**
   * @param {Bar[]} raw
   * @param {number | null} seconds
   * @returns {Bar[]}
   */
  function aggregate(raw, seconds) {
    if (seconds === null) return [];
    /** @type {Bar[]} */
    const out = [];
    /** @type {Bar | null} */
    let cur = null;
    for (const b of raw) {
      const start = Math.floor((b.t + IST_OFFSET) / seconds) * seconds - IST_OFFSET;
      if (!cur || cur.t !== start) {
        cur = { t: start, o: b.o, h: b.h, l: b.l, c: b.c, v: b.v };
        out.push(cur);
      } else {
        if (b.h > cur.h) cur.h = b.h;
        if (b.l < cur.l) cur.l = b.l;
        cur.c = b.c;
        cur.v += b.v;
      }
    }
    return out;
  }

  /* ======================================================================
     THE CHART'S HALF OF THE ONE LIST.
     ----------------------------------------------------------------------
     The cascade continues downward and EVERY branch is named. There is no
     branch that returns "empty" without a reason, and no fall-through that
     draws nothing and says nothing.
     ====================================================================== */

  /**
   * THE NINE ANSWERS, AS ONE SHAPE.
   *
   * Inferred from the returns alone this was a union of nine anonymous object
   * literals, and TypeScript takes the intersection of their keys as the only
   * safe read — so `chartReason.raw` and `chartReason.chartOnly`, which four
   * branches set and five do not, were reported as properties that do not
   * exist. They DO exist; they are absent, and absent is what the readers
   * already test for. Declaring them optional says exactly that, and says it
   * once instead of at every read site.
   *
   * `kind` IS THE NINE, LISTED. The template branches on it and its own
   * comment calls those branches "a closed partition of `chartReason`" — a
   * claim nothing could check while the type was `string`. A tenth kind added
   * here without a branch there is now a compile error rather than a panel
   * that renders as nothing at all.
   *
   * `month` and `tf` ride on the `pinmiss` answer for the same reason the
   * sentence does: the panel that draws it needs the pinned pair, and reading
   * it back off `pin` made the panel depend on a mutable value agreeing with
   * the reason that was computed from it. One read, carried.
   *
   * @typedef {{
   *   kind: 'nolist'|'gone'|'idle'|'error'|'wait'|'unreadable'|'nodata'|'norung'|'pinmiss',
   *   tsub: string,
   *   why: string | null,
   *   raw?: boolean,
   *   chartOnly?: boolean,
   *   month?: string,
   *   tf?: string
   * }} ChartReason
   */

  /* THE ANNOTATION GOES ON THE FUNCTION, NOT ON THE `const`. `@returns` above
     the declaration describes `chartReason` AS a function — which it is not,
     it is the value one returned — so the callback's nine object literals went
     on being inferred as a bare union and the reads went on failing. Inside
     the call, attached to the arrow, it is the callback's return type. */
  const chartReason = $derived.by(/** @returns {ChartReason | null} */ () => {
    if (blocked) return blocked;
    // "NOT LISTED" IS A CLAIM ABOUT THE VENDOR AND IT NEEDS A LIST TO MAKE IT.
    // A row missing from an EMPTY set of rows is not evidence of anything.
    if (pickedKey && !picked && rows.length === 0)
      return {
        kind: 'nolist',
        tsub: 'nothing to look it up in',
        why:
          `No row is on hand for ${pickedKey} because there are no rows on hand at all. ` +
          (observedAt
            ? `The read of ${feedLabel(feeds.active)}'s master completed at ${fClock.format(observedAt)} IST and returned zero rows, so whether this feed lists this instrument is not known here.`
            : `No read of ${feedLabel(feeds.active)}'s master has completed, so whether this feed lists this instrument is not known here.`)
      };
    if (pickedKey && !picked)
      return {
        kind: 'gone',
        tsub: 'not listed',
        why: `${pickedKey} is not listed by ${feedLabel(feeds.active)}, which listed ${group(rows.length)} other instruments in the same answer.`
      };
    if (!picked) return { kind: 'idle', tsub: 'nothing selected', why: 'No instrument selected.' };
    if (censusState === 'error') return { kind: 'error', raw: true, tsub: 'the census failed', why: censusError };
    if (censusState !== 'ready')
      return { kind: 'wait', tsub: 'reading the census', why: `Reading the store census for ${feedLabel(feeds.active)}…` };
    // UNREADABLE IS NOT EMPTY AND MAY NOT FALL THROUGH TO IT. Reaching
    // `nodata` with an unparsed census row would print "holds no bars" — a
    // measurement — from the absence of one.
    if (heldMonths.length === 0 && badMonths.length > 0)
      return {
        kind: 'unreadable',
        tsub: 'the census cannot be read here',
        why: `${group(badMonths.length)} census row${badMonths.length === 1 ? '' : 's'} for ${picked.key} could not be parsed — ${badMonths[0].why}. How many bars are on disk is UNKNOWN, and no zero will be printed for it.`
      };
    if (heldMonths.length === 0)
      return { kind: 'nodata', tsub: 'not stored', why: `${feedLabel(feeds.active)} holds no bars for ${picked.key}.` };
    if (base === null)
      return {
        kind: 'norung',
        tsub: 'no readable rung',
        why: 'This series is filed under a rung this build cannot map to a bar length, so no base is claimed and nothing is derived from a guess.'
      };
    if (pinState && !pinState.ok && pin)
      return {
        kind: 'pinmiss',
        tsub: 'that month is at another rung',
        month: pin.month,
        tf: pin.tf,
        why: `${monthLabel(pin.month)} holds no ${base} bars.`
      };
    if (barsLoading)
      return {
        kind: 'wait',
        tsub: `reading ${group(wantMonths.length)} month file${wantMonths.length === 1 ? '' : 's'}`,
        why: `Reading ${group(wantMonths.length)} month file${wantMonths.length === 1 ? '' : 's'} at ${base}…`
      };
    if (barsError) return { kind: 'error', raw: true, tsub: 'that read failed', chartOnly: true, why: barsError };
    return null;
  });

  // THE SAME LIST, FILTERED TO WHAT THE RIGHT-HAND BAY CANNOT ANSWER. It
  // survives a failed `/bars.json` read (which says nothing about the census),
  // an empty store ("you hold nothing" is an answer this bay exists to give),
  // a bad pin and an unreadable census row — the last three are answered IN
  // the bay, in its own words, rather than replacing it with a sentence.
  const RAIL_ANSWERS = new Set(['nodata', 'pinmiss', 'unreadable']);
  const railReason = $derived(
    chartReason && !chartReason.chartOnly && !RAIL_ANSWERS.has(chartReason.kind) ? chartReason : null
  );

  // NOTHING IS "LOADED" WHILE A REASON SAYS IT IS NOT. The rail printed
  // `Sessions loaded` from a series it had just aggregated locally, so a
  // failed read still showed a confident session count.
  // WHEN THE DRAWN RUNG IS THE STORED RUNG, NOTHING IS BUCKETED. Re-bucketing
  // a bar into the window it already fills is arithmetic with one wrong
  // consequence: it moves the bar's STAMP to the start of that window, and a
  // daily bar would then be reported at 00:00 IST — a time no session has.
  const drawn = $derived(
    chartReason
      ? []
      : asStored
        ? rawBars.map((b) => ({ t: b.t, o: b.o, h: b.h, l: b.l, c: b.c, v: b.v }))
        : aggregate(rawBars, tfSeconds)
  );
  const hasVolume = $derived(drawn.some((b) => b.v > 0));
  const byTime = $derived(new Map(drawn.map((b) => [b.t, b])));

  const sessions = $derived.by(() => {
    const days = [];
    let last = null;
    for (const b of drawn) {
      const d = istDay(b.t);
      if (d !== last) {
        days.push(d);
        last = d;
      }
    }
    return days;
  });
  // READ AND SHOWN ARE TWO NUMBERS AND THE LINE PRINTS BOTH. `1D` has to read
  // a whole month file to answer for one session; saying only "21 sessions"
  // would credit the chart with days it is not drawing.
  const wantSessions = $derived(pin ? 0 : (RANGES.find((r) => r.id === range)?.sessions ?? 0));
  const shownSessions = $derived(
    wantSessions > 0 && sessions.length > wantSessions ? wantSessions : sessions.length
  );

  const lastBar = $derived(drawn.length ? drawn[drawn.length - 1] : null);
  const firstBar = $derived(drawn.length ? drawn[0] : null);

  // {time} — the bar under the crosshair, or null.
  // THE CAST IS INSIDE `$state`, not a `@type` above it. Above the
  // declaration the annotation did not reach the binding and `hover` stayed
  // `null`, so `hover?.time != null` narrowed the true branch to `never` and
  // both reads inside it failed. Annotating the initialiser types the value
  // `$state` is given, which is what the binding takes its type from.
  let hover = $state(/** @type {{ time: number } | null} */ (null));
  const shown = $derived(hover?.time != null ? (byTime.get(hover.time) ?? lastBar) : lastBar);
  const shownUp = $derived(shown ? shown.c >= shown.o : true);

  // THE REFERENCE FOR "CHANGE" IS NAMED, because there are two honest answers
  // and they are different numbers.
  const reference = $derived.by(() => {
    if (!lastBar) return null;
    const today = istDay(lastBar.t);
    for (let i = drawn.length - 1; i >= 0; i -= 1) if (istDay(drawn[i].t) !== today) return { paisa: drawn[i].c, label: 'prev close' };
    return firstBar ? { paisa: firstBar.o, label: 'vs open' } : null;
  });
  const change = $derived(lastBar && reference ? lastBar.c - reference.paisa : null);
  const changePct = $derived(change != null && reference?.paisa ? (change / reference.paisa) * 100 : null);

  /** THE TWO ANSWERS DISAGREEING IS ITSELF A FINDING. */
  const stated = $derived(picked ? barsOf(picked) : null);
  const skew = $derived(
    picked && censusState === 'ready' && stated != null && heldMonths.length > 0 && stated !== censusTotal
  );

  /* ---- the canvas ------------------------------------------------------ */
  /** @type {HTMLElement | null} */
  let host = $state(null);
  /** @type {any} */
  let chart = $state(null);
  /** @type {any} */
  let candles = $state(null);
  // PLAIN, NOT `$state`. Nothing in the markup reads it, and the draw effect
  // both tests and assigns it — as reactive state that is an effect depending
  // on its own output.
  /* TYPED `any`, TO MATCH `chart` AND `candles` DIRECTLY ABOVE.
     `lightweight-charts` does publish `IChartApi` and `ISeriesApi`, and these
     four handles could carry them — but two of the four already say `any` and
     a file that types half its handles precisely and half loosely is harder to
     read than one that is consistent. The reason the loose choice stands is
     that `null` was the only inferred type here, which made `volume.setData`
     an error on a value the draw effect assigns before it reads: an implicit
     `any` in a place where the checker could not even see the assignment.
     Saying `any` explicitly is what `noImplicitAny` is asking for. Tightening
     all four to the library's interfaces is a separate change with its own
     measurement, not a thing to do halfway inside an annotation pass. */
  /** @type {any} */
  let volume = null;
  /** @type {any} */
  let libMod = null;
  /** @type {string | null} */
  let chartError = $state(null);

  // THE CHART TAKES ITS COLOURS FROM THE THEME, not from hexes typed into it.
  // A canvas cannot inherit CSS, so the tokens are read out of the document
  // and re-applied whenever the theme changes.
  let themeTick = $state(0);
  function tokens() {
    const cs = getComputedStyle(document.documentElement);
    const v = (name) => cs.getPropertyValue(name).trim();
    return {
      up: v('--up'),
      down: v('--down'),
      grid: v('--line-soft'),
      border: v('--line'),
      text: v('--dim'),
      faint: v('--faint'),
      acc: v('--acc'),
      onAcc: v('--on-acc')
    };
  }

  $effect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const bump = () => (themeTick += 1);
    mq.addEventListener('change', bump);
    const mo = new MutationObserver(bump);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    return () => {
      mq.removeEventListener('change', bump);
      mo.disconnect();
    };
  });

  // DOES THIS CHART NEED A VOLUME PANE — asked ONLY when there is data to
  // answer it with. Between two selections `drawn` is briefly empty, and an
  // index and a stock disagree, so reading `hasVolume` directly would flip
  // false-true-false and rebuild the chart twice for one click.
  let volumeMode = $state(false);
  $effect(() => {
    if (drawn.length > 0) volumeMode = hasVolume;
  });

  // THE CHART IS REBUILT WHEN VOLUME-NESS CHANGES, and that is deliberate.
  // `removePane` left an emptied volume pane behind, so an index inherited a
  // dead band from whichever stock was looked at before it. A pane that will
  // not go away is a pane that should never have been built.
  $effect(() => {
    const el = host;
    void volumeMode; // READ, NOT USED — the dependency that forces the rebuild
    if (!el) return;
    let dead = false;
    /** @type {any} the chart handle, same looseness as `chart` above. */
    let made = null;

    (async () => {
      try {
        // TRADINGVIEW'S OWN LIBRARY, MIT, bundled — no CDN, no font, nothing
        // fetched at runtime. Imported dynamically so the shell paints first.
        const mod = await import('lightweight-charts');
        if (dead) return;
        libMod = mod;
        const c = tokens();
        made = mod.createChart(el, {
          autoSize: true,
          layout: {
            background: { color: 'transparent' },
            textColor: c.text,
            attributionLogo: false,
            panes: { separatorColor: c.border, separatorHoverColor: c.acc, enableResize: true }
          },
          grid: { vertLines: { color: c.grid }, horzLines: { color: c.grid } },
          rightPriceScale: { borderColor: c.border, scaleMargins: { top: 0.08, bottom: 0.08 } },
          timeScale: {
            borderColor: c.border,
            timeVisible: true,
            secondsVisible: false,
            rightOffset: 3,
            // MAX HAS TO MEAN MAX. The default floor on bar spacing is half a
            // pixel, so `fitContent` silently refused to fit more bars than the
            // canvas had half-pixels and drew only the last few hundred while
            // the bay beside it named a first bar that was nowhere on screen.
            minBarSpacing: 0.02,
            // 0 Year, 1 Month, 2 DayOfMonth, 3 Time, 4 TimeWithSeconds. The
            // month name comes from the literal table, never from `Intl`.
            tickMarkFormatter: (time, type) => {
              const ms = Number(time) * 1000;
              if (type <= 1) return monthLabel(isoMonth(ms));
              if (type === 2) return stampLabel(ms).slice(0, 11);
              return stampLabel(ms).slice(13);
            }
          },
          localization: { locale: LOC, timeFormatter: (time) => `${stampLabel(Number(time) * 1000)} IST` },
          crosshair: {
            mode: 1, // magnet: snaps to OHLC, which is what the price under the pointer means
            vertLine: { color: c.faint, labelBackgroundColor: c.acc },
            horzLine: { color: c.faint, labelBackgroundColor: c.acc }
          },
          handleScroll: { mouseWheel: true, pressedMouseMove: true },
          handleScale: { mouseWheel: true, pinch: true }
        });
        const cs = made.addSeries(mod.CandlestickSeries, {
          upColor: c.up,
          downColor: c.down,
          wickUpColor: c.up,
          wickDownColor: c.down,
          borderVisible: false,
          priceFormat: { type: 'price', precision: 2, minMove: 0.01 }
        });
        // ONLY THE TIME COMES BACK OFF THE CHART. The bar itself is looked up
        // in a Map of the paisa integers, so no number this page displays has
        // been through the library's floats.
        made.subscribeCrosshairMove((param) => {
          hover = param?.time != null ? { time: Number(param.time) } : null;
        });
        chart = made;
        candles = cs;
      } catch (why) {
        // A chart library that will not load is a NAMED failure, not a blank
        // rectangle. Everything else on the page still works.
        /* `catch` BINDS `unknown`, AND THE GUARD IS NOT CEREMONY. `?.message`
           on a thrown STRING is `undefined`, so the old form fell through to
           `String(why)` and worked — but on a thrown object with a `message`
           that is itself an object it printed `[object Object]` as the reason
           a chart is missing. The idiom is already used at `loadVocab`. */
        if (!dead) chartError = why instanceof Error ? why.message : String(why);
      }
    })();

    return () => {
      dead = true;
      chart = null;
      candles = null;
      volume = null;
      try {
        made?.remove();
      } catch {
        // Already disposed by a hot reload. Nothing to undo.
      }
    };
  });

  /** Epoch ms -> the `YYYY-MM` KEY form, so `monthLabel` can spell it. */
  function isoMonth(ms) {
    const d = new Date(ms + IST_OFFSET * 1000);
    return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, '0')}`;
  }

  // Re-theme in place. Cheaper than rebuilding and it keeps the operator's zoom.
  $effect(() => {
    void themeTick;
    const c = chart;
    const cs = candles;
    if (!c || !cs) return;
    const t = tokens();
    c.applyOptions({
      layout: { textColor: t.text, panes: { separatorColor: t.border, separatorHoverColor: t.acc } },
      grid: { vertLines: { color: t.grid }, horzLines: { color: t.grid } },
      rightPriceScale: { borderColor: t.border },
      timeScale: { borderColor: t.border },
      crosshair: { vertLine: { color: t.faint, labelBackgroundColor: t.acc }, horzLine: { color: t.faint, labelBackgroundColor: t.acc } }
    });
    cs.applyOptions({ upColor: t.up, downColor: t.down, wickUpColor: t.up, wickDownColor: t.down });
  });

  // DRAW. Separate from both the mount and the fetch, so an async library load
  // racing an already-finished fetch cannot leave the canvas empty.
  $effect(() => {
    const cs = candles;
    const c = chart;
    const data = drawn;
    if (!cs || !c) return;

    const t = tokens();
    cs.setData(data.map((b) => ({ time: b.t, open: b.o / 100, high: b.h / 100, low: b.l / 100, close: b.c / 100 })));

    // VOLUME: THE PANE IS BORN WITH ITS DATA, in this same tick. Created empty
    // in the mount effect and fed a frame later, it laid out at zero height.
    if (volumeMode && !volume && libMod) {
      const pane = c.panes().length > 1 ? c.panes()[1] : c.addPane();
      const at = pane.paneIndex();
      // An overlay under the candles would share the price axis, and 4,00,000
      // shares and ₹1,325 do not belong on one scale.
      volume = c.addSeries(libMod.HistogramSeries, { priceFormat: { type: 'volume' } }, at);
      c.priceScale('right', at).applyOptions({ borderColor: t.border, scaleMargins: { top: 0.12, bottom: 0 } });
    }
    if (volume) volume.setData(data.map((b) => ({ time: b.t, value: b.v, color: b.c >= b.o ? t.up : t.down })));
    if (data.length === 0) return;

    // THE RANGE SETS THE VIEW, the fetch sets what exists. `1D` and `5D` are
    // SESSIONS, counted from the data, not calendar days — a Sunday is not a
    // day of trading and a range that counted it would show four and call it
    // five. The reading line states both numbers, so a range that reads more
    // than it shows is not quietly claiming the larger one.
    if (wantSessions > 0 && sessions.length > wantSessions) {
      const cutoff = sessions[sessions.length - wantSessions];
      const from = data.find((b) => istDay(b.t) >= cutoff)?.t;
      if (from != null) {
        c.timeScale().setVisibleRange({ from, to: data[data.length - 1].t });
        return;
      }
    }
    c.timeScale().fitContent();
  });

  /* ======================================================================
     RE-READS. Both are GETs this page already makes; neither is a pull.
     ====================================================================== */
  const rereadMaster = () => {
    if (feeds.active) loadCatalogue(feeds.active);
  };
  // A GENERATION BUMP, NOT A PRIVATE FETCH. Every page subscribed to the shared
  // census re-reads off the same answer, so this press cannot leave one surface
  // holding a newer total than another.
  const rereadCensus = () => refreshStore();
</script>

<svelte:window onkeydown={onWindowKey} onclick={() => (openTray = null)} />

<!-- ======================================================================
     SNIPPETS — the shapes that appear in more than one bay.
     ====================================================================== -->

<!-- THE FOUR NUMBER STATES, AND THEY NEVER LOOK ALIKE. -->
{#snippet barCell(st)}
  {#if st.kind === 'count'}
    <span class="n">{group(st.n)}</span>
  {:else if st.kind === 'zero'}
    <span
      class="n0"
      title="Measured. The master answered for this series and reports zero bars on disk — it has never been pulled."
      >0</span
    >
  {:else if st.kind === 'unread'}
    <span
      class="nq"
      title="NOT a zero and not a measurement: {st.why}. The count is unknown, and nothing on this page will print a number for it."
      >unread</span
    >
  {:else if st.wait}
    <span class="nw" title="No count has been measured for this series yet — {st.why}.">…</span>
  {:else}
    <span class="nq" title="Nothing has been measured for this series: {st.why}.">no master</span>
  {/if}
{/snippet}

<!-- THE EVIDENCE LIST. What is known, quoted, beside what is not. -->
{#snippet evid(items)}
  <dl class="evid">
    {#each items as [k, v, quiet] (k)}
      <div><dt>{k}</dt><dd class={quiet ? 'q' : ''}>{v}</dd></div>
    {/each}
  </dl>
{/snippet}

{#snippet skels(n, seed)}
  <div class="skels" aria-hidden="true">
    {#each Array(n) as _, i (i)}
      <span class="skel" style="width:{38 + ((i * seed) % 46)}%"></span>
    {/each}
  </div>
{/snippet}

<div class="mkt">
  <!-- ==================================================================
       THE HELM — the store address, as a control.
       ================================================================== -->
  <!-- `rise` HERE AND ON THE THREE BAYS, AND NOWHERE ELSE ON THIS PAGE.
       `theme.css` already owns the entrance: the class carries its own
       `:nth-child` stagger, so the panels arrive in reading order for the cost
       of one word each and no new keyframe. The helm is child 1 of `.mkt`; the
       bays are children 1..3 of `.deck`. The helm's own KNOBS are pointedly
       not given it -- a `<span class="sep">/</span>` sits between every pair,
       so the count would step 0/120/240 and then fall off the six-child cap,
       which is a stagger that looks like a stutter.

       THE VIRTUAL LIST IS DELIBERATELY LEFT STILL. `.rrow` is recycled out of
       `hits.slice(firstRow, ...)`, so `row-in` there would re-fire on every
       scroll frame -- motion on a row that never arrived and only scrolled
       into being. That is decoration, and the rule on this console is that
       motion marks a fact that MOVED. -->
  <div class="helm rise">
    <!-- THE FEED IS NOT CHOSEN HERE. It is chosen once, in the top bar, and
         this page writes it zero times. The knob carries the CURRENT VALUE so
         the address is legible, and says on its own face where the choice is
         made — a second picker would be a second place to disagree. -->
    <div class="knob scope fixed">
      <b>Broker feed<em>chosen in the top bar</em></b>
      <span
        class="face"
        title="Every number on this page is requested as feed={feeds.active ?? '—'}. One feed at a time, never two — and the choice is made once, in the top bar, so no two surfaces can hold different answers."
        >{feedLabel(feeds.active)}</span
      >
    </div>
    <span class="sep">/</span>

    <div class="knob fixed">
      <b>Exchange</b>
      <span
        class="face"
        title="The engine surface is exactly two instruments on NSE — NSE-NIFTY and NSE-BANKNIFTY. BSE and MCX are stored where they already exist and are never swept."
        >{picked ? picked.exchange : '—'}</span
      >
    </div>
    <span class="sep">/</span>

    <div class="knob fixed">
      <b>Segment</b>
      <span class="face" title="The store dimension this series is filed under. The path is the index.">{picked ? picked.segment : '—'}</span>
    </div>
    <span class="sep">/</span>

    <!-- SYMBOL — a tray with the same search and the same rows as the bay on
         the left, because two lists that can disagree eventually do. -->
    <div class="knob">
      <b>Symbol</b>
      <button
        class="face"
        type="button"
        aria-haspopup="true"
        aria-expanded={openTray === 'sym'}
        title={picked ? picked.key : pickedKey ? 'This key is not in the master on hand.' : 'Nothing selected.'}
        onclick={(e) => {
          e.stopPropagation();
          openTray = openTray === 'sym' ? null : 'sym';
        }}
      >
        <span>{picked ? picked.symbol : pickedKey ? pickedKey.split('-').slice(1).join('-') : 'none'}</span>
        {#if pickedKey && !picked}<span class="sub">not listed here</span>{/if}
        <i>▼</i>
      </button>
      {#if openTray === 'sym'}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="tray open wide" onclick={(e) => e.stopPropagation()}>
          <div class="sift">
            <!-- svelte-ignore a11y_autofocus -->
            <input
              type="text"
              bind:value={typed}
              autocomplete="off"
              spellcheck="false"
              autofocus
              placeholder="Type a symbol — NIFTY, RELIA…"
              aria-label="Search symbols"
            />
            <p class="hint">The same search as the bay on the left, and the same rows. This tray shows the first twelve.</p>
          </div>
          {#each hits.slice(0, 12) as r (r.key)}
            <button class="trow" type="button" aria-pressed={pickedKey === r.key} onclick={() => pick(r)}>
              <span class="nm">{r.symbol}</span>
              <span class="de">{@render barCell(barState(r))} bars</span>
            </button>
          {/each}
          {#if hits.length === 0}
            <p class="traynote">Nothing matches. The bay on the left states which narrowing emptied it.</p>
          {/if}
        </div>
      {/if}
    </div>
    <span class="sep">/</span>

    <!-- RUNG ON DISK — the file this chart is READ from. Read off the census,
         never assumed: a rung on the address bar that no file answers to is
         how a chart ends up asking for `1min/2021-08.bin` on a `1day` series. -->
    <div class="knob">
      <b>Rung on disk{#if storedRungs.length > 1}<em>{storedRungs.length} held</em>{/if}</b>
      {#if storedRungs.length}
        <button
          class="face"
          type="button"
          aria-haspopup="true"
          aria-expanded={openTray === 'base'}
          title="The file this chart is READ from. Everything coarser is arithmetic; nothing finer can be derived from it and none is offered."
          onclick={(e) => {
            e.stopPropagation();
            openTray = openTray === 'base' ? null : 'base';
          }}
        >
          <span>{base}</span><i>▼</i>
        </button>
      {:else}
        <span
          class="face"
          title={unmappedRungs.length
            ? `This series is filed under ${unmappedRungs.join(' and ')}, which this build cannot map to a bar length, so no base is claimed.`
            : 'The census lists no month file for this series, so there is no rung to read.'}
          >{unmappedRungs.length ? 'nothing this build maps' : 'nothing stored'}</span
        >
      {/if}
      {#if openTray === 'base'}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="tray open" onclick={(e) => e.stopPropagation()}>
          {#each storedRungs as r (r)}
            {@const ms = heldMonths.filter((m) => m.timeframe === r)}
            <button
              class="trow"
              type="button"
              aria-pressed={base === r}
              onclick={() => {
                basePref = r;
                openTray = null;
                /* ONE READ, THEN THE TEST. The guard's narrowing does not
                   reach inside `some`, and the reason it does not is real:
                   `pin` is mutable, and a pin observed by the guard need not
                   be the pin observed by the predicate. Deciding whether to
                   drop THIS pin has to be answered about one pin. */
                const at = pin;
                if (at && !heldMonths.some((m) => m.month === at.month && m.timeframe === r))
                  pin = null;
              }}
            >
              <span class="nm">{r}</span>
              <span class="de">{group(ms.length)} months · {group(ms.reduce((a, x) => a + x.rows, 0))} bars</span>
            </button>
          {/each}
          <p class="traynote">
            A series can genuinely hold two rungs — separate backfills, same instrument. Only one is read; reading both would put two
            bars on one timestamp.
          </p>
        </div>
      {/if}
    </div>
    <span class="sep">→</span>

    <!-- EVERYTHING AFTER THE ARROW IS ARITHMETIC DONE IN THIS BROWSER, and it
         is violet for that reason and no other. -->
    <div class="knob derived">
      <b>Timeframe<em>{offered.length ? (asStored ? 'as stored' : 'derived here') : ''}</em></b>
      {#if offered.length}
        <button
          class="face"
          type="button"
          aria-haspopup="true"
          aria-expanded={openTray === 'tf'}
          title="Aggregated in this browser from the rung on disk, on the paisa integers, bucketed to IST midnight."
          onclick={(e) => {
            e.stopPropagation();
            openTray = openTray === 'tf' ? null : 'tf';
          }}
        >
          <span>{tf}</span><i>▼</i>
        </button>
      {:else}
        <span class="face" title="With no readable rung on disk there is nothing to derive, and naming a timeframe here would name one nothing can be drawn at."
          >nothing derivable</span
        >
      {/if}
      {#if openTray === 'tf'}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <div class="tray open" onclick={(e) => e.stopPropagation()}>
          {#each offered as t (t.id)}
            <button
              class="trow"
              type="button"
              aria-pressed={tf === t.id}
              onclick={() => {
                tfPref = t.id;
                openTray = null;
                hover = null;
              }}
            >
              <span class="nm">{t.id}</span>
              <span class="de"
                >{base !== null && t.seconds === rungSec(base)
                  ? 'as stored'
                  : `${Math.ceil(22500 / t.seconds)} candles per session, derived from ${base}`}</span
              >
            </button>
          {/each}
          <hr />
          <!-- N RUNGS AT RUNTIME. Anything finer than the bars being read is
               refused WHERE THE TYPING HAPPENS, not only in a tooltip. -->
          <div class="addr">
            <input
              type="number"
              min="1"
              max="1440"
              placeholder="min"
              bind:value={addRung}
              aria-invalid={rungRefusal !== null}
              aria-label="Add a timeframe, in minutes"
              onkeydown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  doAdd();
                }
              }}
            />
            <button type="button" disabled={!addable} title={rungRefusal ?? 'Add this rung to the ladder'} onclick={doAdd}>Add rung</button>
          </div>
          {#if rungRefusal}
            <span class="refusal">{rungRefusal}</span>
          {:else}
            <p class="traynote">A rung finer than the bars being read cannot be derived from them and is refused here, in the box, rather than silently ignored.</p>
          {/if}
        </div>
      {/if}
    </div>

    <span class="helmgap"></span>

    <div
      class="stamp"
      title={censusState === 'ready'
        ? `GET /store.json?feed=${feeds.active} answered at ${censusStamp ? fClock.format(censusStamp) : '—'} IST, as observed by this browser. Every month and every count in the right-hand bay is from that one read.`
        : censusState === 'none'
          ? 'The census is asked per feed. No feed is chosen, so no request has been made — this is not a census that failed and not one in flight.'
          : (censusError ?? 'The census has not answered yet.')}
    >
      <b>census</b>
      <span class:bad={censusState === 'error'} class:off={censusState === 'none'}>
        {censusState === 'ready'
          ? `read ${censusStamp ? fClock.format(censusStamp) : '—'} IST`
          : censusState === 'wait'
            ? 'in flight…'
            : censusState === 'none'
              ? 'not requested'
              : 'failed'}
      </span>
    </div>
    {#if blocked && blocked.kind === 'error'}
      <div class="stamp" title={blocked.why}><b>master</b><span class="bad">{blocked.tsub}</span></div>
    {/if}
  </div>

  <div class="deck">
    <!-- ================================================================
         LEFT BAY — WHICH SERIES
         ================================================================ -->
    <section class="bay bay-list rise" aria-label="Which instrument">
      <div class="bayhead">
        <h2>Instruments</h2>
        <span class="rt">{feeds.active ? feedLabel(feeds.active) : ''}</span>
      </div>

      <div class="sbox">
        <input
          bind:this={searchEl}
          bind:value={typed}
          onkeydown={onSearchKey}
          oninput={() => (cursor = -1)}
          disabled={!feeds.active}
          placeholder={!feeds.active
            ? 'no feed chosen'
            : masterIsThisFeed
              ? `Search ${group(rows.length)} instruments`
              : 'Search instruments'}
          aria-label="Search instruments by symbol"
          role="combobox"
          aria-expanded="true"
          aria-controls="roster"
          aria-autocomplete="list"
          aria-activedescendant={cursor >= 0 ? `inst-${cursor}` : undefined}
          autocomplete="off"
          spellcheck="false"
        />
        <kbd class="kbd">/</kbd>
      </div>

      <!-- THE ROOT IS UNANSWERED -> THE RUNGS BELOW IT ARE GONE, NOT GREYED.
           A disabled universe strip asserts that a choice exists to be made
           here, and none does: the choice is the feed, and it is upstairs. -->
      {#if feeds.active}
        <div class="facet">
          <b>Universe</b>
          <div class="fset">
            <button
              class="pip wide"
              type="button"
              aria-haspopup="true"
              aria-expanded={openTray === 'uni'}
              onclick={(e) => {
                e.stopPropagation();
                openTray = openTray === 'uni' ? null : 'uni';
              }}
            >
              <span class="nm">{UNIVERSES.find((u) => u.id === universe)?.label ?? universe}</span>
              <span class="de">{masterIsThisFeed ? `${group(counts.uni[universe] ?? 0)} listed` : 'not counted'}</span>
            </button>
            <!-- A REFUSAL INSIDE A CLOSED DROPDOWN IS A REFUSAL NOBODY IS TOLD
                 ABOUT. Four of the eight cannot be answered by these rows, and
                 that is a fact about the whole control. -->
            {#if !tiersAnswerable}
              <span
                class="flag warn"
                title="NIFTY 50 / 100 / 200 / 500 cannot be answered: the instrument rows on hand carry no `universes` array. The frozen `universe` string has never carried a tier. They are refused in the list, never synthesised from an alphabetical order."
                >4 tiers unanswerable</span
              >
            {/if}
            {#if openTray === 'uni'}
              <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
              <div class="tray open wide" onclick={(e) => e.stopPropagation()}>
                {#each UNIVERSES as u (u.id)}
                  {@const refused = TIERSET.has(u.token) && !tiersAnswerable}
                  <button
                    class="trow"
                    class:off={refused}
                    type="button"
                    disabled={refused}
                    aria-pressed={universe === u.id}
                    title={refused
                      ? 'Not counted here: the instrument rows on hand carry no `universes` array, so this membership cannot be read off them. Shown in place and refused, never synthesised.'
                      : u.label}
                    onclick={() => {
                      universe = u.id;
                      openTray = null;
                    }}
                  >
                    <span class="nm">{u.label}</span>
                    <span class="de"
                      >{refused
                        ? 'cannot be answered'
                        : masterIsThisFeed
                          ? `${group(counts.uni[u.id] ?? 0)} listed by ${feedLabel(feeds.active)}`
                          : `not counted: ${countRefusal}`}</span
                    >
                  </button>
                {/each}
                {#if !tiersAnswerable}
                  <p class="traynote">
                    <b>The four NIFTY tiers are refused, not empty. </b>These rows carry no `universes` array. The API emits one (D-0089)
                    beside the frozen three-bit `universe` string (D-0090); the binary answering this console predates it. Rebuild and
                    restart the API and these four answer on their own.
                  </p>
                {/if}
              </div>
            {/if}
          </div>
        </div>

        <div class="facet">
          <b>Segment</b>
          <div class="fset">
            <button
              class="pip"
              type="button"
              aria-pressed={segment === ''}
              title="Every segment this feed lists — {masterIsThisFeed
                ? `${group(rows.length)} listed by ${feedLabel(feeds.active)}`
                : `not counted: ${countRefusal}`}"
              onclick={() => (segment = '')}>Any</button
            >
            {#each segments as sg (sg)}
              <button
                class="pip"
                type="button"
                aria-pressed={segment === sg}
                title="Store segment {sg} — {masterIsThisFeed
                  ? `${group(counts.seg.get(sg) ?? 0)} listed by ${feedLabel(feeds.active)}`
                  : `not counted: ${countRefusal}`}"
                onclick={() => (segment = sg)}>{sg}</button
              >
            {/each}
            <button
              class="pip"
              type="button"
              aria-pressed={heldOnly}
              disabled={!masterIsThisFeed}
              title={masterIsThisFeed
                ? `At least one bar on disk — ${group(counts.held)} of ${group(rows.length)} held under ${feedLabel(feeds.active)}; ${group(counts.zero)} listed and not stored${counts.unread ? `; ${group(counts.unread)} whose count could not be read` : ''}.`
                : `Cannot be answered: which series hold bars comes from the master, and ${countRefusal}.`}
              onclick={() => (heldOnly = !heldOnly)}>Held</button
            >
            <!-- THE THIRD BUCKET GETS A CONTROL, because a fact with no control
                 is a fact nobody finds. It appears only where the count
                 actually failed to read — a permanent `Unread 0` is furniture. -->
            {#if counts.unread > 0}
              <button
                class="pip amber"
                type="button"
                aria-pressed={unreadOnly}
                title="Series whose `bars` field could not be read. Their count is UNKNOWN — not zero — and the list prints `unread` for them rather than a number they do not have."
                onclick={() => (unreadOnly = !unreadOnly)}>Unread {group(counts.unread)}</button
              >
            {/if}
          </div>
        </div>

        <div class="rhead">
          <button class="hcell" class:on={sortKey === 'symbol'} type="button" aria-pressed={sortKey === 'symbol'} onclick={() => sortBy('symbol')}>
            Symbol<span aria-hidden="true">{arrow('symbol')}</span>
          </button>
          <button class="hcell" class:on={sortKey === 'segment'} type="button" aria-pressed={sortKey === 'segment'} onclick={() => sortBy('segment')}>
            Seg<span aria-hidden="true">{arrow('segment')}</span>
          </button>
          <!-- ORDERING A COLUMN OF UNKNOWNS IS NOT AN ORDERING. -->
          <button
            class="hcell r"
            class:on={sortKey === 'bars'}
            type="button"
            disabled={!masterIsThisFeed}
            aria-pressed={sortKey === 'bars'}
            title={masterIsThisFeed
              ? 'Bars on disk. Four answers, and they never look alike: a count, a ghost 0 for a measured zero, an amber `unread` when the field could not be read, and a non-answer when no master for this feed is on hand.'
              : `Cannot sort: every cell in this column is unmeasured — ${countRefusal}. The list is in symbol order.`}
            onclick={() => sortBy('bars')}
          >
            Bars<span aria-hidden="true">{arrow('bars')}</span>
          </button>
        </div>
      {/if}

      <div
        class="roster"
        id="roster"
        role="listbox"
        aria-label="Instruments"
        tabindex="-1"
        bind:this={scroller}
        bind:clientHeight={viewportH}
        onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
      >
        {#if blocked && blocked.kind === 'wait' && !feeds.active}
          <div class="empty">
            <h3>{cap(blocked.tsub)}</h3>
            <p>{blocked.why}</p>
            {#if feeds.all.length}
              {@render evid([
                ['cascade', 'feed → universe → instrument'],
                ['effect', 'the two rungs below the feed are not rendered at all'],
                ['chosen in', 'the top bar of every page, once, for all of them'],
                ['offered', `${group(feeds.all.length)} feeds`]
              ])}
            {:else}
              <div class="waiting"><span class="dot acc live"></span><span>GET /feeds.json</span></div>
              {@render skels(6, 17)}
            {/if}
          </div>
        {:else if blocked && blocked.kind === 'error'}
          <div class="empty">
            <h3>{cap(blocked.tsub)}</h3>
            <p>{blocked.why}</p>
            {@render evid([
              ['on hand', `${group(rows.length)} rows, from ${feedLabel(catalogue.feed)}`],
              ['printed', `nothing is counted under ${feedLabel(feeds.active)}'s name`]
            ])}
            <p><button class="lnk" type="button" onclick={rereadMaster}>Re-read the master</button></p>
          </div>
        {:else if blocked}
          <div class="empty">
            <div class="waiting"><span class="dot acc live"></span><span>{blocked.why}</span></div>
            {@render skels(14, 17)}
          </div>
        {:else if hits.length === 0}
          <!-- TWO DIFFERENT NOTHINGS, AND THEY ARE TOLD APART BY WHETHER A READ
               OF THIS FEED'S MASTER HAS COMPLETED. The old page asserted "its
               master was read and holds nothing" from a variable that only
               knows about this browser. -->
          {#if narrowing.length}
            <div class="empty">
              <h3>No row matches this selection</h3>
              <p>
                {feedLabel(feeds.active)} lists {group(rows.length)} instruments; none of them match
                {#each narrowing as t, i (t)}{i ? ', ' : ''}<b>{t}</b>{/each}. The master is not empty — this selection is.
                <button class="lnk" type="button" onclick={clearNarrowing}>Clear every filter below the feed</button>
              </p>
            </div>
          {:else if observedAt}
            <div class="empty">
              <h3>The master answered with no rows</h3>
              <p>
                What is known is the read, and it is quoted: nothing here is a statement about what {feedLabel(feeds.active)} lists in
                general, only about what this request returned.
              </p>
              {@render evid([
                ['request', `GET /instruments.json?feed=${feeds.active}`],
                ['answered', `${fClock.format(observedAt)} IST, as observed by this browser`],
                ['rows', '0'],
                ['not known', 'whether the vendor lists nothing, or lists nothing this build can parse. This page has not measured that and does not say.', true]
              ])}
              <p><button class="lnk" type="button" onclick={rereadMaster}>Re-read the master</button></p>
            </div>
          {:else}
            <div class="empty">
              <h3>No completed read of this master</h3>
              <p>
                There are no rows on hand and no read has finished, so this bay makes no claim about what {feedLabel(feeds.active)} holds.
                An empty list and an unanswered request look identical from here, and naming one of them would be inventing a state this
                page cannot observe.
              </p>
              {@render evid([
                ['request', `GET /instruments.json?feed=${feeds.active}`],
                ['answered', 'no answer recorded'],
                ['rows on hand', '0'],
                ['claim made', 'none', true]
              ])}
              <p><button class="lnk" type="button" onclick={rereadMaster}>Read the master now</button></p>
            </div>
          {/if}
        {:else}
          <div class="rspace" style="height:{hits.length * ROW}px" role="presentation">
            <div class="win" style="transform:translateY({firstRow * ROW}px)" role="presentation">
              {#each visible as row, i (row.key)}
                <div
                  class="rrow"
                  id="inst-{firstRow + i}"
                  role="option"
                  tabindex="-1"
                  aria-selected={pickedKey === row.key}
                  aria-posinset={firstRow + i + 1}
                  aria-setsize={hits.length}
                  data-cursor={cursor === firstRow + i}
                  title={row.key}
                  onclick={() => pick(row)}
                  onkeydown={(e) => e.key === 'Enter' && pick(row)}
                >
                  <span class="sy">{row.symbol}</span>
                  <span class="sg">{row.segment === 'INDEX' ? 'IDX' : row.segment}</span>
                  <span class="bl">{@render barCell(barState(row))}</span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>

      <!-- THE FOOTER PARTITION. Three buckets, and they add to the number
           beside them. `unread` is a bucket and not a rounding of zero. -->
      <div class="rfoot">
        {#if blocked}
          <span class="sum" title={blocked.why}>{blocked.tsub}</span>
        {:else}
          <span class="sum" title="Rows on screen, out of what {feedLabel(feeds.active)}'s master lists."
            >{group(hits.length)} of {group(rows.length)} shown</span
          >
          {#if rows.length === 0}
            <span class="sum off" title="There are no rows on hand, so there is nothing to sort into held / not stored. This is not a partition of zero."
              >· no rows to partition</span
            >
          {:else if !masterIsThisFeed}
            <span class="sum amber" title="Which rows hold bars is a fact about the master, and {countRefusal}. No row is counted into a bucket from a read that did not answer."
              >· held / not stored: not counted</span
            >
          {:else}
            <span class="sum" title="A partition, and it sums: every listed row is in exactly one of these buckets."
              >· {group(counts.held)} held + {group(counts.zero)} not stored{counts.unread ? ` + ${group(counts.unread)} unread` : ''} =
              {group(counts.held + counts.zero + counts.unread)}</span
            >
          {/if}
          {#if storedNotListed.length}
            <span
              class="sum amber"
              title="Held under {feedLabel(feeds.active)} and absent from its master: {storedNotListed
                .slice(0, 6)
                .join(', ')}{storedNotListed.length > 6 ? `, and ${group(storedNotListed.length - 6)} more` : ''}. The store does not forget what a vendor has dropped, and this list is the vendor's — it cannot show them."
              >· {group(storedNotListed.length)} stored, not in this master</span
            >
          {/if}
          {#if masterIsThisFeed}
            <span class="lg">0 = measured zero · unread = the count could not be read</span>
          {/if}
        {/if}
      </div>
    </section>

    <!-- ================================================================
         CENTRE BAY — WHAT IT LOOKS LIKE
         ================================================================ -->
    <section class="bay bay-plot rise" aria-label="The series">
      <div class="bayhead">
        <h2 class="ident">{picked ? picked.symbol : 'No instrument'}</h2>
        {#if picked}
          <span class="rt">{picked.key}</span>
          <span class="flag" class:info={picked.segment === 'INDEX'}>{picked.segment}</span>
        {/if}
        {#if shown}
          <span class="quote">
            <b class="px" class:up={shownUp} class:dn={!shownUp}>{rupee(shown.c)}</b>
            {#if change != null}
              <span class="ch" class:up={change >= 0} class:dn={change < 0}>
                {change >= 0 ? '+' : '−'}{rupee(Math.abs(change))} ({change >= 0 ? '+' : '−'}{Math.abs(changePct ?? 0).toFixed(2)}%)
              </span>
              <span class="rf">{reference?.label}</span>
            {/if}
          </span>
        {:else}
          <span class="quote"></span>
        {/if}
        <span
          class="feedmark"
          title="Every number in this bay is {feedLabel(feeds.active)}'s, requested as feed={feeds.active ?? '—'}."
        >
          <span class="dot acc"></span>{feedLabel(feeds.active)}
        </span>
      </div>

      <!-- THE READING LINE. The one line that answers "why does the canvas
           look like this" — read from where, how many bars, aggregated to
           what, drawn how. IT IS NEVER BLANK: when nothing is drawn it carries
           the reason in its short form and the glass carries it in full. -->
      <div class="reading">
        {#if chartReason}
          <span class="seg" class:bad={chartReason.kind === 'error'} class:warn={chartReason.kind !== 'error' && chartReason.kind !== 'wait'}>
            <u>not drawn</u><b>{chartReason.tsub}</b>
          </span>
          <span class="arrow">—</span>
          <span class="whyshort">{chartReason.why}</span>
        {:else}
          <span class="seg"><u>read</u><b>{group(wantRecords)} × {base} bars</b></span>
          <span class="arrow">from</span>
          <span class="seg">
            <u>files</u>
            <b
              >{group(wantMonths.length)} month file{wantMonths.length === 1 ? '' : 's'}
              {#if wantMonths.length}({monthLabel(wantMonths[0])}{wantMonths.length > 1
                  ? ` … ${monthLabel(wantMonths[wantMonths.length - 1])}`
                  : ''}){/if}</b
            >
          </span>
          <span class="arrow">→</span>
          <span class="seg vic"><u>drawn</u><b>{group(drawn.length)} × {tf} {asStored ? 'as stored' : 'derived'}</b></span>
          <span class="arrow">·</span>
          <span
            class="seg"
            title={shownSessions < sessions.length
              ? `The store files one month at a time — the path is the index — so ${range} reads the month and shows the last ${group(shownSessions)} session${shownSessions === 1 ? '' : 's'} of it.`
              : 'Distinct IST trading sessions in what was read.'}
          >
            <u>sessions</u>
            <b>{shownSessions < sessions.length ? `${group(shownSessions)} shown of ${group(sessions.length)} read` : group(sessions.length)}</b>
          </span>
        {/if}
      </div>

      <!-- THE OHLC READOUT. Follows the crosshair; falls back to the last bar
           so the strip is never blank while the pointer is off the canvas. -->
      <div class="readout" aria-live="off">
        {#if shown}
          <span class="pair"><i>O</i><b class:up={shownUp} class:dn={!shownUp}>{rupee(shown.o)}</b></span>
          <span class="pair"><i>H</i><b class:up={shownUp} class:dn={!shownUp}>{rupee(shown.h)}</b></span>
          <span class="pair"><i>L</i><b class:up={shownUp} class:dn={!shownUp}>{rupee(shown.l)}</b></span>
          <span class="pair"><i>C</i><b class:up={shownUp} class:dn={!shownUp}>{rupee(shown.c)}</b></span>
          <span class="pair"><i>{hover ? 'AT' : 'LAST'}</i><b>{stampLabel(shown.t * 1000)} IST</b></span>
          {#if !hasVolume}
            <span class="flag warn" title="The store records zero traded volume for this series. An index has none — this is a measured zero, not a missing field."
              >no volume</span
            >
          {/if}
          <span class="readgap"></span>
          <span class="pair"><i>BARS</i><b>{group(drawn.length)}</b></span>
        {:else if chartReason && chartReason.kind === 'wait'}
          <span class="waiting"><span class="dot acc live"></span><span>{chartReason.why}</span></span>
        {:else}
          <span class="pair"><i>no bar is loaded</i></span>
        {/if}
      </div>

      <!-- THE RANGE AND THE PIN. Same rule the facets follow: when the feed is
           unanswered every rung below it is NOT RENDERED rather than rendered
           disabled — a greyed strip asserts a choice exists to be made here. -->
      <div class="span">
        {#if blocked}
          <span class="gone" title={blocked.why}>range — not offered while {blocked.tsub}</span>
        {:else}
          <b>Range</b>
          {#each RANGES as r (r.id)}
            <button
              class="pip"
              type="button"
              aria-pressed={!pin && range === r.id}
              title={r.months === Infinity
                ? `Every month held at ${base ?? 'the read rung'}`
                : `Read the last ${r.months} month file${r.months === 1 ? '' : 's'}${r.sessions ? `, show the last ${r.sessions} session${r.sessions === 1 ? '' : 's'}` : ''}`}
              onclick={() => {
                range = r.id;
                pin = null;
                hover = null;
              }}>{r.id}</button
            >
          {/each}
        {/if}
        {#if pin}
          <button
            class="pin"
            type="button"
            title="Pinned: {pin.month} at {pin.tf}. A pin is a (month, rung) PAIR — it can never name a month at a rung the store does not hold."
            onclick={() => {
              pin = null;
              hover = null;
            }}
          >
            {monthLabel(pin.month)} · {pin.tf}<span>✕</span>
          </button>
        {/if}
        <span class="spangap"></span>
        {#if base}
          <span
            class="flag"
            class:stored={asStored}
            class:info={!asStored}
            title="Read {base} from disk. Coarser rungs are aggregated in this browser on the paisa integers; finer rungs cannot be derived and are not offered."
            >{asStored ? 'as stored' : `derived from ${base}`}</span
          >
        {/if}
      </div>

      <div class="plot">
        <div class="host" bind:this={host}></div>

        <!-- EVERY EMPTY CANVAS HAS AN OVERLAY AND EVERY OVERLAY NAMES ITS
             CAUSE. The branches below are a closed partition of `chartReason`;
             there is no fall-through that draws nothing and says nothing. -->
        {#if chartError}
          <div class="glass">
            <div class="why">
              <h3>The chart did not start</h3>
              <p class="err">{chartError}</p>
              {@render evid([['still working', 'the roster, the search, the census and the ladder on the right', true]])}
            </div>
          </div>
        {:else if chartReason}
          <div class="glass">
            {#if chartReason.kind === 'wait'}
              <div class="why">
                <p class="waiting"><span class="dot acc live"></span><span>{chartReason.why}</span></p>
                {@render skels(3, 23)}
              </div>
            {:else if chartReason.kind === 'nodata'}
              <div class="why">
                <h3>Nothing stored for {picked?.symbol}</h3>
                <p>{feedLabel(feeds.active)} holds no bars for this instrument. The feed LISTS it, so it can be pulled — it just has not been.</p>
                {@render evid([
                  ['feed', feedLabel(feeds.active)],
                  ['instrument', picked?.key ?? pickedKey],
                  ['month files', '0 — the census answered and listed none'],
                  ['bars on disk', '0 — measured, not unknown']
                ])}
                <div class="acts"><a class="cta" href="/ingest">Pull it from Ingest →</a></div>
              </div>
            {:else if chartReason.kind === 'unreadable'}
              <!-- NOT A ZERO. The count is unknown, and the overlay says which
                   of the two it is — the whole point of the third number state. -->
              <div class="why">
                <h3>The census cannot be read for {picked?.symbol}</h3>
                <p class="warn">{badMonths[0]?.why}</p>
                {@render evid([
                  ['instrument', picked?.key ?? pickedKey],
                  ['rows that would not parse', group(badMonths.length)],
                  ['bars on disk', 'unknown — not zero, and not shown as one'],
                  ['month files', 'unknown'],
                  ['what this is NOT', 'a series with nothing stored. That state is drawn differently, on purpose.', true]
                ])}
                <div class="acts"><button class="cta" type="button" onclick={rereadCensus}>Retry the census</button></div>
              </div>
            {:else if chartReason.kind === 'pinmiss'}
              <!-- THE STATE THE OLD PAGE DREW AS A BLANK RECTANGLE.
                   THE PAIR COMES OFF THE REASON, NOT OFF `pin`. The reason was
                   computed from one read of the pin; re-reading the mutable
                   value here to render the sentence that reason produced is
                   two reads of a changing thing describing one event. -->
              {@const other = pinState?.elsewhere ?? []}
              {@const pinnedMonth = chartReason.month ?? ''}
              <div class="why">
                <h3>{monthLabel(pinnedMonth)} is held — at another rung</h3>
                <p>
                  The pin names {monthLabel(pinnedMonth)} and the rung being read is {base}. The store has no {base} file for that month, so
                  there is nothing to draw. This is not an error and not an empty store: the month exists at
                  {other.map((o) => o.timeframe).join(' and ') || 'no rung at all'}.
                </p>
                {@render evid([
                  ['pinned', `${pinnedMonth} at ${chartReason.tf ?? ''}`],
                  ['rung being read', base],
                  [`${base} files for that month`, '0'],
                  ...other.map((o) => [`${o.timeframe} file for that month`, `${group(o.rows)} bars`])
                ])}
                <div class="acts">
                  {#if other[0]}
                    <button class="cta" type="button" onclick={() => pinMonth(pinnedMonth, other[0].timeframe)}
                      >Read {other[0].timeframe} for this month</button
                    >
                  {/if}
                  <button class="cta ghost" type="button" onclick={() => (pin = null)}>Unpin the month</button>
                </div>
              </div>
            {:else if chartReason.kind === 'gone'}
              <!-- WHY it is not listed is not known from here — a master that
                   never carried it and a master that has since dropped it
                   produce the identical answer. The overlay states the fact and
                   stops; it does not pick one of the two. -->
              <div class="why">
                <h3>Not listed by {feedLabel(feeds.active)}</h3>
                <p>
                  {feedLabel(feeds.active)}'s master answered, and {pickedKey} is not in it. Whether this feed never carried it or has since
                  dropped it is not something this answer distinguishes, so neither is claimed. The store still holds it either way.
                </p>
                {@render evid([
                  ['selected', pickedKey],
                  [`listed by ${feedLabel(feeds.active)}`, 'no'],
                  ['listed in the same answer', `${group(rows.length)} other instruments`],
                  [
                    'held on disk',
                    censusState === 'ready'
                      ? `${group((censusByInstrument.get(pickedKey) ?? []).reduce((a, m) => a + m.rows, 0))} bars. The store is a different read and it did answer.`
                      : 'not known — the census has not answered'
                  ]
                ])}
                <div class="acts"><button class="cta" type="button" onclick={() => (pickedKey = null)}>Clear the selection</button></div>
              </div>
            {:else if chartReason.kind === 'nolist'}
              <!-- NOT the `gone` overlay. Nothing here says the vendor refused:
                   it says the lookup has nothing to look in, and names which of
                   the two reads is missing. -->
              <div class="why">
                <h3>No list to look {pickedKey} up in</h3>
                <p>{chartReason.why}</p>
                {@render evid([
                  ['selected', pickedKey],
                  ['rows on hand', '0'],
                  ['master answered', observedAt ? `${fClock.format(observedAt)} IST, with 0 rows` : 'no answer recorded'],
                  [`listed by ${feedLabel(feeds.active)}`, 'not known — and not guessed', true]
                ])}
                <div class="acts"><button class="cta" type="button" onclick={rereadMaster}>Read the master now</button></div>
              </div>
            {:else if chartReason.kind === 'norung'}
              <div class="why">
                <h3>No rung here can be read</h3>
                <p>
                  This series is filed under a timeframe this build cannot map to a bar length. Nothing is drawn, because drawing it would
                  mean guessing how many seconds a bar covers — and a wrong guess draws a convincing chart of the wrong buckets.
                </p>
                {@render evid([
                  ['rungs on disk', unmappedRungs.join(' + ') || 'none'],
                  ['this build maps', [...RUNG_SECONDS.keys()].join(', ')],
                  ['claimed base', 'none']
                ])}
              </div>
            {:else if chartReason.kind === 'idle'}
              <div class="why">
                <h3>Pick an instrument</h3>
                <p>Search on the left, or press <kbd class="kbd">/</kbd> from anywhere, then <kbd class="kbd">↑</kbd><kbd class="kbd">↓</kbd> and <kbd class="kbd">↵</kbd>.</p>
              </div>
            {:else if chartReason.kind === 'error'}
              <!-- THE SERVER'S OWN SENTENCE IS SET IN MONO AND QUOTED WHOLE; a
                   reason this page wrote for itself is prose. Spelling them
                   alike is how a paraphrase is read as the server's words. -->
              <div class="why">
                <h3>{cap(chartReason.tsub)}</h3>
                <p class="err">{chartReason.why}</p>
                {@render evid([
                  ['printed', chartReason.raw ? "the server's own sentence, unedited" : "this page's own words — no server message exists for this", true],
                  ['retried', 'nothing has been retried silently', true],
                  ['drawn', 'no partial result is being drawn', true]
                ])}
                <div class="acts">
                  <button class="cta" type="button" onclick={chartReason.chartOnly ? () => (barsRetry += 1) : rereadCensus}>Retry the read</button>
                </div>
              </div>
            {:else}
              <div class="why">
                <h3>{cap(chartReason.tsub)}</h3>
                <p>{chartReason.why}</p>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      {#if barsFaults}
        <p class="faultbar">
          <b>Partial read. </b>The chart above is drawn from the records that could be read. These could not, and are named rather than
          dropped: <span class="m">{barsFaults}</span>
        </p>
      {/if}

      <footer class="credit">
        Candles by <a href="https://www.tradingview.com/" target="_blank" rel="noreferrer noopener">TradingView</a> —
        <span class="m">lightweight-charts</span>, MIT, bundled. Nothing on this page loads from a network it does not own.
      </footer>
    </section>

    <!-- ================================================================
         RIGHT BAY — WHAT IS ON DISK
         ================================================================ -->
    <aside class="bay bay-store rise" aria-label="What is on disk">
      <div class="bayhead">
        <h2>On disk</h2>
        <span class="rt">{censusState === 'ready' && feeds.active ? feedLabel(feeds.active) : ''}</span>
      </div>

      <div class="storebody">
        {#if railReason && railReason.kind === 'error'}
          <div class="empty">
            <h3>{cap(railReason.tsub)}</h3>
            <p>
              {railReason.why} The ladder below reports what is on disk for one instrument, and it cannot report anything until this read
              answers.
            </p>
            {@render evid([
              ['request', `GET /store.json?feed=${feeds.active ?? '—'}`],
              ['bars printed', 'none. No count is shown from a census that did not answer.', true]
            ])}
            <p><button class="lnk" type="button" onclick={rereadCensus}>Retry the census</button></p>
          </div>
        {:else if railReason && railReason.kind === 'wait'}
          <div class="empty">
            <div class="waiting"><span class="dot acc live"></span><span>{railReason.why}</span></div>
            {@render skels(4, 23)}
          </div>
        {:else if railReason || !picked}
          <div class="empty">
            <h3>{cap(railReason?.tsub ?? 'nothing selected')}</h3>
            <p>{railReason?.why ?? 'No instrument selected.'} This bay reports what is on disk for ONE series.</p>
          </div>
          {#if pickedKey && !picked && censusState === 'ready'}
            {@const orphan = censusByInstrument.get(pickedKey) ?? []}
            {#if orphan.length}
              <p class="alarm">
                <b>The store still holds it. </b>{group(orphan.reduce((a, m) => a + m.rows, 0))} bars across {group(orphan.length)} month
                files. The roster is the vendor's master and cannot reach them; the ladder below cannot be drawn from a series this feed
                will not answer for.
              </p>
            {/if}
          {/if}
        {:else}
          <!-- ---- identity ---- -->
          <div class="brief">
            <b class="sy">{picked.symbol}</b>
            <span class="ky">{picked.key}</span>
            <div class="tags">
              <span class="flag" class:info={picked.segment === 'INDEX'}>{picked.segment}</span>
              <!-- THREE TAGS READING "INDEX" IS NOT THREE FACTS. `kind` only
                   appears when it says something the segment does not. -->
              {#if String(picked.kind ?? '').toUpperCase() !== String(picked.segment ?? '').toUpperCase()}
                <span class="flag">{picked.kind}</span>
              {/if}
            </div>
            <div class="tags">
              <u>universe</u>
              {#if !tiersAnswerable}
                <span
                  class="flag warn"
                  title="These rows carry no `universes` array, so this row's tier membership cannot be read off them. Nothing is claimed here."
                  >tiers unanswerable</span
                >
              {/if}
              {#each uniTokens(picked) as u (u)}
                <span class="flag acc">{u}</span>
              {:else}
                <span class="flag" title="This row lists no universe membership at all.">none</span>
              {/each}
            </div>
          </div>

          {#if badMonths.length}
            <p class="alarm">
              <b>{group(badMonths.length)} census row{badMonths.length === 1 ? '' : 's'} for this series could not be read. </b>
              {badMonths[0].why}. Every total below is therefore incomplete and says so. It is not zero: a series that was never pulled
              reads 0 in grey, and these rows cannot be measured at all.
            </p>
          {/if}

          <!-- ---- the tally. Three number states, never one glyph for two. -->
          <!-- RE-ENTRY, AND POINTEDLY NOT `flash-up`. Every figure below is
               replaced wholesale when a different series is picked, and that
               is worth marking: six numbers changing silently in the corner of
               the eye is how an operator ends up reading the last series'
               totals. But the tick flash is SEMANTIC on this console -- green
               means the value went UP. Nothing went up here; a different
               question was asked. So the panel re-enters and stays colourless,
               and `flash-up`/`flash-down` are left for a figure that genuinely
               moved against its own previous value.

               A keyed block is the retrigger the keyframe's own comment
               prescribes -- named in prose here, not typed as a mustache, for
               the reason set out in the script header.

               Keyed on `picked?.key` and not on the numbers, so re-reading the
               census for the SAME series does not blink a panel that did not
               change. The tally holds no state, so tearing it down is free. -->
          {#key picked?.key}
          <dl class="tally fade-in">
            <div><dt>Bars on disk</dt><dd>{@render barCell(barState(picked))}</dd></div>
            <div>
              <dt>Month files</dt>
              <dd>
                <span class={heldMonths.length ? 'n' : 'n0'}>{group(heldMonths.length)}</span>
                {#if badMonths.length}<span class="nq" title="{group(badMonths.length)} further census rows for this series could not be parsed, so this count is a floor and not a total."
                    >+?</span
                  >{/if}
              </dd>
            </div>
            <!-- "LOADED" IS A FACT ABOUT THIS BROWSER, and while a reason says
                 nothing was loaded it is not a zero either — it is a number
                 that was never taken. -->
            <div>
              <dt>Sessions loaded</dt>
              <dd>
                {#if chartReason}<span class="nq" title={chartReason.why}>not loaded</span>{:else}<span class="n">{group(sessions.length)}</span>{/if}
              </dd>
            </div>
            <div>
              <dt>Rungs on disk</dt>
              <dd>
                <span class={storedRungs.length ? 'n' : 'n0'}
                  >{storedRungs.join(' + ') || (heldMonths.length ? 'none this build can map' : 'none stored')}</span
                >
              </dd>
            </div>
            <div class="full">
              <dt>Newest bar loaded</dt>
              <dd class="sm">
                {#if lastBar}<span class="n">{stampLabel(lastBar.t * 1000)} IST</span>{:else}<span
                    class="nq"
                    title="No bar is in memory for this selection, so there is no stamp to print. This is not a claim that the store has none — the ladder below says what it has."
                    >not loaded</span
                  >{/if}
              </dd>
            </div>
            <div class="full">
              <dt>Oldest bar loaded</dt>
              <dd class="sm">
                {#if firstBar}<span class="n">{stampLabel(firstBar.t * 1000)} IST</span>{:else}<span
                    class="nq"
                    title="No bar is in memory for this selection, so there is no stamp to print."
                    >not loaded</span
                  >{/if}
              </dd>
            </div>
          </dl>
          {/key}

          <!-- ---- THE PARTITION. Buckets on screen add to a total on screen,
               and when they do not the page says MISMATCH rather than picking
               one. Drawn only where there is something to partition. -->
          {#if storedRungs.length}
            {@const byRung = storedRungs.map((t) => {
              const ms = heldMonths.filter((m) => m.timeframe === t);
              return { t, n: ms.reduce((a, m) => a + m.rows, 0), m: ms.length };
            })}
            {@const max = Math.max(1, ...byRung.map((x) => x.n))}
            <div class="part">
              <div class="ph">bars by rung — this partition must sum</div>
              {#each byRung as b (b.t)}
                <div class="prow">
                  <span>{b.t}</span>
                  <span class="bar"><i style="width:{(b.n / max) * 100}%"></i></span>
                  <span class="n" title="{group(b.m)} month files at {b.t}">{group(b.n)}</span>
                </div>
              {/each}
              <div class="prow sum" class:bust={skew}>
                <span class="lbl">sum</span>
                <span>{skew ? 'does NOT match the count above' : 'matches the count above'}</span>
                <span class="n">{group(censusTotal)}</span>
              </div>
            </div>
          {/if}
          {#if skew}
            <p class="alarm">
              <b>The two counts disagree. </b>/instruments.json says {group(stated ?? 0)} bars; /store.json's rows total {group(censusTotal)},
              a difference of {group(Math.abs((stated ?? 0) - censusTotal))}. Both are computed from one census, so a difference means one
              of them was read across a write. Neither has been chosen as the truth here, and nothing on this page silently prefers one.
            </p>
          {/if}

          <!-- ---- THE LADDER: month × rung. A pin is a chip, so it always
               carries the rung it exists at, and a month that is not held at
               the rung being read cannot be pinned into an empty chart. ---- -->
          <div class="stackhead">
            <b>month × rung</b>
            <span>{group(heldMonths.length)} files · newest first</span>
          </div>

          {#if heldMonths.length === 0}
            <p class="empty">
              No month file exists for <span class="m">{picked.key}</span> under {feedLabel(feeds.active)}. That is a measured zero: the
              census answered for this series and listed none. <a class="lnk" href="/ingest">Pull it from Ingest →</a>
            </p>
          {:else}
            {@const months = [...new Set(heldMonths.map((m) => m.month))].sort((a, b) => cmp(b, a))}
            {@const inView = new Set(wantMonths)}
            <div class="stack">
              {#each months as mo (mo)}
                {@const rowsHere = heldMonths.filter((m) => m.month === mo)}
                {@const hereAtBase = rowsHere.some((m) => m.timeframe === base)}
                {@const pinnedAt = pin}
                {@const pinned = pinnedAt != null && pinnedAt.month === mo}
                <div class="step" class:pinned>
                  {#if hereAtBase}
                    <button class="mo" type="button" title="Pin {monthLabel(mo)} at {base} — the rung being read." onclick={() => pinMonth(mo, base)}
                      >{monthLabel(mo)}</button
                    >
                  {:else}
                    <!-- NOT CLICKABLE, AND THE REASON IS ON IT. This is the
                         click that used to produce an empty canvas with no
                         overlay: the month exists, but not at the rung being
                         read. The chips beside it are the way in. -->
                    <button
                      class="mo"
                      type="button"
                      aria-disabled="true"
                      title="{monthLabel(mo)} holds no {base} file. It is held at {rowsHere
                        .map((r) => r.timeframe)
                        .join(' and ')} — use the chip beside it, which pins the month AND switches the read to that rung."
                      onclick={() => pinMonth(mo, rowsHere[0].timeframe)}>{monthLabel(mo)}</button
                    >
                  {/if}
                  {#each rowsHere as r (r.timeframe)}
                    <button
                      class="rung"
                      class:base={r.timeframe === base}
                      type="button"
                      aria-pressed={pinned && pinnedAt?.tf === r.timeframe}
                      title="{group(r.rows)} bars in {picked.key}/{r.timeframe}/{r.month}.bin{r.first != null &&
                      r.last != null
                        ? `, covering ${stampLabel(r.first / 1000)} to ${stampLabel(r.last / 1000)} IST`
                        : ''}. {r.timeframe === base
                        ? 'This is the rung being read.'
                        : `Pressing this pins the month AND switches the read to ${r.timeframe} — a pin is a (month, rung) pair, never a bare month.`}"
                      onclick={() => pinMonth(mo, r.timeframe)}
                    >
                      <span>{r.timeframe}</span><span class="ct">{group(r.rows)}</span>
                    </button>
                  {/each}
                  <span
                    class="st"
                    class:on={pinned || inView.has(mo)}
                    title={pinned
                      ? 'Pinned: this month alone is being read.'
                      : inView.has(mo)
                        ? 'Inside the range being read.'
                        : 'On disk and not currently read.'}>{pinned ? 'only' : inView.has(mo) ? 'in view' : 'stored'}</span
                  >
                </div>
              {/each}
            </div>
          {/if}

          {#if badMonths.length}
            <div class="stackhead"><b>rows that would not parse</b><span>{group(badMonths.length)}</span></div>
            <div class="stack">
              {#each badMonths as b, i (i)}
                <div class="step">
                  <span class="mo bad">{typeof b.month === 'string' && MONTH_KEY.test(b.month) ? monthLabel(b.month) : '—'}</span>
                  <span class="rung dead" title={b.why}><span>{b.timeframe ?? '—'}</span><span class="ct">unread</span></span>
                </div>
              {/each}
            </div>
          {/if}

          {#if storedNotListed.length}
            <p class="alarm flat">
              <b>{group(storedNotListed.length)} stored series {storedNotListed.length === 1 ? 'is' : 'are'} not in this master. </b>
              {storedNotListed.slice(0, 8).join(', ')}{storedNotListed.length > 8 ? `, and ${group(storedNotListed.length - 8)} more` : ''}. The
              store does not forget: BSE history the charter keeps, and anything a master has since dropped, is held and is not reachable
              from the roster.
            </p>
          {/if}
          <p class="railnote">
            Every figure in this bay is <b>{feedLabel(feeds.active)}</b>'s, from one read of /store.json. This console never puts one
            feed's numbers beside another's — switch the feed in the top bar and the whole page re-answers.
          </p>
        {/if}
      </div>
    </aside>
  </div>
</div>

<style>
  /* ======================================================================
     ONE VISUAL WORLD, AND IT IS ALREADY WRITTEN DOWN.
     ----------------------------------------------------------------------
     NOTHING BELOW NAMES A COLOUR. Every value is a `theme.css` token, which
     is the file /ingest, /db, /autopilot and /audit already render through.
     A literal in a page <style> is a second theme the toggle cannot reach —
     it has happened on this page once already.

     What IS this page's own is the CONTROL vocabulary, and only that:
     /ingest speaks .pickers/.pk/.ddb/.ddm, /db speaks .strip/.cell/.picker/
     .menu, and this page speaks .helm/.knob/.tray + .bay/.roster/.plot/
     .stack. Same ground, same density, own controls.
     ====================================================================== */
  .mkt {
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 100%;
    padding: 0 var(--s6);
  }

  /* NUMBERS ARE ALWAYS MONO AND ALWAYS TABULAR. A column of counts whose
     digits change width shimmers while it is being read. */
  .m,
  .n,
  .n0,
  .nq,
  .nw {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-feature-settings: 'tnum' 1;
  }
  .up {
    color: var(--up);
  }
  .dn {
    color: var(--down);
  }

  /* ======================================================================
     .helm — THE STORE ADDRESS, AS A CONTROL.
     ====================================================================== */
  .helm {
    display: flex;
    align-items: stretch;
    gap: 0;
    margin: var(--s5) 0 0;
    flex: none;
    background: linear-gradient(180deg, var(--bg-2), var(--panel));
    border-radius: var(--r);
    box-shadow:
      inset 0 1px 0 var(--line-soft),
      var(--e2);
  }
  .knob {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--s1);
    padding: 7px 13px;
    min-width: 0;
    border-right: 1px solid var(--line-soft);
  }
  .knob > b {
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-bold);
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .knob > b :global(em) {
    font-style: normal;
    color: var(--faint);
    letter-spacing: 0;
    text-transform: none;
    font-weight: var(--w-mid);
    font-size: var(--fs-mini);
  }
  .face {
    appearance: none;
    background: none;
    border: 0;
    padding: 0;
    text-align: left;
    /* A FACE THAT CANNOT BE OPENED DOES NOT OFFER A HAND. Only the knobs that
       actually open a tray are buttons; the rest carry a value decided
       elsewhere and say so in their title. */
    cursor: default;
    font-family: var(--mono);
    font-size: var(--fs-base);
    font-weight: var(--w-bold);
    letter-spacing: -0.015em;
    color: var(--ink);
    display: flex;
    align-items: center;
    gap: 7px;
    height: 19px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  button.face {
    cursor: pointer;
  }
  button.face:hover {
    color: var(--acc);
  }
  .face i {
    font-style: normal;
    color: var(--faint);
    font-size: var(--fs-micro);
    transform: translateY(1px);
  }
  .face .sub {
    font-family: var(--sans);
    font-size: var(--fs-xs);
    font-weight: var(--w-mid);
    color: var(--faint);
  }
  /* A FIXED KNOB IS NOT A DISABLED ONE. It carries a value that is decided
     elsewhere and says where, on its own face. */
  .knob.fixed .face {
    cursor: default;
    color: var(--dim);
  }
  .knob.scope {
    background: var(--acc-soft);
    border-radius: var(--r) 0 0 var(--r);
  }
  .knob.scope > b {
    color: var(--acc);
  }
  .knob.derived > b,
  .knob.derived .face {
    color: var(--info);
  }
  .helmgap {
    flex: 1;
  }
  .sep {
    display: flex;
    align-items: center;
    color: var(--faint);
    font-family: var(--mono);
    font-size: var(--fs-md);
    padding: 0 var(--s1);
    user-select: none;
  }
  .stamp {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: var(--s1);
    padding: 7px 14px;
    text-align: right;
    border-left: 1px solid var(--line-soft);
  }
  .stamp b {
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-bold);
  }
  .stamp span {
    font-family: var(--mono);
    font-size: var(--fs-xs);
    color: var(--dim);
  }
  .stamp span.bad {
    color: var(--down);
  }
  .stamp span.off {
    color: var(--faint);
  }

  /* ---- .tray — what a knob opens. Its search box is a real box. ---- */
  .tray {
    position: absolute;
    left: 0;
    top: calc(100% + 6px);
    z-index: 60;
    min-width: 330px;
    max-height: 390px;
    overflow-y: auto;
    background: var(--bg-2);
    border: 1px solid var(--line);
    border-radius: var(--r);
    padding: 5px;
    box-shadow: var(--e3);
  }
  .tray.wide {
    min-width: 430px;
  }
  .sift {
    position: sticky;
    top: -5px;
    background: var(--bg-2);
    padding: var(--s3) 5px var(--s4);
    z-index: 2;
    border-bottom: 1px solid var(--line);
    margin-bottom: 5px;
  }
  .sift input {
    width: 100%;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r);
    font: inherit;
    color: inherit;
    font-family: var(--mono);
    font-size: var(--fs-lg);
    font-weight: var(--w-semi);
    padding: 14px;
    outline: none;
    line-height: 1.2;
  }
  .sift input:focus {
    border-color: var(--acc);
    box-shadow: 0 0 0 3px var(--acc-soft);
  }
  .sift input::placeholder {
    color: var(--faint);
    font-weight: 400;
    font-family: var(--sans);
    font-size: var(--fs-md);
  }
  .sift .hint {
    padding: 7px var(--s2) 0;
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .trow {
    display: flex;
    align-items: center;
    gap: var(--s5);
    width: 100%;
    text-align: left;
    appearance: none;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    padding: var(--s4) 11px;
    border-radius: var(--r2);
    cursor: pointer;
    min-height: 34px;
  }
  .trow:hover {
    background: var(--panel-2);
  }
  .trow[aria-pressed='true'] {
    background: var(--acc-soft);
  }
  .trow[aria-pressed='true'] .nm {
    color: var(--acc);
  }
  .trow .nm {
    font-family: var(--mono);
    font-weight: var(--w-bold);
    font-size: var(--fs-base);
    flex: 0 0 auto;
  }
  .trow .de {
    color: var(--faint);
    font-size: var(--fs-xs);
    margin-left: auto;
    text-align: right;
    font-family: var(--mono);
    flex: 0 1 auto;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .trow.off {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .trow.off:hover {
    background: none;
  }
  .trow.off .de {
    color: var(--warn);
  }
  .traynote {
    padding: var(--s4) 11px;
    font-size: var(--fs-xs);
    color: var(--faint);
    line-height: 1.45;
  }
  .traynote b {
    color: var(--warn);
    font-weight: var(--w-bold);
  }
  .tray hr {
    border: 0;
    border-top: 1px solid var(--line-soft);
    margin: 5px var(--s4);
  }
  .addr {
    display: flex;
    gap: var(--s3);
    padding: var(--s3) var(--s3) 3px;
    align-items: center;
  }
  .addr input {
    width: 78px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    font: inherit;
    color: inherit;
    font-family: var(--mono);
    font-size: var(--fs-base);
    font-weight: var(--w-semi);
    padding: 7px 9px;
    outline: none;
  }
  .addr input[aria-invalid='true'] {
    border-color: var(--down);
    color: var(--down);
  }
  .addr button {
    appearance: none;
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--dim);
    font: inherit;
    font-size: var(--fs-base);
    font-weight: var(--w-bold);
    padding: 7px 13px;
    border-radius: var(--r2);
    cursor: pointer;
  }
  .addr button:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  .addr button:not(:disabled):hover {
    color: var(--acc);
    border-color: var(--acc);
  }
  .refusal {
    display: block;
    padding: var(--s1) var(--s4) var(--s4);
    font-size: var(--fs-xs);
    color: var(--warn);
    line-height: 1.4;
  }

  /* ======================================================================
     .deck — three bays. WHICH SERIES · WHAT IT LOOKS LIKE · WHAT IS ON DISK.
     ----------------------------------------------------------------------
     The row is `minmax(0,1fr)` and not left implicit: an auto row sizes to
     its content, and the three bays would then push the deck past the
     viewport instead of scrolling inside themselves.
     ====================================================================== */
  .deck {
    display: grid;
    grid-template-columns: 328px minmax(0, 1fr) var(--rail-w);
    grid-template-rows: minmax(0, 1fr);
    gap: var(--s5);
    flex: 1;
    min-height: 0;
    padding: var(--s5) 0;
  }
  .bay {
    background: var(--panel);
    border-radius: var(--r);
    box-shadow: 0 0 0 1px var(--line);
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }
  /* THE LIST BAY MAY NOT CLIP, because the universe tray hangs out of it and
     a dropdown that opens into a clip is a control that does not open. The
     radius it loses is put back on the two rows that touch the corners. */
  .bay-list {
    overflow: visible;
  }
  .bay-list .bayhead {
    border-radius: var(--r) var(--r) 0 0;
  }
  .bay-list .rfoot {
    border-radius: 0 0 var(--r) var(--r);
  }
  .bayhead {
    display: flex;
    align-items: center;
    gap: var(--s4);
    padding: var(--s4) var(--s5);
    border-bottom: 1px solid var(--line-soft);
    background: var(--panel-2);
    flex: none;
  }
  .bayhead h2 {
    font-size: var(--fs-mini);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-heavy);
  }
  .bayhead h2.ident {
    font-family: var(--mono);
    font-size: var(--fs-md);
    font-weight: var(--w-heavy);
    letter-spacing: -0.02em;
    text-transform: none;
    color: var(--ink);
  }
  .bayhead .rt {
    font-size: var(--fs-xs);
    color: var(--faint);
    font-family: var(--mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bay-list .bayhead .rt,
  .bay-store .bayhead .rt {
    margin-left: auto;
  }

  /* ---- left bay: the roster ---- */
  .sbox {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: var(--s4) var(--s5);
    border-bottom: 1px solid var(--line-soft);
    flex: none;
  }
  .sbox input {
    flex: 1;
    min-width: 0;
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: var(--r);
    font: inherit;
    color: inherit;
    font-family: var(--mono);
    font-size: var(--fs-base);
    font-weight: var(--w-semi);
    padding: var(--s4) 11px;
    outline: none;
  }
  .sbox input:focus {
    border-color: var(--acc);
    box-shadow: 0 0 0 3px var(--acc-soft);
  }
  .sbox input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .sbox input::placeholder {
    font-family: var(--sans);
    font-weight: var(--w-mid);
    color: var(--faint);
  }

  .facet {
    display: grid;
    /* THE LABEL COLUMN HOLDS THE LONGEST LABEL, and 44px did not.
       Measured in the browser at this font, size and letter-spacing:
       UNIVERSE renders 72px and SEGMENT 69px, against a 44px track. `b` is
       `overflow: visible` with no ellipsis, so the overflow did not clip — it
       DREW, straight over the control beside it, which is why the pip read
       "Everything · 882 listed" with the caption sitting on top of it.
       Both labels are static in the markup directly below; there is no third.
       Fixed and not `max-content` on purpose: each `.facet` is its own grid,
       so a content-sized track would let the two rows disagree by the 3px
       between them and the captions would stop lining up. */
    grid-template-columns: 76px minmax(0, 1fr);
    gap: var(--s3);
    align-items: start;
    padding: 7px var(--s5);
    background: var(--panel-2);
    flex: none;
  }
  .facet + .facet {
    border-top: 1px solid var(--line-soft);
  }
  .facet:last-of-type {
    border-bottom: 1px solid var(--line-soft);
  }
  .facet > b {
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-bold);
    padding-top: var(--s3);
  }
  .fset {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
    align-items: center;
    min-width: 0;
    position: relative;
  }
  /* .pip — the one pressed-button shape on this page. */
  .pip {
    appearance: none;
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--dim);
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    padding: 3px 9px;
    border-radius: var(--r2);
    cursor: pointer;
  }
  .pip:hover:not(:disabled) {
    color: var(--ink);
    border-color: var(--faint);
  }
  .pip[aria-pressed='true'] {
    background: var(--acc);
    color: var(--on-acc);
    border-color: transparent;
    font-weight: var(--w-bold);
  }
  .pip:disabled {
    opacity: 0.4;
    cursor: not-allowed;
    border-style: dashed;
  }
  .pip.amber:not([aria-pressed='true']) {
    color: var(--warn);
    border-color: var(--warn);
  }
  .pip.wide {
    width: 100%;
    text-align: left;
    display: flex;
    align-items: center;
    gap: var(--s4);
    padding: var(--s3) var(--s5);
    font-family: var(--mono);
    font-size: var(--fs-sm);
    font-weight: var(--w-bold);
  }
  .pip.wide .de {
    margin-left: auto;
    color: var(--faint);
    font-weight: var(--w-mid);
    font-size: var(--fs-xs);
  }

  .rhead {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 52px 84px;
    border-bottom: 1px solid var(--line);
    background: var(--panel-2);
    flex: none;
  }
  .hcell {
    appearance: none;
    border: 0;
    background: none;
    color: var(--faint);
    font: inherit;
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    font-weight: var(--w-heavy);
    padding: var(--s3) var(--s5);
    cursor: pointer;
    text-align: left;
    display: flex;
    gap: var(--s2);
  }
  .hcell.r {
    justify-content: flex-end;
    text-align: right;
  }
  .hcell:hover:not(:disabled) {
    color: var(--ink);
  }
  .hcell.on {
    color: var(--acc);
  }
  .hcell:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .roster {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    position: relative;
  }
  /* NOT `.spacer`: `theme.css` already owns that name globally, for the
     `flex: 1` shim the top bar uses. A second meaning for one class name is a
     collision waiting for the first layout that happens to be flex. */
  .rspace {
    position: relative;
  }
  /* `translateY` and not `top`: a transform is composited, so a fast scroll
     does not relayout the window on every frame. */
  .win {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    will-change: transform;
  }
  .rrow {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 52px 84px;
    align-items: center;
    height: var(--row-h);
    padding: 0 var(--s5);
    cursor: pointer;
    border-bottom: 1px solid var(--line-soft);
  }
  .rrow:hover {
    background: var(--panel-2);
  }
  .rrow[aria-selected='true'] {
    background: var(--acc-soft);
    box-shadow: inset 2px 0 0 var(--acc);
  }
  /* THE KEYBOARD CURSOR IS A DIFFERENT THING FROM THE SELECTION and must not
     look like it — the cursor is where Enter would go, the selection is what
     the chart is showing. */
  .rrow[data-cursor='true'] {
    background: var(--panel-2);
    box-shadow: inset 0 0 0 1px var(--acc);
  }
  .rrow .sy {
    font-family: var(--mono);
    font-size: var(--fs-sm);
    font-weight: var(--w-bold);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rrow[aria-selected='true'] .sy {
    color: var(--acc);
  }
  .rrow .sg {
    font-size: var(--fs-mini);
    color: var(--faint);
    font-family: var(--mono);
  }
  .rrow .bl {
    text-align: right;
  }

  .rfoot {
    display: flex;
    align-items: center;
    gap: var(--s4);
    padding: var(--s3) var(--s5);
    border-top: 1px solid var(--line);
    background: var(--panel-2);
    font-size: var(--fs-xs);
    color: var(--faint);
    flex: none;
    flex-wrap: wrap;
  }
  .rfoot .sum {
    font-family: var(--mono);
  }
  .rfoot .sum.off {
    color: var(--faint);
  }
  .rfoot .sum.amber {
    color: var(--warn);
  }
  .rfoot .lg {
    margin-left: auto;
    font-size: var(--fs-mini);
    color: var(--faint);
  }

  /* ---- the shape every refusal takes, in any bay ---- */
  /* `theme.css` also declares `.empty`, CENTRED and in `--faint`. This is a
     reason block with evidence under it, and a centred paragraph of evidence
     is unreadable — so the alignment is stated rather than inherited. */
  .empty {
    padding: 14px var(--s5);
    font-size: var(--fs-sm);
    color: var(--dim);
    line-height: 1.55;
    text-align: left;
  }
  .empty h3 {
    font-size: var(--fs-sm);
    color: var(--ink);
    margin-bottom: 5px;
  }
  .empty p {
    margin-top: var(--s3);
  }
  .evid {
    margin: 9px 0 0;
    border: 1px solid var(--line-soft);
    border-radius: var(--r);
    overflow: hidden;
  }
  .evid div {
    display: flex;
    gap: var(--s5);
    padding: 5px 9px;
    border-bottom: 1px solid var(--line-soft);
    font-size: var(--fs-xs);
  }
  .evid div:last-child {
    border-bottom: 0;
  }
  .evid dt {
    flex: 0 0 96px;
    color: var(--faint);
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    font-weight: var(--w-bold);
    padding-top: var(--s1);
  }
  .evid dd {
    font-family: var(--mono);
    font-size: var(--fs-xs);
    color: var(--ink);
    min-width: 0;
    word-break: break-word;
  }
  .evid dd.q {
    color: var(--dim);
    font-family: var(--sans);
    font-style: italic;
  }
  .lnk {
    appearance: none;
    border: 0;
    background: none;
    color: var(--acc);
    font: inherit;
    font-weight: var(--w-semi);
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .waiting {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: var(--fs-base);
    color: var(--dim);
  }
  /* `theme.css`'s `.skel` supplies the shimmer and the radius and no HEIGHT —
     it is sized by whoever places it. A skeleton row with no height is an
     invisible wait, which is the one thing a wait may not be. */
  .skels {
    display: flex;
    flex-direction: column;
    gap: 7px;
    margin-top: var(--s5);
  }
  .skels .skel {
    height: 9px;
    min-height: 9px;
  }

  /* ---- centre bay: the plot ---- */
  .quote {
    display: flex;
    align-items: baseline;
    gap: 9px;
    margin-left: auto;
    white-space: nowrap;
    min-width: 0;
  }
  .quote .px {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-xl);
    font-weight: var(--w-heavy);
    letter-spacing: -0.02em;
  }
  .quote .ch {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-sm);
    font-weight: var(--w-bold);
  }
  .quote .rf {
    font-size: var(--fs-mini);
    color: var(--faint);
    text-transform: uppercase;
    letter-spacing: var(--track-caps);
  }
  .feedmark {
    display: flex;
    align-items: center;
    gap: var(--s3);
    font-size: var(--fs-xs);
    color: var(--dim);
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: var(--r-full);
    padding: var(--s1) var(--s5) var(--s1) var(--s4);
    flex: none;
  }

  /* THE READING LINE. Never blank: when nothing is drawn it carries the
     reason in its short form and the glass carries it in full. */
  .reading {
    display: flex;
    align-items: center;
    gap: var(--s4);
    flex-wrap: wrap;
    padding: var(--s3) var(--s5);
    border-bottom: 1px solid var(--line-soft);
    background: var(--bg);
    font-size: var(--fs-xs);
    color: var(--dim);
    flex: none;
  }
  .reading .seg {
    display: flex;
    align-items: center;
    gap: 5px;
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
  }
  .reading .seg u {
    text-decoration: none;
    color: var(--faint);
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    font-weight: var(--w-bold);
    font-family: var(--sans);
  }
  .reading .arrow {
    color: var(--faint);
  }
  .reading .seg.bad {
    color: var(--down);
  }
  .reading .seg.warn {
    color: var(--warn);
  }
  .reading .seg.vic b {
    color: var(--info);
  }
  .reading .whyshort {
    font-size: var(--fs-xs);
    /* IT MUST BE ALLOWED TO SHRINK. Without a basis of zero a long reason sets
       the line's minimum width and pushes the segments that name the FAULT off
       the bay — the short form would then be the part that is lost. The full
       sentence is on the glass either way. */
    flex: 1 1 0;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .readout {
    display: flex;
    align-items: center;
    gap: var(--s5);
    flex-wrap: wrap;
    padding: var(--s3) var(--s5);
    border-bottom: 1px solid var(--line-soft);
    flex: none;
    min-height: 31px;
  }
  .pair {
    display: flex;
    align-items: baseline;
    gap: 5px;
  }
  .pair i {
    font-style: normal;
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-bold);
  }
  .pair b {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-sm);
    font-weight: var(--w-bold);
  }
  .readgap {
    flex: 1;
  }

  /* TWO COLOURS, ONE DISTINCTION, EVERYWHERE ON THE PAGE: green is what the
     disk holds, violet is what this browser computed from it. */
  .flag {
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    font-weight: var(--w-heavy);
    padding: var(--s1) 7px;
    border-radius: var(--r1);
    background: var(--panel-2);
    color: var(--faint);
    white-space: nowrap;
  }
  .flag.stored {
    background: var(--up-soft);
    color: var(--up);
  }
  .flag.info {
    background: var(--info-soft);
    color: var(--info);
  }
  .flag.warn {
    background: var(--warn-soft);
    color: var(--warn);
  }
  .flag.acc {
    background: var(--acc-soft);
    color: var(--acc);
  }

  .span {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: var(--s3) var(--s5);
    border-bottom: 1px solid var(--line-soft);
    flex: none;
    flex-wrap: wrap;
  }
  .span > b {
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-bold);
    margin-right: 3px;
  }
  .span .gone {
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .spangap {
    flex: 1;
  }
  .pin {
    display: flex;
    align-items: center;
    gap: 7px;
    background: var(--acc-soft);
    color: var(--acc);
    border: 0;
    border-radius: var(--r2);
    padding: 3px var(--s4);
    font-family: var(--mono);
    font-size: var(--fs-xs);
    font-weight: var(--w-bold);
    cursor: pointer;
  }

  /* THE CANVAS IS THE LAST THING THAT MAY SHRINK, not the first. With only
     `flex:1` the bay's fixed chrome is 190-odd pixels that cannot compress,
     so on a short bay the chart absorbed the whole shortfall and collapsed to
     a height of ZERO — a chart pane containing no chart, silently. */
  .plot {
    position: relative;
    flex: 1 1 auto;
    min-height: 160px;
  }
  .host {
    position: absolute;
    inset: 0;
  }
  .glass {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: 18px;
    overflow-y: auto;
    /* Solid first and translucent second: where `color-mix` is not understood
       the overlay still COVERS, rather than sitting unreadable over whatever
       the chart last drew. */
    background: var(--panel);
    background: color-mix(in srgb, var(--panel) 84%, transparent);
  }
  .why {
    max-width: 520px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    border-radius: var(--r);
    padding: var(--s6) 17px;
    box-shadow: var(--e3);
  }
  .why h3 {
    font-size: var(--fs-md);
    font-weight: var(--w-bold);
    letter-spacing: -0.02em;
    margin-bottom: var(--s3);
  }
  .why p {
    font-size: var(--fs-sm);
    color: var(--dim);
    line-height: 1.55;
  }
  .why p.err {
    font-family: var(--mono);
    color: var(--down);
    background: var(--down-soft);
    border-radius: var(--r2);
    padding: var(--s4) var(--s5);
    margin: 9px 0 0;
    word-break: break-word;
  }
  .why p.warn {
    font-family: var(--mono);
    color: var(--warn);
    background: var(--warn-soft);
    border-radius: var(--r2);
    padding: var(--s4) var(--s5);
    margin: 9px 0 0;
    word-break: break-word;
  }
  .why .acts {
    display: flex;
    gap: 7px;
    margin-top: var(--s5);
    flex-wrap: wrap;
  }
  .cta {
    appearance: none;
    border: 1px solid var(--acc);
    background: var(--acc-soft);
    color: var(--acc);
    font: inherit;
    font-size: var(--fs-sm);
    font-weight: var(--w-bold);
    padding: 7px 13px;
    border-radius: var(--r);
    cursor: pointer;
    text-decoration: none;
  }
  .cta:hover {
    background: var(--acc);
    color: var(--on-acc);
    border-color: var(--acc);
  }
  .cta.ghost {
    border-color: var(--line);
    background: var(--panel);
    color: var(--dim);
  }
  .cta.ghost:hover {
    color: var(--ink);
    border-color: var(--faint);
    background: var(--panel);
  }

  .faultbar {
    padding: 7px var(--s5);
    background: var(--warn-soft);
    color: var(--warn);
    font-size: var(--fs-xs);
    border-top: 1px solid var(--line);
    flex: none;
    line-height: 1.5;
  }
  .faultbar b {
    font-weight: var(--w-heavy);
  }
  .credit {
    padding: 5px var(--s5);
    font-size: var(--fs-mini);
    color: var(--faint);
    border-top: 1px solid var(--line-soft);
    flex: none;
  }
  .credit a {
    color: var(--faint);
  }

  /* ---- right bay: what is on disk ---- */
  .storebody {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .brief {
    padding: 11px var(--s5);
    border-bottom: 1px solid var(--line-soft);
  }
  .brief .sy {
    font-family: var(--mono);
    font-size: var(--fs-lg);
    font-weight: var(--w-heavy);
    letter-spacing: -0.02em;
    word-break: break-all;
  }
  .brief .ky {
    font-family: var(--mono);
    font-size: var(--fs-xs);
    color: var(--faint);
    display: block;
    margin-top: 1px;
  }
  .brief .tags {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
    margin-top: var(--s4);
    align-items: center;
  }
  .brief .tags > u {
    text-decoration: none;
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-bold);
    margin-right: var(--s1);
  }

  .tally {
    display: grid;
    grid-template-columns: 1fr 1fr;
    border-bottom: 1px solid var(--line-soft);
  }
  .tally > div {
    padding: var(--s4) var(--s5);
    border-right: 1px solid var(--line-soft);
    border-bottom: 1px solid var(--line-soft);
    min-width: 0;
  }
  .tally > div:nth-child(2n) {
    border-right: 0;
  }
  .tally > div.full {
    grid-column: 1 / -1;
    border-right: 0;
  }
  .tally dt {
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-bold);
  }
  .tally dd {
    font-size: var(--fs-md);
    font-weight: var(--w-bold);
    margin-top: var(--s1);
    letter-spacing: -0.02em;
    display: flex;
    align-items: baseline;
    gap: 5px;
  }
  .tally dd.sm {
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    color: var(--dim);
    letter-spacing: 0;
  }

  /* THE PARTITION. Buckets on screen add to a total on screen, and the sum
     row states the arithmetic rather than implying it. */
  .part {
    padding: 9px var(--s5);
    border-bottom: 1px solid var(--line-soft);
  }
  .part .ph {
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-bold);
    margin-bottom: var(--s3);
  }
  .prow {
    display: grid;
    grid-template-columns: 52px 1fr auto;
    gap: var(--s4);
    align-items: center;
    font-family: var(--mono);
    font-size: var(--fs-xs);
    padding: var(--s1) 0;
  }
  .prow .bar {
    height: var(--s3);
    border-radius: 3px;
    background: var(--panel-2);
    overflow: hidden;
  }
  .prow .bar i {
    display: block;
    height: 100%;
    background: var(--acc);
    border-radius: 3px;
  }
  .prow.sum {
    border-top: 1px solid var(--line);
    margin-top: 5px;
    padding-top: 5px;
    font-weight: var(--w-heavy);
  }
  .prow.sum .lbl {
    color: var(--faint);
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    font-family: var(--sans);
    font-weight: var(--w-bold);
  }
  .prow.bust {
    color: var(--warn);
  }

  /* ---- .stack — the month × rung ladder ---- */
  .stackhead {
    display: grid;
    grid-template-columns: 1fr auto;
    padding: 7px var(--s5) 5px;
    align-items: baseline;
  }
  .stackhead b {
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-bold);
  }
  .stackhead span {
    font-size: var(--fs-mini);
    color: var(--faint);
    font-family: var(--mono);
  }
  .stack {
    padding: 0 var(--s4) var(--s5);
  }
  .step {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: var(--s2);
    border-radius: var(--r2);
    min-height: 28px;
  }
  .step:hover {
    background: var(--panel-2);
  }
  .step .mo {
    appearance: none;
    border: 0;
    background: none;
    font-family: var(--mono);
    font-size: var(--fs-sm);
    font-weight: var(--w-bold);
    color: var(--ink);
    cursor: pointer;
    padding: var(--s1) var(--s2);
    border-radius: var(--r1);
    flex: 0 0 74px;
    text-align: left;
  }
  .step .mo:hover {
    color: var(--acc);
  }
  .step .mo[aria-disabled='true'],
  .step .mo[aria-disabled='true']:hover {
    color: var(--faint);
    cursor: not-allowed;
  }
  .step .mo.bad {
    color: var(--warn);
    cursor: default;
  }
  .step.pinned {
    background: var(--acc-soft);
  }
  .step.pinned .mo {
    color: var(--acc);
  }
  /* A RUNG CHIP IS THE ATOM OF THIS LADDER. It carries the month AND the
     rung, so a pin can never name a month at a rung that does not hold it. */
  .rung {
    appearance: none;
    border: 1px solid var(--line);
    background: var(--panel);
    border-radius: var(--r2);
    padding: var(--s1) 7px;
    font-family: var(--mono);
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    color: var(--dim);
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: var(--s3);
    white-space: nowrap;
  }
  button.rung:hover {
    border-color: var(--acc);
    color: var(--acc);
  }
  .rung .ct {
    color: var(--faint);
    font-weight: var(--w-semi);
  }
  .rung[aria-pressed='true'] {
    background: var(--acc);
    border-color: transparent;
    color: var(--on-acc);
  }
  .rung[aria-pressed='true'] .ct {
    color: var(--on-acc);
    opacity: 0.75;
  }
  .rung.base {
    box-shadow: inset 0 0 0 1px var(--acc-soft);
  }
  .rung.dead {
    border-style: dashed;
    border-color: var(--warn);
    color: var(--warn);
    cursor: help;
  }
  .rung.dead .ct {
    color: var(--warn);
  }
  .step .st {
    margin-left: auto;
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    font-weight: var(--w-bold);
    flex: 0 0 auto;
  }
  .step .st.on {
    color: var(--acc);
  }

  .alarm {
    margin: 9px var(--s5);
    padding: 9px 11px;
    border-radius: var(--r);
    font-size: var(--fs-xs);
    line-height: 1.5;
    background: var(--warn-soft);
    color: var(--warn);
  }
  /* A RESIDUE IS NOT A FAULT. `.flat` is the same block in the neutral ramp:
     bars held that this master cannot name is a fact about the store's memory,
     not something that went wrong, and amber would read as an alarm. */
  .alarm.flat {
    background: var(--panel-2);
    color: var(--dim);
  }
  .alarm b {
    font-weight: var(--w-heavy);
  }
  .railnote {
    padding: var(--s5) var(--s5) var(--s6);
    font-size: var(--fs-xs);
    color: var(--faint);
    line-height: 1.5;
  }
  .railnote b {
    color: var(--dim);
  }

  /* ---- number states: four of them, and they never look alike ----------
     A count is a measurement. A zero is a DIFFERENT measurement. An
     unreadable field is NOT a measurement at all, and an unanswered read is
     not one either. The page may not spell any two with the same glyph. */
  .n {
    font-weight: var(--w-semi);
  }
  .n0 {
    color: var(--faint);
    font-weight: var(--w-semi);
  }
  .nq {
    font-size: var(--fs-mini);
    font-weight: var(--w-heavy);
    letter-spacing: 0.06em;
    color: var(--warn);
    text-decoration: underline dotted;
    text-underline-offset: 3px;
    cursor: help;
  }
  /* NOT AMBER: nothing is wrong yet, the read simply has not answered. */
  .nw {
    font-size: var(--fs-xs);
    color: var(--faint);
    cursor: help;
  }

  @media (max-width: 1180px) {
    .deck {
      grid-template-columns: 280px minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr) 240px;
    }
    .bay-store {
      grid-column: 1 / -1;
    }
    .helm {
      flex-wrap: wrap;
    }
  }
  @media (max-width: 760px) {
    .mkt {
      padding: 0 var(--s4);
    }
    .deck {
      /* PROPORTIONS OF THE GRID, NOT `vh`. `vh` measures the WINDOW while this
         grid is only as tall as the shell's second row. */
      grid-template-columns: minmax(0, 1fr);
      grid-template-rows: minmax(96px, 0.8fr) minmax(280px, 1.6fr) minmax(140px, 0.8fr);
    }
    .bay-store {
      grid-column: 1;
    }
  }

  /* ======================================================================
     MOTION — all of it, and only here.
     ----------------------------------------------------------------------
     Same discipline as `theme.css`: every transition this component declares
     is inside the guard, so an operator who has asked for less motion gets a
     completely static page and nothing has to opt out.
     ====================================================================== */
  @media (prefers-reduced-motion: no-preference) {
    .pip,
    .hcell,
    .rung,
    .trow,
    .rrow,
    .step .mo {
      transition:
        background-color var(--d-hover) var(--ease-out),
        border-color var(--d-hover) var(--ease-out),
        box-shadow var(--d-state) var(--ease-out),
        color var(--d-hover) var(--ease-out);
    }
  }
</style>

<script>
  /**
   * MARKETS — the front door, and the only page an operator opens cold.
   *
   * Three regions, left to right, in the order the question is actually asked:
   *
   *   1. WHICH INSTRUMENT   search, filter, sort, keyboard. ~800 rows today and
   *                         designed for many more: the list is virtualised and
   *                         every keystroke is one Map probe.
   *   2. WHAT DOES IT LOOK   TradingView's own `lightweight-charts`, candles and
   *      LIKE                volume, a timeframe strip and a range strip.
   *   3. WHAT DO I ACTUALLY  the census for THIS instrument on THIS feed: how
   *      HOLD                many bars, which months, which was the last bar.
   *
   * ---------------------------------------------------------------------------
   * THE RULES THIS FILE IS WRITTEN AGAINST
   * ---------------------------------------------------------------------------
   *
   * ONE FEED. `feeds.active` is chosen once, in the top bar, and everything
   * here is that feed's answer. There is no second series, no "compare", and no
   * place where two vendors' numbers sit beside each other. The two brokers do
   * not list the same instruments, do not hold the same months and do not
   * snap prices the same way; a difference between them reads as signal and is
   * noise. The feed's name is printed beside every number that came from it so
   * a screenshot cannot be misread later.
   *
   * O(1) PER KEYSTROKE. `search()` is a prefix probe into a Map built once at
   * load — see `$lib/index.svelte.js`. Filtering and ordering run over the
   * MATCHES, never over the universe, so typing narrows to a handful first and
   * the sort walks that. Nothing here issues a request per character.
   *
   * ONLY THE VISIBLE SLICE IS IN THE DOM. The census is heading for ~93,776
   * rows and the instrument list will follow the universe; a list that puts
   * every row in the DOM stops being usable long before either number arrives.
   *
   * PAISA ARE INTEGERS UNTIL THE LAST POSSIBLE MOMENT. `CLAUDE.md` section 7.
   * Aggregation to 5m/15m/1h/1D is done on the integers — min, max and last are
   * exact on integers and lossy on floats — and the single divide by 100 happens
   * when a number is handed to the canvas or to a formatter, and nowhere else.
   *
   * NO INNERHTML. Every value below is interpolated by Svelte, which escapes.
   * `M&M` and `M&MFIN` are real symbols and a legal `&` is one careless
   * concatenation away from being an injection.
   *
   * NOTHING FAILS QUIETLY. Every fetch here reports the SERVER'S OWN message —
   * `/bars.json` refuses with `{"error": "..."}` and that string names the file
   * it could not open, which is worth ten "HTTP 400"s. An empty chart always
   * says which of the three empties it is: no instrument picked, nothing stored
   * for this instrument on this feed, or a read that failed.
   */
  import { catalogue, search } from '$lib/index.svelte.js';
  import { feeds } from '$lib/feeds.svelte.js';

  /* ======================================================================
     TIME — the exchange's, not the browser's.
     ----------------------------------------------------------------------
     Bars are stamped in UTC seconds and NSE trades 09:15–15:30 IST. A chart
     that labels the open "03:45" is not wrong about the instant, it is wrong
     about the question. Everything visible is formatted in Asia/Kolkata
     through `Intl`, which ships with the browser — no timezone library, no
     asset, and correct without a network.

     The timestamps themselves are NEVER shifted. Shifting them to make the
     default formatter read IST would make every value handed back by the
     chart a lie by 19,800 seconds.
     ====================================================================== */
  const IST_OFFSET = 19800; // +05:30, in seconds. Used for BUCKETING only.
  const TZ = { timeZone: 'Asia/Kolkata' };
  const LOC = 'en-IN';
  const fTime = new Intl.DateTimeFormat(LOC, { ...TZ, hour: '2-digit', minute: '2-digit', hour12: false });
  const fDay = new Intl.DateTimeFormat(LOC, { ...TZ, day: '2-digit', month: 'short' });
  const fMonth = new Intl.DateTimeFormat(LOC, { ...TZ, month: 'short', year: 'numeric' });
  const fStamp = new Intl.DateTimeFormat(LOC, {
    ...TZ,
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false
  });

  /** The IST calendar day a UTC second falls in, as an integer. */
  const istDay = (t) => Math.floor((t + IST_OFFSET) / 86400);

  /** Paisa integer -> a rupee string. THE ONLY DIVIDE, and it is at the edge. */
  const rupees = (paisa, dp = 2) =>
    paisa == null ? '—' : (paisa / 100).toLocaleString(LOC, { minimumFractionDigits: dp, maximumFractionDigits: dp });
  const count = (n) => (n == null ? '—' : Number(n).toLocaleString(LOC));

  // THE LIST PRINTS THE EXACT COUNT, and a rounded one was actively misleading.
  // A "47.9k" column read the same on all twenty visible rows while the real
  // counts were 47,885, 47,877, 47,875 — so the column that the list is SORTED
  // BY showed no differences, and an operator checking which instrument was
  // fullest saw twenty identical numbers in a deliberate order. Rounding is
  // fine for a headline and wrong for a sort key.

  /* ======================================================================
     THE STORE CENSUS — which months exist, per instrument, for THIS feed.
     ----------------------------------------------------------------------
     `/store.json` answers (instrument, month, timeframe, rows) for the whole
     store. It is fetched ONCE per feed and folded into a Map keyed by
     `EXCHANGE-SEGMENT-SYMBOL`, so asking "what do I hold for this instrument"
     is one probe and not a filter over three thousand rows per selection.

     The key is BUILT from the three parts, never parsed out of the string: a
     symbol may legally contain `-`, and splitting on it would put `NIFTY-50`
     in the wrong bucket silently.
     ====================================================================== */
  const seriesKey = (row) => `${row.exchange}-${row.segment}-${row.symbol}`;

  // FLAT STATE, WRITTEN BY THE LOADER AND NEVER READ BY IT. An effect that
  // reads a `$state` object and later reassigns it depends on its own output
  // and re-runs forever — the loader would refetch the census every time the
  // census arrived. Separate variables that the effect only writes cannot form
  // that cycle, and the shape is checked by the fact that `loadCensus` contains
  // no read of any of them.
  let censusReady = $state(false);
  let censusError = $state(null);
  let censusMonths = $state(new Map());
  let censusToken = 0;

  async function loadCensus(feed) {
    const mine = ++censusToken;
    censusReady = false;
    censusError = null;
    try {
      const r = await fetch(`/store.json?feed=${encodeURIComponent(feed)}`);
      if (!r.ok) throw new Error(`HTTP ${r.status} from /store.json`);
      const rows = await r.json();
      if (mine !== censusToken) return; // a newer feed won; this answer is stale
      const map = new Map();
      for (const row of rows) {
        let bucket = map.get(row.instrument);
        if (!bucket) map.set(row.instrument, (bucket = []));
        bucket.push({ month: row.month, rows: row.rows, timeframe: row.timeframe });
      }
      // Ascending, so "the latest month" is the last element everywhere below.
      for (const bucket of map.values()) bucket.sort((a, b) => (a.month < b.month ? -1 : 1));
      censusMonths = map;
      censusReady = true;
      censusError = null;
    } catch (why) {
      if (mine !== censusToken) return;
      censusMonths = new Map();
      censusReady = false;
      censusError = String(why);
    }
  }

  $effect(() => {
    const feed = feeds.active;
    if (feed) loadCensus(feed);
  });

  /* ======================================================================
     THE BROWSER — search, filter, sort.
     ====================================================================== */
  let typed = $state('');
  let bucket = $state('all');
  let sortKey = $state('bars');
  let sortDir = $state(-1);
  let searchEl = $state(null);

  const BUCKETS = [
    ['all', 'All', 'Every instrument this feed lists'],
    ['index', 'Index', 'Segment INDEX — the engine surface plus reference indices'],
    ['cash', 'Cash', 'Segment CASH — single stocks'],
    ['fno', 'F&O', 'Has a listed derivative'],
    ['ntm', 'NTM', 'NIFTY Total Market constituent'],
    ['held', 'Held', 'This feed has at least one bar stored']
  ];

  function inBucket(row, which) {
    switch (which) {
      case 'all':
        return true;
      case 'index':
        return row.segment === 'INDEX';
      case 'cash':
        return row.segment === 'CASH';
      case 'held':
        return (row.bars ?? 0) > 0;
      default:
        // `universe` is a bitset rendered as `index+fno+ntm`. An instrument in
        // several buckets appears under each, which is the truth and not a bug.
        return String(row.universe ?? '').split('+').includes(which);
    }
  }

  // Counted over the WHOLE universe and only when the universe changes — never
  // per keystroke. A count that moved as you typed would be answering a
  // different question from the one on the tab.
  const counts = $derived.by(() => {
    const rows = catalogue.rows;
    const out = Object.fromEntries(BUCKETS.map(([k]) => [k, 0]));
    for (const row of rows) for (const [k] of BUCKETS) if (inBucket(row, k)) out[k] += 1;
    return out;
  });

  const hits = $derived.by(() => {
    // READ THE CATALOGUE EXPLICITLY. `search()` probes a plain Map that is not
    // reactive, so with a non-empty query this derived would never learn that
    // the universe was rebuilt under it — switch feeds while a query is typed
    // and the list would keep showing the previous feed's instruments. These
    // two reads are the dependency that makes the rebuild land.
    void catalogue.rows.length;
    void catalogue.ready;

    const matched = search(typed).filter((row) => inBucket(row, bucket));
    const dir = sortDir;
    // Ordering runs over the MATCHES. A typed query narrows to a handful first,
    // so this is n log n over the bucket and not over the universe.
    return [...matched].sort((a, b) => {
      if (sortKey === 'bars') {
        const d = (a.bars ?? 0) - (b.bars ?? 0);
        // Ties broken by symbol, so the order is total and the list does not
        // reshuffle between renders on the 542 rows that all hold zero.
        return d !== 0 ? d * dir : a.symbol.localeCompare(b.symbol);
      }
      if (sortKey === 'segment') {
        const d = String(a.segment).localeCompare(String(b.segment));
        return d !== 0 ? d * dir : a.symbol.localeCompare(b.symbol);
      }
      return a.symbol.localeCompare(b.symbol) * dir;
    });
  });

  function sortBy(key) {
    if (sortKey === key) sortDir = -sortDir;
    else {
      sortKey = key;
      sortDir = key === 'bars' ? -1 : 1; // biggest first for a count, A→Z for a name
    }
  }
  const arrow = (key) => (sortKey !== key ? '' : sortDir === 1 ? '▲' : '▼');

  /* ---- the virtual window ----------------------------------------------
     30px rows, the height `--row-h` gives every other list in the console.
     The spacer carries the full height so the scrollbar tells the truth about
     how much there is; only the visible slice plus an overscan is rendered. */
  const ROW = 30;
  const OVERSCAN = 6;
  let scroller = $state(null);
  let scrollTop = $state(0);
  let viewportH = $state(600);
  const firstRow = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVERSCAN));
  const windowLen = $derived(Math.ceil(viewportH / ROW) + OVERSCAN * 2);
  const visible = $derived(hits.slice(firstRow, firstRow + windowLen));

  // ENTERING ROWS ARE STAGGERED, BUT ONLY WHEN THE ANSWER CHANGED. Animating
  // on scroll would make the list flicker every frame the wheel moves, which is
  // motion that obscures rather than explains. The flag is raised when the
  // QUERY changes and lowered a beat later, so a row scrolled into view after
  // that arrives instantly.
  let staggering = $state(false);
  $effect(() => {
    void typed;
    void bucket;
    void sortKey;
    void sortDir;
    void catalogue.rows.length;
    staggering = true;
    // A new answer starts at the top and with no cursor. Leaving the cursor
    // where it was would put "what Enter opens" on a row nobody is looking at.
    cursor = -1;
    if (scroller) scroller.scrollTop = 0;
    scrollTop = 0;
    const t = setTimeout(() => (staggering = false), 420);
    return () => clearTimeout(t);
  });

  /* ======================================================================
     SELECTION AND KEYBOARD
     ----------------------------------------------------------------------
     Only the KEY is state. The row itself is derived, so when the feed changes
     and the new feed does not list this instrument, `picked` becomes null and
     the pane says so by name — rather than charting a stale row the selected
     feed cannot be asked for.
     ====================================================================== */
  let pickedKey = $state(null);
  let cursor = $state(-1);

  const byKey = $derived(new Map(catalogue.rows.map((r) => [r.key, r])));
  const picked = $derived(pickedKey ? (byKey.get(pickedKey) ?? null) : null);
  const missing = $derived(pickedKey != null && picked == null && catalogue.ready);

  // A COLD PAGE SHOWS SOMETHING. An empty chart on first paint reads as a
  // broken build; the first instrument that actually holds bars reads as a
  // console that is alive. `didPick` is a plain variable, not state, so this
  // runs once and never fights an operator's own choice.
  let didPick = false;
  $effect(() => {
    const rows = catalogue.rows;
    if (didPick || rows.length === 0) return;
    didPick = true;
    const held = rows.filter((r) => (r.bars ?? 0) > 0);
    pickedKey = (held.find((r) => r.key === 'NSE-NIFTY') ?? held[0] ?? rows[0]).key;
  });

  function pick(row) {
    if (!row) return;
    pickedKey = row.key;
    focusMonth = null;
  }

  function moveCursor(delta, absolute = null) {
    const n = hits.length;
    if (n === 0) return;
    const next = absolute != null ? absolute : cursor < 0 ? (delta > 0 ? 0 : n - 1) : cursor + delta;
    cursor = Math.max(0, Math.min(n - 1, next));
  }

  // Keep the cursor on screen. Writes to the DOM, never to reactive state, so
  // there is no loop between "the cursor moved" and "the list scrolled".
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

  // `/` focuses the search from anywhere on the page — the shortcut every
  // terminal has. Never while the operator is typing into something else.
  function onWindowKey(e) {
    if (e.key !== '/' || e.metaKey || e.ctrlKey || e.altKey) return;
    const el = document.activeElement;
    const tag = el?.tagName;
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || el?.isContentEditable) return;
    e.preventDefault();
    searchEl?.focus();
    searchEl?.select();
  }

  /* ======================================================================
     WHAT IS HELD FOR THE PICKED INSTRUMENT
     ====================================================================== */
  const heldMonths = $derived(picked ? (censusMonths.get(seriesKey(picked)) ?? []) : []);
  const heldRows = $derived(heldMonths.reduce((a, m) => a + m.rows, 0));

  // THE TWO ANSWERS DISAGREEING IS ITSELF A FINDING. `/instruments.json` counts
  // bars per instrument and `/store.json` counts rows per instrument-month;
  // they are computed from the same census and must agree. If they do not, the
  // page says so rather than picking one and looking confident.
  const censusSkew = $derived(
    picked && censusReady && (picked.bars ?? 0) !== heldRows
      ? `instruments.json says ${count(picked.bars ?? 0)} bars, store.json totals ${count(heldRows)}`
      : null
  );

  /* ======================================================================
     THE CHART
     ====================================================================== */
  // Named fields, not positional tuples. `spec[2]` read as "the third thing"
  // is how a range strip ends up asking for three sessions of a one-month read
  // and nobody notices, because both are numbers.
  const TIMEFRAMES = [
    { id: '1m', seconds: 60 },
    { id: '5m', seconds: 300 },
    { id: '15m', seconds: 900 },
    { id: '1h', seconds: 3600 },
    { id: '1D', seconds: 86400 }
  ];
  /** `months` is how many to READ; `sessions` is how many to SHOW, or 0 for all. */
  const RANGES = [
    { id: '1D', months: 1, sessions: 1 },
    { id: '5D', months: 1, sessions: 5 },
    { id: '1M', months: 1, sessions: 0 },
    { id: '3M', months: 3, sessions: 0 },
    { id: 'MAX', months: Infinity, sessions: 0 }
  ];

  let tf = $state('5m');
  let range = $state('1M');
  let focusMonth = $state(null); // set by the census table; overrides `range`

  // WHICH MONTHS TO ACTUALLY READ. `/bars.json` is one month per request — the
  // path is the index — so the range decides how many requests, and asking for
  // MAX on an instrument with six months is six parallel reads and not a scan.
  const wantMonths = $derived.by(() => {
    if (heldMonths.length === 0) return [];
    if (focusMonth) return heldMonths.filter((m) => m.month === focusMonth).map((m) => m.month);
    const spec = RANGES.find((r) => r.id === range) ?? RANGES[2];
    const all = heldMonths.map((m) => m.month);
    return spec.months === Infinity ? all : all.slice(-spec.months);
  });

  // Same discipline as the census: written by the loader, never read by it.
  let barsLoading = $state(false);
  let barsError = $state(null);
  let barsFaults = $state(null);
  let rawBars = $state([]);
  let barsToken = 0;

  async function readMonth(row, month, feed) {
    const q = new URLSearchParams({
      feed,
      exchange: row.exchange,
      segment: row.segment,
      symbol: row.symbol,
      month
    });
    const r = await fetch(`/bars.json?${q}`);
    let body = null;
    try {
      body = await r.json();
    } catch {
      throw new Error(`${month}: HTTP ${r.status}, and the body was not JSON`);
    }
    // THE SERVER'S OWN SENTENCE, not a status code. `/bars.json` refuses with
    // `{"error": "...2026-08.bin does not exist, opening it"}`, which names the
    // file; "HTTP 400" names nothing and sends the operator to the logs.
    if (body && typeof body === 'object' && !Array.isArray(body) && body.error) {
      throw new Error(`${month}: ${body.error}`);
    }
    if (!r.ok && r.status !== 206) throw new Error(`${month}: HTTP ${r.status}`);
    // A PARTIAL read answers 206 with `{bars, faults}`. Both shapes are handled
    // and a faulty record is surfaced, never silently dropped — a chart with an
    // unexplained hole is worse than a chart with a warning over it.
    const bars = Array.isArray(body) ? body : (body?.bars ?? []);
    return { bars, faults: Array.isArray(body) ? null : (body?.faults ?? null) };
  }

  $effect(() => {
    const row = picked;
    const feed = feeds.active;
    const months = wantMonths;
    const mine = ++barsToken;

    // No instrument, no feed, or nothing on disk. None of these is an error and
    // the overlay below says which of the three nothings it is.
    if (!row || !feed || months.length === 0) {
      barsLoading = false;
      barsError = null;
      barsFaults = null;
      rawBars = [];
      return;
    }
    barsLoading = true;
    barsError = null;
    Promise.all(months.map((m) => readMonth(row, m, feed)))
      .then((parts) => {
        if (mine !== barsToken) return; // a newer selection won; drop this answer
        rawBars = parts.flatMap((p) => p.bars).sort((a, b) => a.t - b.t);
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
     The store holds ONE minute and nothing else, so 5m/15m/1h/1D are DERIVED
     here and the strip says so. Pretending the store has a 1h file would be an
     invention; deriving it in the browser is arithmetic on data already sent.

     Done on the paisa integers. `Math.min`/`Math.max`/last are exact on
     integers; on floats the high of a candle can end up a hair below one of the
     bars it contains. The divide happens once, when a number leaves for the
     canvas.

     Buckets are aligned to IST, not to UTC, so an hourly candle breaks on the
     hour an Indian trader sees rather than 30 minutes off it. */
  function aggregate(raw, seconds) {
    if (seconds === 60) return raw.map((b) => ({ t: b.t, o: b.o, h: b.h, l: b.l, c: b.c, v: b.v }));
    const out = [];
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

  const tfSeconds = $derived((TIMEFRAMES.find((t) => t.id === tf) ?? TIMEFRAMES[1]).seconds);
  const series = $derived(aggregate(rawBars, tfSeconds));
  const hasVolume = $derived(series.some((b) => b.v > 0));

  // DOES THIS CHART NEED A VOLUME PANE — asked ONLY when there is data to
  // answer it with. Between two selections `series` is briefly empty, and an
  // index and a stock disagree about this, so reading `hasVolume` directly
  // would flip false-true-false and rebuild the chart twice for one click.
  // Holding the last answered value across the gap makes it flip once.
  let volumeMode = $state(false);
  $effect(() => {
    if (series.length > 0) volumeMode = hasVolume;
  });

  // Distinct IST sessions in what is loaded. Used both by the range strip
  // ("last five days") and by the rail, which reports sessions rather than
  // calendar days — a month with one trading day held is not "a month".
  const sessions = $derived.by(() => {
    const days = [];
    let last = null;
    for (const b of rawBars) {
      const d = istDay(b.t);
      if (d !== last) {
        days.push(d);
        last = d;
      }
    }
    return days;
  });

  const lastBar = $derived(series.length ? series[series.length - 1] : null);
  const firstBar = $derived(series.length ? series[0] : null);

  // THE REFERENCE FOR "CHANGE" IS NAMED, because there are two honest answers
  // and they are different numbers. With more than one session loaded it is the
  // previous session's close, which is what a quote screen means. With one
  // session it can only be that session's open, and the label says "vs open"
  // rather than implying a close that was never read.
  const reference = $derived.by(() => {
    if (!lastBar) return null;
    const today = istDay(lastBar.t);
    for (let i = rawBars.length - 1; i >= 0; i -= 1) {
      if (istDay(rawBars[i].t) !== today) return { paisa: rawBars[i].c, label: 'Prev close' };
    }
    return firstBar ? { paisa: firstBar.o, label: 'vs open' } : null;
  });
  const change = $derived(lastBar && reference ? lastBar.c - reference.paisa : null);
  const changePct = $derived(change != null && reference?.paisa ? (change / reference.paisa) * 100 : null);

  /* ---- the canvas ------------------------------------------------------ */
  let host = $state(null);
  let chart = $state(null);
  let candles = $state(null);
  // PLAIN, NOT `$state`. Nothing in the markup reads it, and the draw effect
  // both tests and assigns it — as reactive state that is an effect depending
  // on its own output, which is the cycle the census and the bar loader were
  // each restructured to avoid. A non-reactive handle cannot form it.
  let volume = null;
  let libMod = null; // the module handle, for building the volume pane on demand
  let chartError = $state(null);
  let hover = $state(null); // the bar under the crosshair, or null for "the last one"

  // THE CHART TAKES ITS COLOURS FROM THE THEME, not from hexes typed into it.
  // A canvas cannot inherit CSS, so the tokens are read out of the document and
  // re-applied whenever the theme changes — otherwise the console flips to
  // light and the chart stays a dark rectangle in the middle of it.
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
      panel: v('--panel')
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

  // THE CHART IS REBUILT WHEN VOLUME-NESS CHANGES, and that is deliberate.
  // Removing an emptied volume pane from a live chart did not work: the series
  // came off, `removePane` left the pane behind anyway, and an index inherited
  // a dead band of nothing from whichever stock was looked at before it. A
  // pane that will not go away is a pane that should never have been built, so
  // the chart is built for the instrument it is about to draw. It costs a few
  // milliseconds on an index/stock switch — a switch that is already fetching
  // a month of bars — and it removes the whole teardown path.
  $effect(() => {
    const el = host;
    // READ, NOT USED — and that is the point. This is the dependency that makes
    // the chart be rebuilt when an index follows a stock, so the pane the stock
    // needed is simply never built rather than having to be removed.
    void volumeMode;
    if (!el) return;
    let dead = false;
    let made = null;

    (async () => {
      try {
        // TRADINGVIEW'S OWN LIBRARY, MIT, already a dependency and already
        // vendored into node_modules — no CDN, no font, nothing fetched at
        // runtime. Imported dynamically so the shell paints before it lands.
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
            // canvas had half-pixels: BANKNIFTY on MAX loaded 3,195 bars from
            // 2020 onward and drew only the last few hundred, with the rail
            // beside it stating a first bar from January 2020 that was nowhere
            // on the chart. A range control that quietly shows a different
            // range is the fallback-that-hides-a-failure CLAUDE.md section 4 bans.
            minBarSpacing: 0.02,
            // 0 Year, 1 Month, 2 DayOfMonth, 3 Time, 4 TimeWithSeconds.
            tickMarkFormatter: (time, type) => {
              const ms = Number(time) * 1000;
              if (type <= 1) return fMonth.format(ms);
              if (type === 2) return fDay.format(ms);
              return fTime.format(ms);
            }
          },
          localization: { locale: LOC, timeFormatter: (time) => fStamp.format(Number(time) * 1000) },
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
        made.subscribeCrosshairMove((param) => {
          const point = param?.seriesData?.get(cs);
          // `...point` LAST would put the library's own `time` back over mine.
          // It happens to hold the same number today; relying on that is how a
          // readout starts disagreeing with the crosshair after a library
          // upgrade, silently, in the one place the operator trusts most.
          hover = point && param.time != null ? { ...point, time: Number(param.time) } : null;
        });
        chart = made;
        candles = cs;
      } catch (why) {
        // A chart library that will not load is a named failure, not a blank
        // rectangle. Everything else on the page still works.
        if (!dead) chartError = String(why?.message ?? why);
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
      crosshair: {
        vertLine: { color: t.faint, labelBackgroundColor: t.acc },
        horzLine: { color: t.faint, labelBackgroundColor: t.acc }
      }
    });
    cs.applyOptions({ upColor: t.up, downColor: t.down, wickUpColor: t.up, wickDownColor: t.down });
  });

  // DRAW. Separate from both the mount and the fetch, so an async library load
  // racing an already-finished fetch cannot leave the canvas empty — whichever
  // lands last runs this.
  $effect(() => {
    const cs = candles;
    const c = chart;
    const data = series;
    if (!cs || !c) return;

    const t = tokens();
    cs.setData(
      data.map((b) => ({
        time: b.t,
        open: b.o / 100,
        high: b.h / 100,
        low: b.l / 100,
        close: b.c / 100
      }))
    );

    // VOLUME: THE PANE IS BORN WITH ITS DATA, in this same tick. Created empty
    // in the mount effect and fed a frame later, it laid out at zero height and
    // rendered nothing — a pane that has never held a value has no size to
    // claim. Creating it here, immediately followed by `setData`, is the exact
    // sequence that was measured to produce a real two-pane chart.
    //
    // There is no teardown path and there does not need to be one: `volumeMode`
    // is a dependency of the mount effect, so an index and a stock get
    // different CHARTS rather than one chart being talked out of a pane it
    // would not give up. `removePane` left the empty band behind; not building
    // it cannot.
    if (volumeMode && !volume && libMod) {
      const pane = c.panes().length > 1 ? c.panes()[1] : c.addPane();
      const at = pane.paneIndex();
      // An overlay under the candles would share the price axis, and 4,00,000
      // shares and ₹1,325 do not belong on one scale.
      volume = c.addSeries(libMod.HistogramSeries, { priceFormat: { type: 'volume' } }, at);
      c.priceScale('right', at).applyOptions({ borderColor: t.border, scaleMargins: { top: 0.12, bottom: 0 } });
      // No `setHeight`: a pane's height is a share of a layout that has not
      // happened yet, and set synchronously it resolved to a stretch of ZERO.
      // The library's default split is the ~2:1 this wanted, and `enableResize`
      // leaves the ratio with the operator rather than with a constant.
    }
    if (volume) {
      volume.setData(data.map((b) => ({ time: b.t, value: b.v, color: b.c >= b.o ? t.up : t.down })));
    }
    if (data.length === 0) return;

    // THE RANGE STRIP SETS THE VIEW, the fetch sets what exists. `1D` and `5D`
    // are sessions, counted from the data, not calendar days — a Sunday is not
    // a day of trading and a range that counted it would show four days and
    // call it five.
    const spec = RANGES.find((r) => r.id === range);
    const wantDays = focusMonth ? 0 : (spec?.sessions ?? 0);
    if (wantDays > 0 && sessions.length > wantDays) {
      const cutoff = sessions[sessions.length - wantDays];
      const from = data.find((b) => istDay(b.t) >= cutoff)?.t;
      if (from != null) {
        c.timeScale().setVisibleRange({ from, to: data[data.length - 1].t });
        return;
      }
    }
    c.timeScale().fitContent();
  });

  const shown = $derived(hover ?? (lastBar ? { time: lastBar.t, open: lastBar.o / 100, high: lastBar.h / 100, low: lastBar.l / 100, close: lastBar.c / 100 } : null));
  const shownUp = $derived(shown ? shown.close >= shown.open : true);

  /* ---- the three empties, told apart --------------------------------- */
  const emptyReason = $derived.by(() => {
    if (catalogue.error) return { kind: 'error', text: `The instrument index could not be read — ${catalogue.error}` };
    if (feeds.error) return { kind: 'error', text: `The feed list could not be read — ${feeds.error}` };
    if (!feeds.active) return { kind: 'wait', text: 'Waiting for a feed to be selected in the top bar.' };
    if (missing) return { kind: 'gone', text: `${pickedKey} is not listed by ${feeds.active}.` };
    if (!picked) return { kind: 'idle', text: 'No instrument selected.' };
    if (censusError) return { kind: 'error', text: `The store census could not be read — ${censusError}` };
    if (!censusReady) return { kind: 'wait', text: 'Reading the store census…' };
    if (heldMonths.length === 0) return { kind: 'nodata', text: `${feeds.active} holds no bars for ${picked.key}.` };
    if (barsError) return { kind: 'error', text: barsError };
    return null;
  });
</script>

<svelte:window onkeydown={onWindowKey} />

<div class="mkt">
  <!-- ==================================================================
       1. THE INSTRUMENT BROWSER
       ================================================================== -->
  <section class="pane col-list">
    <div class="pane-head srch">
      <input
        class="search"
        bind:this={searchEl}
        bind:value={typed}
        onkeydown={onSearchKey}
        oninput={() => (cursor = -1)}
        placeholder={catalogue.ready ? `Search ${count(catalogue.rows.length)} instruments` : 'Loading the universe…'}
        aria-label="Search instruments by symbol"
        role="combobox"
        aria-expanded="true"
        aria-controls="instrument-list"
        aria-autocomplete="list"
        aria-activedescendant={cursor >= 0 ? `inst-${cursor}` : undefined}
        autocomplete="off"
        spellcheck="false"
      />
      <kbd class="kbd slash">/</kbd>
    </div>

    <nav class="tabs" aria-label="Filter by universe">
      {#each BUCKETS as [key, label, why] (key)}
        <button class="utab" type="button" aria-pressed={bucket === key} title={why} onclick={() => (bucket = key)}>
          {label}<span class="n">{count(counts[key] ?? 0)}</span>
        </button>
      {/each}
    </nav>

    <div class="lhead" aria-hidden="true">
      <button class="hcell" type="button" onclick={() => sortBy('symbol')} class:on={sortKey === 'symbol'}>
        Symbol <span class="ar">{arrow('symbol')}</span>
      </button>
      <button class="hcell" type="button" onclick={() => sortBy('segment')} class:on={sortKey === 'segment'}>
        Seg <span class="ar">{arrow('segment')}</span>
      </button>
      <button class="hcell r" type="button" onclick={() => sortBy('bars')} class:on={sortKey === 'bars'}>
        Bars <span class="ar">{arrow('bars')}</span>
      </button>
    </div>

    <div
      class="vlist"
      id="instrument-list"
      role="listbox"
      aria-label="Instruments"
      tabindex="-1"
      bind:this={scroller}
      bind:clientHeight={viewportH}
      onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
    >
      {#if catalogue.error}
        <p class="note bad">
          The instrument index could not be read — {catalogue.error}. Nothing below is a
          filtered view of stale data; there is no data.
        </p>
      {:else if !catalogue.ready}
        <!-- A SHAPE CLAIM, not a spinner. It says "rows go here, this wide". -->
        <div class="skels" aria-hidden="true">
          {#each Array(18) as _, i (i)}
            <div class="skelrow">
              <span class="skel" style="width:{38 + ((i * 17) % 42)}%"></span>
              <span class="skel" style="width:34px"></span>
              <span class="skel" style="width:{28 + ((i * 11) % 30)}px"></span>
            </div>
          {/each}
        </div>
      {:else if hits.length === 0}
        <p class="note">
          {#if typed}
            Nothing in <b>{BUCKETS.find(([k]) => k === bucket)?.[1]}</b> starts with “{typed}”.
            <button class="link" type="button" onclick={() => (typed = '')}>Clear the search</button>
            {#if bucket !== 'all'}
              or <button class="link" type="button" onclick={() => (bucket = 'all')}>search everything</button>
            {/if}.
          {:else}
            {feeds.active ?? 'This feed'} lists nothing in <b>{BUCKETS.find(([k]) => k === bucket)?.[1]}</b>.
            <button class="link" type="button" onclick={() => (bucket = 'all')}>Show all</button>.
          {/if}
        </p>
      {:else}
        <div class="spacer-v" style="height:{hits.length * ROW}px" role="presentation">
          <div class="win" style="transform:translateY({firstRow * ROW}px)" role="presentation">
            {#each visible as row, i (row.key)}
              <div
                class="vrow"
                id="inst-{firstRow + i}"
                role="option"
                tabindex="-1"
                aria-selected={pickedKey === row.key}
                aria-posinset={firstRow + i + 1}
                aria-setsize={hits.length}
                data-cursor={cursor === firstRow + i}
                class:row-in={staggering}
                style="animation-delay:{Math.min(i, 20) * 14}ms"
                onclick={() => pick(row)}
                onkeydown={(e) => e.key === 'Enter' && pick(row)}
              >
                <span class="sym">{row.symbol}</span>
                <span class="seg">{row.segment}</span>
                <span class="bars" class:zero={(row.bars ?? 0) === 0}>
                  {(row.bars ?? 0) === 0 ? '—' : count(row.bars)}
                </span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    </div>

    <footer class="lfoot">
      <span>{count(hits.length)} of {count(catalogue.rows.length)}</span>
      <span class="spacer"></span>
      <span class="hint"><kbd class="kbd">↑</kbd><kbd class="kbd">↓</kbd> move <kbd class="kbd">↵</kbd> open</span>
    </footer>
  </section>

  <!-- ==================================================================
       2. THE CHART
       ================================================================== -->
  <section class="pane col-chart">
    <div class="pane-head chead">
      {#if picked}
        {#key picked.key}
          <span class="ident fade-in">
            <b class="tsym">{picked.symbol}</b>
            <span class="tkey">{picked.key}</span>
            <span class="tag {picked.segment === 'INDEX' ? 'info' : ''}">{picked.segment}</span>
          </span>
        {/key}
      {:else}
        <span class="ident"><b class="tsym dimmed">No instrument</b></span>
      {/if}

      {#if shown}
        <span class="quote">
          <b class="last mono" class:up={shownUp} class:down={!shownUp}>{shown.close.toLocaleString(LOC, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</b>
          {#if change != null}
            <span class="chg mono" class:up={change >= 0} class:down={change < 0}>
              {change >= 0 ? '+' : '−'}{rupees(Math.abs(change))}
              ({change >= 0 ? '+' : '−'}{Math.abs(changePct ?? 0).toFixed(2)}%)
            </span>
            <span class="ref">{reference?.label}</span>
          {/if}
        </span>
      {/if}

      <span class="spacer"></span>
      <span class="feedmark" title="Every number on this page is {feeds.active}'s. One feed at a time, never two.">
        <span class="dot acc"></span>{feeds.active ?? '—'}
      </span>
    </div>

    <!-- The OHLC readout. Follows the crosshair; falls back to the last bar so
         the strip is never blank while the pointer is off the canvas. -->
    <div class="legend" aria-live="off">
      {#if shown}
        <span class="pair"><i>O</i><b class="mono" class:up={shownUp} class:down={!shownUp}>{shown.open.toFixed(2)}</b></span>
        <span class="pair"><i>H</i><b class="mono" class:up={shownUp} class:down={!shownUp}>{shown.high.toFixed(2)}</b></span>
        <span class="pair"><i>L</i><b class="mono" class:up={shownUp} class:down={!shownUp}>{shown.low.toFixed(2)}</b></span>
        <span class="pair"><i>C</i><b class="mono" class:up={shownUp} class:down={!shownUp}>{shown.close.toFixed(2)}</b></span>
        <span class="pair stamp"><i>{hover ? 'AT' : 'LAST'}</i><b class="mono">{fStamp.format(shown.time * 1000)} IST</b></span>
        {#if !hasVolume}
          <span class="tag warn" title="The store records zero traded volume for this instrument. An index has none.">no volume</span>
        {/if}
      {:else}
        <span class="pair"><i>—</i></span>
      {/if}
      <span class="spacer"></span>
      {#if barsLoading}
        <span class="pair loadmark"><span class="dot acc live"></span><b>reading {wantMonths.length} month{wantMonths.length === 1 ? '' : 's'}…</b></span>
      {:else if series.length}
        <span class="pair"><i>BARS</i><b class="mono">{count(series.length)}</b></span>
      {/if}
    </div>

    <div class="strips">
      <div class="strip" role="group" aria-label="Timeframe">
        <span class="lbl">TF</span>
        {#each TIMEFRAMES as t (t.id)}
          <button class="chip" type="button" aria-pressed={tf === t.id} onclick={() => (tf = t.id)}>{t.id}</button>
        {/each}
        <!-- HONEST ABOUT WHERE THE CANDLE CAME FROM. The store holds one
             minute and nothing else; everything above 1m is arithmetic done
             here, not a file on disk. -->
        <span class="derived" title="The store holds 1-minute bars only. 5m, 15m, 1h and 1D are aggregated in the browser from those.">
          {tf === '1m' ? 'as stored' : 'derived from 1m'}
        </span>
      </div>
      <div class="strip" role="group" aria-label="Range">
        <span class="lbl">Range</span>
        {#each RANGES as r (r.id)}
          <button
            class="chip"
            type="button"
            aria-pressed={!focusMonth && range === r.id}
            onclick={() => {
              range = r.id;
              focusMonth = null;
            }}>{r.id}</button
          >
        {/each}
        {#if focusMonth}
          <button class="chip pinned" type="button" aria-pressed="true" onclick={() => (focusMonth = null)}>
            {focusMonth} ✕
          </button>
        {/if}
      </div>
    </div>

    <div class="canvas" class:busy={barsLoading}>
      <div class="host" bind:this={host}></div>

      {#if chartError}
        <div class="over">
          <div class="blank">
            <h2>The chart library did not load</h2>
            <p>{chartError}</p>
            <p class="fine">
              Everything else on this page still works — the list, the search and the census on
              the right are unaffected.
            </p>
          </div>
        </div>
      {:else if emptyReason}
        <div class="over">
          <div class="blank panel-in">
            {#if emptyReason.kind === 'nodata'}
              <h2>Nothing stored for {picked?.symbol}</h2>
              <p>
                <b>{feeds.active}</b> holds no bars for <span class="mono">{picked?.key}</span>. The
                instrument is listed by this feed, so it can be pulled — it just has not been.
              </p>
              <div class="alt">
                <span class="lbl">Feed</span><b>{feeds.active}</b>
                <span class="lbl">Instrument</span><b class="mono">{picked?.key}</b>
                <span class="lbl">Months held</span><b>0</b>
              </div>
              <a class="cta" href="/ingest">Pull it from Ingest →</a>
            {:else if emptyReason.kind === 'gone'}
              <h2>Not listed by {feeds.active}</h2>
              <p>
                <span class="mono">{pickedKey}</span> was selected under a different feed. The two
                brokers do not list the same instruments, so this one cannot be asked for here.
              </p>
              <button class="cta" type="button" onclick={() => (pickedKey = null)}>Clear the selection</button>
            {:else if emptyReason.kind === 'error'}
              <h2>That read failed</h2>
              <p class="err mono">{emptyReason.text}</p>
              <p class="fine">
                This is the server's own message, unedited. Nothing has been retried silently and
                no partial result is being drawn.
              </p>
            {:else if emptyReason.kind === 'wait'}
              <p class="waiting"><span class="dot acc live"></span>{emptyReason.text}</p>
            {:else}
              <h2>Pick an instrument</h2>
              <p>
                Search on the left, or press <kbd class="kbd">/</kbd> from anywhere, then
                <kbd class="kbd">↑</kbd><kbd class="kbd">↓</kbd> and <kbd class="kbd">↵</kbd>.
              </p>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    {#if barsFaults}
      <p class="faultbar">
        <b>Partial read.</b> The chart above is drawn from the records that could be read; these could
        not: <span class="mono">{barsFaults}</span>
      </p>
    {/if}

    <footer class="credit">
      Charts by <a href="https://www.tradingview.com/" target="_blank" rel="noreferrer noopener">TradingView</a> —
      <span class="mono">lightweight-charts</span>, MIT, bundled. Nothing on this page loads from a network it does not own.
    </footer>
  </section>

  <!-- ==================================================================
       3. THE SUMMARY RAIL — what is actually on disk
       ================================================================== -->
  <aside class="pane col-rail" aria-label="What is stored for the selected instrument">
    <div class="pane-head">
      <span class="pane-title">Stored</span>
      <span class="spacer"></span>
      {#if censusReady}<span class="pane-title">{feeds.active}</span>{/if}
    </div>

    {#if !picked}
      <p class="note">Nothing selected. The rail reports what is on disk for one instrument.</p>
    {:else if censusError}
      <p class="note bad">The store census could not be read — {censusError}</p>
    {:else if !censusReady}
      <div class="skels" aria-hidden="true">
        {#each Array(4) as _, i (i)}<span class="skel line" style="width:{50 + ((i * 23) % 40)}%"></span>{/each}
      </div>
    {:else}
      {#key picked.key}
        <div class="railbody fade-in">
          <div class="idcard">
            <b class="idsym">{picked.symbol}</b>
            <span class="idkey mono">{picked.key}</span>
            <!-- THREE TAGS READING "INDEX" IS NOT THREE FACTS. `kind` is
                 "Index", `segment` is "INDEX" and `universe` contains "index";
                 printing all three put the same word on the card three times
                 in three cases and made the operator look for a difference
                 that was not there. The segment is the store dimension and is
                 shown; `kind` only appears when it says something the segment
                 does not; the universe is a separate, labelled line because it
                 answers a different question — which sweep list this is on. -->
            <span class="idtags">
              <span class="tag {picked.segment === 'INDEX' ? 'info' : ''}">{picked.segment}</span>
              {#if String(picked.kind ?? '').toUpperCase() !== String(picked.segment ?? '').toUpperCase()}
                <span class="tag">{picked.kind}</span>
              {/if}
            </span>
            {#if String(picked.universe ?? '').length}
              <span class="idtags">
                <span class="lbl">Universe</span>
                {#each String(picked.universe).split('+').filter(Boolean) as u (u)}
                  <span class="tag acc">{u}</span>
                {/each}
              </span>
            {/if}
          </div>

          <dl class="facts">
            <div><dt>Bars stored</dt><dd class="mono big">{count(picked.bars ?? 0)}</dd></div>
            <div><dt>Months held</dt><dd class="mono big">{count(heldMonths.length)}</dd></div>
            <div>
              <dt>Sessions loaded</dt>
              <dd class="mono big">{count(sessions.length)}</dd>
            </div>
            <div>
              <dt>Last bar</dt>
              <dd class="mono">
                {#if lastBar}{fStamp.format(lastBar.t * 1000)} <span class="unit">IST</span>{:else}—{/if}
              </dd>
            </div>
            <div>
              <dt>First bar loaded</dt>
              <dd class="mono">
                {#if firstBar}{fStamp.format(firstBar.t * 1000)} <span class="unit">IST</span>{:else}—{/if}
              </dd>
            </div>
            <div><dt>Timeframe on disk</dt><dd class="mono">1m<span class="unit"> — everything else is derived</span></dd></div>
          </dl>

          {#if censusSkew}
            <p class="note warnbox">
              <b>The two counts disagree.</b> {censusSkew}. Both come from the same census, so a
              difference means one of them was read across a write. Neither has been picked as the
              truth here.
            </p>
          {/if}

          <div class="months">
            <div class="mhead">
              <span>Month</span><span class="r">Bars</span><span class="r">Load</span>
            </div>
            {#if heldMonths.length === 0}
              <p class="note">
                No month files exist for <span class="mono">{picked.key}</span> under
                <b>{feeds.active}</b>. <a class="link" href="/ingest">Pull it →</a>
              </p>
            {:else}
              {#each heldMonths as m, i (m.month)}
                <button
                  class="mrow row-in"
                  type="button"
                  style="animation-delay:{Math.min(i, 12) * 22}ms"
                  aria-pressed={focusMonth === m.month || (!focusMonth && wantMonths.includes(m.month))}
                  onclick={() => (focusMonth = focusMonth === m.month ? null : m.month)}
                >
                  <span class="mono">{m.month}</span>
                  <span class="r mono">{count(m.rows)}</span>
                  <span class="r state">
                    {#if focusMonth === m.month}only{:else if wantMonths.includes(m.month)}in view{:else}—{/if}
                  </span>
                </button>
              {/each}
            {/if}
          </div>

          <p class="railnote">
            Every figure here is <b>{feeds.active}</b>'s. This console never puts one feed's numbers
            beside another's — switch the feed in the top bar and the whole page re-answers.
          </p>
        </div>
      {/key}
    {/if}
  </aside>
</div>

<style>
  /* ======================================================================
     LAYOUT
     ----------------------------------------------------------------------
     Three columns at desk width, because the three questions — which
     instrument, what does it look like, what do I hold — are asked together
     and a tab between them costs a click every time.

     Below 1180px the rail moves under the chart rather than disappearing:
     the census is the reason the operator trusts the chart, and hiding it at
     laptop width would hide the evidence.
     ====================================================================== */
  .mkt {
    display: grid;
    grid-template-columns: var(--rail-w) minmax(0, 1fr) 286px;
    grid-template-rows: minmax(0, 1fr);
    min-height: 0;
    height: 100%;
  }
  /* The dividing rules come from the theme's `.pane + .pane`, so there is
     exactly one of them between any two columns. Drawing a second here would
     put a 2px seam down a console whose every other seam is 1px. */
  .col-list {
    grid-column: 1;
  }
  .col-chart {
    grid-column: 2;
  }
  .col-rail {
    grid-column: 3;
    overflow-y: auto;
  }

  @media (max-width: 1180px) {
    .mkt {
      grid-template-columns: var(--rail-w) minmax(0, 1fr);
      grid-template-rows: minmax(0, 1fr) auto;
    }
    .col-rail {
      grid-column: 1 / -1;
      grid-row: 2;
      max-height: 240px;
      border-left: 0;
      border-top: 1px solid var(--line);
    }
    .col-list,
    .col-chart {
      grid-row: 1;
    }
  }
  @media (max-width: 760px) {
    .mkt {
      grid-template-columns: minmax(0, 1fr);
      /* PROPORTIONS OF THE GRID, NOT `vh`. `44vh` measured the WINDOW while
         this grid is only as tall as the shell's second row, so on a short
         viewport the list took a share it had not earned and the chart pane
         was left shorter than its own header. Fractions divide what actually
         exists, and the floors keep both regions usable. */
      grid-template-rows: minmax(96px, 0.8fr) minmax(280px, 1.6fr) auto;
    }
    .col-list {
      grid-column: 1;
      grid-row: 1;
      border-bottom: 1px solid var(--line);
    }
    .col-chart {
      grid-column: 1;
      grid-row: 2;
      /* Stacked, so the theme's vertical divider would be a rule down the left
         edge of a full-width column. */
      border-left: 0;
      border-top: 1px solid var(--line);
    }
    .col-rail {
      grid-row: 3;
    }
  }

  /* ---- list ----------------------------------------------------------- */
  .srch {
    gap: var(--s3);
  }
  .srch .search {
    flex: 1;
  }
  .slash {
    flex: none;
    opacity: 0.7;
  }

  .lhead {
    display: grid;
    grid-template-columns: 1fr 48px 76px;
    gap: var(--s3);
    padding: 0 var(--s5);
    height: 24px;
    align-items: center;
    background: var(--bg-2);
    border-bottom: 1px solid var(--line);
    flex: none;
  }
  .hcell {
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
  }
  .hcell.r {
    text-align: right;
  }
  .hcell:hover,
  .hcell.on {
    color: var(--ink);
  }
  .hcell .ar {
    font-size: 7px;
    color: var(--acc);
    vertical-align: 1px;
  }

  .spacer-v {
    position: relative;
  }
  /* `translateY` and not `top`: a transform is composited, so a fast scroll
     does not relayout the window on every frame. */
  .win {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    will-change: transform;
  }

  /* The row is the theme's `.vrow`, given a third column so the counts form a
     real column instead of drifting with the length of the symbol. */
  .vrow {
    grid-template-columns: 1fr 48px 76px;
    gap: var(--s3);
  }
  .vrow .seg {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: 0.06em;
    color: var(--faint);
  }
  .vrow .bars {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-xs);
    text-align: right;
    color: var(--up);
  }
  .vrow .bars.zero {
    color: var(--n7);
  }
  /* The KEYBOARD cursor is a different thing from the SELECTION and must not
     look like it — the cursor is where Enter would go, the selection is what
     the chart is showing. */
  .vrow[data-cursor='true'] {
    background: var(--panel-2);
    box-shadow: inset 0 0 0 1px var(--acc);
  }
  .vrow[aria-selected='true'] .seg {
    color: var(--dim);
  }

  .skels {
    padding: var(--s2) var(--s5);
  }
  .skelrow {
    display: grid;
    grid-template-columns: 1fr 48px 76px;
    gap: var(--s3);
    align-items: center;
    height: var(--row-h);
  }
  .skelrow .skel {
    height: 9px;
    justify-self: stretch;
  }

  .lfoot {
    display: flex;
    align-items: center;
    gap: var(--s4);
    flex: none;
    padding: var(--s3) var(--s5);
    border-top: 1px solid var(--line);
    background: var(--bg-2);
    font-size: var(--fs-mini);
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .lfoot .hint {
    display: flex;
    align-items: center;
    gap: 3px;
  }
  .spacer {
    flex: 1;
  }

  .note {
    padding: var(--s6) var(--s5);
    color: var(--dim);
    font-size: var(--fs-sm);
    line-height: 1.55;
  }
  .note.bad {
    color: var(--down);
  }
  .note.warnbox {
    margin: 0 var(--s5) var(--s5);
    padding: var(--s4);
    border: 1px solid var(--warn);
    background: var(--warn-soft);
    border-radius: var(--r2);
    color: var(--ink);
    font-size: var(--fs-xs);
  }

  /* ---- chart head ----------------------------------------------------- */
  .chead {
    gap: var(--s5);
    flex-wrap: nowrap;
    overflow: hidden;
  }
  .ident {
    display: flex;
    align-items: baseline;
    gap: var(--s4);
    min-width: 0;
  }
  .tsym {
    font-size: var(--fs-lg);
    font-weight: var(--w-heavy);
    letter-spacing: -0.3px;
    white-space: nowrap;
  }
  .tsym.dimmed {
    color: var(--faint);
    font-weight: var(--w-semi);
  }
  .tkey {
    font-family: var(--mono);
    font-size: var(--fs-mini);
    color: var(--faint);
    white-space: nowrap;
  }
  .quote {
    display: flex;
    align-items: baseline;
    gap: var(--s4);
    white-space: nowrap;
  }
  .last {
    font-size: var(--fs-xl);
    font-weight: var(--w-bold);
    letter-spacing: -0.6px;
  }
  .chg {
    font-size: var(--fs-sm);
    font-weight: var(--w-bold);
  }
  .ref {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: var(--track-caps);
    color: var(--faint);
    font-weight: var(--w-bold);
  }
  .feedmark {
    display: flex;
    align-items: center;
    gap: var(--s3);
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--dim);
    flex: none;
  }

  .legend {
    display: flex;
    align-items: center;
    gap: var(--s5);
    flex: none;
    padding: var(--s3) var(--s5);
    border-bottom: 1px solid var(--line-soft);
    background: var(--bg-2);
    font-size: var(--fs-xs);
    overflow: hidden;
    white-space: nowrap;
  }
  .legend .pair {
    display: inline-flex;
    align-items: baseline;
    gap: var(--s3);
  }
  .legend i {
    font-style: normal;
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    color: var(--faint);
  }
  .legend b {
    font-weight: var(--w-semi);
  }
  .legend .stamp b {
    color: var(--dim);
    font-size: var(--fs-mini);
  }
  .loadmark b {
    color: var(--acc);
    font-weight: var(--w-bold);
  }

  .strips {
    display: flex;
    align-items: center;
    gap: var(--s6);
    flex: none;
    flex-wrap: wrap;
    padding: var(--s3) var(--s5);
    border-bottom: 1px solid var(--line);
  }
  .strip {
    display: flex;
    align-items: center;
    gap: var(--s2);
  }
  .strip .lbl {
    margin-right: var(--s2);
  }
  .chip {
    padding: 3px var(--s4);
    min-width: 30px;
    border-radius: var(--r1);
    border: 1px solid transparent;
    background: transparent;
    color: var(--dim);
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: 650;
    cursor: pointer;
    font-variant-numeric: tabular-nums;
  }
  .chip:hover {
    color: var(--ink);
    background: var(--panel-2);
  }
  .chip[aria-pressed='true'] {
    background: var(--acc-soft);
    border-color: var(--acc);
    color: var(--ink);
  }
  .chip.pinned {
    border-style: dashed;
  }
  .derived {
    margin-left: var(--s4);
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: var(--track-caps);
    font-weight: var(--w-bold);
    color: var(--faint);
    border-left: 1px solid var(--line);
    padding-left: var(--s4);
  }

  /* ---- the canvas ----------------------------------------------------- */
  /* THE CANVAS IS THE LAST THING THAT MAY SHRINK, not the first. With only
     `flex:1` the pane's fixed chrome — header, OHLC legend, two strips and the
     credit line — is 190-odd pixels that cannot compress, so on a short pane
     the chart was the one item that absorbed the whole shortfall and collapsed
     to a height of ZERO. A chart pane containing no chart is not a degraded
     layout, it is a missing feature, and it happened silently. The floor makes
     the credit line clip instead, which is the right thing to lose. */
  .canvas {
    position: relative;
    flex: 1 1 auto;
    min-height: 200px;
  }
  .host {
    position: absolute;
    inset: 0;
  }
  .over {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    padding: var(--s6);
    background: var(--bg);
    overflow-y: auto;
  }
  .over .blank {
    margin: 0;
  }
  .blank .fine {
    font-size: var(--fs-xs);
    color: var(--faint);
    margin-top: var(--s5);
  }
  .blank .err {
    color: var(--down);
    font-size: var(--fs-sm);
    text-align: left;
    background: var(--down-soft);
    border: 1px solid var(--down);
    border-radius: var(--r2);
    padding: var(--s5);
    word-break: break-word;
  }
  .waiting {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--s4);
    color: var(--dim);
    font-size: var(--fs-sm);
  }

  .faultbar {
    flex: none;
    padding: var(--s4) var(--s5);
    border-top: 1px solid var(--warn);
    background: var(--warn-soft);
    color: var(--ink);
    font-size: var(--fs-xs);
    line-height: 1.5;
  }
  .credit {
    flex: none;
    padding: var(--s2) var(--s5);
    border-top: 1px solid var(--line-soft);
    background: var(--bg-2);
    color: var(--n7);
    font-size: var(--fs-micro);
    letter-spacing: 0.02em;
  }
  .credit a {
    color: var(--faint);
  }

  /* ---- rail ------------------------------------------------------------ */
  .railbody {
    padding: var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s5);
  }
  .idcard {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .idsym {
    font-size: var(--fs-xl);
    font-weight: var(--w-heavy);
    letter-spacing: -0.6px;
    line-height: var(--lh-tight);
    word-break: break-all;
  }
  .idkey {
    font-size: var(--fs-mini);
    color: var(--faint);
  }
  .idtags {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
    margin-top: var(--s2);
  }

  .facts {
    display: grid;
    grid-template-columns: 1fr;
    gap: 1px;
    background: var(--line);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    overflow: hidden;
  }
  .facts > div {
    background: var(--panel);
    padding: var(--s4) var(--s5);
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--s4);
  }
  .facts dt {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    white-space: nowrap;
  }
  .facts dd {
    font-size: var(--fs-xs);
    text-align: right;
    color: var(--ink);
  }
  .facts dd.big {
    font-size: var(--fs-md);
    font-weight: var(--w-bold);
  }
  .facts .unit {
    color: var(--faint);
    font-size: var(--fs-micro);
    font-weight: var(--w-mid);
  }

  .months {
    border: 1px solid var(--line);
    border-radius: var(--r2);
    overflow: hidden;
  }
  .mhead,
  .mrow {
    display: grid;
    grid-template-columns: 1fr 62px 52px;
    gap: var(--s3);
    align-items: center;
    padding: var(--s3) var(--s5);
    font-size: var(--fs-xs);
    text-align: left;
  }
  .mhead {
    background: var(--bg-2);
    border-bottom: 1px solid var(--line);
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }
  .mrow {
    width: 100%;
    border: 0;
    border-top: 1px solid var(--line-soft);
    background: var(--panel);
    color: var(--ink);
    font-family: inherit;
    cursor: pointer;
  }
  .mrow:first-of-type {
    border-top: 0;
  }
  .mrow:hover {
    background: var(--panel-2);
  }
  .mrow[aria-pressed='true'] {
    background: var(--acc-soft);
    box-shadow: inset 2px 0 0 var(--acc);
  }
  .r {
    text-align: right;
  }
  .mrow .state {
    font-size: var(--fs-micro);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: var(--w-bold);
    color: var(--faint);
  }
  .mrow[aria-pressed='true'] .state {
    color: var(--acc);
  }

  .railnote {
    font-size: var(--fs-micro);
    line-height: 1.6;
    color: var(--faint);
    border-top: 1px solid var(--line-soft);
    padding-top: var(--s4);
  }

  /* ======================================================================
     MOTION — all of it, and only here.
     ----------------------------------------------------------------------
     Same discipline as `theme.css`: every `transition` and `animation` this
     component declares is inside the guard, so an operator who has asked for
     less motion gets a completely static page and nothing has to opt out.
     The `.row-in` / `.fade-in` / `.panel-in` classes used above are the
     theme's, and are already guarded there; the inline `animation-delay` that
     staggers them is inert when the animation does not run.
     ====================================================================== */
  @media (prefers-reduced-motion: no-preference) {
    .chip,
    .hcell,
    .mrow,
    .vrow {
      transition:
        background-color var(--d-hover) var(--ease-out),
        border-color var(--d-hover) var(--ease-out),
        box-shadow var(--d-state) var(--ease-out),
        color var(--d-hover) var(--ease-out);
    }
    /* SELECTING AN INSTRUMENT TRANSITIONS, IT DOES NOT SNAP. The canvas dims
       while the new month is being read and comes back when it is drawn, so
       the eye is told "this is being replaced" instead of finding a different
       chart where the old one was. */
    .canvas {
      transition: opacity var(--d-state) var(--ease-out);
    }
    .canvas.busy {
      opacity: 0.42;
    }
  }
</style>

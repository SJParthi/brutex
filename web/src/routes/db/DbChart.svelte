<script module>
  /**
   * /db — THE CHART PANEL (WP-57; DB-080, RA-175).
   *
   * Candles of the one series the selection names, at any rung the feed can
   * show it at, with an open-interest histogram, the implied volatility and
   * one greek as lines, and the halt, closing-auction, gap and flagged-day
   * overlays — for an index, a share or a contract, on either feed, through
   * one path. The plan (which rung is read from where, one page's query, how
   * pages join, which instants are priced, where a band sits) is
   * `$lib/db/chart.js`; this file draws it and says what it cost.
   *
   * CLOSED UNLESS THE READER OPENED IT. A picture does not outrank the rows
   * on a page whose job is the rows, so the panel is one line until it is
   * asked for, and while it is closed it draws nothing and asks nothing.
   *
   * COST. Opening reads the rung table once and the newest page; a selection
   * of at most CHART_AUTO_ALL bars is then read whole. After that the chart
   * reads ONE page a gesture — a pan to its left edge, or the Load earlier
   * button — and one page is at most PAN_REQUESTS requests, counted here and
   * printed under the chart. It never reads past CHART_KEEP bars: the page
   * that would is refused by name.
   *
   * EVERY FIGURE IS THE SERVER'S. Bars come from `/bars/window.json` (a
   * stored rung) or `/sources/bars.json` (a rung the server folds with
   * `pull::fold`); greeks from `/greeks.json`; halts, auctions and day flags
   * from `/dayflags.json`. The readout prints the paisa integers the routes
   * sent, never a number that has been through the chart library's floats.
   *
   * THE OPERATOR'S WINDOW (D-3338). Every rung on offer is clipped to the
   * reader's window unless the page's 'outside your window' toggle is on;
   * what is held back is said, and with the toggle on the bars drawn outside
   * the window are counted and labelled, in the header and in the readout.
   *
   * THE SPOT RULE (D-3328). Under the opt-in 'last at or before t' the IV
   * note names the rule, counts the points priced from an earlier spot and
   * gives the oldest age; those points are drawn in the warning colour and
   * the readout prints the hovered bar's own spot age.
   */
  import { untrack } from 'svelte';
  import { ask } from '$lib/ask.js';
  import { refusalFrom } from '$lib/refusal.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { fmt } from '$lib/db/notes-labels.js';
  import { paisaText } from '$lib/db/bars-plan.js';
  import { dayflagsUrl } from '$lib/db/dayflags.js';
  import { spotAgeText } from '$lib/db/notes-options.js';
  import { readWindowPage } from '$lib/database-pages.js';
  import { askAgainWhileWritten } from '$lib/being-written.js';
  import { readGreeks, ivText, greekText, GREEK_WHY } from '$lib/greeks.js';
  import { dayLabel, monthLabel, barTimeLabel, istMonth } from '$lib/dates.js';
  import { archiveOf } from './DbDayFlags.svelte';
  import {
    chartSubject, planChoice, rungChoices, sourcesUrl, pageRequest, pageBars, joinBars, keepRefusal, rupees,
    greekPlan, fileGreeks, greekLine, flagSpan, dayBands, holeBands, bandSlots, flagMarkers, tradeDriven, alphaHex,
    costText, BAND_KINDS, GREEK_LINES, CHART_AUTO_ALL, CHART_KEEP, CHART_PAGE, PAN_REQUESTS, DAYFLAGS_PAGE_DAYS,
    windowChoice, outsideBars, isOutside, spotAges, spotRuleText, spotAgeWord, rungStateText
  } from '$lib/db/chart.js';

  /** Where this browser remembers whether the panel is open. */
  const OPEN_KEY = 'brutex.db.chart.open';

  /** Whether the panel was left open; closed when nothing is remembered or storage is refused. */
  function rememberedOpen() {
    try {
      return globalThis.localStorage?.getItem(OPEN_KEY) === '1';
    } catch {
      return false;
    }
  }

  /** A pan this many bars from the oldest held bar asks for the next page. */
  const EDGE_BARS = 24;
  /** The most bands and flagged days the text under the chart lists; the rest are counted. */
  const LISTED = 8;
  /** Pixel heights: the candle pane, the open-interest pane, each line pane. */
  const CANDLE_H = 340;
  const OI_H = 84;
  const LINE_H = 104;

  /**
   * What the panel draws from. Written out so a field keeps its own type:
   * `Object.freeze` on a literal types `key` as `''` and `asof` as `'exact'`.
   * @typedef {{key: string, bars: any[], byT: Map<number, any>, greekAt: Map<number, any>, slots: any[],
   *   bands: any[], markers: any[], holesMore: number, more: number | null, total: number | null,
   *   hasOi: boolean, notes: string[], why: string, fit: number, pages: number, requests: number,
   *   last: {requests: number, ms: number, kind: string} | null, greeks: {asked: number, skipped: number},
   *   rate: any, flagsWhy: string, flagDays: number, asof: 'exact' | 'prev', feedRow: any}} ChartView
   */
  /** What the panel shows before a row set answers. @type {Readonly<ChartView>} */
  const EMPTY = Object.freeze({
    key: '', bars: /** @type {any[]} */ ([]), byT: /** @type {Map<number, any>} */ (new Map()),
    greekAt: /** @type {Map<number, any>} */ (new Map()), slots: /** @type {any[]} */ ([]),
    bands: /** @type {any[]} */ ([]), markers: /** @type {any[]} */ ([]), holesMore: 0,
    more: /** @type {number | null} */ (null), total: /** @type {number | null} */ (null), hasOi: false,
    notes: /** @type {string[]} */ ([]), why: '', fit: 0, pages: 0, requests: 0,
    last: /** @type {{requests: number, ms: number, kind: string} | null} */ (null),
    greeks: { asked: 0, skipped: 0 }, rate: /** @type {any} */ (null), flagsWhy: '', flagDays: 0,
    asof: 'exact', feedRow: /** @type {any} */ (null)
  });
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();

  let open = $state(rememberedOpen());
  /** The rung the reader chose for the chart, or `''` for the selection's own. */
  let rungPick = $state('');
  let greekField = $state('delta');
  let reload = $state(0);
  let busy = $state(false);
  /** The stamp under the crosshair, or `null`. */
  let hover = $state(/** @type {number | null} */ (null));
  let themeTick = $state(0);
  let host = $state(/** @type {HTMLDivElement | null} */ (null));
  let lib = $state.raw(/** @type {any} */ (null));
  let libWhy = $state('');
  /** The server's rung table for the series on screen. */
  let table = $state.raw({ key: '', choices: /** @type {any[]} */ ([]), why: '', loading: false });
  let view = $state.raw(EMPTY);
  let built = $state.raw(/** @type {any} */ (null));

  const feed = $derived(feeds.active ?? '');
  const showing = $derived(db.view === 'bars' && (db.barQueryPlan?.keys.length ?? 0) > 0);
  const subject = $derived(chartSubject(db.barQueryPlan));
  /** The selection's own rung, from the census plan: charted with no rung table. */
  const base = $derived(subject.key ? planChoice(db.barQueryPlan) : null);
  const tableKey = $derived(subject.key && feed ? `${feed}|${sourcesUrl(subject)}` : '');
  /** The active feed's own `/feeds.json` row: where its operator window is written. */
  const feedRow = $derived(db.activeFeed?.wire === feed ? db.activeFeed : null);
  /** Whether the page shows months outside the operator's window (D-3338). */
  const oow = $derived(Boolean(db.oow));
  /**
   * EVERY RUNG ON OFFER, finest first: the selection's own, then the server's
   * others — each bounded by the operator's window unless the reader asked
   * for what lies outside it. What the window holds back is kept, to be said.
   */
  const offered = $derived.by(() => {
    /** @type {any[]} */
    const list = [];
    /** @type {Map<string, string>} */
    const hidden = new Map();
    /** @type {string[]} */
    const unoffered = [];
    /** @type {Set<string>} */
    const outside = new Set();
    if (!base) return { list, hidden, unoffered, outside };
    const rows = table.key === tableKey ? table.choices : [];
    const others = rows.filter((/** @type {any} */ c) => c.rung !== base.rung);
    // THE SELECTION'S OWN RUNG is read from the census plan, and takes how
    // whole it is from the server's row of the same rung once that is read.
    const row = rows.find((/** @type {any} */ c) => c.rung === base.rung && c.source === 'window');
    const own = row ? { ...base, state: row.state, behind: row.behind, derivedFrom: row.derivedFrom } : base;
    for (const c of [own, ...others]) {
      const w = windowChoice(c, feedRow, oow);
      if (!w.choice) {
        unoffered.push(w.hidden);
        continue;
      }
      list.push(w.choice);
      if (w.hidden) hidden.set(w.choice.rung, w.hidden);
      if (w.outside) outside.add(w.choice.rung);
    }
    list.sort((a, b) => a.secs - b.secs);
    return { list, hidden, unoffered, outside };
  });
  const choices = $derived(offered.list);
  const choice = $derived(choices.find((c) => c.rung === rungPick) ?? choices.find((c) => c.rung === base?.rung) ?? null);
  const bounds = $derived(db.barBounds ?? {});
  /** The identity of what the chart reads: series, rung, source, months, bounds, and the grid's own row set. */
  const setKey = $derived(
    open && showing && subject.key && choice && feed
      ? [feed, subject.key, choice.rung, choice.source, choice.firstMonth, choice.lastMonth, choice.firstDay, choice.lastDay,
          bounds.fromDay, bounds.toDay, bounds.fromTime, bounds.toTime, db.barSetKey, db.spotAsof, reload].join('|')
      : ''
  );
  const shapeKey = $derived(`${view.hasOi ? 'oi' : ''}|${subject.option ? 'opt' : ''}|${(choice?.secs ?? 60) < 60 ? 's' : ''}`);
  const hostH = $derived(CANDLE_H + (view.hasOi ? OI_H : 0) + (subject.option ? 2 * LINE_H : 0));

  /** The working set of the row set on screen. Plain, not reactive: `publish` is what the page sees. */
  let set = newSet('');
  /** Whether a reader's gesture is waiting to be spent on one page. */
  let armed = false;
  let fitSeen = -1;

  /** @param {string} key */
  function newSet(key) {
    return {
      key, controller: new AbortController(), held: /** @type {any[]} */ ([]), greekAt: new Map(), days: new Map(),
      more: /** @type {number | null} */ (null), total: /** @type {number | null} */ (null), notes: /** @type {string[]} */ ([]),
      why: '', fit: 0, pages: 0, requests: 0, last: /** @type {any} */ (null), asked: 0, skipped: 0, rate: /** @type {any} */ (null),
      flagsWhy: '', feed: '', subject: /** @type {any} */ (null), choice: /** @type {any} */ (null), bounds: /** @type {any} */ ({}),
      asof: /** @type {'exact' | 'prev'} */ ('exact'), feedRow: /** @type {any} */ (null), started: false, busy: false, pageWhy: ''
    };
  }

  /** @param {boolean} next */
  function setOpen(next) {
    open = next;
    try {
      globalThis.localStorage?.setItem(OPEN_KEY, next ? '1' : '0');
    } catch {
      /* A REFUSED STORE IS A PREFERENCE NOT KEPT, never a broken panel. */
    }
  }

  /** The instant of a bar as the grid prints one. @param {number} t */
  const when = (t) =>
    (choice?.secs ?? 60) >= 86_400 ? dayLabel(t * 1000) : `${dayLabel(t * 1000)} ${barTimeLabel(t * 1000, choice?.secs ?? 60)}`;

  /** WHAT THE PAGE SEES of the working set: one new object a read. */
  function publish() {
    const s = set;
    if (!s.choice) return;
    const secs = s.choice.secs;
    const { bands: named, cells } = dayBands([...s.days.values()], { archive: archiveOf(s.feed) });
    const holes = holeBands(s.held, secs, cells, named.filter((b) => b.kind === 'halt'), tradeDriven(s.subject, secs));
    const all = [...named, ...holes.bands];
    const slots = bandSlots(all, s.held, secs);
    const lo = s.held.length ? s.held[0].t : 0;
    const hi = s.held.length ? s.held[s.held.length - 1].t + secs : 0;
    // A SELECTION THAT HOLDS NO BAR still lists what its days' records say:
    // there is no bar to clip a band to, so every band of the days asked stays.
    const none = s.held.length === 0;
    view = {
      key: s.key, bars: s.held, byT: new Map(s.held.map((b) => [b.t, b])), greekAt: s.greekAt, slots,
      bands: all.filter((b) => none || (b.to > lo && b.from < hi)).sort((a, b) => b.from - a.from),
      markers: flagMarkers(cells, s.held, slots), holesMore: holes.more, more: s.more, total: s.total,
      hasOi: s.held.some((b) => b.oi !== null), notes: [...new Set(s.pageWhy ? [s.pageWhy, ...s.notes] : s.notes)], why: s.why, fit: s.fit, pages: s.pages,
      requests: s.requests, last: s.last, greeks: { asked: s.asked, skipped: s.skipped }, rate: s.rate,
      flagsWhy: s.flagsWhy, flagDays: s.days.size, asof: s.asof, feedRow: s.feedRow
    };
  }

  /**
   * ONE PAGE: its bars, then its greeks, then its day flags — at most
   * PAN_REQUESTS requests, each counted where it is sent.
   * @param {string} kind what asked for it: open, pan, button
   */
  async function loadPage(kind) {
    const s = set;
    if (s.busy || !s.choice || (s.started && s.more === null)) return;
    s.busy = true;
    s.pageWhy = '';
    if (set === s) busy = true;
    const began = performance.now();
    let requests = 0;
    const signal = s.controller.signal;
    /** @type {typeof ask} */
    const counted = (url, options) => {
      requests += 1;
      return ask(url, options);
    };
    try {
      const request = pageRequest(s.feed, s.subject, s.choice, s.bounds, s.started ? s.more : null);
      s.notes.push(...request.notes);
      const refusal = request.why ?? (s.started ? keepRefusal(s.held.length, request.limit) : null);
      if (refusal) {
        if (s.started) s.notes.push(refusal);
        else s.why = refusal;
        s.more = null;
        return;
      }
      /** @type {any} */
      let body;
      if (request.route === '/bars/window.json') {
        // ONE ATTEMPT A PAGE: a busy scan pool (429) is said, not retried here,
        // so a page's request count stays its bound.
        body = await readWindowPage(request.query, request.limit, signal, async (url, options) => {
          const response = await counted(url, options);
          if (response.status === 429) throw new Error(await refusalFrom(request.route, response));
          return response;
        });
      } else {
        // A FOLDED RUNG BESIDE A WRITER ASKS AGAIN (R3-08, RA-117), as the window does.
        const response = await askAgainWhileWritten(counted, `${request.route}?${request.query}`, { cache: 'no-store', signal });
        if (!response.ok) throw new Error(await refusalFrom(request.route, response));
        body = await response.json();
      }
      if (set !== s) return;
      const page = pageBars(s.choice.source, body, s.feed, s.started ? (s.more ?? 0) : 0);
      if (page.total !== null && s.total !== null && page.total !== s.total) {
        s.notes.push(`this selection's bar count changed from ${fmt(s.total)} to ${fmt(page.total)} while the chart was paging, so the store changed under it: press Reload`);
        s.more = null;
        return;
      }
      s.started = true;
      s.total = page.total ?? s.total;
      s.more = page.more;
      s.pages += 1;
      const joined = joinBars(s.held, page.bars);
      s.held = joined.bars;
      if (page.unread > 0) s.notes.push(`${fmt(page.unread)} row(s) of a page carried no whole-paisa prices and are not drawn`);
      if (page.dupes + joined.dupes > 0) s.notes.push(`${fmt(page.dupes + joined.dupes)} row(s) repeated a stamp already drawn and are not drawn twice`);
      if (page.faults) s.notes.push(page.faults);
      publish();

      const plan = greekPlan(page.bars, s.subject, s.choice.source === 'fold' ? s.choice.from : s.choice.rung);
      if (plan.rows.length > 0) {
        const priced = await readGreeks(s.feed, plan.rows, signal, counted, s.asof);
        if (set !== s) return;
        fileGreeks(plan.rows, priced.cells, s.greekAt);
        if (priced.rate) s.rate = priced.rate;
      }
      s.asked += plan.rows.length;
      s.skipped += plan.skipped;

      // A SELECTION WITH NO BAR asks the day flags for its own day bounds, so
      // a session the store holds nothing of is named, not passed over.
      const span = flagSpan(page.bars, s.held.length === 0 ? s.bounds : undefined);
      if (span) {
        try {
          const response = await askAgainWhileWritten(counted, dayflagsUrl(s.feed, s.subject.key, span.from, span.to), { cache: 'no-store', signal });
          if (!response.ok) throw new Error(await refusalFrom('/dayflags.json', response));
          const flags = await response.json();
          if (set !== s) return;
          if (!Array.isArray(flags?.days)) throw new Error('/dayflags.json answered without a days list');
          for (const rec of flags.days) s.days.set(String(rec?.day ?? ''), rec);
          s.flagsWhy = '';
        } catch (why) {
          if (signal.aborted) return;
          s.flagsWhy = `halts, auctions and day flags are not drawn for this page: ${why instanceof Error ? why.message : String(why)}`;
        }
        if (span.clipped) s.notes.push(`day flags are read for the newest ${fmt(DAYFLAGS_PAGE_DAYS)} days of each page; earlier days on a page are not flagged`);
      }
    } catch (why) {
      if (signal.aborted || set !== s) return;
      const said = why instanceof Error ? why.message : String(why);
      if (s.started) s.pageWhy = `the earlier page was not read: ${said} Press Load earlier to ask again.`;
      else s.why = said;
    } finally {
      s.busy = false;
      if (set === s) {
        s.requests += requests;
        s.last = { requests, ms: performance.now() - began, kind };
        busy = false;
        publish();
      }
    }
  }

  /** OPENING: the newest page, then the whole of a selection small enough to read whole. */
  async function openLoad() {
    const s = set;
    await loadPage('open');
    while (set === s && s.choice?.source === 'window' && s.total !== null && s.total <= CHART_AUTO_ALL && s.more !== null) {
      // A PAGE THAT DID NOT ARRIVE ENDS THE AUTO-READ: it is said, and asked
      // again only by the reader, never in a loop.
      const before = s.pages;
      await loadPage('open');
      if (s.pages === before) break;
    }
    if (set !== s) return;
    s.fit += 1;
    publish();
  }

  /** One earlier page, for a pan or the button. @param {string} kind */
  function earlier(kind) {
    if (!busy && view.more !== null) void loadPage(kind);
  }

  /* THE RUNG FOLLOWS THE SELECTION: a new series or feed forgets the rung picked for the last one. */
  $effect(() => {
    void tableKey;
    untrack(() => {
      rungPick = '';
    });
  });

  /* THE RUNG TABLE: one request per series and feed, only while the panel is open. */
  $effect(() => {
    const key = tableKey;
    if (!open || !showing || !key || subject.why) return;
    if (untrack(() => table.key) === key) return;
    const controller = new AbortController();
    table = { key, choices: [], why: '', loading: true };
    const wanted = feed;
    (async () => {
      try {
        const response = await ask(sourcesUrl(subject), { cache: 'no-store', signal: controller.signal });
        if (!response.ok) throw new Error(await refusalFrom('/sources.json', response));
        const answer = rungChoices(await response.json(), wanted);
        if (!controller.signal.aborted) table = { key, choices: answer.choices, why: '', loading: false };
      } catch (why) {
        if (controller.signal.aborted) return;
        table = { key, choices: [], why: `the other rungs are not offered: ${why instanceof Error ? why.message : String(why)}`, loading: false };
      }
    })();
    return () => {
      controller.abort();
      if (untrack(() => table.loading)) table = { key: '', choices: [], why: '', loading: false };
    };
  });

  /* THE READ: a new row set drops the old one's bars and requests, and reads its newest page. */
  $effect(() => {
    const key = setKey;
    untrack(() => {
      set.controller.abort();
      set = newSet(key);
      busy = false;
      armed = false;
      hover = null;
      if (!key) {
        view = EMPTY;
        return;
      }
      set.feed = feed;
      set.subject = subject;
      set.choice = choice;
      set.bounds = { ...bounds };
      set.asof = db.spotAsof === 'prev' ? 'prev' : 'exact';
      set.feedRow = feedRow;
      view = { ...EMPTY, key };
      void openLoad();
    });
    return () => set.controller.abort();
  });

  /* THE LIBRARY: TradingView's lightweight-charts, MIT, bundled; loaded when the panel first opens. */
  $effect(() => {
    if (!open || lib) return;
    let dead = false;
    import('lightweight-charts').then(
      (mod) => {
        if (!dead) lib = mod;
      },
      (why) => {
        if (!dead) libWhy = why instanceof Error ? why.message : String(why);
      }
    );
    return () => {
      dead = true;
    };
  });

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

  /** The theme's own colours, read where they are defined. */
  function tokens() {
    const cs = getComputedStyle(document.documentElement);
    const v = (/** @type {string} */ name) => cs.getPropertyValue(name).trim();
    return {
      up: v('--up'), down: v('--down'), grid: v('--line-soft'), border: v('--line'), text: v('--dim'),
      faint: v('--faint'), acc: v('--acc'), warn: v('--warn'), info: v('--info')
    };
  }

  /* THE CHART: built for the panes this series has, rebuilt when they or the theme change. */
  $effect(() => {
    const el = host;
    const mod = lib;
    const shape = shapeKey;
    void themeTick;
    if (!el || !mod) return;
    const c = tokens();
    const [hasOi, option, seconds] = shape.split('|').map(Boolean);
    const made = mod.createChart(el, {
      autoSize: true,
      layout: {
        background: { color: 'transparent' }, textColor: c.text, attributionLogo: false,
        panes: { separatorColor: c.border, separatorHoverColor: c.acc, enableResize: true }
      },
      grid: { vertLines: { color: c.grid }, horzLines: { color: c.grid } },
      rightPriceScale: { borderColor: c.border, scaleMargins: { top: 0.08, bottom: 0.08 } },
      timeScale: {
        borderColor: c.border, timeVisible: true, secondsVisible: seconds, rightOffset: 2,
        // MAX HAS TO MEAN MAX: the default floor of half a pixel a bar would
        // refuse to fit a whole history in the pane.
        minBarSpacing: 0.02,
        // 0 Year, 1 Month, 2 DayOfMonth, 3 Time, 4 TimeWithSeconds — in IST.
        tickMarkFormatter: (/** @type {unknown} */ time, /** @type {number} */ type) => {
          const ms = Number(time) * 1000;
          if (type <= 1) return monthLabel(istMonth(ms));
          if (type === 2) return dayLabel(ms);
          return barTimeLabel(ms, type === 4 ? 1 : 60);
        }
      },
      localization: {
        timeFormatter: (/** @type {unknown} */ time) =>
          `${dayLabel(Number(time) * 1000)} ${barTimeLabel(Number(time) * 1000, seconds ? 1 : 60)} IST`
      },
      crosshair: {
        mode: 1,
        vertLine: { color: c.faint, labelBackgroundColor: c.acc },
        horzLine: { color: c.faint, labelBackgroundColor: c.acc }
      },
      handleScroll: { mouseWheel: true, pressedMouseMove: true },
      handleScale: { mouseWheel: true, pinch: true }
    });
    const quiet = { priceLineVisible: false, lastValueVisible: false };
    const candle = made.addSeries(mod.CandlestickSeries, {
      upColor: c.up, downColor: c.down, wickUpColor: c.up, wickDownColor: c.down, borderVisible: false,
      priceFormat: { type: 'price', precision: 2, minMove: 0.01 }
    }, 0);
    // THE BANDS: a histogram on a scale of its own that fills the pane, one bar a shaded slot.
    const band = made.addSeries(mod.HistogramSeries, { ...quiet, priceScaleId: 'bands', priceFormat: { type: 'volume' } }, 0);
    band.priceScale().applyOptions({ visible: false, scaleMargins: { top: 0, bottom: 0 } });
    let pane = 1;
    const oi = hasOi
      ? made.addSeries(mod.HistogramSeries, { ...quiet, color: alphaHex(c.acc, 0.55), title: 'OI', priceFormat: { type: 'volume' } }, pane++)
      : null;
    const iv = option
      ? made.addSeries(mod.LineSeries, { ...quiet, color: c.acc, lineWidth: 1, title: 'IV %', priceFormat: { type: 'price', precision: 2, minMove: 0.01 } }, pane++)
      : null;
    const greek = option
      ? made.addSeries(mod.LineSeries, { ...quiet, color: c.info, lineWidth: 1, priceFormat: { type: 'price', precision: 4, minMove: 0.0001 } }, pane++)
      : null;
    // EACH PANE'S SHARE OF THE HEIGHT, as a weight: a pixel height set before
    // the chart has measured its box is renormalised away.
    const shares = [CANDLE_H, ...(hasOi ? [OI_H] : []), ...(option ? [LINE_H, LINE_H] : [])];
    made.panes().forEach((/** @type {any} */ p, /** @type {number} */ i) => p.setStretchFactor(shares[i] ?? LINE_H));
    const marks = mod.createSeriesMarkers(candle, []);
    made.subscribeCrosshairMove((/** @type {{ time?: unknown } | undefined} */ param) => {
      hover = param?.time != null ? Number(param.time) : null;
    });
    // ONE PAGE A GESTURE. The range also moves when data arrives, so a page
    // is asked for only when a reader's own pan, wheel or key brought the
    // oldest held bar into view, and that gesture is spent on it.
    const arm = () => {
      armed = true;
    };
    const onRange = (/** @type {{ from: number, to: number } | null} */ range) => {
      if (!armed || !range || range.from > EDGE_BARS) return;
      armed = false;
      earlier('pan');
    };
    made.timeScale().subscribeVisibleLogicalRangeChange(onRange);
    for (const name of ['pointerdown', 'wheel', 'keydown', 'touchstart']) el.addEventListener(name, arm, { passive: true });
    fitSeen = -1;
    built = {
      chart: made, candle, band, oi, iv, greek, marks,
      colours: { halt: alphaHex(c.warn, 0.2), gap: alphaHex(c.down, 0.2), cas: alphaHex(c.info, 0.18) },
      tones: { bad: c.down, warn: c.warn, info: c.info, quiet: c.faint, ok: c.up }, aged: c.warn
    };
    return () => {
      built = null;
      for (const name of ['pointerdown', 'wheel', 'keydown', 'touchstart']) el.removeEventListener(name, arm);
      try {
        made.remove();
      } catch {
        // Already disposed by a hot reload. Nothing to undo.
      }
    };
  });

  /* THE DATA: every series from the bars held, each time a page is published. */
  $effect(() => {
    const b = built;
    const v = view;
    const field = greekField;
    if (!b) return;
    /** @type {any[]} */
    const candles = [];
    let i = 0;
    for (const bar of v.bars) {
      // AN EMPTY SLOT IS WHITESPACE in the candle series, so the axis has it.
      while (i < v.slots.length && v.slots[i].time < bar.t) candles.push({ time: v.slots[i++].time });
      if (i < v.slots.length && v.slots[i].time === bar.t) i += 1;
      candles.push({ time: bar.t, open: rupees(bar.o), high: rupees(bar.h), low: rupees(bar.l), close: rupees(bar.c) });
    }
    while (i < v.slots.length) candles.push({ time: v.slots[i++].time });
    b.candle.setData(candles);
    b.band.setData(v.slots.map((/** @type {any} */ s) => ({ time: s.time, value: 1, color: b.colours[s.kind] })));
    b.oi?.setData(v.bars.filter((/** @type {any} */ x) => x.oi !== null).map((/** @type {any} */ x) => ({ time: x.t, value: x.oi })));
    // A POINT PRICED FROM AN EARLIER SPOT is drawn in the warning colour (D-3328).
    const drawn = (/** @type {{time: number, value: number, aged?: boolean}[]} */ points) =>
      points.map((p) => (p.aged ? { time: p.time, value: p.value, color: b.aged } : p));
    b.iv?.setData(drawn(greekLine(v.bars, v.greekAt, 'iv').points));
    if (b.greek) {
      b.greek.setData(drawn(greekLine(v.bars, v.greekAt, field).points));
      b.greek.applyOptions({ title: GREEK_LINES.find((g) => g.field === field)?.label ?? field });
    }
    // A MARK NEEDS A CANDLE OR A SLOT: with no bar held the flagged days are listed under the chart only.
    b.marks.setMarkers((v.bars.length === 0 ? [] : v.markers).map((/** @type {any} */ m) => ({
      time: m.time, position: 'aboveBar', shape: 'square', color: b.tones[m.tone] ?? b.tones.quiet, text: m.word
    })));
    if (v.fit !== fitSeen && v.bars.length > 0) {
      fitSeen = v.fit;
      b.chart.timeScale().fitContent();
    }
  });

  const ivLine = $derived(greekLine(view.bars, view.greekAt, 'iv'));
  /** The bar the readout prints: the one under the crosshair, else the newest held. */
  const shown = $derived((hover !== null ? view.byT.get(hover) : null) ?? view.bars[view.bars.length - 1] ?? null);
  const shownGreek = $derived(shown ? (view.greekAt.get(shown.t) ?? null) : null);
  /** Where the hovered bar's spot came from, under the spot rule in force. */
  const shownAge = $derived(spotAgeWord(shownGreek, view.asof));
  /** The spot rule in force and how many points it aged, for the IV note. */
  const spotNote = $derived(spotRuleText(view.asof, spotAges(view.bars, view.greekAt)));
  /** How many of the bars drawn lie outside the operator's window: two probes. */
  const outsideN = $derived(outsideBars(view.bars, view.feedRow));
  /** The operator's window as the notes print it. */
  const windowText = $derived(view.feedRow?.window ? `${view.feedRow.window.from} to ${view.feedRow.window.to}` : '');
  /** What the chosen rung misses, when the server says it is not whole. */
  const stateNote = $derived(rungStateText(choice));
  const summary = $derived(
    view.bars.length === 0
      ? `Chart of ${subject.key || 'the selection'}: no bar is drawn`
      : `Candles of ${subject.key} at ${choice?.rung ?? ''}: ${fmt(view.bars.length)} bar(s) from ${when(view.bars[0].t)} to ${when(view.bars[view.bars.length - 1].t)} IST`
  );
  /** The sentence for a greek reason word, for a title. @param {string | null} why */
  const whyText = (why) => /** @type {Record<string, string>} */ (GREEK_WHY)[why ?? ''] ?? '';
  const bandWord = (/** @type {string} */ kind) => BAND_KINDS.find((k) => k.kind === kind)?.word ?? kind;
  /** How a rung reads in the picker: its name, and where its bars come from. @param {any} c */
  const rungText = (c) =>
    c.source === 'fold'
      ? `${c.rung} — folded from ${c.from || 'the stored rung'} by the server`
      : c.rung === base?.rung
        ? `${c.rung} — this selection`
        : `${c.rung} — ${c.kind || 'stored'}${rungStateText(c) ? `, ${c.state}` : ''}`;
</script>

{#if showing}
  <section class="ch" aria-label="Chart of this selection">
    <div class="chead">
      <h3>Chart</h3>
      {#if !open}
        <span class="csub">hidden; nothing is read or drawn while it is</span>
      {:else if subject.why || !choice}
        <span class="csub"></span>
      {:else}
        <span class="csub" role="status">
          {#if view.why}
            not drawn; choose another rung or press Reload
          {:else if view.bars.length === 0}
            {busy ? 'reading the newest page…' : 'no bar is held for this selection'}{#if !busy && view.bands.length + view.markers.length > 0}; what its days' own records say is listed below{/if}
          {:else}
            {fmt(view.bars.length)}{view.total !== null ? ` of ${fmt(view.total)}` : ''}
            {choice?.source === 'fold' ? 'folded bucket(s)' : 'bar(s)'} drawn,
            {when(view.bars[0].t)} to {when(view.bars[view.bars.length - 1].t)} IST{#if outsideN.n > 0}{' · '}<b class="cwarn">{fmt(outsideN.n)} outside your window</b>{/if}{#if busy} · reading…{/if}
          {/if}
        </span>
        <label class="cpick">
          <span>Rung</span>
          <select class="csel" value={choice?.rung ?? ''} onchange={(e) => (rungPick = e.currentTarget.value)}>
            {#each choices as c (c.rung)}
              <option value={c.rung}>{rungText(c)}</option>
            {/each}
          </select>
        </label>
        {#if subject.option}
          <label class="cpick">
            <span>Greek</span>
            <select class="csel" bind:value={greekField}>
              {#each GREEK_LINES as g (g.field)}
                <option value={g.field}>{g.label}</option>
              {/each}
            </select>
          </label>
        {/if}
        <button class="btn ghost sm" type="button" disabled={busy || view.more === null}
          title="Read the page before the oldest bar drawn: at most {PAN_REQUESTS} requests. Panning the chart to its left edge does the same."
          onclick={() => earlier('button')}>Load earlier</button>
        <button class="btn ghost sm" type="button" disabled={view.bars.length === 0}
          onclick={() => built?.chart.timeScale().fitContent()}>Fit</button>
        <button class="btn ghost sm" type="button" disabled={busy} onclick={() => (reload += 1)}>Reload</button>
      {/if}
      <button class="btn ghost sm" type="button" aria-expanded={open} onclick={() => setOpen(!open)}>{open ? 'Hide' : 'Show chart'}</button>
    </div>
    {#if open}
      {#if subject.why}
        <p class="cwhy" role="status">The chart is not drawn: {subject.why}.</p>
      {:else if !choice}
        <p class="cwhy" role="status">
          The chart is not drawn: {offered.unoffered[0] ?? `the rung ${db.barQueryPlan?.timeframe ?? ''} has no length this page knows, so there is nothing to place on a time axis`}.
        </p>
      {:else if libWhy}
        <p class="cwhy" role="status">The chart is not drawn: the chart library did not load ({libWhy}).</p>
      {:else if view.why}
        <p class="cwhy" role="status">The chart is not drawn: {view.why}</p>
        {#if offered.hidden.get(choice.rung)}<p class="cwhy">{offered.hidden.get(choice.rung)}.</p>{/if}
      {:else}
        <div class="chost" bind:this={host} style:height="{hostH}px" role="img" aria-label={summary}></div>
        {#if shown}
          <p class="cread" aria-live="off">
            <b>{when(shown.t)} IST</b>
            <span>O {paisaText(shown.o)}</span><span>H {paisaText(shown.h)}</span><span>L {paisaText(shown.l)}</span><span>C {paisaText(shown.c)}</span>
            <span>V {fmt(shown.v)}</span>
            {#if shown.oi !== null}<span>OI {fmt(shown.oi)}</span>{/if}
            {#if shown.seconds !== null}<span title="How many stored bars this bucket folds.">{fmt(shown.seconds)} stored bar(s)</span>{/if}
            {#if shown.phase && shown.phase !== 'continuous'}<span class="cphase">{shown.phase}</span>{/if}
            {#if isOutside(shown.t, view.feedRow)}<span class="cwarn" title="This bar's month lies outside your window ({windowText}); it is drawn because ‘outside your window’ is on.">outside your window</span>{/if}
            {#if subject.option}
              {#if shownGreek && shownGreek.why === null}
                <span>IV {ivText(shownGreek.iv)}%</span>
                {#if shownAge}<span class:cwarn={(shownGreek.age ?? 0) > 0} title={spotAgeText(shownGreek)}>{shownAge}</span>{/if}
                {#each GREEK_LINES as g (g.field)}<span>{g.label} {greekText(shownGreek[g.field], g.digits)}</span>{/each}
              {:else if shownGreek}
                <span class="cphase" title={whyText(shownGreek.why)}>IV and greeks: {shownGreek.why}</span>
              {:else}
                <span class="cphase">IV and greeks: not asked for this bar</span>
              {/if}
            {/if}
          </p>
        {/if}
        <ul class="cnotes">
          {#if view.last}
            <li>
              Last read ({view.last.kind}): {costText(view.last.requests, view.last.ms)}. {fmt(view.pages)} page(s) and
              {fmt(view.requests)} request(s) for this chart so far; a page is {fmt(CHART_PAGE)} stored bars or one page of
              folded buckets, and the chart keeps at most {fmt(CHART_KEEP)}.
            </li>
          {/if}
          {#if view.more !== null}
            <li>Earlier bars are not read yet: pan to the left edge or press Load earlier for one more page.</li>
          {/if}
          {#if outsideN.n > 0}
            <li class="cwarn">
              {fmt(outsideN.n)} of the {fmt(view.bars.length)} bar(s) drawn are outside your window ({windowText}):
              {fmt(outsideN.before)} before it opens, {fmt(outsideN.after)} after it closes. They are drawn because
              ‘outside your window’ is on; the readout labels each one.
            </li>
          {:else if offered.outside.has(choice.rung)}
            <li>
              {choice.rung} holds months outside your window ({windowText}) and ‘outside your window’ is on: none of the
              bars drawn so far lie there; an earlier page may, and will be labelled.
            </li>
          {/if}
          {#if offered.hidden.get(choice.rung)}<li>{offered.hidden.get(choice.rung)}.</li>{/if}
          {#each offered.unoffered as said (said)}<li>{said}.</li>{/each}
          {#if stateNote}<li class="cwarn">{stateNote}</li>{/if}
          {#if choice?.source === 'fold'}
            <li>
              {choice.rung} is folded by the server from {choice.from || 'the stored rung'} on request (open first, high max,
              low min, close last, volume summed, open interest last carried); this feed stores no {choice.rung} bar of this series.
              {#if subject.option}IV and greeks are solved at each bucket's first stored bar.{/if}
            </li>
          {/if}
          {#if subject.option && view.greeks.asked + view.greeks.skipped > 0}
            <li>
              IV is drawn for {fmt(ivLine.points.length)} of {fmt(view.bars.length)} bar(s).
              {#each ivLine.whys as [why, n] (why)}
                <span title={whyText(why)}>{`${fmt(n)} not priced: ${why}. `}</span>
              {/each}
              {#if view.greeks.skipped > 0}{fmt(view.greeks.skipped)} instant(s) were not asked: a page asks at most what its requests hold.{/if}
              <span class:cwarn={view.asof === 'prev'}>{spotNote}</span>
              {#if view.rate?.state && view.rate.state !== 'loaded'}The risk-free rate table is {view.rate.state}: {view.rate.error ?? ''}{/if}
            </li>
          {/if}
          {#if table.loading}<li>Reading the rungs this feed can show…</li>{/if}
          {#if table.why}<li class="cwarn">{table.why}</li>{/if}
          {#if view.flagsWhy}<li class="cwarn">{view.flagsWhy}</li>{/if}
          {#each view.notes as note (note)}<li class="cwarn">{note}</li>{/each}
        </ul>
        {#if view.bands.length > 0 || view.markers.length > 0}
          <ul class="cbands" aria-label="Overlays drawn on the chart">
            {#each view.bands.slice(0, LISTED) as b (`${b.kind}|${b.from}`)}
              <li class="cband" data-kind={b.kind} title={b.detail}>
                <i class="cdot" data-kind={b.kind}></i><b>{bandWord(b.kind)}</b> {dayLabel(b.from * 1000)} — {b.label}
              </li>
            {/each}
            {#if view.bands.length > LISTED || view.holesMore > 0}
              <li class="cband">and {fmt(Math.max(0, view.bands.length - LISTED) + view.holesMore)} more shaded band(s)</li>
            {/if}
            {#each view.markers.slice(-LISTED).reverse() as m (m.day)}
              <li class="cband" data-tone={m.tone} title={m.says.join('; ')}>
                <i class="cdot" data-tone={m.tone}></i><b>{m.word}</b> {dayLabel(m.time * 1000)}{m.says.length ? ` — ${m.says[0]}` : ''}
              </li>
            {/each}
            {#if view.markers.length > LISTED}
              <li class="cband">and {fmt(view.markers.length - LISTED)} more flagged day(s) marked on the chart</li>
            {/if}
          </ul>
        {/if}
      {/if}
    {/if}
  </section>
{/if}

<style>
  .ch {
    flex: none;
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: var(--panel-2);
    padding: var(--s3) var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    min-width: 0;
  }
  .chead {
    display: flex;
    align-items: baseline;
    gap: var(--s3) var(--s4);
    flex-wrap: wrap;
  }
  .chead h3 {
    margin: 0;
    font-size: var(--fs-xs);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }
  .csub {
    font-size: var(--fs-xs);
    color: var(--faint);
    flex: 1;
    min-width: 0;
  }
  .cpick {
    display: inline-flex;
    align-items: baseline;
    gap: var(--s2);
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .csel {
    font: inherit;
    font-size: var(--fs-xs);
    color: var(--ink);
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    padding: var(--s1) var(--s2);
    max-width: 18rem;
  }
  .chost {
    width: 100%;
    min-width: 0;
  }
  .cwhy {
    margin: 0;
    font-size: var(--fs-xs);
    color: var(--warn);
  }
  .cread {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: var(--s1) var(--s4);
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-xs);
    color: var(--dim);
  }
  .cread b {
    color: var(--ink);
    font-weight: var(--w-semi);
  }
  .cphase {
    color: var(--warn);
  }
  .cnotes,
  .cbands {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--s1);
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .cwarn {
    color: var(--warn);
  }
  .cband {
    color: var(--dim);
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .cband b {
    font-weight: var(--w-semi);
    color: var(--ink);
  }
  .cdot {
    display: inline-block;
    width: 0.6rem;
    height: 0.6rem;
    margin-right: var(--s2);
    border-radius: 2px;
    background: var(--faint);
  }
  .cdot[data-kind='halt'],
  .cdot[data-tone='warn'] {
    background: var(--warn);
  }
  .cdot[data-kind='gap'],
  .cdot[data-tone='bad'] {
    background: var(--down);
  }
  .cdot[data-kind='cas'],
  .cdot[data-tone='info'] {
    background: var(--info);
  }
  .btn.sm {
    padding: var(--s1) var(--s3);
    font-size: var(--fs-xs);
  }
</style>

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
   * **Two columns are present and REFUSED.** `Δ %` and `Prev Δ %` are the
   * operator's stated ask. They cannot be computed from what the server serves,
   * and computing them from raw stored prices would print fabricated crashes
   * across corporate actions. They are rendered as columns that NAME their own
   * blockage rather than quietly omitted — see `BLOCKED` below and the notice
   * the header opens. `CLAUDE.md` §4: degrade loudly and name the reason.
   */
  import { untrack } from 'svelte';
  import { feeds } from '$lib/feeds.svelte.js';

  /* ======================================================================
     DATA
     ====================================================================== */
  let rows = $state([]);
  let error = $state(null);
  let loading = $state(false);
  let fetchedAt = $state(0);
  let nonce = $state(0); /* bumped by Refresh; the store grows under a pull */

  let filter = $state('');
  let month = $state(''); /* '' = every month */
  let kind = $state(''); /* '' = every kind  */
  let holesOnly = $state(false);
  let sortKey = $state('instrument');
  let desc = $state(false);

  /**
   * A session is 375 one-minute bars, 09:15–15:29 inclusive.
   *
   * NSE's Closing Auction Session moved the close: from 2026-08-03 the
   * continuous session ends at 15:29. Stated because a completeness figure with
   * an unstated denominator is a number nobody can check.
   */
  const BARS_PER_SESSION = 375;

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
  const nf = new Intl.NumberFormat();
  const fmt = (n) => nf.format(n);
  /* FIXED TWO DECIMALS, ALWAYS. Adaptive precision made the digit count change
     from row to row, so the column's decimal point moved and the eye had to
     re-find it on every line. A column of numbers is a column or it is not. */
  const pctText = (p) => `${(p * 100).toFixed(2)}%`;
  const dayText = (d) => (Number.isInteger(d) ? String(d) : d.toFixed(2));
  const clock = (t) => (t ? new Date(t).toLocaleTimeString() : '—');

  $effect(() => {
    const f = feeds.active;
    void nonce; /* read, so Refresh re-runs this effect */
    if (!f) return;
    let dead = false;
    loading = true;
    error = null;
    fetch(`/store.json?feed=${encodeURIComponent(f)}`)
      .then((r) =>
        r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status} from /store.json`))
      )
      .then((j) => {
        if (dead) return;
        rows = Array.isArray(j) ? j : [];
        fetchedAt = Date.now();
        loading = false;
      })
      .catch((why) => {
        if (dead) return;
        // NAMED, NOT SHRUGGED. The operator gets the actual failure and a
        // button; a page that says "no data" over a failed request is the
        // fallback that hides a failure `CLAUDE.md` §4 bans.
        error = String(why && why.message ? why.message : why);
        rows = [];
        loading = false;
      });
    return () => (dead = true);
  });

  // WHERE ELSE THE DATA IS. An empty page that only says "nothing here" makes
  // the operator hunt. This names the feeds that have rows in this store — a
  // fact about the store, and a place to go. It renders no other feed's
  // numbers: one feed is selected and only that one's data is shown.
  let elsewhere = $state([]);
  $effect(() => {
    if (loading || error || rows.length || !feeds.all.length) return;
    let dead = false;
    Promise.all(
      feeds.all.map((f) =>
        fetch(`/store.json?feed=${encodeURIComponent(f.wire)}`)
          .then((r) => (r.ok ? r.json() : []))
          .then((d) => ({ feed: f, any: Array.isArray(d) && d.length > 0 }))
          .catch(() => ({ feed: f, any: false }))
      )
    ).then((all) => {
      if (!dead) elsewhere = all.filter((x) => x.any && x.feed.wire !== feeds.active);
    });
    return () => (dead = true);
  });

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
    const most = new Map();
    for (const r of rows) {
      const seen = most.get(r.month);
      if (seen === undefined || r.rows > seen) most.set(r.month, r.rows);
    }
    return most;
  });

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
   */
  function shortBy(r) {
    return Math.max(0, (monthFull.get(r.month) ?? r.rows) - r.rows);
  }

  /** Whole sessions held, from the bar count. Fractional means a partial day. */
  function sessions(r) {
    return r.rows / BARS_PER_SESSION;
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
      const denom = monthFull.get(r.month) ?? r.rows;
      const cut = r.instrument.lastIndexOf('-');
      const parts = r.instrument.split('-');
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
        sym: cut > 0 ? r.instrument.slice(cut + 1) : r.instrument,
        kind: parts.length > 1 ? parts[1] : '—',
        short,
        days: sessions(r),
        lost: short / BARS_PER_SESSION,
        pct: denom > 0 ? r.rows / denom : 1,
        denom,
        // THREE STATES, NOT TWO. "not full" hides the difference between a
        // late start on one day and two whole trading days that never landed,
        // and the second is the one that needs a re-pull.
        state: short === 0 ? 'full' : short < BARS_PER_SESSION ? 'near' : 'gap'
      };
    })
  );

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

  const index = $derived.by(() => {
    const by = new Map();
    for (const it of deco) {
      const seen = new Set();
      for (const token of [it.instrument, it.sym, it.month]) {
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
    if (!typed) return deco;
    if (typed.length <= MAX_PREFIX) return index.get(typed) ?? [];
    const seed = index.get(typed.slice(0, MAX_PREFIX)) ?? [];
    return seed.filter(
      (r) => r.instrument.startsWith(typed) || r.sym.startsWith(typed) || r.month.startsWith(typed)
    );
  });

  /* ---- facets ---------------------------------------------------------
     A FACET'S OWN SELECTION IS EXCLUDED FROM ITS OWN COUNT, and every other
     selection is applied. Otherwise a toggle promises a number and delivers a
     different one: "INDEX 60" while a month holding 23 of them is selected is
     a count of rows the click cannot produce.

     The month band is the deliberate exception, and only for `holesOnly`: the
     cards are a ROLL-UP, not a filter chip — "2026-07 is 92.9% complete" is a
     statement about the month, and narrowing it to the incomplete rows would
     make every card read 0% complete and say nothing. */
  const byMonth = $derived(month ? textMatched.filter((r) => r.month === month) : textMatched);

  const kinds = $derived.by(() => {
    const c = new Map();
    for (const it of byMonth) c.set(it.kind, (c.get(it.kind) ?? 0) + 1);
    return [...c.entries()].sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1));
  });

  /** Text and kind: the base the month cards roll up. */
  const pool = $derived(kind ? textMatched.filter((r) => r.kind === kind) : textMatched);

  /** Everything except the holes toggle — what that toggle counts against. */
  const scoped = $derived(month ? pool.filter((r) => r.month === month) : pool);
  const holed = $derived(scoped.reduce((a, r) => a + (r.short > 0 ? 1 : 0), 0));

  const matched = $derived(holesOnly ? scoped.filter((r) => r.short > 0) : scoped);

  /* ======================================================================
     THE TWO COLUMNS THAT ARE PRESENT AND REFUSED
     ----------------------------------------------------------------------
     Everything asserted here was measured against the live server on the day
     it was written, and the measurement is quoted so the next reader can
     repeat it rather than trust it.
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

  const BLOCKED = $derived([
    {
      k: 'The endpoint carries no price.',
      v: `/store.json returns exactly four fields per row — instrument, month, timeframe, rows. Checked against the union of keys over all ${fmt(deco.length)} rows currently served, not against one sample. There is no close, no open, no basis point, so no percentage exists to render and none can be derived in the browser.`
    },
    {
      k: 'The affordable alternative is not affordable.',
      v: `/bars.json serves one whole instrument-month per request and takes no batch parameter — measured at 668,251 bytes for 8,250 bars, ${BYTES_PER_BAR.toFixed(1)} bytes per bar. Filling these two columns once means ${fmt(deco.length)} requests and about ${((storeBars * BYTES_PER_BAR) / 1e9).toFixed(1)} GB moved to extract roughly ${fmt(Math.round((deco.length * 16) / 1024))} KB of signal. That is a scan per view, and it breaks the O(1)-per-interaction rule the rest of this page is built on.`
    },
    {
      k: 'A guard is already decided and not yet built.',
      v: 'docs/05-decisions.md D-0018 requires a suspected corporate action to be REFUSED LOUDLY with its date named, never computed. crates/ contains no corporate-action handling and docs/00-charter.md records no verified adjustment source, so an overnight percentage taken across a split would print a fabricated crash — a 10:1 split reads as −89.9% and is indistinguishable from a real move in stored prices.'
    },
    {
      k: 'What would unblock it.',
      v: `store_json emitting first and last close per instrument-month as INTEGER BASIS POINTS (CLAUDE.md §7 — never a float for money), alongside a split-suspicion flag so a refused row can be marked instead of numbered. Indices never split, so the ${fmt(kindCount.get('INDEX') ?? 0)} INDEX rows are safe the day the field exists; the ${fmt(kindCount.get('CASH') ?? 0)} CASH rows stay refused until the guard is real.`
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
    // NO `key`: THESE DO NOT SORT, BECAUSE THEY HOLD NOTHING TO SORT BY. A
    // control that reorders 20,516 identical blanks is a control that lies
    // about having data. The header opens the reason instead.
    { key: null, label: 'Δ %', num: true, blocked: true },
    { key: null, label: 'Prev Δ %', num: true, blocked: true }
  ];

  function txt(a, b) {
    return a < b ? -1 : a > b ? 1 : 0;
  }

  const sorted = $derived.by(() => {
    const k = sortKey;
    const dir = desc ? -1 : 1;
    return [...matched].sort((a, b) => {
      let d = 0;
      if (k === 'rows' || k === 'days') d = a.rows - b.rows;
      else if (k === 'short') d = a.short - b.short;
      else if (k === 'pct') d = a.pct - b.pct;
      else if (k === 'month') d = txt(a.month, b.month);
      else if (k === 'timeframe') d = txt(a.timeframe, b.timeframe);
      else d = txt(a.instrument, b.instrument);
      if (d !== 0) return dir * d;
      // Deterministic tie-break, NOT inverted with the direction: two runs of
      // the same sort produce byte-identical order, so equal values never
      // jitter when the direction is flipped and flipped back.
      return txt(a.instrument, b.instrument) || txt(a.month, b.month);
    });
  });

  function head(key) {
    if (!key) {
      noteOpen = true;
      return;
    }
    if (sortKey === key) desc = !desc;
    else {
      sortKey = key;
      // Open a numeric column on its interesting end: most bars first, biggest
      // hole first — but completeness ascending, because the WORST row is the
      // one being looked for.
      desc = key === 'rows' || key === 'days' || key === 'short';
    }
  }

  function ariaSort(key) {
    if (!key) return undefined;
    return sortKey === key ? (desc ? 'descending' : 'ascending') : 'none';
  }

  /* ======================================================================
     THE SUMMARY, over the MATCHED set — the question actually on screen
     ====================================================================== */
  const total = $derived(matched.reduce((a, r) => a + r.rows, 0));
  const full = $derived(matched.reduce((a, r) => a + (r.short === 0 ? 1 : 0), 0));
  const gaps = $derived(matched.reduce((a, r) => a + r.short, 0));
  const owed = $derived(total + gaps); /* what the matched set would hold if full */
  const coverage = $derived(owed > 0 ? total / owed : 1);
  const monthsIn = $derived.by(() => {
    const s = new Set();
    for (const r of matched) s.add(r.month);
    return [...s].sort();
  });
  const filtered = $derived(Boolean(typed || month || kind || holesOnly));

  /* ---- the month roll-up ---------------------------------------------
     WHICH MONTHS ARE COMPLETE, WHICH HAVE HOLES, HOW BIG. A hole is almost
     never one instrument — it is a session the pull did not finish — so the
     month is the altitude the answer actually lives at. Each card is also the
     filter for that month, because seeing a hole and then having to type its
     name is a page that shows a problem and hides the way in. */
  const monthCards = $derived.by(() => {
    const by = new Map();
    for (const it of pool) {
      let m = by.get(it.month);
      if (!m) {
        by.set(
          it.month,
          (m = {
            month: it.month,
            n: 0,
            bars: 0,
            full: 0,
            missing: 0,
            fullest: monthFull.get(it.month) ?? 0,
            worst: null
          })
        );
      }
      m.n += 1;
      m.bars += it.rows;
      m.missing += it.short;
      if (it.short === 0) m.full += 1;
      if (!m.worst || it.short > m.worst.short) m.worst = it;
    }
    return [...by.values()]
      .map((m) => ({
        ...m,
        sessions: m.fullest / BARS_PER_SESSION,
        pct: m.n > 0 && m.fullest > 0 ? m.bars / (m.n * m.fullest) : 1,
        // THE BAND IS THE MONTH'S COVERAGE, NOT A BAR COUNT. Comparing an
        // aggregate over hundreds of rows against the 375-bar single-row
        // threshold is a category error: it makes every month with more than
        // one short instrument read "gap" whatever its real coverage, so the
        // signal saturates and stops distinguishing 99.9% from 81%.
        state: m.missing === 0 ? 'full' : m.bars >= 0.99 * m.n * m.fullest ? 'near' : 'gap'
      }))
      .sort((a, b) => txt(b.month, a.month)); /* newest first: that is where a pull lands */
  });

  const holeMonths = $derived(monthCards.filter((m) => m.state !== 'full').length);

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
  let flash = $state({});
  const prev = new Map();
  const timers = new Map();
  let seeded = false;

  function mark(name, v) {
    const p = prev.get(name);
    prev.set(name, v);
    if (p === undefined || p === v) return;
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
      const snapshot = [
        ['n', matched.length],
        ['bars', total],
        ['full', full],
        ['gaps', gaps],
        ['cov', Math.round(coverage * 10000)],
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
  const ROW = 32;
  const HEAD = 30; /* the sticky header covers the top of the scroll box */
  const OVER = 6;
  let scroller = $state(null);
  let scrollTop = $state(0);
  let viewportH = $state(600);

  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVER));
  const count = $derived(Math.ceil(viewportH / ROW) + OVER * 2);
  const slice = $derived(sorted.slice(first, first + count));

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
  let openKey = $state(null); /* the row whose detail drawer is open */
  let searchEl = $state(null);
  let drawerEl = $state(null);

  function moveTo(next) {
    const n = sorted.length;
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

  function step(d) {
    moveTo(cursor < 0 ? (d > 0 ? 0 : sorted.length - 1) : cursor + d);
  }

  function openRow(it) {
    if (it) openKey = it.key;
  }

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
        moveTo(sorted.length - 1);
        break;
      case 'Enter':
      case ' ':
        if (cursor >= 0) openRow(sorted[cursor]);
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
  function onWindowKey(e) {
    const t = e.target;
    const typing =
      t instanceof HTMLElement &&
      (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable);
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
    for (const m of openMonths) {
      bars += m.rows;
      missing += m.short;
      if (m.short === 0) complete += 1;
    }
    return { bars, missing, complete, n: openMonths.length };
  });

  $effect(() => {
    if (openKey && drawerEl) drawerEl.querySelector('.dclose')?.focus();
  });

  /* A NEW VIEW STARTS AT THE TOP. Keeping the old offset after a filter
     change lands the operator in the middle of a list they have not seen. */
  let stamp = $state(0);
  $effect(() => {
    void [typed, month, kind, holesOnly, sortKey, desc];
    untrack(() => {
      stamp += 1;
      if (scroller) scroller.scrollTop = 0;
      scrollTop = 0;
      cursor = -1;
    });
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

  const feedName = $derived(feeds.all.find((f) => f.wire === feeds.active)?.display ?? '—');

  function reset() {
    filter = '';
    month = '';
    kind = '';
    holesOnly = false;
  }

  const SKELETON = Array.from({ length: 24 }, (_, i) => i);
</script>

<svelte:window onkeydown={onWindowKey} />

<div class="pane db">
  <div class="pane-head">
    <span class="pane-title">DB · {feedName}</span>

    <input
      class="search dbsearch"
      type="search"
      placeholder="Filter instrument or month"
      aria-label="Filter by instrument or month"
      autocomplete="off"
      spellcheck="false"
      bind:this={searchEl}
      bind:value={filter}
    />
    <kbd class="kbd slash" aria-hidden="true">/</kbd>

    {#if typed}
      <button class="link" onclick={() => (filter = '')}>clear</button>
    {/if}

    <span class="spacer"></span>

    <span class="asof" title="When /store.json was last read for this feed">
      {#if loading && rows.length}
        <span class="dot acc live"></span> refreshing…
      {:else}
        as of {clock(fetchedAt)}
      {/if}
    </span>
    <button
      class="btn ghost sm"
      type="button"
      onclick={() => (nonce += 1)}
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
      <button class="btn primary" onclick={() => (nonce += 1)}>Try again</button>
    </div>
  {:else if loading && rows.length === 0}
    <!-- SKELETONS, SHAPED LIKE WHAT IS COMING. A spinner claims "something is
         happening"; a skeleton claims "a summary, a month band and a table go
         here", which is the useful half of the claim. -->
    <section class="stats" aria-hidden="true">
      {#each [0, 1, 2, 3, 4, 5] as i (i)}
        <div class="stat">
          <span class="k"><span class="skel line" style="width:70%"></span></span>
          <span class="skel" style="width:60%;height:19px"></span>
        </div>
      {/each}
    </section>
    <div class="months" aria-hidden="true">
      {#each [0, 1, 2, 3, 4] as i (i)}
        <!-- `skelcard` neutralises the card's state spine. A placeholder that
             inherits the "complete" colour is a placeholder making a claim. -->
        <div class="mcard skelcard">
          <span class="skel line" style="width:52%"></span>
          <span class="skel line" style="width:74%"></span>
          <span class="skel" style="width:100%;height:6px;margin:6px 0"></span>
          <span class="skel line" style="width:64%"></span>
        </div>
      {/each}
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
  {:else if rows.length === 0}
    <div class="blank fade-in">
      <h2>Nothing stored for {feedName}</h2>
      <p>This feed has never landed a bar in this store.</p>
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
         SUMMARY BEFORE DETAIL
         ================================================================== -->
    <section class="stats" aria-label="Store summary">
      <div class="stat">
        <span class="k">Instrument-months</span>
        {#key flash.n?.n}
          <span
            class="v"
            class:flash-up={flash.n?.dir === 'up'}
            class:flash-down={flash.n?.dir === 'down'}>{fmt(matched.length)}</span
          >
        {/key}
        <span class="n">{filtered ? `of ${fmt(deco.length)} stored` : 'every row in the store'}</span
        >
      </div>

      <div class="stat">
        <span class="k">Bars</span>
        {#key flash.bars?.n}
          <span
            class="v"
            class:flash-up={flash.bars?.dir === 'up'}
            class:flash-down={flash.bars?.dir === 'down'}>{fmt(total)}</span
          >
        {/key}
        <span class="n">{fmt(Math.round(total / BARS_PER_SESSION))} session-equivalents</span>
      </div>

      <div class="stat">
        <span class="k">Complete</span>
        {#key flash.full?.n}
          <span
            class="v"
            class:flash-up={flash.full?.dir === 'up'}
            class:flash-down={flash.full?.dir === 'down'}>{fmt(full)}</span
          >
        {/key}
        <span class="n">of {fmt(matched.length)} · {fmt(matched.length - full)} short</span>
      </div>

      <div class="stat">
        <span class="k">Bars missing</span>
        {#key flash.gaps?.n}
          <span
            class="v"
            class:risk={gaps > 0}
            class:flash-up={flash.gaps?.dir === 'down'}
            class:flash-down={flash.gaps?.dir === 'up'}>{fmt(gaps)}</span
          >
        {/key}
        <span class="n"
          >{fmt(Math.floor(gaps / BARS_PER_SESSION))} whole sessions, vs the fullest instrument each
          month</span
        >
      </div>

      <div class="stat">
        <span class="k">Coverage</span>
        {#key flash.cov?.n}
          <span
            class="v"
            class:flash-up={flash.cov?.dir === 'up'}
            class:flash-down={flash.cov?.dir === 'down'}>{pctText(coverage)}</span
          >
        {/key}
        <span
          class="meter wide"
          data-state={coverage >= 1 ? 'full' : coverage >= 0.99 ? 'near' : 'gap'}
          aria-hidden="true"><span class="fill" style="width:{coverage * 100}%"></span></span
        >
      </div>

      <div class="stat">
        <span class="k">Months</span>
        {#key flash.months?.n}
          <span
            class="v"
            class:flash-up={flash.months?.dir === 'up'}
            class:flash-down={flash.months?.dir === 'down'}>{fmt(monthsIn.length)}</span
          >
        {/key}
        <span class="n">
          {#if monthsIn.length}
            {monthsIn[0]} → {monthsIn[monthsIn.length - 1]} ·
            <span class:risk={holeMonths > 0}>{holeMonths} with holes</span>
          {:else}
            nothing matches
          {/if}
        </span>
      </div>
    </section>

    <!-- ==================================================================
         THE MONTH ROLL-UP. Complete / holed / how big, at a glance, and the
         card is the filter.
         ================================================================== -->
    <div class="months" role="group" aria-label="Months in the store — select one to filter">
      {#each monthCards as m (m.month)}
        <button
          class="mcard"
          type="button"
          data-state={m.state}
          aria-pressed={month === m.month}
          onclick={() => (month = month === m.month ? '' : m.month)}
          title={m.missing === 0
            ? `${m.month}: every one of the ${fmt(m.n)} instruments holds all ${fmt(m.fullest)} bars of the ${dayText(m.sessions)} sessions observed.`
            : `${m.month}: ${fmt(m.n - m.full)} of ${fmt(m.n)} instruments short, ${fmt(m.missing)} bars missing — ${fmt(Math.floor(m.missing / BARS_PER_SESSION))} session-equivalents. Worst: ${m.worst?.instrument} holds ${fmt(m.worst?.rows ?? 0)} of ${fmt(m.fullest)}.`}
        >
          <span class="mtop">
            <span class="mname">{m.month}</span>
            <span class="pip" aria-hidden="true"></span>
          </span>
          <span class="msub"
            >{dayText(m.sessions)} session{m.sessions === 1 ? '' : 's'} observed · {fmt(m.n)} row{m.n ===
            1
              ? ''
              : 's'}</span
          >
          <span class="meter" aria-hidden="true"
            ><span class="fill" style="width:{m.pct * 100}%"></span></span
          >
          <span class="mfoot">
            <span class="mpct">{pctText(m.pct)}</span>
            {#if m.missing === 0}
              <span class="tag">complete</span>
            {:else}
              <span class="mmiss">−{fmt(m.missing)}</span>
              <span class="tag warn">{fmt(m.n - m.full)} short</span>
            {/if}
          </span>
        </button>
      {/each}
      {#if monthCards.length === 0}
        <p class="mnone">No month matches the filter.</p>
      {/if}
    </div>

    <!-- ==================================================================
         FACETS. Derived from the data — a new kind appears the day a row
         carries it, and nothing here is edited.
         ================================================================== -->
    <div class="dbbar">
      <span class="lbl">Kind</span>
      <button class="utab" aria-pressed={kind === ''} onclick={() => (kind = '')}>
        All<span class="n">{fmt(byMonth.length)}</span>
      </button>
      {#each kinds as [k, n] (k)}
        <button class="utab" aria-pressed={kind === k} onclick={() => (kind = kind === k ? '' : k)}>
          {k}<span class="n">{fmt(n)}</span>
        </button>
      {/each}

      <span class="sep" aria-hidden="true"></span>

      <button
        class="utab holes"
        aria-pressed={holesOnly}
        onclick={() => (holesOnly = !holesOnly)}
        title="Show only rows short of the fullest instrument in their month"
      >
        Holes only<span class="n">{fmt(holed)}</span>
      </button>

      {#if month}
        <button class="utab pick" aria-pressed="true" onclick={() => (month = '')}>
          {month} <span class="x" aria-hidden="true">×</span>
        </button>
      {/if}

      <span class="spacer"></span>

      <button
        class="utab whyblocked"
        aria-pressed={noteOpen}
        aria-controls="db-blocked"
        onclick={() => (noteOpen = !noteOpen)}
      >
        Δ % unavailable — why
      </button>

      {#if filtered}
        <button class="link" onclick={reset}>Reset all filters</button>
      {/if}
      <span class="sr-only" aria-live="polite">{fmt(sorted.length)} rows shown</span>
    </div>

    {#if noteOpen}
      <!-- ================================================================
           THE REFUSAL, IN FULL. `CLAUDE.md` §4: degrade loudly and name the
           reason. A column silently missing reads as forgotten; a column that
           states the endpoint, the cost, the decision that blocks it and the
           one server change that would unblock it reads as blocked — and the
           next person does not have to re-derive any of it.
           ================================================================ -->
      <section class="why panel-in" id="db-blocked" aria-label="Why the percentage columns are empty">
        <div class="whyhead">
          <span class="tag info">refused, not forgotten</span>
          <h3>
            <b>Δ %</b> and <b>Prev Δ %</b> cannot be computed from what this store serves today.
          </h3>
          <button class="link" onclick={() => (noteOpen = false)}>close</button>
        </div>
        <ol class="whylist">
          {#each BLOCKED as b (b.k)}
            <li><b>{b.k}</b> <span>{b.v}</span></li>
          {/each}
        </ol>
        <p class="whyfoot">
          Until then this page prints no percentage rather than a plausible one, and green and red
          stay reserved for direction — they appear on a figure that moved between two reads of the
          store, and nowhere else. A missing bar is a severity, not a direction, and wears amber.
        </p>
      </section>
    {/if}

    <!-- ==================================================================
         THE TABLE. Windowed: only the visible slice exists in the DOM.
         ================================================================== -->
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
        aria-rowcount={sorted.length}
        aria-activedescendant={cursor >= 0 ? `dbrow-${cursor}` : undefined}
        aria-label="Stored instrument-months, {fmt(sorted.length)} rows. Arrow keys move, Enter opens."
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
                class:blocked={c.blocked}
                role="columnheader"
                aria-sort={ariaSort(c.key)}
              >
                <button
                  class="sortbtn"
                  type="button"
                  onclick={() => head(c.key)}
                  title={c.blocked
                    ? 'This column holds no data. Click for the endpoint, the cost and the decision that block it.'
                    : `Sort by ${c.label}`}
                >
                  <span>{c.label}</span>
                  {#if c.blocked}
                    <span class="nomark" aria-label="unavailable">n/a</span>
                  {:else}
                    <span class="caret" class:on={sortKey === c.key} class:desc aria-hidden="true"
                    ></span>
                  {/if}
                </button>
              </div>
            {/each}
          </div>

          {#if sorted.length === 0}
            <div class="tnone">
              <h3>No row matches</h3>
              <p>
                {feedName} holds {fmt(deco.length)} instrument-months, and none of them match
                {#if typed}the text “{filter}”{/if}{#if typed && (kind || month || holesOnly)} with {/if}{#if kind}kind
                  <b>{kind}</b>{/if}{#if kind && month}, {/if}{#if month}month <b>{month}</b>{/if}{#if (kind || month) && holesOnly},
                {/if}{#if holesOnly}holes only{/if}.
              </p>
              <button class="btn" onclick={reset}>Clear the filters</button>
            </div>
          {:else}
            <!-- The spacer is the FULL height of the sorted set, so the
                 scrollbar tells the truth about 93,776 rows while 30 of them
                 exist. Rows are absolutely positioned at their own offset —
                 no accumulated rounding down a long list. -->
            <div class="tspace" role="rowgroup" style="height:{sorted.length * ROW}px">
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
                    <span class="cell mono" role="gridcell">{it.month}</span>
                    <span class="cell tf" role="gridcell">{it.timeframe}</span>
                    <span
                      class="cell num"
                      role="gridcell"
                      class:flash-up={moved.get(it.key) === 'up'}
                      class:flash-down={moved.get(it.key) === 'down'}>{fmt(it.rows)}</span
                    >
                    <span class="cell num dimnum" role="gridcell">{dayText(it.days)}</span>
                    <span class="cell num short" role="gridcell">
                      {#if it.short === 0}
                        <span class="okword">full</span>
                      {:else}
                        <span class="missnum">−{fmt(it.short)}</span>
                        {#if it.lost >= 1}
                          <span class="lost">{Math.floor(it.lost)}d</span>
                        {/if}
                      {/if}
                    </span>
                    <span class="cell comp" role="gridcell">
                      <span class="meter" aria-hidden="true"
                        ><span class="fill" style="width:{it.pct * 100}%"></span></span
                      >
                      <span class="cpct">{pctText(it.pct)}</span>
                    </span>
                    <!-- NOT A DASH AND NOT A ZERO. A dash reads as "nothing
                         happened"; a zero is a lie. The hatched band says the
                         column is dead ground, and the header says why. -->
                    <span class="cell num dead" role="gridcell" aria-label="unavailable">—</span>
                    <span class="cell num dead" role="gridcell" aria-label="unavailable">—</span>
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
              <span class="k">Complete</span><span class="v">{fmt(openTotals.complete)}</span>
            </div>
            <div class="dstat">
              <span class="k">Bars</span><span class="v">{fmt(openTotals.bars)}</span>
            </div>
            <div class="dstat">
              <span class="k">Short by</span
              ><span class="v" class:risk={openTotals.missing > 0}>{fmt(openTotals.missing)}</span>
            </div>
          </div>

          <div class="dlist">
            {#each openMonths as m (m.key)}
              <div class="drow" data-state={m.state} class:this={m.key === openKey}>
                <span class="dm">{m.month}</span>
                <span class="meter" aria-hidden="true"
                  ><span class="fill" style="width:{m.pct * 100}%"></span></span
                >
                <span class="dpct">{pctText(m.pct)}</span>
                <span class="dshort"
                  >{#if m.short === 0}full{:else}−{fmt(m.short)}{/if}</span
                >
              </div>
            {/each}
          </div>

          <p class="dfoot">
            Each month is measured against the fullest instrument stored for that month —
            {openRowData.month} held {fmt(openRowData.denom)} bars at its fullest, so this row's {fmt(
              openRowData.rows
            )} is {pctText(openRowData.pct)} of the month.
          </p>
        </div>
      {/if}
    </div>

    <p class="foot">
      <b>{fmt(sorted.length)}</b> rows shown · <b>{slice.length}</b> in the DOM ·
      <kbd class="kbd">↑</kbd><kbd class="kbd">↓</kbd> move,
      <kbd class="kbd">Enter</kbd> opens, <kbd class="kbd">/</kbd> filters · a month's denominator is
      the fullest instrument stored for it, so a session NSE never held is never counted as missing —
      and a whole trading day that never landed always is.
    </p>
  {/if}
</div>

<style>
  /* Everything here is expressed through `theme.css` tokens. No literal colour,
     no literal font, no second theme system — and every `transition` and
     `animation` sits inside the reduced-motion guard, the same discipline the
     theme file holds itself to. */

  .db {
    --dbrow: 32px;
  }

  .dbsearch {
    max-width: 20rem;
  }

  /* SIX TILES, NEVER FIVE AND AN ORPHAN. `theme.css` lays the strip out with
     `auto-fit` + `minmax(150px, 1fr)`, which at a ~820px pane fits five and
     drops the sixth onto a row of its own beside four columns of empty. Six
     divides by three and by two, so the count is stated at the widths where
     `auto-fit` would guess wrong. The token vocabulary is unchanged and
     `theme.css` is untouched. */
  @media (max-width: 1180px) {
    .stats {
      grid-template-columns: repeat(3, minmax(0, 1fr));
    }
  }
  @media (max-width: 640px) {
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
  .slash {
    margin-left: calc(-1 * var(--s6));
    opacity: 0.55;
    pointer-events: none;
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

  .btn.sm {
    padding: var(--s2) var(--s4);
    font-size: var(--fs-xs);
  }

  .loadnote {
    padding: var(--s4) var(--s5);
    border-top: 1px solid var(--line);
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
     refused percentage columns they are reserved for. Everything about DATA
     QUALITY — short, near, gap — is a severity and uses `--warn` at two
     intensities. `.risk` is that hue with a name, so the intent is legible at
     the call site and cannot be mistaken for a direction. */
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
  .meter.wide {
    margin-top: var(--s2);
  }
  .meter .fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    border-radius: var(--r-full);
    background: var(--n7);
  }
  [data-state='full'] .meter .fill,
  .meter[data-state='full'] .fill {
    background: var(--n7);
  }
  [data-state='near'] .meter .fill,
  .meter[data-state='near'] .fill {
    background: color-mix(in srgb, var(--warn) 55%, var(--n7));
  }
  [data-state='gap'] .meter .fill,
  .meter[data-state='gap'] .fill {
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

  /* ---- the month roll-up --------------------------------------------- */
  .months {
    display: flex;
    gap: var(--s4);
    padding: var(--s5);
    overflow-x: auto;
    border-bottom: 1px solid var(--line);
    background: var(--bg);
    flex: none;
  }
  .mcard {
    display: flex;
    flex-direction: column;
    gap: 3px;
    flex: none;
    width: 13.5rem;
    text-align: left;
    padding: var(--s4) var(--s5);
    border: 1px solid var(--line);
    border-left: 3px solid var(--n7);
    border-radius: var(--r3);
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    cursor: pointer;
  }
  .mcard.skelcard {
    border-left-color: var(--line);
    cursor: default;
  }
  .mcard[data-state='near'] {
    border-left-color: color-mix(in srgb, var(--warn) 55%, var(--n7));
  }
  .mcard[data-state='gap'] {
    border-left-color: var(--warn);
  }
  .mcard:hover {
    border-color: var(--line-hard);
    background: var(--panel-2);
  }
  .mcard[aria-pressed='true'] {
    background: var(--acc-soft);
    border-color: var(--acc);
    box-shadow: var(--e1);
  }
  .mcard .mtop {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
  }
  .mcard .mname {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-md);
    font-weight: var(--w-bold);
    letter-spacing: -0.2px;
  }
  .mcard .msub {
    font-size: var(--fs-mini);
    color: var(--faint);
    white-space: nowrap;
  }
  .mcard .meter {
    margin: var(--s2) 0 3px;
  }
  .mcard .mfoot {
    display: flex;
    align-items: center;
    gap: var(--s3);
    font-size: var(--fs-xs);
  }
  .mcard .mpct {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-weight: var(--w-bold);
  }
  .mcard .mmiss {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    color: var(--warn);
  }
  .mnone {
    color: var(--faint);
    font-size: var(--fs-sm);
    padding: var(--s4);
  }

  /* ---- the facet bar -------------------------------------------------- */
  .dbbar {
    display: flex;
    align-items: center;
    gap: var(--s2);
    flex-wrap: wrap;
    padding: var(--s3) var(--s5);
    border-bottom: 1px solid var(--line);
    background: var(--bg-2);
    flex: none;
  }
  .dbbar .lbl {
    margin-right: var(--s2);
  }
  .dbbar .sep {
    width: 1px;
    height: 16px;
    background: var(--line);
    margin: 0 var(--s3);
  }
  .dbbar .spacer {
    flex: 1;
  }
  .holes[aria-pressed='true'] {
    background: var(--warn-soft);
    border-color: var(--warn);
    color: var(--warn);
  }
  .pick[aria-pressed='true'] {
    background: var(--acc-soft);
    border-color: var(--acc);
  }
  .pick .x {
    opacity: 0.7;
    margin-left: var(--s2);
  }
  /* THE BLOCKED COLUMNS GET A PERMANENT DOOR, not a tooltip. `--info` because
     this is neither an error nor a warning about the data: it is a statement
     about the SURFACE, and it must not be mistaken for either. */
  .whyblocked {
    border-color: var(--info);
    color: var(--info);
  }
  .whyblocked[aria-pressed='true'] {
    background: var(--info-soft);
  }

  /* ---- the refusal notice --------------------------------------------- */
  .why {
    flex: none;
    padding: var(--s5) var(--s6);
    border-bottom: 1px solid var(--line);
    background: var(--panel);
    box-shadow: inset 3px 0 0 var(--info);
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
  .tbl {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    position: relative; /* the drawer's containing block */
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
  .tbl-inner {
    min-width: 1044px;
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
      minmax(150px, 1fr) 96px 96px;
    align-items: center;
  }
  .th,
  .cell {
    padding-right: var(--s5);
  }
  .th.inst,
  .cell.inst {
    padding-left: var(--s5);
  }

  .thead {
    position: sticky;
    top: 0;
    z-index: 3;
    height: 30px;
    background: var(--bg-2);
    border-bottom: 1px solid var(--line);
  }
  .th {
    font-size: var(--fs-mini);
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
  .cell.inst {
    position: sticky;
    left: 0;
    z-index: 2;
    background: inherit;
  }
  .th.inst {
    background: var(--bg-2);
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
  .cell.inst::after {
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
  /* A BLOCKED HEADER DOES NOT WEAR A SORT CARET. Offering the affordance and
     then not sorting is worse than not offering it. */
  .th.blocked {
    color: var(--info);
  }
  .nomark {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: 0;
    text-transform: none;
    padding: 0 3px;
    border: 1px solid currentColor;
    border-radius: 3px;
    opacity: 0.85;
    flex: none;
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
  .cell.inst {
    box-shadow: inset 3px 0 0 transparent;
  }
  [data-state='near'] .cell.inst {
    box-shadow: inset 3px 0 0 color-mix(in srgb, var(--warn) 45%, transparent);
  }
  [data-state='gap'] .cell.inst {
    box-shadow: inset 3px 0 0 var(--warn);
  }

  .cell {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* EVERY NUMERIC COLUMN IS TABULAR AND RIGHT-ALIGNED — no exceptions, so a
     digit in one row sits over the same digit in the next. */
  .cell.num,
  .th.numh {
    font-variant-numeric: tabular-nums;
  }
  .cell.num {
    font-family: var(--mono);
    text-align: right;
  }
  .cell.mono,
  .cell.tf {
    font-family: var(--mono);
    color: var(--dim);
  }
  .cell.dimnum {
    color: var(--dim);
  }
  /* DEAD GROUND, DRAWN AS DEAD GROUND. A hatched band over the whole column
     is legible at a glance in a way thirty em-dashes are not: it reads as
     "there is nothing here", which is exactly the claim being made. */
  .cell.dead {
    color: var(--faint);
    opacity: 0.65;
    background-image: repeating-linear-gradient(
      135deg,
      transparent 0 5px,
      color-mix(in srgb, var(--info) 22%, transparent) 5px 6px
    );
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

  .foot {
    flex: none;
    padding: var(--s3) var(--s5);
    border-top: 1px solid var(--line);
    background: var(--bg-2);
    color: var(--faint);
    font-size: var(--fs-xs);
  }
  .foot b {
    color: var(--ink-2);
    font-variant-numeric: tabular-nums;
  }
  .foot .kbd,
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
    .mcard,
    .sortbtn {
      transition:
        background-color var(--d-hover) var(--ease-out),
        border-color var(--d-hover) var(--ease-out),
        color var(--d-hover) var(--ease-out),
        opacity var(--d-hover) var(--ease-out),
        box-shadow var(--d-state) var(--ease-out);
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
    .cell.inst::after {
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
</style>

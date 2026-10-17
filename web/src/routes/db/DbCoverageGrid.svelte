<script module>
  /**
   * /db — THE COVERAGE GRID
   *
   * One row per stored instrument-month — and one per month a series does not
   * hold, between its first and its last — windowed; the drill-through drawer;
   * and the % change notes panel (`notesPanel`, rendered by the shell above
   * the view split).
   *
   * # Judged against the calendar of record (WP-54, D-3336)
   *
   * Every row used to be measured against the fullest stored series at the
   * same month and rung (DB-014, DB-G34, DB-G63). Each is now measured by
   * `$lib/completeness.js` `judgeRow` against the bars `/calendar.json` says
   * the month owes at the row's rung: sessions times what each owes, a
   * special session by its windows, a halted minute neither owed nor missing,
   * a CAS-eligible share's auction day at 360 (D-3320); a one-second share or
   * an option is trade-driven and owes no count. A hole every series shares
   * is a "store-wide hole", never "full" (DB-012); a month a series does not
   * hold is drawn as an absent row (DB-012, DB-053); a derived rung behind or
   * short of its 1min source says so (DB-013, DB-G35); a month outside the
   * operator's window is shaded (DB-052).
   *
   * Owner: WP-54 (coverage, coverage calendar and day flags).
   */
  import {
    denomKey,
    judgeRow,
    rollUpMonths,
    absentMonths,
    segmentOfKey,
    istDayOf,
    isTradeDriven,
    peersAtMost
  } from '$lib/completeness.js';
  import { calendarUrl, readLedger, ledgerSpan, ledgerDay, monthSpan, epochDayOfIso, isoOfEpochDay } from '$lib/calendar-owed.js';
  import { OVER, heldWindow, contractLife } from '$lib/db/decoration.js';
  import { txt, fmt, tfCmp, TF_SECS } from '$lib/db/notes-labels.js';
  import {
    bytesText,
    BYTES_PER_BAR,
    COLS,
    whyText,
    noteText,
    STATE_WORD,
    coverPct,
    coverWindow,
    monthInWindow,
    windowText,
    windowEmptyWhy,
    coverTotals,
    missingText
  } from '$lib/db/notes-coverage.js';
  import { untrack } from 'svelte';
  import { ROW, HEAD } from '$lib/db/bars-plan.js';
  import { dayLabel, monthLabel } from '$lib/dates.js';
  import { dirOf, bpsText, basisPoints } from '$lib/bps.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { ask } from '$lib/ask.js';
  import { fetchAdmitted } from '$lib/admitted-fetch.js';
  import { createPageRequests } from '$lib/page-requests.js';
  import { dayflagsUrl } from '$lib/db/dayflags.js';
  import DbCalendar from './DbCalendar.svelte';
  /** @typedef {import('$lib/store.svelte.js').StoreRow} StoreRow */
  /** @typedef {import('$lib/db/decoration.js').DecoRow} DecoRow */
  /** @typedef {import('$lib/completeness.js').Cover} Cover */

  /** The rungs Zerodha folds from its 1min bars when `/feeds.json` names none. */
  const DERIVED_FALLBACK = ['2min', '3min', '5min', '10min', '15min', '30min', '60min'];

  /**
   * One absent month of a series, shaped like a decorated row so the grid,
   * the sort and the drawer read it as one: zero bars, no stamps, no change,
   * and `absent: true`.
   *
   * @param {any} like a stored row of the same series
   * @param {string} month
   */
  function absentRow(like, month) {
    return {
      ...like,
      month,
      rows: 0,
      first_ts: null,
      last_ts: null,
      firstAt: null,
      lastAt: null,
      chg: null,
      chgWhy: 'absent',
      prevChg: null,
      prevChgWhy: 'absent',
      chg_bps: null,
      chg_why: 'absent',
      prev_chg_bps: null,
      prev_chg_why: 'absent',
      key: `${like.instrument}\u0000${month}\u0000${like.timeframe}`,
      absent: true
    };
  }

  /**
   * THE COVERAGE GRID — its calendar ledgers, its judged rows, its sort, its
   * windowed rows, the drawer, the counted totals, and the % change notes.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function coverageState(db) {
    let sortKey = $state('instrument');
    let desc = $state(false);

    /* ======================================================================
       THE CALENDAR OF RECORD, ONE LEDGER PER (FEED, RUNG, SERIES)
       ----------------------------------------------------------------------
       `/calendar.json?feed&rung[&symbol&segment]` answers the dense day
       string and the bars each kind of day owes at that rung (D-3336); asked
       for one series, it also carries that series' own held days. A ledger
       is read once per URL and kept until the census is read again; a
       refusal is kept with its reason and named on every row it judges.
       ====================================================================== */
    /** @type {Map<string, { ledger: any, why: string }>} */
    let ledgers = $state.raw(new Map());
    let ledgersFor = $state(0);
    const ledgerReads = createPageRequests();

    /** The rungs this feed folds from its 1min bars (D-3000). */
    const derivedRungs = $derived(
      new Set(/** @type {any} */ (db.activeFeed)?.derived_rungs ?? (feeds.active === 'zerodha' ? DERIVED_FALLBACK : []))
    );

    /**
     * THE ONE SERIES THE SELECTION IS ABOUT, when it is one: spot (an index
     * or a share), not a contract — `/calendar.json` reads a series by
     * exchange, segment and symbol and holds no contract's days. `null` when
     * the matched rows span series, and the exchange's calendar judges them.
     */
    const oneSeries = $derived.by(() => {
      /** @type {{ symbol: string, segment: string } | null} */
      let one = null;
      for (const r of db.matched) {
        const seg = segmentOfKey(r.instrument);
        if (seg === 'FNO') return null;
        const symbol = r.underlying ?? '';
        if (one === null) one = { symbol, segment: seg };
        else if (one.symbol !== symbol || one.segment !== seg) return null;
      }
      return one;
    });

    /**
     * The EXCHANGE ledger URL a row is judged with: what each day owes at
     * its rung, over the feed's whole span (D-3336).
     *
     * @param {{ instrument: string, timeframe: string, underlying?: string }} r
     */
    function ledgerUrlOf(r) {
      return calendarUrl(feeds.active ?? '', r.timeframe, null);
    }

    /**
     * The row's OWN series ledger URL, whose held digits count the days it
     * holds, or `null` when the selection is not that one series. Its span is
     * the series' own, so it never decides what is owed: a series that stops
     * short would otherwise read full.
     *
     * @param {{ instrument: string, timeframe: string, underlying?: string }} r
     */
    function heldUrlOf(r) {
      const series = oneSeries;
      const own = series !== null && series.symbol === (r.underlying ?? '') && series.segment === segmentOfKey(r.instrument);
      return own ? calendarUrl(feeds.active ?? '', r.timeframe, series) : null;
    }

    /** Every ledger the coverage view needs now: each matched rung, and 1min for a derived rung's source. */
    const wanted = $derived.by(() => {
      if (db.view !== 'census' || !feeds.active) return [];
      const urls = new Set();
      for (const r of db.matched) {
        for (const at of derivedRungs.has(r.timeframe) ? [r, { ...r, timeframe: '1min' }] : [r]) {
          urls.add(ledgerUrlOf(at));
          const own = heldUrlOf(at);
          if (own) urls.add(own);
        }
      }
      return [...urls];
    });

    /* A CENSUS READ IS A NEW STORE: the held days may have moved, so every
       ledger is read again. */
    $effect(() => {
      void db.fetchedAt;
      void feeds.active;
      untrack(() => {
        ledgerReads.cancel();
        ledgers = new Map();
        ledgersFor += 1;
      });
    });
    $effect(() => {
      const need = wanted;
      void ledgersFor;
      const missing = need.filter((url) => !untrack(() => ledgers).has(url));
      if (missing.length === 0) return;
      void ledgerReads.run(async (ticket) => {
        for (const url of missing) {
          if (!ticket.current()) return;
          const read = await readLedger(url, ask, ticket.signal);
          if (!ticket.current()) return;
          ledgers = new Map(ledgers).set(url, read);
        }
      });
    });
    $effect(() => () => ledgerReads.dispose());

    /* ======================================================================
       EVERY SERIES' SPAN, FROM THE WHOLE CENSUS
       ----------------------------------------------------------------------
       First and last month and day of each (instrument, rung) the store
       holds, and the months it holds. Built once per census read, O(rows):
       a series' first month is measured from its first bar, and the months
       between its two ends it does not hold are drawn absent.
       ====================================================================== */
    /* ON EITHER TAB (R3-04): the Coverage tab's count is read on the Bars
       tab too, and it counts the absent months this index finds. One pass
       per census read, as before; no request. */
    const seriesIndex = $derived.by(() => {
      /** @type {Map<string, { first: string, last: string, firstDay: number | null, lastDay: number | null, months: Set<string>, like: any }>} */
      const by = new Map();
      for (const r of db.deco) {
        const key = `${r.instrument}\u0000${r.timeframe}`;
        let s = by.get(key);
        if (!s) {
          s = { first: r.month, last: r.month, firstDay: null, lastDay: null, months: new Set(), like: r };
          by.set(key, s);
        }
        s.months.add(r.month);
        if (r.month < s.first) s.first = r.month;
        if (r.month > s.last) s.last = r.month;
        if (r.firstAt !== null) {
          const d = istDayOf(r.firstAt);
          if (s.firstDay === null || d < s.firstDay) s.firstDay = d;
        }
        if (r.lastAt !== null) {
          const d = istDayOf(r.lastAt);
          if (s.lastDay === null || d > s.lastDay) s.lastDay = d;
        }
      }
      return by;
    });

    /**
     * HOW FAR THE STORE REACHES AT EACH RUNG: the last day any series holds
     * a bar at it (GDFL's archive ends 2026-09-24). A month is owed only to
     * there, and a series short of it in that month owes the rest.
     */
    const reach = $derived.by(() => {
      /** @type {Map<string, number>} */
      const by = new Map();
      for (const s of seriesIndex.values()) {
        if (s.lastDay === null) continue;
        const tf = s.like.timeframe;
        if (!by.has(tf) || /** @type {number} */ (by.get(tf)) < s.lastDay) by.set(tf, s.lastDay);
      }
      return by;
    });

    /** Every census row by its key, for a derived rung's 1min source: a probe, not a scan. */
    const byKey = $derived.by(() => {
      /** @type {Map<string, DecoRow>} */
      const m = new Map();
      if (db.view !== 'census') return m;
      for (const it of db.deco) m.set(it.key, it);
      return m;
    });

    const observed = $derived(db.prepared.input === db.rows ? db.prepared.body?.observed ?? null : null);

    /**
     * THE BAR COUNTS OF EVERY CALENDAR-OWED SERIES AT EACH (MONTH, RUNG),
     * sorted, so "how many series hold no more than this row" is one binary
     * search (`peersAtMost`): the corroboration that names a store-wide hole
     * (DB-012). Trade-driven series are left out: they owe no count. Built
     * once per census read, O(rows log rows).
     */
    const peerRows = $derived.by(() => {
      /** @type {Map<string, number[]>} */
      const by = new Map();
      if (db.view !== 'census') return by;
      for (const r of db.deco) {
        if (isTradeDriven(r.timeframe, segmentOfKey(r.instrument), r.strike)) continue;
        const k = denomKey(r.month, r.timeframe);
        let a = by.get(k);
        if (!a) by.set(k, (a = []));
        a.push(r.rows);
      }
      for (const a of by.values()) a.sort((x, y) => x - y);
      return by;
    });

    /**
     * Judge one row against its ledger.
     *
     * @param {any} r a decorated row or an absent one
     * @returns {Cover}
     */
    function coverOf(r) {
      const read = ledgers.get(ledgerUrlOf(r));
      const ownUrl = heldUrlOf(r);
      const own = ownUrl ? ledgers.get(ownUrl)?.ledger ?? null : null;
      const s = seriesIndex.get(`${r.instrument}\u0000${r.timeframe}`);
      const at = denomKey(r.month, r.timeframe);
      const derived = derivedRungs.has(r.timeframe);
      /** @type {any} */
      let source = null;
      if (derived) {
        const one = byKey.get(`${r.instrument}\u0000${r.month}\u00001min`);
        if (one) source = { rows: one.rows, lastAt: one.lastAt, cover: coverOf(one) };
      }
      const secs = TF_SECS.get(r.timeframe);
      return judgeRow(r, {
        ledger: read?.ledger ?? null,
        ledgerWhy: read ? read.why : 'reading the calendar of record…',
        held: own,
        storeLastDay: reach.get(r.timeframe) ?? null,
        seriesFirstDay: s?.firstDay ?? null,
        seriesLastDay: s?.lastDay ?? null,
        fullest: observed?.fullest.get(at) ?? null,
        support: observed?.support.get(at) ?? 0,
        peers: peerRows.has(at) ? peersAtMost(/** @type {number[]} */ (peerRows.get(at)), r.rows) : null,
        window: /** @type {any} */ (db.activeFeed)?.window ?? null,
        archive: /** @type {any} */ (db.activeFeed)?.archive ?? null,
        derived,
        source,
        rungMs: secs === undefined ? 0 : secs * 1000,
        // A SHARE'S DATED AUCTION FLAG IS NOT ON THE CENSUS WIRE: unknown, so
        // a count between the two auction-day counts is never called full
        // (SC-CLOSE-17).
        casFlag: null
      });
    }

    /**
     * The months one series does not hold between its two ends, as absent
     * rows — and, for a derived rung, the months its 1min source holds that
     * it does not ("derivation pending", DB-013). Months outside the
     * operator's window are left out unless the window toggle shows them.
     *
     * @param {string} instrument
     * @param {string} timeframe
     */
    function absentOf(instrument, timeframe) {
      const s = seriesIndex.get(`${instrument}\u0000${timeframe}`);
      return s ? absentMonthsOf(instrument, timeframe).map((m) => absentRow(s.like, m)) : [];
    }

    /**
     * The absent months of one series as `YYYY-MM`, before any row is built:
     * what `absentOf` draws and what the Coverage tab's count counts.
     *
     * @param {string} instrument
     * @param {string} timeframe
     */
    function absentMonthsOf(instrument, timeframe) {
      const s = seriesIndex.get(`${instrument}\u0000${timeframe}`);
      if (!s) return [];
      let first = s.first;
      let last = s.last;
      if (derivedRungs.has(timeframe)) {
        const one = seriesIndex.get(`${instrument}\u00001min`);
        if (one) {
          if (one.first < first) first = one.first;
          if (one.last > last) last = one.last;
        }
      }
      const win = /** @type {any} */ (db.activeFeed)?.window ?? null;
      return absentMonths(first, last, s.months).filter(
        (m) => db.oow || !win || (m >= win.from.slice(0, 7) && m <= win.to.slice(0, 7))
      );
    }

    /* ======================================================================
       THE ROWS COVERAGE LISTS, AND THEIR COUNT ON EITHER TAB (R3-04)
       ----------------------------------------------------------------------
       The Coverage tab's count read the matched rows on the Bars tab and the
       judged rows on Coverage — matched plus absent months — so one tab read
       4 and then 7 for one selection. And the list ignored the From and To
       days the Bars grid applies (DB-046), so a window that excludes a
       contract emptied Bars and left Coverage full. Both read this now: the
       matched rows and every absent month of each matched series, each in a
       month the window touches. Absent months are listed only when no single
       month is chosen, since a chosen month is already a narrowing.
       ====================================================================== */
    /** The From and To days, ordered, when they narrow; `null` when not. */
    const covWindow = $derived(coverWindow(db.fromDay ?? '', db.toDay ?? '', Boolean(db.dayNarrowing)));
    const listedStored = $derived(db.matched.filter((/** @type {any} */ r) => monthInWindow(r.month, covWindow)));
    const listedAbsent = $derived.by(() => {
      /** @type {{ like: any, month: string }[]} */
      const out = [];
      if (db.month) return out;
      const seen = new Set();
      for (const r of db.matched) {
        const key = `${r.instrument}\u0000${r.timeframe}`;
        if (seen.has(key)) continue;
        seen.add(key);
        const s = seriesIndex.get(key);
        if (!s) continue;
        for (const m of absentMonthsOf(r.instrument, r.timeframe)) {
          if (monthInWindow(m, covWindow)) out.push({ like: s.like, month: m });
        }
      }
      return out;
    });
    /** How many rows Coverage lists: its tab's count on either tab, and its pager's. */
    const covCount = $derived(listedStored.length + listedAbsent.length);

    /** THE ROWS THE GRID JUDGES: the listed rows, each with its verdict. */
    const judged = $derived.by(() => {
      if (db.view !== 'census') return [];
      /** @type {any[]} */
      const out = [];
      for (const r of listedStored) out.push({ ...r, cover: coverOf(r) });
      for (const a of listedAbsent) {
        const row = absentRow(a.like, a.month);
        out.push({ ...row, cover: coverOf(row) });
      }
      return out;
    });

    /** The selection's held days, first and last, from the census stamps: what an emptied window is set against. */
    const heldSpan = $derived.by(() => {
      let lo = Infinity;
      let hi = -Infinity;
      for (const r of db.matched) {
        if (r.firstAt !== null) lo = Math.min(lo, istDayOf(r.firstAt));
        if (r.lastAt !== null) hi = Math.max(hi, istDayOf(r.lastAt));
      }
      return Number.isFinite(lo) && Number.isFinite(hi) ? { lo: isoOfEpochDay(lo), hi: isoOfEpochDay(hi) } : null;
    });
    /** Why the grid is empty when the window emptied it, said once; `''` otherwise. */
    const windowEmpty = $derived(windowEmptyWhy({ matched: db.matched.length, listed: covCount, window: covWindow, held: heldSpan }));

    /**
     * REFUSED FRONTS THE DAY BAND READ (D-3315): days before a series' first
     * `1day` bar that a pull refused, so the rung has no daily bars there.
     * Coverage measures a series from its first bar, so they are not counted
     * missing; they are named beside the coverage figure instead, from the
     * band's own answer — no request of their own.
     */
    const refusedFront = $derived.by(() => {
      const days = (db.dayBandCells ?? []).filter((/** @type {any} */ c) => Array.isArray(c.kinds) && c.kinds.includes('front'));
      if (days.length === 0) return null;
      return {
        n: days.length,
        from: days[0].day,
        to: days[days.length - 1].day,
        instrument: db.dayBandInstrument ?? '',
        text: `${fmt(days.length)} day(s) of a refused front, ${dayLabel(days[0].day)} → ${dayLabel(days[days.length - 1].day)}${db.dayBandInstrument ? ` (${db.dayBandInstrument})` : ''}: the vendor served them and a pull refused them before append, so no daily bar is stored there (D-3315). Coverage measures from the first stored bar, so these days are named here, never counted as held.`
      };
    });

    const storeBars = $derived(db.deco.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + r.rows, 0));
    const kindCount = $derived.by(() => {
      const c = new Map();
      for (const r of db.deco) c.set(r.kind, (c.get(r.kind) ?? 0) + 1);
      return c;
    });

    /** How many of the whole dataset carry each reason code, per column. */
    const whyCount = $derived.by(() => {
      const chg = new Map();
      const prev = new Map();
      for (const r of db.deco) {
        if (r.chg === null) chg.set(r.chgWhy, (chg.get(r.chgWhy) ?? 0) + 1);
        if (r.prevChg === null) prev.set(r.prevChgWhy, (prev.get(r.prevChgWhy) ?? 0) + 1);
      }
      return { chg, prev };
    });

    const numbered = $derived(db.deco.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + (r.chg === null ? 0 : 1), 0));
    const refusedForSplits = $derived(whyCount.chg.get('corporate_action_unverified') ?? 0);

    /**
     * Every reason code present, counted in both columns, most common first.
     * Enumerated, never listed by hand.
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

    const NOTES = $derived([
      {
        k: 'What the two columns are.',
        v: `% in month is the month's OWN first close to its OWN last close (D-0069): it leaves out the move from the previous month's last close to this month's first. Prev month % is the previous calendar month's same statistic for the same instrument. The change against the previous month's close is in the row drawer, read from the two months' last bars. Both arrive as integer basis points and the decimal point is inserted for display: 125 on the wire is +1.25% on screen. CLAUDE.md §7 bans the float for money and a ratio derived from money gets the same treatment.`
      },
      {
        k: 'No corporate-action threshold is sourced. That is why most share cells are a dash.',
        v: `${fmt(refusedForSplits)} of ${fmt(db.deco.length)} rows are refused for this reason, and it is the only refusal no pull fixes. docs/00-charter.md names no verified split-and-bonus feed, and docs/05-decisions.md D-0018 both requires a suspected corporate action to be refused loudly and rejects back-adjusting from an unverified source. So every instrument that can be re-based — a cash share, and an option on a share — is marked rather than numbered. An index, and an option on an index, can never be re-based and both carry a number (D-3325, DB-G66); ${fmt(kindCount.get('INDEX') ?? 0)} INDEX row(s) are in this census.`
      },
      {
        k: 'A month-level field can flag a month. It cannot name the day.',
        v: `D-0018's stated detector is an unexplained OVERNIGHT gap, which is a bar-to-bar test over a month's bars — O(bars) work that cannot live in a census row (D-0067). Even with a threshold in hand, this column could only ever say "a corporate action may fall in this month", never "split on 2024-06-14".`
      },
      {
        k: 'Every reason on this dataset, counted.',
        v:
          reasonRoll.length === 0
            ? 'Nothing is refused: every row in this dataset carries a number in both columns.'
            : `${reasonRoll.map((r) => `${r.code}: ${fmt(r.chg)} in % in month, ${fmt(r.prev)} in prev`).join(' · ')}. Counted from the rows themselves rather than listed here.`
      },
      {
        k: 'What it cost to make this affordable.',
        v: `Reading the two closes from /bars.json instead would be ${fmt(db.deco.length)} request(s) and about ${bytesText(storeBars * BYTES_PER_BAR)} moved to extract ${bytesText(db.deco.length * 16)} of signal. D-0067 put the two closes in the census entry instead.`
      }
    ]);

    let noteOpen = $state(false);

    /**
     * A row's value for the sort: the coverage verdict's numbers, never the
     * peer comparison's.
     *
     * @param {any} r
     * @param {string} k
     */
    function sortValue(r, k) {
      const c = r.cover;
      if (k === 'days') return c.heldSessions ?? -1;
      if (k === 'short') return c.missing ?? -1;
      if (k === 'pct') return c.ratio ?? -1;
      return 0;
    }

    /* THE GRID'S ROWS. On the bar view nobody draws them, but the Coverage
       tab's count and the pager read their length: the matched rows stand
       in there, so the census is not judged for a grid that is not shown. */
    const sorted = $derived.by(() => {
      if (db.view !== 'census') return db.matched;
      const k = sortKey;
      const dir = desc ? -1 : 1;
      return [...judged].sort((a, b) => {
        let d = 0;
        if (k === 'rows') d = a.rows - b.rows;
        // AN UNKNOWN SORTS AS AN UNKNOWN: a trade-driven row owes no count and
        // an unmeasured one has none, so they sit below every real number.
        else if (k === 'days' || k === 'short' || k === 'pct') d = sortValue(a, k) - sortValue(b, k);
        else if (k === 'month') d = txt(a.month, b.month);
        else if (k === 'timeframe') d = tfCmp(a.timeframe, b.timeframe);
        else if (k === 'chg' || k === 'prevChg') {
          if (a[k] === null && b[k] === null) d = 0;
          else if (a[k] === null) return 1;
          else if (b[k] === null) return -1;
          else d = a[k] - b[k];
        } else d = txt(a.instrument, b.instrument);
        if (d !== 0) return dir * d;
        return txt(a.instrument, b.instrument) || txt(a.month, b.month);
      });
    });

    /**
     * A click on a column heading: same column flips the direction, a new
     * column opens on the end that column is interesting at.
     *
     * @param {string} key a `COLS[].key`
     */
    function head(key) {
      if (sortKey === key) desc = !desc;
      else {
        sortKey = key;
        desc = key === 'rows' || key === 'days' || key === 'short' || key === 'chg' || key === 'prevChg';
      }
    }

    /**
     * The `aria-sort` value for one heading.
     *
     * @param {string} key a `COLS[].key`
     */
    function ariaSort(key) {
      return sortKey === key ? (desc ? 'descending' : 'ascending') : 'none';
    }
    const censusPage = $derived(sorted.slice((db.pageNow - 1) * db.pageSize, db.pageNow * db.pageSize));

    /* ======================================================================
       THE SUMMARY, over the judged set — the question actually on screen
       ====================================================================== */
    /* ONE FUNCTION COUNTS THE LINE ABOVE THE GRID AND THE DRAWER (R3-04):
       `coverTotals` over the rows each shows, so neither disagrees with the
       rows under it. Missing is per row, and a row's missing is per day
       where its held days were read (`judgeRow`). */
    const totals = $derived(coverTotals(judged));
    /** Bars the selection holds — every matched row, whichever view is shown (the Bars tab's census count). */
    const total = $derived(db.matched.reduce((/** @type {any} */ a, /** @type {any} */ r) => a + r.rows, 0));
    /** Rows the calendar judged complete. */
    const full = $derived(totals.full);
    /** Rows no calendar could judge: their reason is on each. */
    const unverified = $derived(totals.unverified);
    /** Trade-driven rows: owed no count (D-3336 §2). */
    const tradeRows = $derived(totals.trade);
    /** Bars the calendar owes the judged rows and the store does not hold. */
    const gaps = $derived(totals.missing);
    /** Bars owed to the calendar-judged rows whose missing count is known. */
    const owed = $derived(totals.owed);
    /** Bars those rows hold: owed less missing, so a per-day floor lowers it. */
    const judgedBars = $derived(totals.held);
    /** Bars held over bars owed, or `null` where nothing is owed. */
    const coverage = $derived(totals.coverage);
    /** Months a calendar-owed series does not hold between its two ends (`months_missing`): holes. */
    const absentCount = $derived(totals.absent);
    /** Months a trade-driven series does not hold between its two ends: nothing traded, not holes. */
    const noTradeMonths = $derived(totals.noTrade);
    /** Rows short by a hole every stored series shares. */
    const storeWideRows = $derived(totals.storeWide);
    /** Derived rows behind or short of their 1min source. */
    const derivedLagging = $derived(totals.derivedLagging);
    const monthsIn = $derived.by(() => {
      const s = new Set();
      for (const r of db.matched) s.add(r.month);
      return [...s].sort();
    });

    /* ---- the month roll-up (calendar-judged; absent months included) ---- */
    const monthCards = $derived(rollUpMonths(judged).sort((a, b) => txt(b.month, a.month)));
    const holeMonths = $derived(monthCards.filter((m) => m.state === 'near' || m.state === 'gap' || m.state === 'absent').length);
    const unprovenMonths = $derived(monthCards.filter((m) => m.state === 'unmeasured').length);

    /** @type {HTMLElement | null} */
    let scroller = $state(null);
    let scrollTop = $state(0);
    let viewportH = $state(600);

    const first = $derived(Math.max(0, Math.floor(scrollTop / ROW) - OVER));
    const count = $derived(Math.ceil(viewportH / ROW) + OVER * 2);
    const slice = $derived(censusPage.slice(first, first + count));

    let xscrolled = $state(false);
    function onScroll() {
      if (!scroller) return;
      scrollTop = scroller.scrollTop;
      const x = scroller.scrollLeft > 0;
      if (x !== xscrolled) xscrolled = x;
    }

    /* ---- KEYBOARD: one tab stop, `aria-activedescendant` names the row ---- */
    let cursor = $state(-1);
    /** @type {string | null} */
    let openKey = $state(null);
    /** @type {HTMLElement | null} */
    let drawerEl = $state(null);

    /** @param {number} next */
    function moveTo(next) {
      const n = censusPage.length;
      if (n === 0) return;
      cursor = Math.max(0, Math.min(n - 1, next));
      const el = scroller;
      if (!el) return;
      const top = HEAD + cursor * ROW;
      if (top < el.scrollTop + HEAD) el.scrollTop = cursor * ROW;
      else if (top + ROW > el.scrollTop + viewportH) el.scrollTop = top + ROW - viewportH;
    }

    /** @param {number} d */
    function step(d) {
      moveTo(cursor < 0 ? (d > 0 ? 0 : censusPage.length - 1) : cursor + d);
    }

    /** @param {any} it */
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

    /* ---- what the drawer shows -------------------------------------------
       EVERY MONTH OF THE OPENED SERIES at the opened rung, stored or absent,
       judged; built as a Map so opening a row is a probe. */
    const byInstrument = $derived.by(() => {
      /** @type {Map<string, DecoRow[]>} */
      const by = new Map();
      if (db.view !== 'census') return by;
      for (const it of db.deco) {
        let a = by.get(it.instrument);
        if (!a) by.set(it.instrument, (a = []));
        a.push(it);
      }
      return by;
    });

    const judgedByKey = $derived.by(() => {
      /** @type {Map<string, any>} */
      const m = new Map();
      for (const it of judged) m.set(it.key, it);
      return m;
    });

    const openRowData = $derived.by(() => {
      if (!openKey) return null;
      const hit = judgedByKey.get(openKey);
      if (hit) return hit;
      const row = byKey.get(openKey);
      return row ? { ...row, cover: coverOf(row) } : null;
    });
    const openMonths = $derived.by(() => {
      const o = openRowData;
      if (!o) return [];
      const rows = (byInstrument.get(o.instrument) ?? []).filter((r) => r.timeframe === o.timeframe);
      return [...rows, ...absentOf(o.instrument, o.timeframe)]
        .map((r) => judgedByKey.get(r.key) ?? { ...r, cover: coverOf(r) })
        .sort((x, y) => txt(y.month, x.month));
    });
    /* THE DRAWER'S FOUR FIGURES ARE THE SAME FUNCTION OVER ITS OWN ROWS
       (R3-04): Missing sums what each row prints under it — an absent
       month's owed bars beside the word, never hidden behind it — and a
       row whose missing count is unknown prints no number and joins no
       sum. */
    const openTotals = $derived.by(() => {
      const t = coverTotals(openMonths);
      return {
        bars: t.bars,
        missing: t.missing,
        complete: t.full,
        unproven: t.unverified,
        absent: t.absent + t.noTrade,
        over: t.over,
        sessions: t.sessions,
        held: t.heldSessions,
        exact: t.heldExact,
        n: t.n
      };
    });

    /* ---- the opened month against the previous month's close (DB-081) ----
       Two reads: the last bar of the month before and of this month. Only an
       index or an option on one: a share can be re-based (D-0018). */
    let vsPrev = $state.raw(/** @type {{ key: string, bps: number | null, why: string, base: number | null, last: number | null, note?: string } | null} */ (null));
    const vsReads = createPageRequests();
    $effect(() => {
      const o = openRowData;
      const feed = feeds.active;
      if (!o || !feed || o.absent) {
        vsPrev = null;
        return;
      }
      if (untrack(() => vsPrev)?.key === o.key) return;
      const seg = segmentOfKey(o.instrument);
      const exch = String(o.instrument).split('-')[0];
      // AN OPTION ON AN INDEX is one whose underlying the census holds as an
      // INDEX series: read from the store, never from a list of names here.
      const indexy = seg === 'INDEX' || (seg === 'FNO' && untrack(() => byInstrument).has(`${exch}-INDEX-${o.underlying}`));
      if (!indexy) {
        vsPrev = {
          key: o.key,
          bps: null,
          base: null,
          last: null,
          why:
            seg === 'FNO'
              ? `no INDEX series named ${o.underlying} is stored, so this option's underlying may be a share, which can be re-based: no change across months is shown (D-0018)`
              : 'a share can be re-based (split, bonus, consolidation): no change across months is shown (D-0018)'
        };
        return;
      }
      vsReads.cancel();
      void vsReads.run(async (ticket) => {
        const y = Number(o.month.slice(0, 4));
        const m = Number(o.month.slice(5, 7));
        const prev = m === 1 ? `${y - 1}-12` : `${y}-${String(m - 1).padStart(2, '0')}`;
        const parts = String(o.instrument).split('-');
        const contract = o.expiry ? String(o.instrument).slice(`${parts[0]}-${parts[1]}-${o.underlying}-`.length) : '';
        /** @param {string} month */
        const lastClose = async (month) => {
          const q = new URLSearchParams({ feed, exchange: parts[0], segment: parts[1], symbol: o.underlying ?? '', timeframe: o.timeframe, from: month, to: month, dir: 'desc', limit: '1' });
          if (contract) q.set('contract', contract);
          // A BUSY SERVER IS ASKED AGAIN (DB-B51, WP-81): a 429 here used to
          // read as "no previous close", which is a different fact.
          const r = await fetchAdmitted(`/bars/window.json?${q}`, { signal: ticket.signal },
            (target, options) => ask(target, { ...options, cache: 'no-store' }));
          if (!r.ok) return null;
          const b = await r.json();
          const bar = Array.isArray(b?.bars) ? b.bars[0] : null;
          return Number.isSafeInteger(bar?.c) ? bar.c : null;
        };
        try {
          const prevRow = byKey.get(`${o.instrument}\u0000${prev}\u0000${o.timeframe}`);
          if (!prevRow) {
            if (ticket.current()) vsPrev = { key: o.key, bps: null, base: null, last: null, why: `${monthLabel(prev)} is not held at this rung, so there is no previous close to measure from` };
            return;
          }
          // A MONTH'S LAST BAR IS ITS CLOSE ONLY IF ITS LAST SESSION IS HELD:
          // in a month with a hole at its end (Zerodha 2024-11 stops on the
          // 8th) the last stored bar is not the month's close, and the change
          // is refused by name rather than measured to the wrong day.
          //
          // A TRADE-DRIVEN SERIES IS NOT CUT SHORT BY A QUIET LAST SESSION: an
          // option's last bar is its last trade, which is its close whether it
          // printed on the month's last session or a week before; the days of
          // both last trades are named beside the figure instead. A contract's
          // month ends at its expiry, never after it.
          const ledger = untrack(() => ledgers).get(ledgerUrlOf(o))?.ledger ?? null;
          const tradeDriven = o.cover?.basis === 'trade';
          const expiryDay = o.expiry ? epochDayOfIso(o.expiry) : null;
          /** @param {string} month @param {number | null} lastAt */
          const cutShort = (month, lastAt) => {
            if (!ledger || lastAt === null || tradeDriven) return null;
            const { first, last: monthLast } = monthSpan(month);
            const last = expiryDay !== null ? Math.min(monthLast, expiryDay) : monthLast;
            for (let d = last; d >= first; d -= 1) {
              const day = ledgerDay(ledger, d);
              if (day && 'RXPHU'.includes(day.code)) return istDayOf(lastAt) < d ? isoOfEpochDay(d) : null;
            }
            return null;
          };
          for (const [month, lastAt] of /** @type {[string, number | null][]} */ ([[prev, prevRow.lastAt], [o.month, o.lastAt]])) {
            const owed = cutShort(month, lastAt);
            if (owed !== null) {
              if (ticket.current()) {
                vsPrev = {
                  key: o.key,
                  bps: null,
                  base: null,
                  last: null,
                  why: `${monthLabel(month)}'s last session, ${dayLabel(owed)}, is not held at this rung (its last bar is on ${dayLabel(lastAt)}), so its last bar is not its close`
                };
              }
              return;
            }
          }
          const base = await lastClose(prev);
          const last = await lastClose(o.month);
          if (!ticket.current()) return;
          const bps = base !== null && last !== null && base > 0 ? basisPoints(base, last) : null;
          vsPrev = {
            key: o.key,
            bps: typeof bps === 'number' ? bps : null,
            base,
            last,
            why: bps === null ? 'a last bar could not be read' : '',
            note:
              tradeDriven && prevRow.lastAt !== null && o.lastAt !== null
                ? `last trades ${dayLabel(prevRow.lastAt)} and ${dayLabel(o.lastAt)}`
                : ''
          };
        } catch (why) {
          if (ticket.current()) vsPrev = { key: o.key, bps: null, base: null, last: null, why: `the last bars could not be read (${String(/** @type {any} */ (why)?.message ?? why)})` };
        }
      });
    });
    $effect(() => () => vsReads.dispose());

    /* ---- a contract's life, and where the store cuts it (DB-G67/G68/G72) ---- */
    let importedTo = $state(/** @type {string | null} */ (null));
    const importReads = createPageRequests();
    $effect(() => {
      const o = openRowData;
      if (!o?.expiry || feeds.active !== 'gdfl' || untrack(() => importedTo) !== null) return;
      void importReads.run(async (ticket) => {
        try {
          const r = await ask('/imports.json?feed=gdfl&kind=options&from=2018-09-03&to=2018-09-03', { cache: 'no-store', signal: ticket.signal });
          if (!r.ok || !ticket.current()) return;
          const b = await r.json();
          const k = (b?.kinds ?? []).find((/** @type {any} */ x) => x?.kind === 'options');
          if (ticket.current()) importedTo = typeof k?.last_day === 'string' ? k.last_day : '';
        } catch {
          if (ticket.current()) importedTo = '';
        }
      });
    });
    $effect(() => () => importReads.dispose());

    /* ---- the import's lineage records for the opened contract (DB-G72) ----
       One `/dayflags.json` day read per opened GDFL contract: its `lineage`
       names the key a contract continues as after a bonus or split renamed
       it (D-3331), or says the ledger is not written yet. Kept per key. */
    let lineage = $state.raw(/** @type {{ key: string, read: { present: boolean, records?: any[], why?: string | null } | null } | null} */ (null));
    const lineageReads = createPageRequests();
    $effect(() => {
      const o = openRowData;
      const feed = feeds.active;
      if (!o?.expiry || feed !== 'gdfl' || o.absent) return;
      if (untrack(() => lineage)?.key === o.instrument) return;
      const s = untrack(() => seriesIndex).get(`${o.instrument}\u0000${o.timeframe}`);
      const on = s?.lastDay ?? null;
      if (on === null) return;
      lineage = { key: o.instrument, read: null };
      lineageReads.cancel();
      void lineageReads.run(async (ticket) => {
        try {
          const day = isoOfEpochDay(on);
          const r = await ask(dayflagsUrl('gdfl', o.instrument, day, day), { cache: 'no-store', signal: ticket.signal });
          if (!r.ok || !ticket.current()) return;
          const b = await r.json();
          const l = b?.lineage;
          if (ticket.current() && l && typeof l === 'object') {
            lineage = { key: o.instrument, read: { present: l.present === true, records: Array.isArray(l.records) ? l.records : [], why: l.why ?? null } };
          }
        } catch {
          /* A FAILED READ LEAVES THE LINEAGE UNREAD (`null`): the life marks
             then say nothing about a rename, rather than "never renamed". */
        }
      });
    });
    $effect(() => () => lineageReads.dispose());

    const openLife = $derived.by(() => {
      const o = openRowData;
      if (!o) return [];
      const s = seriesIndex.get(`${o.instrument}\u0000${o.timeframe}`);
      if (!s || s.firstDay === null || s.lastDay === null) return [];
      // A SPOT SERIES HAS NO LIFE TO CUT: the archive's first day is a fact
      // about the feed, said once on /feeds.json, not about NIFTY.
      if (!o.expiry) return [];
      const read = ledgers.get(ledgerUrlOf(o));
      const expiryDay = o.expiry ? epochDayOfIso(o.expiry) : null;
      const sessionsAfter =
        read?.ledger && expiryDay !== null && s.lastDay < expiryDay
          ? ledgerSpan(read.ledger, s.lastDay + 1, expiryDay).sessions
          : expiryDay !== null && s.lastDay >= expiryDay
            ? 0
            : null;
      return contractLife(
        { firstDay: isoOfEpochDay(s.firstDay), lastDay: isoOfEpochDay(s.lastDay), expiry: o.expiry ?? null },
        {
          archive: /** @type {any} */ (db.activeFeed)?.archive ?? null,
          importedTo: importedTo || null,
          today: db.today ?? '',
          sessionsAfter,
          lineage: lineage !== null && lineage.key === o.instrument ? lineage.read : null
        }
      );
    });

    /**
     * Show the opened month's days on the bar grid: the day window set to the
     * month, and the day band beside it names each day.
     *
     * @param {any} m
     */
    function showDays(m) {
      const y = Number(m.month.slice(0, 4));
      const mo = Number(m.month.slice(5, 7));
      const lastDay = new Date(Date.UTC(y, mo, 0)).getUTCDate();
      db.fromDay = `${m.month}-01`;
      db.toDay = `${m.month}-${String(lastDay).padStart(2, '0')}`;
      db.view = 'bars';
      openKey = null;
    }

    Object.defineProperties(db, {
      sortKey: { get: () => sortKey, set: (v) => (sortKey = v) },
      desc: { get: () => desc, set: (v) => (desc = v) },
      covLedgers: { get: () => ledgers },
      covOneSeries: { get: () => oneSeries },
      covJudged: { get: () => judged },
      covTradeRows: { get: () => tradeRows },
      covAbsent: { get: () => absentCount },
      covNoTrade: { get: () => noTradeMonths },
      covStored: { get: () => totals.stored },
      covOver: { get: () => totals.over },
      covUnresolved: { get: () => totals.unresolved },
      covCount: { get: () => covCount },
      covWindow: { get: () => covWindow },
      covWindowEmpty: { get: () => windowEmpty },
      covRefusedFront: { get: () => refusedFront },
      covStoreWide: { get: () => storeWideRows },
      covDerivedLagging: { get: () => derivedLagging },
      covVsPrev: { get: () => vsPrev },
      covLife: { get: () => openLife },
      covShowDays: { get: () => showDays },
      storeBars: { get: () => storeBars },
      kindCount: { get: () => kindCount },
      whyCount: { get: () => whyCount },
      numbered: { get: () => numbered },
      refusedForSplits: { get: () => refusedForSplits },
      reasonRoll: { get: () => reasonRoll },
      NOTES: { get: () => NOTES },
      noteOpen: { get: () => noteOpen, set: (v) => (noteOpen = v) },
      sorted: { get: () => sorted },
      head: { get: () => head },
      ariaSort: { get: () => ariaSort },
      censusPage: { get: () => censusPage },
      total: { get: () => total },
      full: { get: () => full },
      unverified: { get: () => unverified },
      gaps: { get: () => gaps },
      judgedBars: { get: () => judgedBars },
      owed: { get: () => owed },
      coverage: { get: () => coverage },
      monthsIn: { get: () => monthsIn },
      monthCards: { get: () => monthCards },
      holeMonths: { get: () => holeMonths },
      unprovenMonths: { get: () => unprovenMonths },
      scroller: { get: () => scroller, set: (v) => (scroller = v) },
      scrollTop: { get: () => scrollTop, set: (v) => (scrollTop = v) },
      viewportH: { get: () => viewportH, set: (v) => (viewportH = v) },
      first: { get: () => first },
      count: { get: () => count },
      slice: { get: () => slice },
      xscrolled: { get: () => xscrolled, set: (v) => (xscrolled = v) },
      onScroll: { get: () => onScroll },
      cursor: { get: () => cursor, set: (v) => (cursor = v) },
      openKey: { get: () => openKey, set: (v) => (openKey = v) },
      drawerEl: { get: () => drawerEl, set: (v) => (drawerEl = v) },
      moveTo: { get: () => moveTo },
      step: { get: () => step },
      openRow: { get: () => openRow },
      onKey: { get: () => onKey },
      byInstrument: { get: () => byInstrument },
      byKey: { get: () => byKey },
      openRowData: { get: () => openRowData },
      openMonths: { get: () => openMonths },
      openTotals: { get: () => openTotals }
    });
  }

  /**
   * The words for one row's sessions cell: `5 / 19` (held / owed), or the
   * trade-driven span, or a dash with its reason.
   *
   * @param {Cover} c
   */
  export function sessionsText(c) {
    if (c.sessions === null) return '—';
    if (c.heldSessions === null) return `— / ${fmt(c.sessions)}`;
    const held = c.heldExact || Number.isInteger(c.heldSessions) ? fmt(c.heldSessions) : c.heldSessions.toFixed(2);
    return `${held} / ${fmt(c.sessions)}`;
  }

  /**
   * The full sentence behind one row's verdict, for its title.
   *
   * @param {any} it a judged row
   */
  export function coverTitle(it) {
    const c = /** @type {Cover} */ (it.cover);
    const head =
      c.basis === 'none'
        ? `Not measured: ${c.why}.`
        : c.basis === 'trade'
          ? `Trade-driven: ${fmt(it.rows)} bar(s) over ${c.sessions === null ? 'an unknown number of' : fmt(c.sessions)} session(s) in its span${c.heldExact ? `, ${fmt(c.heldSessions ?? 0)} of them held` : ''}.`
          : it.absent
            ? `Absent: the store holds no file for this month; the calendar owes ${fmt(c.owed ?? 0)} bar(s) over ${fmt(c.sessions ?? 0)} session(s).`
            : `${fmt(it.rows)} of ${fmt(c.owed ?? 0)} bar(s) the calendar owes over ${fmt(c.sessions ?? 0)} session(s)${c.owedAlt !== null ? ` (${fmt(c.owedAlt)} were the share not auction-eligible)` : ''}${c.missing ? `; ${fmt(c.missing)} missing` : ''}${c.extra ? `; ${fmt(c.extra)} beyond what is owed` : ''}.`;
    return [head, ...c.notes.map(noteText)].join(' ');
  }


  /**
   * WHAT MOVED BETWEEN TWO READS — the per-tile and per-row flash, and the reset
   * a feed change performs.
   *
   * Creates the page's effects that stood at old lines 6292; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function coverageFlash(db) {
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
    /** `setTimeout` returns a `Timeout` under node's types and a `number` in the browser; naming the return type takes whichever this build resolves to rather than asserting one of them. */
    let movedTimer = /** @type {ReturnType<typeof setTimeout> | undefined} */ (undefined);

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
      /* THE HERO FIGURES FORGET TOO. Omitting these two was why `bars held` and
         `months covered` flashed on every feed change — and always green, since
         `rows` empties between feeds so the last leg is always upward. Cleared
         alongside `prev`, which is the same fact for the tiles. */
      db.factSeen.clear();
      db.factSeeded = false;
      for (const t of timers.values()) clearTimeout(t);
      timers.clear();
      clearTimeout(movedTimer);
      flash = {};
      moved = new Map();
      db.openKey = null;
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
      const at = db.fetchedAt;
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
          ['n', db.matched.length],
          ['bars', db.total],
          ['full', db.full],
          ['gaps', db.gaps],
          /* `null` PASSES THROUGH AS `null`, never as a rounded zero.
             `Math.round(null * 10000)` is 0, which would enter the baseline as a
             coverage of 0.00% and flash red the instant a real ratio returned.
             `mark` refuses to give an unknown a direction. */
          ['cov', db.coverage === null ? null : Math.round(db.coverage * 10000)],
          ['months', db.monthsIn.length]
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
        for (const it of db.deco) {
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

    Object.defineProperties(db, {
      flash: { get: () => flash, set: (v) => (flash = v) },
      prev: { get: () => prev },
      timers: { get: () => timers },
      seeded: { get: () => seeded, set: (v) => (seeded = v) },
      mark: { get: () => mark },
      moved: { get: () => moved, set: (v) => (moved = v) },
      barsBefore: { get: () => barsBefore },
      barsSeeded: { get: () => barsSeeded, set: (v) => (barsSeeded = v) },
      movedTimer: { get: () => movedTimer, set: (v) => (movedTimer = v) },
      forgetPreviousFeed: { get: () => forgetPreviousFeed }
    });
  }

  /**
   * THE DRAWER TAKES FOCUS WHEN IT OPENS.
   *
   * Creates the page's effects that stood at old lines 7376; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function coverageDrawer(db) {
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
      if (db.openKey && db.drawerEl)
        /** @type {HTMLElement | null} */ (db.drawerEl.querySelector('.dclose'))?.focus();
    });
  }

  /**
   * A NEW VIEW STARTS AT THE TOP — the stamp that re-keys both grids and the
   * coverage grid's entrance.
   *
   * Creates the page's effects that stood at old lines 7440, 7444 and 7452; the shell calls it
   * in that position of the order.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function coverageStamp(db) {
    /* A NEW VIEW STARTS AT THE TOP. Keeping the old offset after a filter
       change lands the operator in the middle of a list they have not seen. */
    let stamp = $state(0);
    /* ONE PLACE THAT PUTS THE GRID BACK AT ITS TOP, called by both effects
       below. It was written out twice for one edit and that is exactly how the
       two copies drift — one of them gaining a reset the other never got. */
    function restartAtTop() {
      stamp += 1;
      if (db.scroller) db.scroller.scrollTop = 0;
      db.scrollTop = 0;
      db.cursor = -1;
    }

    /* MOVING BETWEEN PAGES, BETWEEN PAGE SIZES OR BETWEEN THE TWO GRIDS
       STARTS AT THE TOP TOO - and it is a SEPARATE effect, because this one
       must not write `page`. An effect that both reads `page` and resets it
       to 1 is a control that cannot leave its first page. */
    $effect(() => {
      void [db.page, db.pageSize, db.view];
      untrack(restartAtTop);
    });
    $effect(() => {
      void db.rows;
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

    Object.defineProperties(db, {
      stamp: { get: () => stamp, set: (v) => (stamp = v) },
      restartAtTop: { get: () => restartAtTop },
      entering: { get: () => entering, set: (v) => (entering = v) }
    });
  }

  export { notesPanel };
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

<!-- THE COVERAGE CALENDAR (WP-54): the selection's days, both feeds, above
     the grid of its months. -->
<DbCalendar {db} />

<!-- ==================================================================
     WHAT THE GRID BELOW SAYS, COUNTED — against the calendar of record.
     ================================================================== -->
<section class="covsum" aria-label="Coverage of the rows below, against the calendar of record">
  <span class="cs"><b>{fmt(db.covStored)}</b> month(s) stored</span>
  <span class="cs" class:risk={db.covAbsent > 0} title="Months a series does not hold between its first and last stored month (months_missing), drawn as absent rows below."
    ><b>{fmt(db.covAbsent)}</b> absent</span
  >
  {#if db.covNoTrade > 0}
    <span class="cs dim" title={noteText('no_trade_month')}><b>{fmt(db.covNoTrade)}</b> month(s) with no trade</span>
  {/if}
  <span class="cs"><b>{fmt(db.full)}</b> full</span>
  <span class="cs" class:risk={db.gaps > 0}><b>{fmt(db.gaps)}</b> bar(s) missing</span>
  <span class="cs" title="Bars held over bars the calendar of record owes these rows at their rungs (D-3336) — never against the fullest stored series. Held is owed less missing, and missing is counted day by day where the series' own held days were read, so a surplus on one day never pays for a hole on another. Rounded down: a month missing one bar never reads 100.00%."
    >coverage <b>{coverPct(db.coverage)}</b> of {fmt(db.owed)} owed</span
  >
  {#if db.covRefusedFront}
    <!-- THE REFUSED FRONT, BESIDE THE FIGURE IT IS LEFT OUT OF (R3-04). -->
    <span class="cs risk" title={db.covRefusedFront.text}><b>{fmt(db.covRefusedFront.n)}</b> refused-front day(s), not in coverage</span>
  {/if}
  {#if db.covOver > 0}
    <span class="cs risk" title={noteText('surplus_unplaced')}><b>{fmt(db.covOver)}</b> over what is owed, surplus unplaced</span>
  {/if}
  {#if db.covUnresolved > 0}
    <span class="cs dim" title={noteText('cas_unresolved')}><b>{fmt(db.covUnresolved)}</b> share-month(s) the auction flag decides</span>
  {/if}
  {#if db.covWindow}
    <!-- COVERAGE FOLLOWS THE WINDOW, AND SAYS SO (DB-046, R3-04). -->
    <span class="cs dim" title="The From and To days narrow Coverage as they narrow Bars: a month is listed when the window touches it, and judged whole — its row counts every bar the month holds against every bar the month owes."
      >months touching <b>{windowText(db.covWindow)}</b></span
    >
  {/if}
  {#if db.covStoreWide > 0}
    <span class="cs risk" title={noteText('store_wide')}><b>{fmt(db.covStoreWide)}</b> in a store-wide hole</span>
  {/if}
  {#if db.covDerivedLagging > 0}
    <span class="cs risk" title={noteText('derived_behind')}><b>{fmt(db.covDerivedLagging)}</b> derived month(s) behind or short of 1min</span>
  {/if}
  {#if db.covTradeRows > 0}
    <span class="cs" title={noteText('trade_driven')}><b>{fmt(db.covTradeRows)}</b> trade-driven (no fixed count)</span>
  {/if}
  {#if db.unverified > 0}
    <span class="cs dim" title="Rows no calendar could judge; each names why on hover."><b>{fmt(db.unverified)}</b> not measured</span>
  {/if}
</section>

<!-- ==================================================================
     THE CENSUS TABLE. Windowed: only the visible slice exists in the DOM.
     ================================================================== -->
<div class="tbl rise">
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="tbl-scroll"
    class:xscrolled={db.xscrolled}
    bind:this={db.scroller}
    bind:clientHeight={db.viewportH}
    onscroll={db.onScroll}
    onkeydown={db.onKey}
    tabindex="0"
    role="grid"
    aria-rowcount={db.censusPage.length}
    aria-activedescendant={db.cursor >= 0 ? `dbrow-${db.cursor}` : undefined}
    aria-label="Stored and absent instrument-months, {fmt(db.censusPage.length)} row(s) on page {fmt(
      db.pageNow
    )} of {fmt(db.pageCount)}. Arrow keys move, Enter opens."
  >
    <div class="tbl-inner" role="presentation">
      <div class="thead" role="row">
        {#each COLS as c, ci (ci)}
          <div
            class="th"
            class:numh={c.num}
            class:inst={ci === 0}
            role="columnheader"
            aria-sort={db.ariaSort(c.key)}
          >
            <button
              class="sortbtn"
              type="button"
              onclick={() => db.head(c.key)}
              title={c.why ? `${c.label} — ${c.why}. Click to sort.` : `Sort by ${c.label}`}
            >
              <span>{c.label}</span>
              <span class="caret" class:on={db.sortKey === c.key} class:desc={db.desc} aria-hidden="true"
              ></span>
            </button>
          </div>
        {/each}
      </div>

      {#if db.sorted.length === 0}
        <div class="tnone">
          <h3>No row matches</h3>
          {#if db.covWindowEmpty}
            <!-- THE WINDOW EMPTIED IT, AND THIS IS THE ONE PLACE THAT SAYS SO,
                 in order, beside the days the selection does hold (R3-04). -->
            <p>{db.covWindowEmpty}</p>
          {:else}
            <p>
              {db.feedName} holds {fmt(db.deco.length)} instrument-months, and none of them match
              {#each db.narrowing as n, i (n.k)}{i > 0 ? ', ' : ''}<b
                  title={n.key ? `the store's own key for it is ${n.key}` : undefined}
                  >{n.text}</b
                >{/each}.
            </p>
          {/if}
          {#if db.covRefusedFront}
            <!-- THE SAME WORDS THE BOX ABOVE CARRIES (R3-04). -->
            <p class="tgate">{db.covRefusedFront.text}</p>
          {/if}
          {#if db.expiryHeld > 0}
            <p class="tgate">
              {fmt(db.expiryHeld)} of them are futures or options and are being <b>held back</b>,
              not filtered:
              {#if db.expiry}
                they are on a contract other than {dayLabel(db.expiry)}. Choose theirs on the
                Contract strip above.
              {:else}
                no expiry is chosen. A bar belongs to <b>one contract</b>, so this page will not
                put two expiries under one heading — choose an expiry on the Contract strip and
                they appear.
              {/if}
              Clearing the filters does not release them; only choosing an expiry does.
            </p>
          {/if}
          <button class="btn" onclick={db.reset}>Clear the filters</button>
        </div>
      {:else}
        <div class="tspace" role="rowgroup" style="height:{db.censusPage.length * ROW}px">
          {#key db.stamp}
            {#each db.slice as it, i (it.key)}
              {@const n = db.first + i}
              {@const c = it.cover}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <!-- svelte-ignore a11y_interactive_supports_focus -->
              <div
                class="trow"
                class:odd={n % 2 === 1}
                class:row-in={db.entering}
                class:cur={db.cursor === n}
                class:open={db.openKey === it.key}
                class:oow={c.notes.includes('outside_window')}
                data-state={it.absent && c.basis === 'trade' ? 'notrade' : c.state}
                id="dbrow-{n}"
                role="row"
                aria-rowindex={n + 1}
                aria-selected={db.cursor === n}
                onclick={() => {
                  db.cursor = n;
                  db.openRow(it);
                }}
                style="top:{n * ROW}px; animation-delay:{Math.min(i * 8, 180)}ms"
              >
                <span class="cell inst" role="gridcell" title={it.instrument}>
                  <span class="pip" aria-hidden="true"></span>
                  <span class="ihead">{it.head}</span><span class="isym">{it.sym}</span>
                  {#if it.absent}<span class="tag absent" class:notrade={c.basis === 'trade'} title={noteText(c.basis === 'trade' ? 'no_trade_month' : 'absent_month')}
                      >{c.basis === 'trade' ? 'no trade' : 'absent'}</span
                    >{/if}
                  {#if c.notes.includes('outside_window')}<span class="tag oowtag" title={noteText('outside_window')}>outside your window</span>{/if}
                </span>
                <span
                  class="cell mono"
                  role="gridcell"
                  title={heldWindow(it)
                    ? `${monthLabel(it.month)} · held ${heldWindow(it)} — the store's own key for it is ${it.month}`
                    : `${monthLabel(it.month)} — the store's own key for it is ${it.month}`}
                  >{monthLabel(it.month)}</span
                >
                <span class="cell tf" role="gridcell"
                  >{it.timeframe}{#if c.notes.includes('derived')}<span
                      class="tag drv"
                      class:lag={c.notes.includes('derived_behind') || c.notes.includes('derived_partial')}
                      title={c.notes.filter((/** @type {string} */ x) => x.startsWith('derived')).map(noteText).join(' ')}
                      >{c.notes.includes('derived_behind') ? 'behind' : c.notes.includes('derived_partial') ? 'partial' : 'derived'}</span
                    >{/if}</span
                >
                <span
                  class="cell num"
                  role="gridcell"
                  class:flash-up={db.moved.get(it.key) === 'up'}
                  class:flash-down={db.moved.get(it.key) === 'down'}>{it.absent ? '—' : fmt(it.rows)}</span
                >
                <span class="cell num dimnum" role="gridcell" title={coverTitle(it)}>{sessionsText(c)}</span>
                <span class="cell num short" role="gridcell" title={coverTitle(it)}>
                  {#if c.basis === 'trade'}
                    <span class="unproven">trade-driven</span>
                  {:else if c.basis === 'none'}
                    <span class="unproven">not measured</span>
                  {:else if c.state === 'unmeasured'}
                    <!-- EVERY SESSION OF THE SPAN IS OF UNRECORDED LENGTH (a
                         month outside the calendar of record): nothing is owed,
                         so "−0" would be a measurement nobody took. -->
                    <span class="unproven">unmeasured</span>
                  {:else if c.state === 'full'}
                    <span class="okword">full</span>{#if c.notes.includes('cas')}<span class="tag info">CAS</span>{/if}
                  {:else if c.state === 'over'}
                    <span class="okword">+{fmt(c.extra)}</span>
                  {:else}
                    <span class="missnum">−{fmt(c.missing ?? 0)}</span>
                    {#if c.notes.includes('store_wide')}<span class="tag wide">store-wide</span>{/if}
                  {/if}
                </span>
                <span class="cell comp" role="gridcell" title={coverTitle(it)}>
                  {#if c.ratio !== null}
                    <span class="meter" aria-hidden="true"
                      ><span class="fill" style="width:{c.ratio * 100}%"></span></span
                    >
                  {/if}
                  <span class="cpct">{c.basis === 'trade' ? STATE_WORD.trade : coverPct(c.ratio)}</span>
                </span>
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

  {#if db.openRowData}
    {@const o = db.openRowData}
    {@const oc = o.cover}
    <div
      class="drawer panel-in"
      bind:this={db.drawerEl}
      role="dialog"
      aria-modal="false"
      aria-label="{o.instrument} — every month at {o.timeframe}"
    >
      <div class="dhead">
        <div class="dtitle">
          <span class="ihead">{o.head}</span><span class="isym">{o.sym}</span>
          <span class="dtf">{o.timeframe}</span>
        </div>
        <button class="link dclose" onclick={() => (db.openKey = null)}>close ·<kbd class="kbd">Esc</kbd></button>
      </div>

      <div class="dstats">
        <div class="dstat">
          <span class="k">Months</span><span class="v">{fmt(db.openTotals.n - db.openTotals.absent)}{#if db.openTotals.absent > 0}<span class="risk"
                >&nbsp;+{fmt(db.openTotals.absent)} {oc.basis === 'trade' ? 'with no trade' : 'absent'}</span
              >{/if}</span>
        </div>
        <div class="dstat">
          <span class="k">Full</span><span class="v"
            >{fmt(db.openTotals.complete)}{#if db.openTotals.unproven > 0}<span class="unproven"
                >&nbsp;+{fmt(db.openTotals.unproven)} unmeasured</span
              >{/if}</span
          >
        </div>
        <div class="dstat">
          <span class="k">Sessions</span><span
            class="v"
            title={oc.basis === 'trade'
              ? 'trade-driven: sessions in the months shown; a contract holds a bar only where a trade printed, so days held are in the coverage calendar above'
              : db.openTotals.exact
                ? 'held days counted from the series\' own calendar'
                : 'bars over the average session'}
            >{oc.basis === 'trade' ? '—' : db.openTotals.exact ? fmt(db.openTotals.held) : db.openTotals.held.toFixed(2)} / {fmt(db.openTotals.sessions)}</span
          >
        </div>
        <div class="dstat">
          <span class="k">Missing</span><span
            class="v"
            class:risk={db.openTotals.missing > 0}
            title="The sum of the figures under Missing below: each month's bars owed and not held, an absent month's whole owed count included.{db.openTotals.over > 0 ? ` ${fmt(db.openTotals.over)} month(s) hold an unplaced surplus and print +N: no missing count is known for them, so none is added.` : ''}"
            >{fmt(db.openTotals.missing)}</span
          >
        </div>
      </div>

      <div class="dverdict" data-state={oc.state}>
        <p><b>{monthLabel(o.month)}</b> — {coverTitle(o)}</p>
        {#if db.covVsPrev && db.covVsPrev.key === o.key}
          <p class="dvs">
            <span class="k">% in month</span>
            {o.chg === null ? '—' : bpsText(o.chg)}
            <span class="k">vs previous month close</span>
            {#if db.covVsPrev.bps !== null}
              <b data-dir={dirOf(db.covVsPrev.bps)}>{bpsText(db.covVsPrev.bps)}</b>
              <span class="n">(last bar of {monthLabel(o.month)} against the last bar of the month before, at {o.timeframe}{db.covVsPrev.note ? `; ${db.covVsPrev.note}` : ''})</span>
            {:else}
              — <span class="n">{db.covVsPrev.why}</span>
            {/if}
          </p>
        {/if}
        {#each db.covLife as l (l.code)}
          <p class="dlife" data-code={l.code}>{l.text}.</p>
        {/each}
        <button class="link" onclick={() => db.covShowDays(o)}>Show this month's days on the bar grid, with the day band →</button>
      </div>

      <p class="dspan">
        <span class="k">Days held</span>
        {#if heldWindow(o)}
          <span class="v" title="first_ts → last_ts from /store.json, microseconds, rendered IST"
            >{heldWindow(o)}</span
          >
        {:else}
          <span class="v">—</span>
          <span class="n">{o.absent ? 'no file for this month' : '/store.json carried no first/last stamp for this row'}</span>
        {/if}
      </p>

      <div class="dlist">
        {#each db.openMonths as m (m.key)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="drow"
            data-state={m.absent && m.cover.basis === 'trade' ? 'notrade' : m.cover.state}
            class:this={m.key === db.openKey}
            class:oow={m.cover.notes.includes('outside_window')}
            title={coverTitle(m)}
            onclick={() => (db.openKey = m.key)}
          >
            <span class="dm">{monthLabel(m.month)}</span>
            {#if m.cover.ratio !== null}
              <span class="meter" aria-hidden="true"
                ><span class="fill" style="width:{m.cover.ratio * 100}%"></span></span
              >
            {:else}
              <span class="dword"
                >{m.absent && m.cover.basis === 'trade'
                  ? 'no trade'
                  : (STATE_WORD[/** @type {keyof typeof STATE_WORD} */ (m.cover.state)] ?? m.cover.state)}</span
              >
            {/if}
            <span class="dpct">{sessionsText(m.cover)}</span>
            <span class="dshort">{missingText(m)}</span>
          </div>
        {/each}
      </div>

      <p class="dfoot">
        Each month is measured against the bars the calendar of record owes it at {o.timeframe}
        (/calendar.json, D-3336): the sessions pull::calendar sources, each at its own length — a
        special session by its windows, a suspected halt's minutes neither owed nor missing, a
        CAS-eligible share's auction day at 360. Never against the fullest stored series.
      </p>
    </div>
  {/if}
</div>

{#snippet notesPanel(/** @type {any} */ db)}
  {#if db.noteOpen}
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
        <span class="tag info">{fmt(db.numbered)} of {fmt(db.deco.length)} rows show a number</span>
        <h3>
          <b>% Change</b> and <b>Prev % Change</b> — integer basis points, or a dash that says
          which of five things it is.
        </h3>
        <button class="link" onclick={() => (db.noteOpen = false)}>close</button>
      </div>
      <ol class="whylist">
        {#each db.NOTES as b (b.k)}
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
{/snippet}

<style>
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
  /* ---- THE MONTH WINDOW'S OWN CONTROL, AND WHY NONE OF IT IS LEFT ------
     What stood here styled a value that read as a value and a button that
     opened a calendar. /db has neither. The month control went with the
     from/to range — see the markup note at THE MONTH WINDOW — and both ends
     of the day window are `$lib/DayField.svelte` now.

     `.dcell` WENT WITH THE MARKUP IT POSITIONED. It was the last rule in this
     block still matching anything, and only because /db's own day inputs
     borrowed the class; `DayField` owns that element now and carries its own
     `position: relative`. Its siblings had been dead since the month calendar
     was removed. That is the sentence `26a45dd0` wrote here — and the
     sentence `126152af` then deleted, together with the closing terminator
     that ended it and the `.dwrap` selector line beneath. One contiguous cut —
     and `126152af` made it NINE times across this file. Eight were still open
     when this repair began, and Gate W5 could name only three of them, because
     it prints one finding per comment and seven of the eight are crowded into
     two comments. The terminator cannot be shown to you inside this sentence:
     a CSS comment has no escape, and the star-slash pair ends the comment
     wherever it appears.

     FIVE RULE BODIES WERE ORPHANED, NOT ONE. That cut landed on `.dwrap`,
     `.dval.unset`, `.cal`, `.cmon:disabled` and `.calft`, so everything from
     `.dcell`'s note down to the refusal notice below was a single comment
     body: five rule bodies, thirty-one declarations, emitted by no stylesheet
     since that commit. Gate W5 prints one finding per comment, which is why
     it named only the first of them.

     THEY ARE DELETED RATHER THAN GIVEN THEIR SELECTORS BACK. The selectors
     are recoverable from git; the markup is not. `dwrap`, `dval`, `cal`,
     `calhd`, `calnav`, `calsel`, `calgrid`, `cmon`, `calft`, `dbtn`, `dcell`
     and `dright` occur ZERO times in this file outside comment prose — no
     element, no `class:` directive, no string a class name is built from.
     Restoring them would bring back styling for a control nothing builds,
     which is how a removed control returns without anyone deciding to bring
     it back. Nothing on screen moves: these have painted nothing for as long
     as the defect has existed.
     ---------------------------------------------------------------------- */

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
       scrolls, which is the honest outcome.

       THE FLOOR WAS 220px, AND THAT WAS A SURVIVAL HEIGHT, NOT A USEFUL ONE.
       ---------------------------------------------------------------------
       220 was chosen when NOTHING on this page could scroll: it was the last
       resort that stopped the table disappearing entirely, and at that job it
       worked. It was never a size anyone would choose for the thing the page
       exists to show.

       MEASURED at a 900px viewport, before this line changed: 477px of chrome
       above — two strips, the anchor, the facts row and the window band — left
       the table at its 220px floor with a 201px scrolling area. At 40px a row
       that is FIVE ROWS VISIBLE out of fifty rendered, on a screen tall enough
       for twenty. The table had 22% of the viewport and the controls above it
       had the rest.

       Now that `.board` scrolls (see its own note), the table no longer has to
       fit in what the chrome leaves. The page scrolls to reach whatever will
       not fit, which is the arrangement the pager already assumes, and
       `flex: 1` still lets it take MORE where a tall window offers it.

       THE CEILING WAS 680px AND A REAL WINDOW IS TALLER THAN THAT.
       Measured on the operator's own screen — a maximised window, not the
       900px test viewport — 72vh comes to about 930px and the 680 cap threw
       the difference away: the box stayed letterbox-shaped on a display with
       room for twice the rows. A cap that binds on every real monitor is not a
       safety rail, it is the size.

       `min(80vh, 1100px)` keeps a cap for the case the vh term is written for —
       a very tall or rotated display, where 80vh would otherwise leave no room
       for the pager under it — while letting an ordinary large screen give the
       table the space it obviously has. On a 900px viewport this is 720px, and
       on a 1300px one it is 1040px: about twenty-six rows, which is what a
       reader of a price table is actually looking for.
       --------------------------------------------------------------------- */
    /* AND NOW THE FLOOR IS A ROW COUNT, WHICH IS THE UNIT THE READER ASKED IN.
       ---------------------------------------------------------------------
       Every number above is the history of a box guessing at a height — 220px
       to stop it vanishing, then 680, then min(80vh, 1100px), then no floor at
       all when the box grew to all fifty rows and the page scrolled 2,030px to
       pass them. Each was a PIXEL answer to a question asked in ROWS, which is
       why none of them survived contact: nobody looking at a price table wants
       "eighty percent of the viewport", they want to see twenty-five rows and
       reach the next page without hunting for the button.

       So the floor is written in rows, and it is the same two constants the
       windowing arithmetic uses — `ROW` and `HEAD` in the script — rather than
       a third number that could disagree with them. Twenty-five rows plus the
       sticky header is 1,030px at the current 40px row.

       `flex: none`, AND "STATIC" IS THE WORD THE REQUEST USED. An earlier pass
       here wrote `flex: 1` so a tall window would give the box MORE than 25
       rows. That is a different feature and a worse one: a box that grows with
       the viewport puts the pager somewhere different on every screen, and
       turning pages is the frequent act this layout is being cheap for. 25 rows
       always means the pager is always in the same place.

       IT ALSO BROKE, WHICH IS HOW THE POINT GOT MADE. `flex: 1` here with
       `overflow: hidden` below is a trap worth naming: a flex item's automatic
       minimum size — the rule that stops it shrinking under its own content —
       applies ONLY while overflow is `visible`. `hidden` resolves `min-height:
       auto` to zero, so `flex: 1` shrank this box to the 484px the chrome left
       and the 1,030px scroll box inside it was CLIPPED. Measured: the page
       reported nothing to scroll and the pager sat above the fold, both true,
       both because fourteen rows had been thrown away rather than fitted.
       A layout that reports success by discarding its content is the failure
       wearing a success's clothes that `CLAUDE.md` §4 bans.

       AND IT DOES NOT ALWAYS FIT, WHICH IS THE HONEST PART. `min-height` on a
       flex child is a promise the PARENT pays for: when 25 rows plus the chrome
       exceeds `.board`, the board overflows and the page scrolls the
       difference. Measured on a 1,200px viewport after the trims in this
       commit, the page runs to about 1,550px, so roughly 350 of it scrolls.
       "Twenty-five rows minimum" and "nothing scrolls" are the same constraint
       pulled in opposite directions, and on a short window the row count is the
       one that was asked for. Everything fits outright at a window height of
       about 1,550px.
       --------------------------------------------------------------------- */
    /* THE FLOOR IS ON `.tbl-scroll`, NOT HERE, AND THE FIRST ATTEMPT PUT IT
       HERE AND CAME UP HALF A ROW SHORT. Measured: `.tbl` obeyed 1,030px
       exactly, then spent 17 of them on `.bnotes` and 2 on its own border, so
       the scroll box got 1,011 and showed 24.5 rows under a comment promising
       25. The floor is a statement about how many rows the SCROLLER shows, so
       it belongs on the scroller; this box is then whatever that needs plus its
       own furniture, and a note added under the table later cannot silently
       eat a row. */
    /* ---------------------------------------------------------------------
       AND THE HEIGHT IS THE SCREEN'S TO GIVE, NOT THIS BOX'S TO DEMAND.

       A 25-row box is 1,030px. On a 1,200px window the chrome above leaves
       about 500. Asking for 1,030 of a 500px hole does not fail loudly — it
       hangs 461px of table BELOW THE FOLD and starts a second scrollbar to
       reach it, which is the exact arrangement this page was told twice to
       stop having. MEASURED in that state: two scrollers, and THIRTEEN rows on
       screen under a box whose CSS said twenty-five. The number was true of the
       box and false of the screen, and only the screen counts.

       So the box takes what is left and no more. `flex: 1` against `.board`'s
       remaining height, `min-height` only as a floor against vanishing —
       not as a row count, because a row count is a promise about a viewport
       this rule cannot see. How many rows that comes to is MEASURED at runtime
       and the page size is set from it, so the grid holds exactly the rows that
       fit and the box never scrolls at all. See `rowsThatFit`.

       220px IS A SURVIVAL FLOOR AND NOT A PROMISE. It exists only so a very
       short window degrades to a small table rather than to the nineteen-pixel
       one this file recorded earlier. It is about five rows.

       IT WAS 320 AND THAT TURNED OUT TO BE THE LAST THING HOLDING THE SCROLL
       OPEN. Measured at an 800px window with the readouts already folded away:
       the box sat ON the 320 floor and the page still overflowed by 35px — a
       range far too short to be worth scrolling and long enough to lurch, which
       is the whole complaint. The floor was not protecting a readable table at
       that point, it was buying three rows at the price of the page not
       fitting, which is the wrong side of the trade this page has now made
       three times. Below 220 the page scrolls, which is the honest outcome and
       needs a window shorter than any this was tested on.
       --------------------------------------------------------------------- */
    flex: 1;
    min-height: 220px;
    display: flex;
    flex-direction: column;
    position: relative; /* the drawer's containing block */
    border: 1px solid var(--line);
    border-radius: var(--r4);
    /* `hidden` AGAIN, AND NOW IT IS WANTED RATHER THAN TOLERATED. With the
       rows scrolling inside `.tbl-scroll` once more, the sticky `<th>` should
       pin to THAT box — which is the whole point of a fixed-height table — so
       an ancestor clip here is no longer the trap it was when the header had to
       see the page. It buys back the rounded corners the drawer and the header
       were spilling out of. */
    overflow: hidden;
    background: var(--panel);
    box-shadow: var(--e2);
  }
  .tbl-scroll {
    /* TWENTY-FIVE ROWS AND THE STICKY HEADER — `height`, NOT `min-height`, AND
       BOTH WRONG ANSWERS WERE TRIED FIRST.
       ---------------------------------------------------------------------
       `--dbrow` and `--dbhead` are the same two measurements the windowing
       arithmetic reads as `ROW` and `HEAD`, so the box cannot promise a row
       count the script places differently. That part was right from the start;
       the PROPERTY took two goes.

       `min-height` with `flex: 1` on `.tbl` CLIPPED — `overflow: hidden` there
       zeroes a flex item's automatic minimum size, so the panel shrank to 484px
       and cut fourteen rows off the bottom.
       `min-height` with `flex: none` on `.tbl` OVERGREW — a floor is not a
       ceiling, nothing capped the box, and it stood at 2,030px with all fifty
       rows and an inner scroll range of ZERO. Measured both.

       AND THE THIRD ANSWER IS NEITHER: THE BOX FILLS `.tbl` AND `.tbl` FILLS
       WHAT THE SCREEN LEFT. A fixed `height: calc(25 * var(--dbrow) + ...)`
       stood here and was measured hanging 461px below the fold with a second
       scrollbar to reach it. Any height stated HERE is a guess about a viewport
       this rule cannot see; the only honest source for it is the viewport
       itself, so `.tbl` reads it via `flex` and this box takes all of `.tbl`.

       `min-height: 0` IS BACK AND IS NOW LOAD-BEARING IN THE OTHER DIRECTION:
       a flex child will not shrink under its own content without it, and this
       box's content is fifty rows. It must shrink; the rows are what scrolls —
       except that `rowsThatFit` sets the page size from this box's measured
       height, so in practice there is nothing to scroll and the range is zero.
       The overflow stays `auto` rather than `hidden` so the rare case that
       overruns it — a short window under the 320px floor — degrades to a
       scrollbar instead of to hidden rows. */
    flex: 1;
    min-height: 0;
    /* ---------------------------------------------------------------------
       THIS SCROLLS AGAIN, AND THE ROUND TRIP IS WORTH STATING PLAINLY BECAUSE
       BOTH ARRANGEMENTS ARE DEFENSIBLE AND ONLY ONE ANSWERS THIS PAGE.

       It scrolled originally by ACCIDENT: `.board` could not scroll at all, so
       the rows were the only thing that could, and reaching the last one
       stopped the wheel dead with the pager stranded below. That was fixed
       twice — first by letting the scroll chain out (d34bd03), then by removing
       this scroller entirely so the page held exactly one (8692c60).

       ONE SCROLLER WAS THE RIGHT FIX FOR THE WRONG COMPLAINT. It did make the
       page legible — no more guessing which box the wheel was driving — but it
       bought that by letting the table grow to all fifty rows, 2,030px, which
       put the pager 2,030px below the header. The reader who wants page 6,001
       now scrolls the length of a page they did not want in order to leave it.
       Turning pages is the frequent act on this surface; reading to the bottom
       of one is the rare act. The layout should be cheap for the frequent one.

       So it scrolls, by CHOICE this time, inside a box with a floor written in
       rows — see `.tbl`. Twenty-five rows is enough to read and short enough
       that the pager stays near the fold. The `<th>`s pin to this box, which is
       what a fixed-height table is for.

       THE PAGE IS STILL A SCROLLER TOO, and that is unavoidable rather than a
       regression: 25 rows plus the chrome exceeds a 1,200px window. What has
       changed is that the inner box is now a KNOWN size the reader can learn,
       instead of one that grew with the row count, and the page beneath it
       scrolls ~350px instead of ~1,700px. `overscroll-behavior` is deliberately
       NOT set: the chain from the last row into the page is what makes two
       scrollers survivable, and `contain` is what broke it the first time.
       --------------------------------------------------------------------- */
    overflow: auto;
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
    /* WP-54: the rung cell carries a derived/behind tag, the sessions cell
       reads "held / owed" and the missing cell a store-wide tag, so those
       three are wider than the bare numbers they used to hold. */
    grid-template-columns:
      minmax(224px, 2.2fr) 92px 96px 104px 92px 184px
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
  /* `.pager`, `.pager .of` AND `.pager .of b` ARE GONE WITH THE ELEMENT THEY
     STYLED — see the markup note where the second pager stood. Svelte reports
     an unused selector as a warning rather than an error, so dead rules here
     survive a build and accumulate; three of them for one deleted `<p>` is
     exactly the drift worth not starting. `.pgbar` carries the surviving pager
     and its own rules are untouched. */
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
  /* ---- THE CALENDAR VERDICT (WP-54, D-3336) ---------------------------
     The coverage strip above the grid, the absent and out-of-window rows,
     and the small tags a verdict carries. Severity stays amber; an absent
     month is a hole and wears the strong amber; a row outside the operator's
     window is shaded, never hidden once shown (D-3338). */
  .covsum {
    flex: none;
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2) var(--s5);
    padding: var(--s3) var(--s5);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: var(--panel);
    font-size: var(--fs-sm);
    color: var(--dim);
  }
  .covsum b {
    color: var(--ink);
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
  }
  .covsum .risk b {
    color: var(--warn);
  }
  .covsum .dim {
    color: var(--faint);
  }
  .tag {
    margin-left: var(--s2);
    padding: 0 4px;
    border-radius: 3px;
    font-family: var(--sans);
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: 0.03em;
    background: var(--well);
    color: var(--dim);
    white-space: nowrap;
  }
  .tag.absent,
  .tag.wide,
  .tag.drv.lag {
    background: var(--warn-soft);
    color: var(--warn);
  }
  .tag.info {
    background: var(--info-soft);
    color: var(--info);
  }
  /* A TRADE-DRIVEN MONTH WITH NO TRADE is not a hole: quiet, never amber. */
  .tag.absent.notrade {
    background: var(--well);
    color: var(--faint);
  }
  .trow[data-state='notrade'] .cell,
  .drow[data-state='notrade'] {
    color: var(--faint);
  }
  .tag.oowtag {
    background: var(--n5);
    color: var(--faint);
  }
  .trow.oow,
  .drow.oow {
    background-image: repeating-linear-gradient(
      135deg,
      transparent 0 6px,
      color-mix(in srgb, var(--n7) 22%, transparent) 6px 7px
    );
  }
  .trow[data-state='absent'] .cell.inst,
  .trow[data-state='absent'] .cell {
    color: var(--warn);
  }
  [data-state='absent'] .pip {
    border-color: var(--warn);
    border-style: dashed;
    background: transparent;
  }
  [data-state='trade'] .pip,
  [data-state='unmeasured'] .pip {
    border-color: var(--n7);
    background: transparent;
  }
  [data-state='absent'] .cpct,
  [data-state='absent'] .dshort {
    color: var(--warn);
  }
  .dtf {
    margin-left: var(--s3);
    font-family: var(--mono);
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .dverdict {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    padding: var(--s3) var(--s5);
    border-bottom: 1px solid var(--line);
    font-size: var(--fs-xs);
    line-height: 1.5;
    color: var(--dim);
  }
  .dverdict b {
    color: var(--ink);
  }
  .dvs .k {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    margin: 0 var(--s2);
  }
  .dvs [data-dir='up'] {
    color: var(--up);
  }
  .dvs [data-dir='down'] {
    color: var(--down);
  }
  .dvs .n {
    color: var(--faint);
  }
  .dlife {
    padding-left: var(--s3);
    border-left: 3px solid var(--warn);
  }
  .dword {
    font-size: var(--fs-micro);
    color: var(--faint);
  }
</style>

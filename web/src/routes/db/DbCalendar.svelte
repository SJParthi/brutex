<script>
  /**
   * /db — THE COVERAGE CALENDAR (WP-54; D-3336, D-3335, D-3338).
   *
   * One year of the selected instrument, a row per month and a cell per day,
   * each cell split in two: Zerodha above, GDFL below. A cell says what the
   * calendar of record calls the day (closed, a session, a special session, a
   * suspected halt, a closing-auction day), whether that feed holds a bar on
   * it at the rung the grid shows, and — from `/dayflags.json` — whether the
   * day is flagged (refused or incomplete at import, degraded, a gap, junk,
   * late rows, a refused front). A month outside the operator's Zerodha
   * window is shaded (D-3338). Clicking a day opens it on the bar grid with
   * the day band beside it.
   *
   * WHERE THE DAYS COME FROM. A spot series: `/calendar.json?feed&rung&symbol
   * &segment` per feed, whose `held` digits are that series' own days at that
   * rung — one request per feed for its whole span, kept. A contract:
   * `/calendar.json` keeps no contract's days, so the year's `/dayflags.json`
   * pages (at most four, 92 days each) give both feeds' bar counts per day.
   * The day flags of the year are read for a spot series too, for the marks.
   *
   * Nothing is drawn as held that a source did not say was held: a day no
   * source answered for is drawn unknown, never as a hole and never as held.
   */
  import { untrack } from 'svelte';
  import { feeds } from '$lib/feeds.svelte.js';
  import { ask } from '$lib/ask.js';
  import { refusalFrom } from '$lib/refusal.js';
  import { createPageRequests } from '$lib/page-requests.js';
  import { calendarUrl, readLedger, ledgerDay, ledgerSpan, monthSpan, epochDayOfIso, isoOfEpochDay } from '$lib/calendar-owed.js';
  import { bandOf, dayflagsUrl } from '$lib/db/dayflags.js';
  import { segmentOfKey, istDayOf } from '$lib/completeness.js';
  import { monthLabel, dayLabel } from '$lib/dates.js';
  import { bandInstrument, archiveOf } from './DbDayFlags.svelte';

  /** @type {{ db: any }} */
  let { db } = $props();

  /** The feeds this calendar draws, top half first. */
  const FEEDS = ['zerodha', 'gdfl'];
  const FEED_WORD = /** @type {Record<string, string>} */ ({ zerodha: 'Zerodha', gdfl: 'GDFL' });
  /** A half-cell's state in words, as the legend names it. */
  const HALF_WORD = /** @type {Record<string, string>} */ ({
    hole: 'session, no bar',
    notrade: 'no trade (trade-driven)',
    closed: 'closed',
    extra: 'bars on a closed day',
    nofile: 'no file',
    unknown: 'not answered',
    life: 'before the series begins',
    none: "outside the calendar's span"
  });

  const instrument = $derived(bandInstrument(db.matched ?? []));
  const segment = $derived(segmentOfKey(instrument));
  const contract = $derived(segment === 'FNO' && /-(CE|PE|FUT)$/.test(instrument));
  const underlying = $derived(
    (db.matched ?? []).find((/** @type {any} */ r) => !r.absent && (r.instrument === instrument || instrument.endsWith(`-${r.underlying}`)))
      ?.underlying ?? ''
  );
  /** The spot series whose own days the calendar route can count, or `null`. */
  const series = $derived(!instrument || segment === 'FNO' || !underlying ? null : { symbol: underlying, segment });

  /** Each offered feed and the rung its half shows: the grid's rung on the active feed, else the feed's finest. */
  const lanes = $derived(
    FEEDS.map((wire) => {
      const f = /** @type {any} */ (feeds.all.find((x) => x.wire === wire));
      const rung = wire === feeds.active && db.timeframe ? db.timeframe : f?.rungs?.[0] ?? '';
      // THE DAY CODES ARE THE EXCHANGE'S, OVER THE FEED'S WHOLE SPAN; the
      // held digits are the series' own, over ITS span. A series' own
      // calendar ends at its last bar, so reading codes from it would draw a
      // pull that stopped as "not answered" instead of "session, no bar".
      return {
        wire,
        word: FEED_WORD[wire] ?? wire,
        rung,
        offered: Boolean(f?.offered),
        window: f?.window ?? null,
        url: rung ? calendarUrl(wire, rung, null) : '',
        heldUrl: rung && series ? calendarUrl(wire, rung, series) : ''
      };
    }).filter((l) => l.offered && l.rung)
  );

  /* ---- the two ledgers ------------------------------------------------ */
  /** @type {Map<string, { ledger: any, why: string }>} */
  let ledgers = $state.raw(new Map());
  const ledgerReads = createPageRequests();
  $effect(() => {
    void db.fetchedAt;
    untrack(() => {
      ledgerReads.cancel();
      ledgers = new Map();
    });
  });
  $effect(() => {
    const want = lanes.flatMap((l) => [l.url, l.heldUrl]).filter((u) => u && !untrack(() => ledgers).has(u));
    if (want.length === 0 || db.view !== 'census') return;
    void ledgerReads.run(async (ticket) => {
      for (const url of want) {
        if (!ticket.current()) return;
        const read = await readLedger(url, ask, ticket.signal);
        if (!ticket.current()) return;
        ledgers = new Map(ledgers).set(url, read);
      }
    });
  });
  $effect(() => () => ledgerReads.dispose());

  /* ---- the year ------------------------------------------------------- */
  /** The selection's held span, from the census: first and last stored day. */
  const heldSpan = $derived.by(() => {
    let lo = Infinity;
    let hi = -Infinity;
    for (const r of db.matched ?? []) {
      if (r.absent || r.instrument !== instrument) continue;
      if (r.firstAt !== null) lo = Math.min(lo, istDayOf(r.firstAt));
      if (r.lastAt !== null) hi = Math.max(hi, istDayOf(r.lastAt));
    }
    return Number.isFinite(lo) && Number.isFinite(hi) ? { lo, hi } : null;
  });
  const years = $derived.by(() => {
    let lo = Infinity;
    let hi = -Infinity;
    for (const l of lanes) {
      const L = ledgers.get(l.url)?.ledger;
      if (!L) continue;
      lo = Math.min(lo, L.from);
      hi = Math.max(hi, L.to);
    }
    if (heldSpan) {
      lo = Math.min(lo, heldSpan.lo);
      hi = Math.max(hi, heldSpan.hi);
    }
    if (!Number.isFinite(lo)) return [];
    const a = Number(isoOfEpochDay(lo).slice(0, 4));
    const b = Number(isoOfEpochDay(hi).slice(0, 4));
    return Array.from({ length: b - a + 1 }, (_, i) => a + i);
  });
  let yearPick = $state(0);
  const year = $derived(
    yearPick && years.includes(yearPick)
      ? yearPick
      : heldSpan
        ? Number(isoOfEpochDay(heldSpan.hi).slice(0, 4))
        : years.at(-1) ?? 0
  );

  /* ---- the year's day flags (active feed; both feeds' counts) ---------- */
  /** @type {Map<string, any>} */
  let flags = $state.raw(new Map());
  let flagsWhy = $state('');
  let flagsFor = $state('');
  let flagsLoading = $state(false);
  const flagReads = createPageRequests();
  $effect(() => {
    const feed = feeds.active;
    const inst = instrument;
    const y = year;
    if (!feed || !inst || !y || db.view !== 'census') return;
    const key = `${feed}|${inst}|${y}`;
    if (untrack(() => flagsFor) === key) return;
    flagsFor = key;
    flags = new Map();
    flagsWhy = '';
    flagsLoading = true;
    flagReads.cancel();
    void flagReads.run(async (ticket) => {
      /** @type {Map<string, any>} */
      const got = new Map();
      try {
        for (let page = 0, pages = 1; page < pages; page += 1) {
          const r = await ask(dayflagsUrl(feed, inst, `${y}-01-01`, `${y}-12-31`, page), { cache: 'no-store', signal: ticket.signal });
          if (!ticket.current()) return;
          if (!r.ok) {
            flagsWhy = await refusalFrom('/dayflags.json', r);
            break;
          }
          const band = bandOf(await r.json(), { archive: archiveOf(feed) });
          pages = band.pages;
          for (const c of band.cells) got.set(c.day, c);
          if (ticket.current()) flags = new Map(got);
        }
      } catch (why) {
        if (ticket.current()) flagsWhy = `/dayflags.json could not be read: ${String(/** @type {any} */ (why)?.message ?? why)}`;
      } finally {
        if (ticket.current()) flagsLoading = false;
      }
    });
  });
  $effect(() => () => flagReads.dispose());

  /**
   * One half-cell: what one feed holds on one day.
   *
   * @param {{ wire: string, url: string, heldUrl: string, rung: string }} lane
   * @param {number} day epoch day
   * @returns {{ s: string, code: string, held: number | null }}
   */
  function half(lane, day) {
    const L = ledgers.get(lane.url)?.ledger ?? null;
    const d = L ? ledgerDay(L, day) : null;
    if (!d) return { s: 'none', code: '-', held: null };
    const iso = isoOfEpochDay(day);
    // A SPOT SERIES' DAYS COME FROM ITS OWN CALENDAR'S DIGITS. A day inside
    // the exchange's span and outside the series' own holds no bar of it
    // (its stored span ends there): before its first bar that is its life
    // not begun yet, after its last a session with no bar. Unread is unknown.
    // A CONTRACT'S DAYS COME FROM THE DAY FLAGS: the exchange calendar's
    // digits count NIFTY and BANKNIFTY, never this contract.
    /** @type {number | null} */
    let held = null;
    const own = lane.heldUrl ? ledgers.get(lane.heldUrl)?.ledger ?? null : null;
    if (own) {
      const mine = ledgerDay(own, day);
      if (mine) held = mine.held;
      else if (day < own.from && d.code !== '.') return { s: 'life', code: d.code, held: 0 };
      else held = 0;
    }
    if (contract) {
      const c = flags.get(iso);
      held = c ? (lane.wire === feeds.active ? c.held : c.peer) : null;
      // A FEED WITH NO FILE FOR THE CONTRACT'S MONTH is an answer, not an
      // unread day: Zerodha stores no option at all.
      const why = c ? (lane.wire === feeds.active ? c.heldWhy : c.peerWhy) : null;
      if (held === null && why === 'not_held') return { s: 'nofile', code: d.code, held };
      if (heldSpan && (day < heldSpan.lo || day > heldSpan.hi) && (held === null || held === 0)) {
        return { s: d.code === '.' ? 'closed' : 'life', code: d.code, held };
      }
    }
    if (d.code === '-') return { s: 'none', code: d.code, held };
    if (d.code === '.') return { s: held !== null && held > 0 ? 'extra' : 'closed', code: d.code, held };
    if (held === null) return { s: 'unknown', code: d.code, held };
    // A TRADE-DRIVEN SERIES (a contract, or a one-second share) holds a bar
    // only where a trade printed (D-3336 §2): a session with none is a day
    // nothing traded, never a hole.
    const tradeDriven = contract || (lane.rung === '1s' && segment === 'CASH');
    return { s: held > 0 ? 'held' : tradeDriven ? 'notrade' : 'hole', code: d.code, held };
  }

  const months = $derived.by(() => {
    if (!year) return [];
    return Array.from({ length: 12 }, (_, i) => {
      const month = `${year}-${String(i + 1).padStart(2, '0')}`;
      const { first, last } = monthSpan(month);
      const days = Array.from({ length: last - first + 1 }, (_, k) => first + k);
      const sums = lanes.map((lane) => {
        const L = ledgers.get(lane.url)?.ledger ?? null;
        const own = lane.heldUrl ? ledgers.get(lane.heldUrl)?.ledger ?? null : null;
        if (!L || !own || contract) return null;
        // SESSIONS FROM THE EXCHANGE, FROM THE SERIES' FIRST DAY; DAYS HELD
        // FROM ITS OWN DIGITS: a month after a pull stopped reads 0 of 21.
        const s = ledgerSpan(L, Math.max(first, own.from), last);
        return s.inside > 0 ? { sessions: s.sessions, held: ledgerSpan(own, first, last).heldSessions } : null;
      });
      const oow = lanes.some(
        (l) => l.window && (month < l.window.from.slice(0, 7) || month > l.window.to.slice(0, 7)) && l.wire === 'zerodha'
      );
      return { month, days, sums, oow };
    });
  });

  /** @param {number} day */
  function openDay(day) {
    const iso = isoOfEpochDay(day);
    db.fromDay = iso;
    db.toDay = iso;
    db.view = 'bars';
  }

  /** @param {number} day @param {any[]} halves */
  function cellTitle(day, halves) {
    const iso = isoOfEpochDay(day);
    const f = flags.get(iso);
    const parts = halves.map(
      (h, i) =>
        `${lanes[i]?.word} ${lanes[i]?.rung}: ${h.s === 'held' ? (contract && h.held !== null ? `${h.held} bar(s)` : 'held') : (HALF_WORD[h.s] ?? h.s)}`
    );
    return `${dayLabel(iso)} · ${parts.join(' · ')}${f?.says?.length ? ` — ${f.says.join('; ')}` : ''}`;
  }
</script>

{#if instrument && lanes.length}
  <section class="cal" aria-label="Coverage calendar: one year of {instrument}, both feeds">
    <div class="calhead">
      <span class="calt">Days of <b>{instrument}</b></span>
      <span class="calyears" role="tablist" aria-label="Year">
        {#each years as y (y)}
          <button type="button" role="tab" class="yr" aria-selected={y === year} onclick={() => (yearPick = y)}>{y}</button>
        {/each}
      </span>
      <span class="calkey">
        {#each lanes as l, i (l.wire)}
          <span class="lane">{i === 0 ? 'top' : 'bottom'}: {l.word} {l.rung}</span>
        {/each}
        <span class="k held"><i></i>held</span>
        <span class="k hole"><i></i>session, no bar</span>
        <span class="k notrade"><i></i>no trade (trade-driven)</span>
        <span class="k special"><i></i>special / halt</span>
        <span class="k closed"><i></i>closed</span>
        <span class="k nofile"><i></i>no file</span>
        <span class="k unknown"><i></i>not answered</span>
        <span class="k flag"><i></i>flagged day</span>
      </span>
    </div>
    {#each lanes as l (l.wire)}
      {@const read = ledgers.get(l.url)}
      {#if read && !read.ledger}
        <p class="calwhy">{l.word}: {read.why}</p>
      {/if}
    {/each}
    {#if flagsWhy}<p class="calwhy">{flagsWhy}</p>{/if}
    {#if contract && flagsLoading}<p class="calwhy">reading this contract's days from /dayflags.json…</p>{/if}
    <div class="calgrid">
      {#each months as m (m.month)}
        <div class="calrow" class:oow={m.oow} title={m.oow ? 'outside your window (Zerodha is held for 2018-01 to 2026-09, D-3338)' : undefined}>
          <span class="calm">{monthLabel(m.month)}</span>
          <span class="caldays">
            {#each m.days as day (day)}
              {@const halves = lanes.map((l) => half(l, day))}
              {@const f = flags.get(isoOfEpochDay(day))}
              <button
                type="button"
                class="cd"
                data-code={halves[0]?.code ?? '-'}
                class:flagged={f?.flagged}
                data-tone={f?.flagged ? f.tone : undefined}
                title={cellTitle(day, halves)}
                onclick={() => openDay(day)}
              >
                {#each halves as h, hi (hi)}<i class="h" data-s={h.s}></i>{/each}
              </button>
            {/each}
          </span>
          <span class="calsum">
            {#each m.sums as s, i (i)}
              {#if s}<span class:short={s.held < s.sessions}>{lanes[i]?.word[0]} {s.held}/{s.sessions}</span>{/if}
            {/each}
            {#if m.oow}<span class="oowtag">outside your window</span>{/if}
          </span>
        </div>
      {/each}
    </div>
  </section>
{/if}

<style>
  .cal {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    padding: var(--s4) var(--s5);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: var(--panel);
    box-shadow: var(--e1);
    font-size: var(--fs-xs);
  }
  .calhead {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s3) var(--s5);
  }
  .calt {
    font-size: var(--fs-sm);
    color: var(--dim);
  }
  .calt b {
    color: var(--ink);
    font-family: var(--mono);
  }
  .calyears {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }
  .yr {
    padding: 1px 6px;
    border: 1px solid var(--line);
    border-radius: var(--r2);
    background: var(--panel);
    color: var(--dim);
    font-family: var(--mono);
    font-size: var(--fs-xs);
    cursor: pointer;
  }
  .yr[aria-selected='true'] {
    background: var(--acc-soft);
    border-color: var(--acc);
    color: var(--ink);
  }
  .calkey {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2) var(--s4);
    color: var(--faint);
  }
  .calkey .lane {
    color: var(--dim);
  }
  .calkey i {
    display: inline-block;
    width: 9px;
    height: 9px;
    margin-right: 3px;
    border-radius: 2px;
    vertical-align: -1px;
  }
  .k.held i {
    background: var(--n9);
  }
  .k.hole i {
    background: var(--warn);
  }
  .k.special i {
    background: var(--info);
  }
  .k.closed i {
    background: var(--n5);
  }
  .k.notrade i {
    background: var(--n6);
  }
  .k.nofile i {
    box-shadow: inset 0 0 0 1px var(--n6);
  }
  .k.unknown i {
    background: repeating-linear-gradient(135deg, var(--n5) 0 2px, transparent 2px 4px);
  }
  .k.flag i {
    background: transparent;
    box-shadow: inset 0 0 0 2px var(--down);
  }
  .calwhy {
    color: var(--warn);
    font-family: var(--mono);
  }
  .calgrid {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-x: auto;
  }
  .calrow {
    display: grid;
    grid-template-columns: 5.2rem auto 1fr;
    align-items: center;
    gap: var(--s3);
  }
  .calrow.oow {
    background-image: repeating-linear-gradient(
      135deg,
      transparent 0 6px,
      color-mix(in srgb, var(--n7) 25%, transparent) 6px 7px
    );
  }
  .calm {
    font-family: var(--mono);
    color: var(--dim);
  }
  .caldays {
    display: flex;
    gap: 2px;
  }
  .cd {
    display: flex;
    flex-direction: column;
    width: 13px;
    height: 20px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: 2px;
    background: none;
    cursor: pointer;
    overflow: hidden;
  }
  .cd.flagged {
    border-color: var(--tone, var(--down));
  }
  .cd[data-tone='bad'] {
    --tone: var(--down);
  }
  .cd[data-tone='warn'] {
    --tone: var(--warn);
  }
  .cd[data-tone='info'] {
    --tone: var(--info);
  }
  .h {
    flex: 1;
    display: block;
    background: var(--n4);
  }
  .h + .h {
    border-top: 1px solid var(--panel);
  }
  .h[data-s='held'] {
    background: var(--n9);
  }
  .h[data-s='hole'] {
    background: var(--warn);
  }
  .h[data-s='extra'] {
    background: var(--down);
  }
  .h[data-s='closed'] {
    background: var(--n5);
  }
  .h[data-s='none'],
  .h[data-s='life'] {
    background: transparent;
  }
  .h[data-s='notrade'] {
    background: var(--n6);
  }
  .h[data-s='nofile'] {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--n5);
  }
  .h[data-s='unknown'] {
    background: repeating-linear-gradient(135deg, var(--n5) 0 2px, transparent 2px 4px);
  }
  .cd[data-code='P'] .h[data-s='held'],
  .cd[data-code='H'] .h[data-s='held'] {
    background: var(--info);
  }
  .calsum {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s3);
    font-family: var(--mono);
    color: var(--faint);
  }
  .calsum .short {
    color: var(--warn);
  }
  .oowtag {
    font-family: var(--sans);
    color: var(--faint);
    font-style: italic;
  }
</style>

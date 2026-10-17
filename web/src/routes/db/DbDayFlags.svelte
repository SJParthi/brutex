<script module>
  /**
   * /db — THE DAY BAND (WP-54; D-3335 §3, D-3336, D-3319, D-3320, D-3317).
   *
   * One cell per day of the selection's day window, read from
   * `/dayflags.json`: refused and incomplete import days with the import's
   * own words and the other feed's count beside them, degraded vendor days,
   * late-row deferrals, shared holes, suspected halts, special sessions, the
   * closing-auction phase of a share, Zerodha minute gaps, junk daily rows and
   * refused fronts (DB-016, DB-025, DB-026, DB-028, DB-029, DB-030, DB-031,
   * DB-034, DB-053, DB-G37, DB-G42, DB-G47, DB-G69, DB-G87, DB-G96).
   *
   * The state lives in [`dayFlagsState`], which the shell creates, because
   * three places read it: this band, the bar grid's empty state (DB-016: a
   * day window that holds no bar names each day's reason instead of accusing
   * the store), and the day fields' marks.
   *
   * COST. One request per change of (feed, instrument, window), at most
   * [`DAYFLAGS_PAGE_DAYS`] days: the route is O(days in the page). A window
   * wider than one page asks for its newest page and says so.
   */
  import { createPageRequests } from '$lib/page-requests.js';
  import { ask } from '$lib/ask.js';
  import { refusalFrom } from '$lib/refusal.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { dayLabel } from '$lib/dates.js';
  import { untrack } from 'svelte';
  import {
    bandOf,
    bandSpan,
    dayflagsUrl,
    emptyWindowWhy,
    tally,
    sessionCount,
    DAYFLAGS_PAGE_DAYS
  } from '$lib/db/dayflags.js';
  import { auctionBandSentence, dayFactsUrl } from '$lib/db/notes-labels.js';
  import { keyHref } from '$lib/db/lineage-labels.js';

  /**
   * THE AUCTION RECORDS THIS PAGE HAS READ, by `/sources/day.json` URL: one
   * read per (share, day) opened, for the page's life; the oldest is
   * forgotten past `AUCTION_READS_MAX`. A failed read is remembered as one
   * and said; Read again on the grid reloads the page's store, not this.
   *
   * @type {Map<string, Promise<{ state: 'read', auction: any, found: boolean } | { state: 'failed', why: string }>>}
   */
  const AUCTION_READS = new Map();
  const AUCTION_READS_MAX = 64;

  /**
   * One day's auction record, read once (AH-03, g-ui-dom-vs-json-3).
   *
   * @param {string} url `dayFactsUrl(instrument, day)`
   * @param {string} day `YYYY-MM-DD`
   */
  function readAuctionDay(url, day) {
    const kept = AUCTION_READS.get(url);
    if (kept) return kept;
    /** @type {Promise<{ state: 'read', auction: any, found: boolean } | { state: 'failed', why: string }>} */
    const asked = ask(url, { cache: 'no-store' })
      .then(async (response) => {
        if (!response.ok) return { state: /** @type {const} */ ('failed'), why: await refusalFrom('/sources/day.json', response) };
        const body = await response.json();
        const rec = Array.isArray(body?.days) ? body.days.find((/** @type {any} */ d) => d?.day === day) : undefined;
        return { state: /** @type {const} */ ('read'), auction: rec ? (rec.auction ?? null) : null, found: rec !== undefined };
      })
      .catch((/** @type {any} */ why) => ({ state: /** @type {const} */ ('failed'), why: `/sources/day.json could not be read: ${String(why?.message ?? why)}` }));
    AUCTION_READS.set(url, asked);
    while (AUCTION_READS.size > AUCTION_READS_MAX) AUCTION_READS.delete(/** @type {string} */ (AUCTION_READS.keys().next().value));
    return asked;
  }

  /**
   * `YYYY-MM-DD` moved by `n` days, on the calendar (no time zone).
   *
   * @param {string} iso
   * @param {number} n
   */
  const addDays = (iso, n) =>
    new Date(Date.parse(`${iso}T00:00:00Z`) + n * 86_400_000).toISOString().slice(0, 10);

  /**
   * THE ONE INSTRUMENT A DAY FLAG IS ABOUT, from the rows the selection
   * matches: the exact key when there is one series (a spot series, or one
   * contract), else the underlying's options as a kind (`NSE-FNO-NIFTY`),
   * whose calendar and import days still answer though no one contract's
   * bars can be counted. `''` when the selection spans instruments.
   *
   * @param {any[]} rows
   */
  export function bandInstrument(rows) {
    let one = '';
    let base = '';
    for (const r of rows) {
      if (r.absent) continue;
      if (one === '') one = r.instrument;
      else if (one !== r.instrument) one = '\u0000';
      const parts = String(r.instrument).split('-');
      const kind = parts.length > 2 ? `${parts[0]}-${parts[1]}-${r.underlying ?? parts.slice(2).join('-')}` : '';
      if (base === '') base = kind;
      else if (base !== kind) base = '\u0000';
    }
    if (one !== '' && one !== '\u0000') return one;
    return base !== '' && base !== '\u0000' ? base : '';
  }

  /**
   * The vendor archive's span of `feed`, from `/feeds.json` (D-3338): GDFL's
   * 2018-09-03 to 2026-09-24. `null` for a feed with none.
   *
   * @param {string} feed
   * Which ends BIND travels with it (R3-05 repair 3): a day before a binding
   * first day is outside the vendor's archive, never "not imported".
   *
   * @returns {{ from: string, to: string, from_binds?: boolean, to_binds?: boolean } | null}
   */
  export function archiveOf(feed) {
    const a = /** @type {any} */ (feeds.all.find((f) => f.wire === feed))?.archive;
    return a && typeof a.from === 'string' && typeof a.to === 'string'
      ? { from: a.from, to: a.to, from_binds: a.from_binds, to_binds: a.to_binds }
      : null;
  }

  /**
   * The day band's state, published on `db`.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function dayFlagsState(db) {
    let band = $state.raw(/** @type {ReturnType<typeof bandOf> | null} */ (null));
    let bandWhy = $state('');
    let bandFor = $state('');
    let bandLoading = $state(false);
    const instrument = $derived(bandInstrument(db.matched ?? []));
    const span = $derived(bandSpan(db.fromDay ?? '', db.toDay ?? '', addDays));
    const reads = createPageRequests();

    $effect(() => {
      const feed = feeds.active;
      const inst = instrument;
      const s = span;
      if (!feed || !inst || !s) {
        reads.cancel();
        band = null;
        bandWhy = '';
        bandFor = '';
        bandLoading = false;
        return;
      }
      const url = dayflagsUrl(feed, inst, s.from, s.to);
      if (untrack(() => bandFor) === url) return;
      bandFor = url;
      bandLoading = true;
      bandWhy = '';
      reads.cancel();
      void reads.run(async (ticket) => {
        try {
          const response = await ask(url, { cache: 'no-store', signal: ticket.signal });
          if (!ticket.current()) return;
          if (!response.ok) {
            band = null;
            bandWhy = await refusalFrom('/dayflags.json', response);
            return;
          }
          const body = await response.json();
          if (!ticket.current()) return;
          band = bandOf(body, { archive: archiveOf(feed) });
        } catch (why) {
          if (!ticket.current()) return;
          band = null;
          bandWhy = `/dayflags.json could not be read: ${String(/** @type {any} */ (why)?.message ?? why)}`;
        } finally {
          if (ticket.current()) bandLoading = false;
        }
      });
    });
    $effect(() => () => reads.dispose());

    /** The band's cells, `[]` while nothing has answered. */
    const cells = $derived(band?.cells ?? []);

    /**
     * WHY THE DAY WINDOW HOLDS NO BAR, from the asked days' own records
     * (DB-016), or `null` while the band has not answered for this window.
     */
    const dayEmptyWhy = $derived(
      band && span && !span.clipped ? emptyWindowWhy(cells, db.fromDay, db.toDay, dayLabel) : null
    );

    /**
     * FLAGGED DAYS BY ISO DAY, for the day fields' marks: `(iso) => text`
     * or `null`. Only days the band has read are marked; an unread day is
     * unmarked, never "clean".
     */
    const dayFlagged = $derived.by(() => {
      /** @type {Map<string, string>} */
      const by = new Map();
      for (const c of cells) {
        if (c.flagged) by.set(c.day, `${c.word}${c.says.length ? ` — ${c.says.join('; ')}` : ''}`);
      }
      return (/** @type {string} */ iso) => by.get(iso) ?? null;
    });

    Object.defineProperties(db, {
      dayBand: { get: () => band },
      dayBandCells: { get: () => cells },
      dayBandWhy: { get: () => bandWhy },
      dayBandLoading: { get: () => bandLoading },
      dayBandSpan: { get: () => span },
      dayBandInstrument: { get: () => instrument },
      dayEmptyWhy: { get: () => dayEmptyWhy },
      dayFlagged: { get: () => dayFlagged }
    });
  }

  export { DAYFLAGS_PAGE_DAYS, tally };
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();

  /** The day whose phrases are open, or `''`. */
  let openDay = $state('');

  const legend = $derived(tally(db.dayBandCells));
  const flagged = $derived(db.dayBandCells.filter((/** @type {any} */ c) => c.flagged));
  /* AN AUCTION DAY IS ONE LINE FOR THE WHOLE RUN: every session of a
     CAS-eligible share from 2026-08-03 is one, and listing each would bury
     the days that are faults. The cells still show every one. */
  const listed = $derived(
    flagged.filter((/** @type {any} */ c) => !(c.kind === 'cas' && c.kinds.length === 1))
  );
  const casDays = $derived(flagged.length - listed.length);
  /* SESSIONS ARE THE CALENDAR'S (AH-03, g-ui-dom-vs-json-6): counted from
     each day's calendar code, never from its lead kind, so a closed day
     outside the archive is not a session. */
  const sessions = $derived(sessionCount(db.dayBandCells));

  /* THE CLOSING-AUCTION SENTENCE FOLLOWS THE DAY'S OWN RECORD (AH-03,
     g-ui-dom-vs-json-3). The run line said "the auction print is kept apart
     from trades" for every closing-auction day, while a month imported
     before D-3320's rule holds the print as an ordinary bar and the bar's
     own mark said so. The run line now names the session and says the
     handling is each day's record; opening a closing-auction day of a share
     reads that day's `/sources/day.json` once and words it. */
  const feed = $derived(feeds.all.find((f) => f.wire === feeds.active) ?? null);
  const casAll = $derived(db.dayBandCells.filter((/** @type {any} */ c) => c.kinds.includes('cas')).length);
  const runAuction = $derived(auctionBandSentence(feed, null, { sessions: casAll }));
  const auctionUrl = $derived.by(() => {
    if (!openDay || /** @type {any} */ (feed)?.closing_auction?.handling !== 'print_kept_apart') return null;
    const c = db.dayBandCells.find((/** @type {any} */ x) => x.day === openDay);
    if (!c || !c.kinds.includes('cas')) return null;
    const inst = db.dayBandInstrument;
    return typeof inst === 'string' && inst ? dayFactsUrl(inst, openDay) : null;
  });
  /** @type {{ day: string, url: string, value: any } | null} */
  let auctionGot = $state.raw(null);
  $effect(() => {
    const url = auctionUrl;
    const day = openDay;
    if (!url) {
      auctionGot = null;
      return;
    }
    let live = true;
    auctionGot = { day, url, value: null };
    void readAuctionDay(url, day).then((value) => {
      if (live) auctionGot = { day, url, value };
    });
    return () => {
      live = false;
    };
  });
  /** The opened day's auction sentence, or `null` when none applies. */
  const dayAuction = $derived.by(() => {
    const got = auctionGot;
    if (!got || got.url !== auctionUrl) return null;
    if (got.value === null) return { tone: 'info', state: 'reading', text: `Reading ${dayLabel(got.day)}'s auction record from /sources/day.json…` };
    if (got.value.state === 'failed') return { tone: 'warn', state: 'failed', text: `${dayLabel(got.day)}'s auction record could not be read (${got.value.why}), so whether its print is kept apart is not known.` };
    if (!got.value.found) return { tone: 'warn', state: 'no_record', text: `/sources/day.json returned no record for ${dayLabel(got.day)}, so whether its auction print is kept apart is not known.` };
    return auctionBandSentence(feed, got.value.auction, { day: got.day, read: true });
  });

  /** @param {string} feed */
  const feedWord = (feed) => (feed === 'gdfl' ? 'GDFL' : feed === 'zerodha' ? 'Zerodha' : feed);
</script>

{#if db.dayBandSpan && (db.dayBand || db.dayBandWhy || db.dayBandLoading)}
  <section class="dband" aria-label="What each day of the window is" aria-busy={db.dayBandLoading}>
    <div class="dbhead">
      <span class="dbtitle">
        Days {dayLabel(db.dayBandSpan.from)} → {dayLabel(db.dayBandSpan.to)}
        {#if db.dayBandSpan.clipped}
          <span class="dbclip"
            >· the newest {DAYFLAGS_PAGE_DAYS} days of the window (one request reads at most that many)</span
          >
        {/if}
      </span>
      {#if db.dayBand}
        <span class="dbsum">{sessions} session day(s) · {flagged.length} flagged</span>
        <span class="dblegend">
          {#each legend as k (k.kind)}
            <span class="dbkey" data-tone={k.tone}><i aria-hidden="true"></i>{k.word} {k.n}</span>
          {/each}
        </span>
      {:else if db.dayBandLoading}
        <span class="dbsum">reading /dayflags.json…</span>
      {/if}
    </div>

    {#if db.dayBandWhy}
      <p class="dberr" role="alert">{db.dayBandWhy}</p>
    {/if}

    <!-- A CONTRACT RENAMED AT AN EX-DATE (DB-G72, D-3331): where its series
         continues, from the import's lineage ledger. -->
    <!-- THE DERIVED RUNGS' WITHHELD BUCKETS (DB-G36): a bucket with a
         missing minute is withheld by the fold, counted per month. -->
    {#if db.dayBand?.derive?.length}
      <ul class="dlineage">
        {#each db.dayBand.derive as line, i (i)}<li>{line}</li>{/each}
      </ul>
    {/if}

    {#if db.dayBand?.lineage?.lines.length}
      <ul class="dlineage">
        {#each db.dayBand.lineage.lines as line, i (i)}<li>{line}</li>{/each}
        <!-- WHERE IT CONTINUES, AS A LINK (AH-03): the key the ledger says
             supersedes or continues this one. -->
        {#if db.dayBand.lineage.successor && keyHref(feeds.active ?? '', db.dayBand.lineage.successor, db.timeframe ?? '')}
          <li><a href={keyHref(feeds.active ?? '', db.dayBand.lineage.successor, db.timeframe ?? '')}>Open {db.dayBand.lineage.successor}</a></li>
        {/if}
      </ul>
    {/if}

    {#if db.dayBand}
      <div class="dstrip" role="group" aria-label="One cell per day, oldest first">
        {#each db.dayBandCells as c (c.day)}
          <button
            type="button"
            class="dcell"
            class:open={openDay === c.day}
            data-tone={c.tone}
            data-kind={c.kind}
            title={`${dayLabel(c.day)} · ${c.word}${c.says.length ? ` — ${c.says.join('; ')}` : ''}`}
            onclick={() => (openDay = openDay === c.day ? '' : c.day)}
          ><span class="sr">{dayLabel(c.day)} {c.word}</span></button>
        {/each}
      </div>

      {#if listed.length || casDays}
        <ul class="dlist">
          {#each listed.slice(0, 12) as c (c.day)}
            <li data-tone={c.tone} class:open={openDay === c.day}>
              <b>{dayLabel(c.day)}</b>
              <span class="kw">{c.word}</span>
              <span class="says">{c.says.join(' · ')}</span>
            </li>
          {/each}
          {#if listed.length > 12}
            <li class="more">and {listed.length - 12} more flagged day(s) — each is a cell above; hover it for its reason</li>
          {/if}
          {#if casDays}
            <li data-tone={runAuction.tone} data-auction={runAuction.state}>
              <span class="kw">closing auction</span>
              <span class="says">{runAuction.text}</span>
            </li>
          {/if}
        </ul>
      {/if}

      {#if openDay}
        {@const c = db.dayBandCells.find((/** @type {any} */ x) => x.day === openDay)}
        {#if c}
          <div class="dopen" data-tone={c.tone}>
            <b>{dayLabel(c.day)}</b> · {c.word}
            <span class="counts"
              >{c.held === null ? `this feed holds no file (${c.heldWhy ?? 'not held'})` : `${c.held} bar(s) here`}
              · {c.peer === null ? `${feedWord(c.peerFeed)} holds none` : `${feedWord(c.peerFeed)} holds ${c.peer}`}</span
            >
            {#if c.fills !== null}
              <span class="counts">· {c.fills} zero-volume bar(s), held but not traded</span>
            {/if}
            <ul>
              {#each c.says as s, i (i)}<li>{s}</li>{/each}
            </ul>
            {#if dayAuction}
              <p class="dauction" data-tone={dayAuction.tone} data-auction={dayAuction.state} role="status">{dayAuction.text}</p>
            {/if}
            {#if c.superseded.length}
              <p class="hist">History — refusals an earlier run logged for this day, superseded by the run that closed it:</p>
              <ul class="hist">
                {#each c.superseded as h, i (i)}
                  <li>
                    {h.run === null ? 'a run before the journal recorded runs' : `run ${h.run}${h.commit ? ` (build ${String(h.commit).slice(0, 8)})` : ''}`}{h.at_ms
                      ? `, logged ${new Date(h.at_ms).toISOString().slice(0, 16).replace('T', ' ')} UTC`
                      : ''}: {h.about} — {h.why}
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}
      {/if}

      {#if db.dayBand.notes.length}
        <details class="dnotes">
          <summary>{db.dayBand.notes.length} note(s) from /dayflags.json</summary>
          <ul>
            {#each db.dayBand.notes as n, i (i)}<li>{n}</li>{/each}
          </ul>
        </details>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .dband {
    flex: none;
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    padding: var(--s4) var(--s5);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: var(--panel);
    box-shadow: var(--e1);
    font-size: var(--fs-sm);
  }
  .dbhead {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--s3) var(--s5);
  }
  .dbtitle {
    font-weight: var(--w-semi);
    color: var(--ink);
  }
  .dbclip,
  .dbsum {
    color: var(--dim);
    font-weight: var(--w-reg);
  }
  .dblegend {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2) var(--s4);
    color: var(--dim);
    font-size: var(--fs-xs);
  }
  .dbkey i {
    display: inline-block;
    width: 9px;
    height: 9px;
    margin-right: 4px;
    border-radius: 2px;
    vertical-align: -1px;
    background: var(--tone, var(--n7));
  }
  .dberr {
    color: var(--down);
    font-family: var(--mono);
    font-size: var(--fs-xs);
  }
  .dstrip {
    display: flex;
    flex-wrap: wrap;
    gap: 2px;
  }
  .dcell {
    width: 12px;
    height: 18px;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: 2px;
    background: var(--tone, var(--n4));
    cursor: pointer;
  }
  .dcell.open {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }
  [data-tone='bad'] {
    --tone: var(--down);
  }
  [data-tone='warn'] {
    --tone: var(--warn);
  }
  [data-tone='info'] {
    --tone: var(--info);
  }
  [data-tone='quiet'] {
    --tone: var(--n5);
  }
  [data-tone='ok'] {
    --tone: var(--n7);
  }
  .dlineage {
    list-style: none;
    padding-left: var(--s3);
    border-left: 3px solid var(--info);
    color: var(--ink-2);
  }
  .dlist {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .dlist li {
    padding-left: var(--s3);
    border-left: 3px solid var(--tone, var(--line));
    line-height: 1.45;
    color: var(--dim);
  }
  .dlist li.open {
    background: var(--well);
  }
  .dlist b {
    color: var(--ink);
    font-weight: var(--w-semi);
  }
  .kw {
    margin: 0 var(--s2);
    color: var(--ink-2);
    font-weight: var(--w-semi);
  }
  .says {
    font-size: var(--fs-xs);
  }
  .more {
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .dopen {
    padding: var(--s3) var(--s4);
    border-left: 3px solid var(--tone, var(--line));
    background: var(--well);
    border-radius: var(--r2);
  }
  .dopen .counts {
    color: var(--dim);
    margin-left: var(--s3);
  }
  .dopen ul,
  .dnotes ul {
    margin: var(--s2) 0 0 var(--s5);
    color: var(--dim);
    font-size: var(--fs-xs);
  }
  .hist {
    margin-top: var(--s2);
    color: var(--faint);
    font-size: var(--fs-xs);
  }
  .dauction {
    margin: var(--s2) 0 0;
    color: var(--dim);
    font-size: var(--fs-xs);
  }
  .dauction[data-tone='warn'] {
    color: var(--warn);
  }
  .dnotes summary {
    cursor: pointer;
    color: var(--faint);
    font-size: var(--fs-xs);
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
</style>

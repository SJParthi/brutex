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
    bandInstrument,
    bandExpiry,
    dayflagsUrl,
    emptyWindowWhy,
    tally,
    DAYFLAGS_PAGE_DAYS
  } from '$lib/db/dayflags.js';

  /* `bandInstrument` lives in the plain module so `node --test` reaches it;
     `DbCalendar` imports it from here, as before. */
  export { bandInstrument };

  /**
   * `YYYY-MM-DD` moved by `n` days, on the calendar (no time zone).
   *
   * @param {string} iso
   * @param {number} n
   */
  const addDays = (iso, n) =>
    new Date(Date.parse(`${iso}T00:00:00Z`) + n * 86_400_000).toISOString().slice(0, 10);


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
    /* THE GRID'S RUNG (R3-13 goal 1): the band counts what the grid shows. */
    const timeframe = $derived(typeof db.timeframe === 'string' ? db.timeframe : '');
    const instrument = $derived(bandInstrument(db.matched ?? [], timeframe));
    /* A FAMILY'S ONE EXPIRY, sent only where the band asks the family. */
    const expiry = $derived(bandExpiry(db.matched ?? [], instrument, timeframe));
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
      const url = dayflagsUrl(feed, inst, s.from, s.to, 0, { timeframe, expiry });
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
  /** The lone closing-auction days, split by whether the share's dated flag
      is 1 or no master holds it (SC-CLOSE-8, SC-CLOSE-9). */
  const casLone = $derived(flagged.filter((/** @type {any} */ c) => c.kind === 'cas' && c.kinds.length === 1));
  const casDays = $derived(casLone.filter((/** @type {any} */ c) => c.cas === true).length);
  const casUnheld = $derived(casLone.length - casDays);
  /** Of those, the days whose month keeps the auction print apart (SC-CLOSE-9). */
  const casApart = $derived(
    flagged.filter((/** @type {any} */ c) => c.kind === 'cas' && c.kinds.length === 1 && c.auctionSidecar === true).length
  );
  const sessions = $derived(
    db.dayBandCells.filter((/** @type {any} */ c) => c.kind !== 'closed' && c.kind !== 'outside').length
  );

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
          {#each legend as k (`${k.kind}|${k.word}`)}
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

      {#if listed.length || casDays || casUnheld}
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
            <!-- ONLY DAYS THE SHARE'S DATED FLAG IS 1 (SC-CLOSE-9), and "kept
                 apart" only where a sidecar keeps the prints apart. -->
            <li data-tone="info">
              <span class="kw">closing auction</span>
              <span class="says"
                >{casDays} session(s) of this share are closing-auction days by its dated flag:
                continuous trading ends at 15:15{casApart === casDays
                  ? ' and the auction print is kept apart from trades in the month\'s sidecar'
                  : casApart > 0
                    ? `; ${casApart} of them keep the auction print apart in the month's sidecar, the rest hold it as an ordinary bar`
                    : '; no month holds an auction sidecar, so the print is an ordinary bar here'} (charter §9, D-3320).</span
              >
            </li>
          {/if}
          {#if casUnheld}
            <!-- A FLAG NO MASTER HOLDS IS NEVER AN AUCTION (SC-CLOSE-8). -->
            <li data-tone="info">
              <span class="kw">closing-auction flag not held</span>
              <span class="says"
                >{casUnheld} session(s) of this share have no dated closing-auction flag held, so their
                15:15–15:29 is unverified, neither an auction nor continuous trading (D-3320); the
                notes below say why each master was not read.</span
              >
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
            {#if c.finest !== null}
              <!-- THE GRID'S RUNG IS NOT THE FINEST (R3-13 goal 1): both counts. -->
              <span class="counts">· {c.finest} {c.finestRung} bar(s) beside the {c.rung} count</span>
            {/if}
            {#if c.fills !== null}
              <span class="counts">· {c.fills} zero-volume bar(s), held but not traded</span>
            {/if}
            {#if c.definition}
              <!-- WHICH BAR DEFINITION BUILT THE DAY (R3-13 goal 3). -->
              <span class="counts" class:stale={!c.definition.current}
                >· bar definition {c.definition.value}{c.definition.current ? '' : ` — this build builds ${c.definition.thisBuild ?? 'another'}, labelled until restated`}</span
              >
            {/if}
            <ul>
              {#each c.says as s, i (i)}<li>{s}</li>{/each}
            </ul>
            {#if c.unknown.length}
              <!-- A DAY FIELD THIS PAGE HAS NO NAME FOR (R3-13 goal 5), drawn
                   under the name the ledger wrote it with. -->
              <p class="hist">Day-ledger fields this page does not name yet, as served:</p>
              <ul class="unknown">
                {#each c.unknown as f (`${f.kind}.${f.name}`)}<li><code>{f.kind}.{f.name}</code> = {f.value}</li>{/each}
              </ul>
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
  .dopen .counts.stale {
    color: var(--warn);
  }
  .unknown code {
    font-family: var(--mono);
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

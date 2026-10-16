<script module>
  /**
   * /db — THE QUERY'S EXTREMES (DB-G95, D-3326)
   *
   * The largest and smallest close, the largest high, the smallest low, the
   * widest bar and the heaviest bar of the WHOLE query — every month and every
   * series the selection reaches, after its date and time bounds — each with
   * its IST instant, its series, and a Go to that opens the page holding it
   * and marks the row.
   *
   * # Why it exists
   *
   * `/bars/window.json` folded only the widest range and the heaviest volume,
   * with no instant, and /db printed neither: the widest bar of a month could
   * not be found from the page, and no extreme close was shown at all. The
   * server now folds all six with `{v, t, key}`, and this panel shows them.
   *
   * # What it costs, and when it is paid
   *
   * An extremes fold is a SCAN, so it is asked ONCE per row set — a change of
   * series, rung, months or bounds — never on a page turn or a re-sort, which
   * send `extremes=0` (DB-G83). A whole month's fold is cached on the server
   * by the file's stamp, so the second question over the same months costs a
   * lookup per month. Folded past the server's budget, the panel says the
   * server's own reason. The panel can be hidden, and hidden it asks nothing;
   * the choice is remembered in this browser.
   *
   * Owner: WP-52.
   */
  import { untrack } from 'svelte';
  import { feeds } from '$lib/feeds.svelte.js';
  import { fmt } from '$lib/db/notes-labels.js';
  import { windowQuery, paisaText } from '$lib/db/bars-plan.js';
  import { readWindowPage } from '$lib/database-pages.js';
  import { dayLabel, barTimeLabel } from '$lib/dates.js';

  /** Where this browser remembers whether the panel is open. */
  const OPEN_KEY = 'brutex.db.extremes.open';

  /** Whether the panel was left open; open when nothing is remembered or storage is refused. */
  function rememberedOpen() {
    try {
      return globalThis.localStorage?.getItem(OPEN_KEY) !== '0';
    } catch {
      return true;
    }
  }

  /**
   * The six extremes, in the order the panel draws them: the field on the
   * wire, its label, and how its value reads.
   */
  export const EXTREMES = [
    { field: 'close_max', label: 'Highest close', unit: 'price' },
    { field: 'close_min', label: 'Lowest close', unit: 'price' },
    { field: 'high_max', label: 'Highest high', unit: 'price' },
    { field: 'low_min', label: 'Lowest low', unit: 'price' },
    { field: 'range_max', label: 'Widest bar (high − low)', unit: 'price' },
    { field: 'volume_max', label: 'Heaviest bar (volume)', unit: 'volume' }
  ];

  /**
   * THE EXTREMES' STATE — the fold, asked once per row set, and whether the
   * panel is open. Created by the bar read (`barsRead`), which owns the plan
   * and the bounds it reads.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function extremesState(db) {
    let extremesOpen = $state(rememberedOpen());
    /** @typedef {{ for: string, loading: boolean, value: any, why: string | null, total: number, ms: number }} ExtremesRead */
    let extremesRead = $state.raw(/** @type {ExtremesRead | null} */ (null));

    /** The row set to fold, or `''` when nothing is to be asked. A string, so
        a recomputed plan with the same series, months and bounds asks nothing. */
    const want = $derived(extremesOpen && db.view === 'bars' ? db.barSetKey : '');

    $effect(() => {
      const key = want;
      if (!key) {
        extremesRead = null;
        return;
      }
      const feed = untrack(() => feeds.active);
      const plan = untrack(() => db.barQueryPlan);
      const bounds = untrack(() => db.barBounds);
      if (!feed || !plan || plan.keys.length === 0 || plan.why) {
        extremesRead = null;
        return;
      }
      const controller = new AbortController();
      const started = performance.now();
      extremesRead = { for: key, loading: true, value: null, why: null, total: 0, ms: 0 };
      readWindowPage(
        windowQuery(feed, plan, { ...bounds, sort: 'ts', desc: true, offset: 0, limit: 1, extremes: true }),
        1,
        controller.signal
      )
        .then((answer) => {
          if (controller.signal.aborted) return;
          extremesRead = {
            for: key,
            loading: false,
            value: answer.extremes ?? null,
            why: answer.extremes ? null : (answer.extremes_why ?? 'the server folded no extremes for this query'),
            total: answer.total,
            ms: Math.round(performance.now() - started)
          };
        })
        .catch((why) => {
          if (controller.signal.aborted) return;
          extremesRead = {
            for: key,
            loading: false,
            value: null,
            why: String(why && why.message ? why.message : why),
            total: 0,
            ms: Math.round(performance.now() - started)
          };
        });
      return () => controller.abort();
    });

    /** The fold's values when they answer the row set on screen, else `null`. */
    const extremesNow = $derived(extremesRead && !extremesRead.loading ? extremesRead.value : null);

    /** @param {boolean} open */
    function setExtremesOpen(open) {
      extremesOpen = open;
      try {
        globalThis.localStorage?.setItem(OPEN_KEY, open ? '1' : '0');
      } catch {
        /* A REFUSED STORE IS A PREFERENCE NOT KEPT, never a broken panel. */
      }
    }

    Object.defineProperties(db, {
      extremesOpen: { get: () => extremesOpen, set: (v) => setExtremesOpen(Boolean(v)) },
      extremesRead: { get: () => extremesRead },
      extremesNow: { get: () => extremesNow }
    });
  }

  /**
   * One extreme's value as it reads: a price in rupees from paisa, or a
   * volume in shares.
   * @param {any} x @param {string} unit
   */
  export function extremeText(x, unit) {
    return unit === 'volume' ? `${fmt(x.v)} shares` : paisaText(x.v);
  }
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();

  const several = $derived((db.barQueryPlan?.keys.length ?? 0) > 1);
  const tfSecs = $derived(db.rungSecs);

  /** The instant of an extreme, as the grid prints a bar's. @param {number} t */
  const when = (t) => `${dayLabel(t * 1000)} ${barTimeLabel(t * 1000, tfSecs)}`;
</script>

{#if db.view === 'bars' && (db.barQueryPlan?.keys.length ?? 0) > 0}
  <section class="xt" aria-label="Extremes of this query">
    <div class="xhead">
      <h3>Extremes</h3>
      {#if db.extremesOpen}
        <span class="xsub">
          {#if db.extremesRead?.loading}
            folding every bar of this query…
          {:else if db.extremesRead && !db.extremesRead.why}
            over all {fmt(db.extremesRead.total)} bar(s) of this query — every month and series, after the
            date and time bounds — in {fmt(db.extremesRead.ms)} ms
          {/if}
        </span>
      {:else}
        <span class="xsub">hidden; nothing is folded while it is</span>
      {/if}
      <button class="btn ghost sm" type="button" onclick={() => (db.extremesOpen = !db.extremesOpen)}
        >{db.extremesOpen ? 'Hide' : 'Show extremes'}</button
      >
    </div>
    {#if db.extremesOpen && db.extremesRead}
      {#if db.extremesRead.why}
        <p class="xwhy" role="status">The extremes are not shown: {db.extremesRead.why}</p>
      {:else if db.extremesRead.value}
        <ul class="xfacts">
          {#each EXTREMES as e (e.field)}
            {@const x = db.extremesRead.value[e.field]}
            <li class="xf">
              <span class="xl">{e.label}</span>
              {#if x}
                <b class="xv">{extremeText(x, e.unit)}</b>
                <span class="xw" title="{when(x.t)} IST{x.key ? ` · ${x.key}` : ''}"
                  >{when(x.t)}{#if several && x.key}<span class="xk"> · {x.key}</span>{/if}</span
                >
                <button
                  class="btn ghost sm"
                  type="button"
                  title="Open the page holding this bar, in time order, and mark it."
                  onclick={() => db.jumpToInstant(x.t, when(x.t), x.key)}>Go to</button
                >
              {:else if e.unit === 'volume'}
                <span class="xw" title="An index trades no volume (D-2802, D-3322), so it has no heaviest bar."
                  >n/a — no traded volume</span
                >
              {:else}
                <span class="xw">no bar</span>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </section>
{/if}

<style>
  .xt {
    flex: none;
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: var(--panel-2);
    padding: var(--s3) var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }
  .xhead {
    display: flex;
    align-items: baseline;
    gap: var(--s4);
    flex-wrap: wrap;
  }
  .xhead h3 {
    margin: 0;
    font-size: var(--fs-xs);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }
  .xsub {
    font-size: var(--fs-xs);
    color: var(--faint);
    flex: 1;
  }
  .xwhy {
    margin: 0;
    font-size: var(--fs-xs);
    color: var(--warn);
  }
  .xfacts {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: var(--s3) var(--s5);
  }
  .xf {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--s2) var(--s3);
    min-width: 0;
  }
  .xl {
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    flex-basis: 100%;
  }
  .xv {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-weight: var(--w-semi);
    color: var(--ink);
  }
  .xw {
    font-family: var(--mono);
    font-size: var(--fs-xs);
    color: var(--dim);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .xk {
    color: var(--faint);
  }
  .btn.sm {
    padding: var(--s1) var(--s3);
    font-size: var(--fs-xs);
  }
</style>

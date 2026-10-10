<script>
  /**
   * /db — WHAT THE OPTION CELLS ON THIS PAGE REFUSED, COUNTED ONCE
   *
   * WP-53 (D-3328, D-3329, D-3330). Three lines above the bar grid, drawn
   * only when the page holds option bars:
   *
   * 1. THE TALLY (DB-038): every refusal word the moneyness, spot and greek
   *    cells of this page carry, with its count, and the flags the server set
   *    on priced cells. A refusal used to be the same `—` as "not an option",
   *    so 8.4% of one expiry day's bars read as blanks with no total.
   * 2. ONE SENTENCE PER UNDERLYING WITH NO SPOT FILE (DB-069, DB-G53): the
   *    cause the alias table can state, once per family rather than per cell.
   * 3. AN EXPIRY WHOSE TRADING ENDED BEFORE ITS STATED DAY, AS STORED
   *    (DB-B14): DTE, theta and the IV tenor still run to the stated expiry;
   *    why trading ended is UNVERIFIED.
   */
  import { fmt } from '$lib/db/notes-labels.js';
  import { dayLabel, istDay } from '$lib/dates.js';
  import { parseKey } from '$lib/instrument.js';
  import { GREEK_FLAG } from '$lib/greeks.js';
  import { tallyRows, reasonText, familyNote, endedEarly } from '$lib/db/notes-options.js';

  /** @type {{ db: any }} */
  let { db } = $props();

  const t = $derived(db.optionTally);
  const groups = $derived([
    { label: 'Moneyness', rows: tallyRows(t?.mny ?? new Map()) },
    { label: 'Spot (intrinsic, extrinsic)', rows: tallyRows(t?.spot ?? new Map()) },
    { label: 'IV and greeks', rows: tallyRows(t?.greeks ?? new Map()) }
  ]);
  const flags = $derived(tallyRows(t?.flags ?? new Map()));
  const refused = $derived(groups.reduce((a, g) => a + g.rows.length, 0));

  /** The (exchange, underlying, expiry) of this page's option bars. */
  const expiries = $derived.by(() => {
    /** @type {Map<string, {underlying: string, expiry: string}>} */
    const by = new Map();
    for (const r of db.optionPage ?? []) {
      const p = parseKey(r.instrument);
      if (p.underlying && p.expiry) by.set(`${p.underlying}|${p.expiry}`, { underlying: p.underlying, expiry: p.expiry });
    }
    return [...by.values()];
  });

  /**
   * The newest stored bar of every (underlying, expiry) and of the feed's
   * whole option store, from the census rows already read: ONE pass per
   * store read, then a probe per expiry on the page — never a pass per page.
   */
  const lastStored = $derived.by(() => {
    /** @type {Map<string, number>} */
    const last = new Map();
    let frontier = 0;
    for (const r of db.deco ?? []) {
      if (r.strike === null || typeof r.lastAt !== 'number') continue;
      if (r.lastAt > frontier) frontier = r.lastAt;
      const k = `${r.underlying}|${r.expiry}`;
      if (r.lastAt > (last.get(k) ?? 0)) last.set(k, r.lastAt);
    }
    return { last, frontier };
  });

  const ended = $derived.by(() => {
    const behindFrom = db.ladder?.census?.behindFrom ?? null;
    const frontier = lastStored.frontier ? istDay(lastStored.frontier) : null;
    const out = [];
    for (const e of expiries) {
      const l = lastStored.last.get(`${e.underlying}|${e.expiry}`);
      const said = endedEarly(e.expiry, l ? istDay(l) : null, frontier, behindFrom);
      if (said.ended) out.push({ ...e, last: said.last, days: said.days });
    }
    return out;
  });
</script>

{#if db.view === 'bars' && (db.optionPage?.length ?? 0) > 0}
  <section class="onotes" aria-label="Option cells on this page">
    <p class="tally">
      <b>Option cells on this page</b> · {fmt(t.options)} bar(s) · {fmt(t.named)} with a rung · {fmt(t.priced)} priced{#if t.pending}
        · <span class="pend">{fmt(t.pending)} still being read</span>{/if}{#if refused === 0 && t.pending === 0}
        · nothing refused{/if}
      {#each groups as g (g.label)}
        {#if g.rows.length}
          · {g.label}:
          {#each g.rows as r, i (r.code)}
            <span class="ref" title="{r.code}: {reasonText(r.code)}">{r.word} {fmt(r.n)}</span
            >{i < g.rows.length - 1 ? ', ' : ' '}
          {/each}
        {/if}
      {/each}
      {#if flags.length}
        · flagged:
        {#each flags as f, i (f.code)}
          <span class="flag" title="{f.code}: {/** @type {Record<string, string>} */ (GREEK_FLAG)[f.code] ?? f.code}"
            >{f.code === 'above_rate_tenor' ? 'T = tenor past the 91-day rate' : f.code === 'below_band' ? 'B = below the validated band' : f.code} {fmt(f.n)}</span
          >{i < flags.length - 1 ? ', ' : ' '}
        {/each}
      {/if}
      {#if t.earlier}
        · <span class="age">{fmt(t.earlier)} priced from an earlier spot (the badge on the rung and the intrinsic is its age), oldest {fmt(t.oldest)} s</span>
      {/if}
    </p>
    {#each db.noSpotFamilies ?? [] as f (f.underlying)}
      <p class="fam" title="{fmt(f.bars)} bar(s) of {f.underlying} options on this page refuse no_spot_file">
        {familyNote(f.underlying, { segment: f.segment, key: f.key }, db.aliasAnswers?.get(f.underlying) ?? null, f.months)}
      </p>
    {/each}
    {#each ended as e (`${e.underlying}|${e.expiry}`)}
      <p class="fam">
        <b>{e.underlying} {dayLabel(e.expiry)}: ended early, as stored</b> — no contract of this expiry has a stored
        bar after {dayLabel(e.last)}, {fmt(e.days)} calendar day(s) before its stated expiry. Days to expiry, theta and
        the IV tenor here still run to the stated expiry (D-2831); why trading ended is UNVERIFIED (DB-B14).
      </p>
    {/each}
  </section>
{/if}

<style>
  .onotes {
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    margin: 0 0 var(--s3);
    font-family: var(--mono);
    font-size: var(--fs-micro);
    color: var(--ink-2);
  }
  .onotes p {
    margin: 0;
    line-height: 1.45;
  }
  .ref {
    color: var(--warn);
    font-style: italic;
    cursor: help;
    border-bottom: 1px dashed currentColor;
  }
  .flag,
  .age {
    color: var(--warn);
    cursor: help;
  }
  .pend {
    color: var(--faint);
  }
  .fam {
    color: var(--warn);
  }
</style>

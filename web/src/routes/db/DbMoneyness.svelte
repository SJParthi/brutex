<script module>
  /**
   * /db — THE MONEYNESS RUNG, AND HOW AN OPTION BAR'S SPOT IS TAKEN
   *
   * The Moneyness control on the contract strip, the "Spot" control beside
   * it (exact second, or the last spot at or before t), and the strike chain
   * the Strike control is built from.
   *
   * Moved out of `routes/db/+page.svelte` by WP-00; owned by WP-53, which
   * fixed here:
   *
   * - DB-008: the strike chain is the SELECTION's — this underlying and this
   *   expiry — never a fold of every strike the whole store holds (it read
   *   "1,135 listed · 1 to 19,13,000" for one NIFTY expiry).
   * - DB-075: the Moneyness control was drawn and dead, because a month has
   *   no spot. A rung belongs to a BAR at its own instant (D-3329): the
   *   control keeps the bars whose own rung, answered by `/chain.json` over
   *   the strikes stored by then, is the one picked.
   * - DB-068: an opt-in "last spot at or before t", always shown with its
   *   age, never a later bar (D-3328; the operator's answer 4).
   */
  import { ladderPos, famOf } from '$lib/db/chain.js';
  import { txt, fmt } from '$lib/db/notes-labels.js';
  import { strikeText } from '$lib/db/decoration.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { RUNG_PER_BAR, rungMatches } from '$lib/db/notes-options.js';
  import { rungKey } from '$lib/moneyness.js';
  import { dayLabel, secondLabel } from '$lib/dates.js';
  import Picker from '$lib/Picker.svelte';

  /** The Spot control's two rows (D-3328). */
  const ASOF_ROWS = [
    {
      key: 'exact',
      name: 'Exact second',
      detail: 'default',
      title:
        'The underlying’s close stamped at exactly the option bar’s own instant (D-2807). A second with no underlying bar is named “no spot”, never filled.'
    },
    {
      key: 'prev',
      name: 'Last at or before t',
      detail: 'shows its age',
      title:
        'The underlying’s last bar at or before the option bar’s instant, inside the same IST session — never a later bar, never another month (D-3328). Every cell priced this way shows how many seconds old its spot is.'
    }
  ];

  /**
   * THE STRIKE CHAIN OF THIS SELECTION, AND THE MONEYNESS LADDER AT EACH
   * BAR'S OWN INSTANT — the strike and moneyness rungs' rows, counts,
   * summaries and refusals.
   *
   * Creates no effect, so where the shell calls it does not change behaviour.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function chainState(db) {
    /* ======================================================================
       THE CONTRACT RUNG — THE STRIKE CHAIN, AND THE MONEYNESS LADDER
       ----------------------------------------------------------------------
       MONEYNESS IS A POSITION ON A LADDER, NOT ONE OF THREE WORDS. For a call
       every strike below spot is in the money and further in as you descend —
       ITM-1, ITM-2 … — and every strike above it further out — OTM+1, OTM+2.
       ATM is the single strike nearest spot. `ladderPos` reads the signed
       offset back off the name, so `ITM-10` sorts deeper than `ITM-2`.

       A RUNG IS COUNTED AT AN INSTANT, ALONG THE STRIKES STORED BY THEN
       (D-3329). The server answers it per bar (`/chain.json`); nothing here
       counts one. The chain below is only the Strike control's list.

       THEY STAY TWO CONTROLS. A rung is a distance and a strike is an absolute
       price: `52,000` and `ITM-2` are different questions, and only one of
       them survives a move in spot.
       ====================================================================== */

    /**
     * EVERY STRIKE OF THIS SELECTION'S EXPIRY, ascending, with the sides it
     * holds — the selection's underlying and expiry, from `expiryGated`
     * (DB-008). With no expiry chosen the gate holds every contract row back,
     * so the chain is empty and the Strike control says to choose one.
     *
     * One pass over the gated rows, then O(1) per lookup.
     */
    const chainAll = $derived.by(() => {
      const by = new Map();
      for (const r of db.expiryGated) {
        if (r.strike === null) continue;
        let e = by.get(r.strike);
        if (!e) by.set(r.strike, (e = { strike: r.strike, sides: new Set(), months: 0 }));
        e.sides.add(r.side);
        e.months += 1;
      }
      return [...by.values()].sort((a, b) => a.strike - b.strike);
    });

    /** Strike -> its position in the chain. */
    const chainAt = $derived.by(() => {
      const m = new Map();
      for (let i = 0; i < chainAll.length; i += 1) m.set(chainAll[i].strike, i);
      return m;
    });

    /**
     * THE CHAIN'S OWN GRID, in paisa — the smallest positive gap between two
     * adjacent strikes of THIS expiry, counted, never a table of step sizes
     * per symbol. `null` under two strikes.
     */
    const chainStep = $derived.by(() => {
      let step = null;
      for (let i = 1; i < chainAll.length; i += 1) {
        const gap = chainAll[i].strike - chainAll[i - 1].strike;
        if (gap > 0 && (step === null || gap < step)) step = gap;
      }
      return step;
    });

    /** Whether the feed's store holds any option at all: one pass per store read. */
    const storeOptions = $derived.by(() => {
      let n = 0;
      for (const r of db.deco) if (r.strike !== null) n += 1;
      return n;
    });

    /*
       THE COUNT, COUNTED — and the three states are not two. `—` when nothing
       has been read, `no bars` when it has and this row is empty, a number
       otherwise.
       @param {number} n */
    const barsDetail = (/** @type {number} */ n) => (db.deco.length === 0 ? '—' : n ? `${fmt(n)} bars` : 'no bars');

    /** Bars at each strike, over everything the rungs above this one reached. */
    const strikeBars = $derived.by(() => {
      const c = new Map();
      for (const r of db.expiryGated) {
        if (r.strike !== null) c.set(r.strike, (c.get(r.strike) ?? 0) + r.rows);
      }
      return c;
    });

    /** The strikes this selection reaches — the chain itself now (DB-008). */
    const strikeOffered = $derived(new Set(chainAll.map((c) => c.strike)));

    const strikeRows = $derived(
      chainAll.map((c) => ({
        key: String(c.strike),
        name: strikeText(c.strike),
        detail: barsDetail(strikeBars.get(c.strike) ?? 0)
      }))
    );

    const strikeSummary = $derived(
      chainAll.length === 0
        ? 'No strike here'
        : db.strikePick.size === 0
          ? `Any strike · ${fmt(chainAll.length)} in this expiry`
          : `${strikeText(Number([...db.strikePick][0]))} only`
    );

    /**
     * WHY THE STRIKE PANEL CANNOT BE OPENED, or `null` when it can — a fact
     * about the STORE (no option in it at all), never about the selection:
     * an expiry not yet chosen is said on the control's own clause.
     */
    const strikeRefusal = $derived.by(() => {
      if (storeOptions > 0) return null;
      if (db.deco.length === 0)
        return `nothing has been read for ${db.feedDisplay(feeds.active)} yet, so no instrument name has been looked at — this is an unread store, not a store without options`;
      return `not one of the ${fmt(db.deco.length)} instrument-month(s) ${db.feedDisplay(feeds.active)} holds is named the way this repository names an option. crates/core/src/instrument.rs ends one “-<strike in paisa>-CE” or “-<strike in paisa>-PE”, and nothing in this store ends either way. /store.json sends no strike field, so a strike here can only ever be READ OFF A NAME — never decoded from a spelling this page guessed at.`;
    });

    /* ======================================================================
       THE MONEYNESS LADDER — PER BAR (D-3329, DB-075)
       ====================================================================== */

    /**
     * THE CHAIN AT THE PAGE'S NEWEST INSTANT, as `/chain.json` counted it:
     * the strikes stored by then, ATM and the local step. It is what the
     * control lists; each bar's own rung is asked at its own instant. Of
     * several groups (a page over two rungs), the one counted latest.
     */
    const ladder = $derived.by(() => {
      /** @type {any} */
      let best = null;
      for (const l of db.ladders?.values?.() ?? []) if (!best || l.at > best.at) best = l;
      return best;
    });

    /** Every rung the chain at that instant reaches, on both sides, in ladder order. */
    const ladderRungs = $derived.by(() => {
      const seen = new Set();
      for (const s of ladder?.strikes ?? []) {
        if (s.ce) seen.add(s.ce);
        if (s.pe) seen.add(s.pe);
      }
      /* And every rung a bar on this page actually sits on, so a pick the
         newest instant's chain does not reach is still offered and counted. */
      for (const r of db.optionPage ?? []) {
        const a = db.rungMap?.get(rungKey(r));
        if (a?.rung) seen.add(a.rung);
      }
      return [...seen].sort((a, b) => ladderPos(a) - ladderPos(b) || txt(a, b));
    });

    /** Bars on THIS page at each rung, and at each family. */
    const mnyBars = $derived.by(() => {
      const c = new Map();
      for (const r of db.optionPage ?? []) {
        const k = db.rungMap?.get(rungKey(r))?.rung;
        if (!k) continue;
        c.set(k, (c.get(k) ?? 0) + 1);
        const f = String(famOf(k));
        if (f !== k) c.set(f, (c.get(f) ?? 0) + 1);
      }
      return c;
    });

    /** `n on this page`, or `none on this page` — a count of what was read. */
    const pageDetail = (/** @type {number} */ n) => (n ? `${fmt(n)} on this page` : 'none on this page');

    /**
     * The control's rows: ATM, then the two families (any depth), then every
     * rung in ladder order. A family pick keeps a bar on ANY rung of it — the
     * page's rungs change as it turns, so a family is a word, not a list.
     */
    const mnyRows = $derived.by(() => {
      const out = [
        { key: 'ATM', name: 'ATM', detail: pageDetail(mnyBars.get('ATM') ?? 0), title: 'The stored strike nearest spot at the bar’s own instant.' },
        { key: 'ITM', name: 'ITM · any depth', detail: pageDetail(mnyBars.get('ITM') ?? 0), title: 'Every in-the-money rung, ITM-1 and deeper.' },
        { key: 'OTM', name: 'OTM · any depth', detail: pageDetail(mnyBars.get('OTM') ?? 0), title: 'Every out-of-the-money rung, OTM+1 and further.' }
      ];
      for (const m of ladderRungs) if (m !== 'ATM') out.push({ key: m, name: m, detail: pageDetail(mnyBars.get(m) ?? 0), title: '' });
      for (const m of db.mnyPick) {
        if (!out.some((r) => r.key === m)) out.push({ key: m, name: m, detail: pageDetail(0), title: '' });
      }
      return out;
    });

    const mnySelected = $derived(new Set(db.mnyPick));

    /** Single select: the pick, or nothing. @param {Set<string>} next */
    function onMnyPick(next) {
      const k = [...next].find((x) => !db.mnyPick.has(x)) ?? [...next][0];
      db.mnyPick = k === undefined ? new Set() : new Set([k]);
    }

    /**
     * WHY THE MONEYNESS CONTROL CANNOT BE USED, or `null`. A rung is counted
     * along ONE expiry's chain, so it needs an expiry; it needs no spot here,
     * because every bar's own spot is the server's.
     */
    const moneyRefusal = $derived.by(() => {
      if (strikeRefusal) return `there is no chain to place a rung on — ${strikeRefusal}`;
      if (!db.expiry) return 'choose an expiry first: a rung is counted along one expiry’s chain, at each bar’s own instant';
      if (db.view !== 'bars') return 'a rung belongs to a bar at its own instant, so it is read in the Bars view';
      return null;
    });

    const mnySummary = $derived(
      moneyRefusal ? 'No rung here' : db.mnyPick.size === 0 ? 'Any rung' : `${[...db.mnyPick][0]} only`
    );

    /** What the clause under the control says: the pick against this page, or the ladder. */
    const mnyNote = $derived.by(() => {
      if (moneyRefusal) return moneyRefusal;
      const page = db.optionPage ?? [];
      if (db.mnyPick.size > 0) {
        const pick = [...db.mnyPick][0];
        let kept = 0;
        let unread = 0;
        let refused = 0;
        for (const r of page) {
          const a = db.rungMap?.get(rungKey(r));
          if (!a) unread += 1;
          else if (!a.rung) refused += 1;
          else if (rungMatches(db.mnyPick, a.rung)) kept += 1;
        }
        return `${pick} keeps ${fmt(kept)} of this page’s ${fmt(page.length)} option bar(s), each at its own instant${refused ? `; ${fmt(refused)} have no rung (refused, see the tally)` : ''}${unread ? `; ${fmt(unread)} not read yet` : ''}. The pager counts every bar of the query: the server pages before the rung, so turn pages for more.`;
      }
      if (ladder && !ladder.why && ladder.atm !== null) {
        return `rungs at each bar’s own instant · at ${dayLabel(ladder.at * 1000)} ${secondLabel(ladder.at * 1000)} IST: ${fmt(ladder.n ?? 0)} strike(s) stored by then, ATM ${strikeText(ladder.atm)}, step ${ladder.step === null ? '—' : strikeText(ladder.step)}`;
      }
      return `rungs at each bar’s own instant, over the strikes stored by then (/chain.json)`;
    });

    /* THE SPOT RULE (D-3328). `exact` is the default; `prev` is opted in
       here and changes every spot, rung and greek read on the page. */
    let spotAsof = $state(/** @type {'exact'|'prev'} */ ('exact'));

    Object.defineProperties(db, {
      chainAll: { get: () => chainAll },
      chainAt: { get: () => chainAt },
      chainStep: { get: () => chainStep },
      ladder: { get: () => ladder },
      ladderRungs: { get: () => ladderRungs },
      strikeBars: { get: () => strikeBars },
      mnyBars: { get: () => mnyBars },
      barsDetail: { get: () => barsDetail },
      strikeOffered: { get: () => strikeOffered },
      strikeRows: { get: () => strikeRows },
      mnyRows: { get: () => mnyRows },
      mnySelected: { get: () => mnySelected },
      onMnyPick: { get: () => onMnyPick },
      strikeSummary: { get: () => strikeSummary },
      mnySummary: { get: () => mnySummary },
      mnyNote: { get: () => mnyNote },
      strikeRefusal: { get: () => strikeRefusal },
      moneyRefusal: { get: () => moneyRefusal },
      spotAsof: { get: () => spotAsof, set: (v) => (spotAsof = v === 'prev' ? 'prev' : 'exact') }
    });
  }
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

<!-- MONEYNESS IS A SEPARATE CONTROL AND STAYS ONE. A strike is an
     absolute price and a rung is a distance from spot at one instant; the
     same tick means two different queries and neither is a view of the
     other. -->
{#if db.contractRungs.money}
  <div class="field mcell" class:off={Boolean(db.moneyRefusal)} title={db.moneyRefusal ?? RUNG_PER_BAR}>
    <span class="lab" title={RUNG_PER_BAR}>Moneyness</span>
    <Picker
      single
      filter
      label="rungs"
      disabled={db.moneyRefusal !== null}
      summary={db.mnySummary}
      rows={db.mnyRows}
      selected={db.mnySelected}
      onchange={db.onMnyPick}
    />
    <span class="note" class:warn={Boolean(db.moneyRefusal)} title={db.mnyNote}>{db.mnyNote}</span>
  </div>
  <!-- THE SPOT RULE (D-3328): exact by default, the last spot at or before
       t opted in. It is a control because it changes every spot-derived
       cell on the page, and its age is drawn on every cell it priced. -->
  <div class="field mcell" title={ASOF_ROWS.find((r) => r.key === db.spotAsof)?.title}>
    <span class="lab" title="Which underlying bar an option bar’s spot, rung and greeks are measured from (D-3328).">Spot</span>
    <Picker
      single
      label="spot rules"
      summary={ASOF_ROWS.find((r) => r.key === db.spotAsof)?.name ?? 'Exact second'}
      rows={ASOF_ROWS}
      selected={new Set([db.spotAsof])}
      onchange={(/** @type {Set<string>} */ sel) => (db.spotAsof = [...sel][0] ?? 'exact')}
    />
    <span class="note" class:warn={db.spotAsof === 'prev'}>
      {#if db.spotAsof === 'prev'}
        last underlying bar at or before each option bar, same session, never later — every cell shows its age
      {:else}
        the underlying’s bar at the option bar’s own second, or “no spot” — nothing earlier is used
      {/if}
    </span>
  </div>
{/if}

<style>
  /* Also in DbControls.svelte, where its reasons are written. */
  .field :global(.pmenu .plist input[type='radio']) {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: 0;
    padding: 0;
    opacity: 0;
    pointer-events: none;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  .field :global(.pmenu .plist label:has(input[type='radio']) .pwhy) {
    padding-left: 0;
    order: 1;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  .field :global(.pmenu .plist label:has(input[type='radio']:checked))::after {
    content: '';
    flex: 0 0 auto;
    align-self: center;
    width: 5px;
    height: 10px;
    margin: 0 2px 3px 0;
    border: solid var(--acc);
    border-width: 0 2px 2px 0;
    transform: rotate(43deg);
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  .field :global(.pmenu .plist label:has(input[type='radio']:focus-visible)) {
    outline: 2px solid var(--acc);
    outline-offset: -2px;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  .field :global(.pmenu .plist label.pdis)::after {
    content: none;
  }
  .field :global(.pmenu .plist input[type='checkbox']) {
    width: 14px;
    height: 14px;
    border-radius: 4px;
  }
  .field :global(.pmenu .plist input[type='checkbox']:checked)::after {
    width: 3.5px;
    height: 7px;
    margin-top: -1px;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  :global(.strip) .field {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
    min-width: 0;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  :global(.strip) .field.off > span:first-of-type {
    opacity: 0.55;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  :global(.strip) .field > span:first-of-type {
    font-family: var(--mono);
    font-size: var(--fs-micro);
    font-weight: var(--w-semi);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    line-height: var(--lab-h);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  :global(.strip) .field > .note {
    margin: 0;
    font-family: var(--mono);
    font-size: var(--fs-micro);
    color: var(--faint);
    min-width: 0;
    max-width: 100%;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    white-space: normal;
    line-height: 1.35;
  }
  :global(.strip) .field > .note::before {
    content: '· ';
    color: var(--faint);
  }
  :global(.strip) .field > .note.warn {
    color: var(--warn);
  }
  :global(.strip) .field > .note.warn::before {
    color: var(--warn);
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  :global(.strip) .field.mcell {
    min-width: 0;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  :global(.strip) .field :global(.pbtn) {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* Also in DbControls.svelte, where its reasons are written. */
  :global(.strip) .field :global(.din) {
    width: 100%;
    max-width: none;
    font-size: var(--fs-base);
    padding: 11px 13px;
    border-radius: 9px;
  }
  @media (prefers-reduced-motion: no-preference) {
    /* Also in DbControls.svelte, where its reasons are written. */
    :global(.strip) .field::after {
      transition: transform var(--d-state) var(--ease-out);
    }
  }
</style>

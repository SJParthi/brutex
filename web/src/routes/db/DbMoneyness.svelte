<script module>
  /**
   * /db — THE MONEYNESS RUNG
   *
   * The Moneyness control on the contract strip, and the strike chain and ladder
   * it and the Strike control are built from.
   *
   * Every line below was MOVED out of `routes/db/+page.svelte` by WP-00 (the /db
   * page split). The only edits to a moved line are the ones the move needs: a
   * page-level name that now lives in another file is imported, or, for state,
   * read as `db.<name>`; a plain module exports what it declares; and a callback
   * over such a value marks its parameter `any`, the way the page already did
   * elsewhere. Nothing was fixed in the move: every defect the audit found is
   * still present byte for byte, and is fixed by the package that owns this file
   * next.
   *
   * Owner after WP-00: WP-53 (option cells: own-month pricing, point-in-time moneyness, refusal words).
   */
  import { rungOf, ladderPos, famOf } from '$lib/db/chain.js';
  import { txt, fmt } from '$lib/db/notes-labels.js';
  import { strikeText } from '$lib/db/decoration.js';
  import { feeds } from '$lib/feeds.svelte.js';
  import { NO_SPOT } from '$lib/db/notes-options.js';
  import Picker from '$lib/Picker.svelte';

  /**
   * THE STRIKE CHAIN AND THE MONEYNESS LADDER OVER IT — the strike and moneyness
   * rungs' rows, counts, summaries and refusals.
   *
   * Creates no effect, so where the shell calls it does not change behaviour.
   *
   * @param {any} db the page's shared state object (see the shell)
   */
  export function chainState(db) {
    /* ======================================================================
       THE CONTRACT RUNG — THE STRIKE CHAIN, AND THE MONEYNESS LADDER OVER IT
       ----------------------------------------------------------------------
       MONEYNESS IS A POSITION ON A LADDER, NOT ONE OF THREE WORDS. A strike is
       in or out of the money BY A DISTANCE: for a call every strike below spot
       is in the money and gets further in as you descend — ITM-1, ITM-2, ITM-3 …
       — and every strike above it gets further out — OTM+1, OTM+2, OTM+3. ATM is
       the single strike nearest spot. Collapsing that to three buckets throws
       away the axis a reader trades on: one word cannot answer for a strike one
       step in and a strike six steps in, which are different instruments.

       THE SIGN IS PART OF THE NAME, and it is what makes the order an order.
       `ITM-10` is DEEPER than `ITM-2` because −10 < −2; compared as text `ITM-10`
       lands before `ITM-2` and `ITM-100` lands between them. `ladderPos` reads
       the offset back off the name as a signed NUMBER, so a two-, three- or
       four-digit offset needs no new rule and no new branch.

       NOTHING HERE CARRIES A DEPTH. There is no `CHAIN = 6`, no cap, no band and
       no bound between the chain and the panel: the rungs are counted off the
       strikes the STORE actually names, so a store holding one strike has one
       rung and says so, and a store holding five hundred has five hundred. A
       rung the current selection holds no bar for stays on the ladder and
       reports `no bars`, which is a finding about the window rather than a
       shorter ladder.

       ONE RENDERER, TWO DATA SOURCES. Both panels are `$lib/Picker.svelte` — the
       same control `/markets` and `/ingest` use and the same one this page
       already uses for Segment and Instrument. Everything that differs between
       strike and moneyness is DATA: what the rows are, what the counts are read
       from, what the button says, what the refusal says. A second lookalike
       control is how two panels start the same and end up two pixels and one
       wording apart, and there is not one here.

       THEY STAY TWO CONTROLS. A rung is a distance and a strike is an absolute
       price, and a reader who wants `52,000` and a reader who wants `ITM-2` are
       asking different questions — one of them survives a move in spot and the
       other does not.
       ====================================================================== */

    /**
     * EVERY STRIKE THE STORE NAMES, ascending, with the sides it holds at it.
     *
     * Built from `deco` — the WHOLE store for this feed — and not from the
     * filtered view, for the same reason `monthFull` is: the chain is what exists
     * on disk, and narrowing the table to one month must not quietly shorten the
     * ladder the remaining rows are placed on. What the narrowing changes is the
     * COUNT beside each rung, which is counted separately and against
     * `matchedPreContract`.
     *
     * One pass over the rows, then O(1) per lookup. Recomputed when the store is
     * re-read, never per keystroke and never per scroll frame.
     */
    const chainAll = $derived.by(() => {
      const by = new Map();
      for (const r of db.deco) {
        if (r.strike === null) continue;
        let e = by.get(r.strike);
        if (!e) by.set(r.strike, (e = { strike: r.strike, sides: new Set(), months: 0 }));
        e.sides.add(r.side);
        e.months += 1;
      }
      return [...by.values()].sort((a, b) => a.strike - b.strike);
    });

    /** Strike -> its position in the chain. The rung lookup is a probe, not a scan. */
    const chainAt = $derived.by(() => {
      const m = new Map();
      for (let i = 0; i < chainAll.length; i += 1) m.set(chainAll[i].strike, i);
      return m;
    });

    /**
     * THE CHAIN'S OWN GRID, in paisa — counted, never `sym === 'BANKNIFTY' ? 500
     * : 50`. The smallest positive gap between two adjacent listed strikes is a
     * fact about what is on disk; a table of step sizes per symbol would be this
     * page asserting an exchange's contract specification, which
     * `docs/00-charter.md` sources for nothing.
     *
     * `null` under two strikes: one strike has no gap, and a step invented for it
     * would be a number nobody measured.
     */
    const chainStep = $derived.by(() => {
      let step = null;
      for (let i = 1; i < chainAll.length; i += 1) {
        const gap = chainAll[i].strike - chainAll[i - 1].strike;
        if (gap > 0 && (step === null || gap < step)) step = gap;
      }
      return step;
    });

    /**
     * SPOT, IN PAISA — the price the ladder is drawn around.
     *
     * IT IS `null`, ON EVERY STORE THIS PAGE CAN READ, AND THAT IS A FINDING
     * RATHER THAN A STUB. `/store.json` carries ten fields and none is a price:
     * `chg_bps` is a RATIO — this month's first close over its last, in integer
     * basis points — and a ratio has no scale, so no absolute price can be
     * recovered from it. `/instruments.json`, the other endpoint this page holds,
     * carries `symbol`, `key`, `kind`, `exchange`, `segment`, `universe`,
     * `universes`, `bars` and `href`, and none of those is a price either.
     * `/bars.json` does carry prices — and reading one would be a request per
     * contract and about 81 bytes per bar (see the note panel's last line, where
     * that number is measured) to recover a single figure, which is the exact
     * trade D-0067 refused when it put the two closes in the census instead.
     *
     * A DERIVED AND NOT A LITERAL `null`, on purpose. The day a price reaches
     * this page the anchor is a computation over the rows on hand, not an edit to
     * a constant — and `moneyRefusal` beside it has to be able to say "not
     * measured" rather than "impossible".
     *
     * @type {number | null}
     */
    const spotPaisa = $derived.by(() => {
      /* Nothing to read it from. Every field of every endpoint this page calls
         is enumerated above; when one of them becomes a price, it is read here
         and the ladder below lights up with no other change to this file. */
      return null;
    });

    /**
     * WHICH CHAIN ENTRY IS THE MONEY — the index of the strike nearest spot, or
     * `null` when spot is not on hand.
     *
     * Nearest by absolute distance, and the FIRST of two equidistant strikes, so
     * the answer is the same on every read of the same chain. A ladder whose
     * centre moved between two paints would renumber every rung under the
     * reader's selection.
     */
    const atmIndex = $derived.by(() => {
      if (spotPaisa === null || chainAll.length === 0) return null;
      let at = 0;
      for (let i = 1; i < chainAll.length; i += 1) {
        if (Math.abs(chainAll[i].strike - spotPaisa) < Math.abs(chainAll[at].strike - spotPaisa)) {
          at = i;
        }
      }
      return at;
    });

    /**
     * EVERY RUNG THE CHAIN REACHES, in ladder order: deepest ITM … ITM-1, ATM,
     * OTM+1 … deepest OTM.
     *
     * The union across the sides each strike actually holds, not one rung per
     * strike. A strike is ITM for one side and OTM for the other, so on a chain
     * that runs out of room at one end the union is the wider of the two sides —
     * stating it as the union is true in both cases and "one per strike" is true
     * only on a chain centred exactly on the money.
     *
     * Empty when there is no anchor, which is what disables the control. A rung
     * NAMED without a money to measure from would be this page inventing a
     * distance, and every name on the ladder would be wrong by however far the
     * guess sat from spot.
     */
    const ladderRungs = $derived.by(() => {
      if (atmIndex === null) return [];
      const seen = new Set();
      for (let i = 0; i < chainAll.length; i += 1) {
        for (const side of chainAll[i].sides) seen.add(rungOf(i - atmIndex, side));
      }
      /* SORTED BY SIGNED POSITION, NEVER BY TEXT — see the block header. `txt` is
         the tie-break only, and it can only fire between two names with the same
         offset, which is `ATM` against itself and therefore never. */
      return [...seen].sort((a, b) => ladderPos(a) - ladderPos(b) || txt(a, b));
    });

    /**
     * The rung one stored row sits on, or `null` — no anchor, no contract, or a
     * strike the chain does not list.
     *
     * @param {{ strike: number | null, side: 'CE' | 'PE' | null }} r
     */
    function rungKeyOf(r) {
      if (atmIndex === null || r.strike === null || r.side === null) return null;
      const i = chainAt.get(r.strike);
      return i === undefined ? null : rungOf(i - atmIndex, r.side);
    }

    /* `expiryGated` AND NOT `matchedPreContract`, on both of these. The expiry
       is the rung directly above the strike, so counting a strike over rows the
       gate is holding back would print a bar count the click cannot produce —
       the exact rule the facet block states. With no expiry chosen every strike
       honestly reads `no bars`, because that is what selecting it returns. */

    /** Bars at each strike, over everything the rungs above this one reached. */
    const strikeBars = $derived.by(() => {
      const c = new Map();
      for (const r of db.expiryGated) {
        if (r.strike !== null) c.set(r.strike, (c.get(r.strike) ?? 0) + r.rows);
      }
      return c;
    });

    /** The same count one level up, per rung. Empty while there is no anchor. */
    const mnyBars = $derived.by(() => {
      const c = new Map();
      if (atmIndex === null) return c;
      for (const r of db.expiryGated) {
        const k = rungKeyOf(r);
        if (k !== null) c.set(k, (c.get(k) ?? 0) + r.rows);
      }
      return c;
    });

    /**
     * THE COUNT, COUNTED — and the three states are not two.
     *
     * `—` when nothing has been read, because a store nobody read counted
     * nothing. `no bars` when it HAS been read and this row is empty, because
     * that is a finding about the window and not about the control. A number
     * otherwise. `0 bars` would be the third spelling of the second state and
     * reads as a measurement of the first.
     *
     * @param {number} n
     */
    const barsDetail = (n) => (db.deco.length === 0 ? '—' : n ? `${fmt(n)} bars` : 'no bars');

    /** One row per strike the store names, ascending, priced in rupees. */
    /* THE STRIKES THIS SELECTION ACTUALLY REACHES — the cascade's seventh rung.
     *
     * `chainAll` is the CHAIN: every strike the store lists, in ladder order,
     * folded from `deco`. It has to stay that way — it is the coordinate frame
     * `atmIndex` indexes into and `ladderRungs` measures distance along, and
     * narrowing a frame renumbers every rung under a live moneyness pick.
     *
     * The OFFER is a different question: which of those strikes the rungs above
     * leave. Two values, because they answer two things — the same split
     * `expiryOffered` makes against `expiriesAll`, and for the same reason.
     */
    const strikeOffered = $derived.by(() => {
      const seen = new Set();
      for (const r of db.expiryGated) if (r.strike !== null) seen.add(r.strike);
      return seen;
    });

    const strikeRows = $derived(
      chainAll
        .filter((c) => strikeOffered.has(c.strike))
        .map((c) => ({
          key: String(c.strike),
          name: strikeText(c.strike),
          detail: barsDetail(strikeBars.get(c.strike) ?? 0)
        }))
    );

    /**
     * The ladder as rows, with the FAMILY rows interleaved so that "all of ITM"
     * and "exactly ITM-2" are the same gesture.
     *
     * A FAMILY OF ONE IS ITS RUNG and is drawn once. Printing both would be the
     * page repeating a control it already drew, one row apart — and ATM is always
     * a family of one, by definition.
     *
     * The family key is prefixed `fam:` so it can never collide with a rung name:
     * `ATM` is both a family and a rung, and one string standing for two rows is
     * how a tick lands on the wrong one.
     */
    /* THE RUNGS THIS SELECTION REACHES. `ladderRungs` is the ladder — the frame —
     * and this is which of its rungs the strike above actually leaves. */
    const mnyOffered = $derived.by(() => {
      const seen = new Set();
      for (const r of db.expiryGated) {
        if (db.strikePick.size > 0 && !(r.strike !== null && db.strikePick.has(String(r.strike)))) continue;
        const k = rungKeyOf(r);
        if (k !== null) seen.add(k);
      }
      return seen;
    });

    const mnyRows = $derived.by(() => {
      const out = [];
      for (const f of ['ITM', 'ATM', 'OTM']) {
        const run = ladderRungs.filter((m) => famOf(m) === f && mnyOffered.has(m));
        if (run.length === 0) continue;
        if (run.length > 1) {
          out.push({
            key: `fam:${f}`,
            name: f,
            detail: `${fmt(run.length)} rungs · ${barsDetail(
              run.reduce((a, m) => a + (mnyBars.get(m) ?? 0), 0)
            )}`
          });
        }
        for (const m of run) out.push({ key: m, name: m, detail: barsDetail(mnyBars.get(m) ?? 0) });
      }
      return out;
    });

    /**
     * The selection as the PANEL sees it: the rungs, plus a family key for every
     * run that is wholly taken.
     *
     * A ROW IS TICKED WHEN EVERY KEY UNDER IT IS, so half a family reads as
     * unticked and ticking it takes the rest — never the other way round.
     */
    const mnySelected = $derived.by(() => {
      const s = new Set(db.mnyPick);
      for (const f of ['ITM', 'OTM']) {
        const run = ladderRungs.filter((m) => famOf(m) === f);
        if (run.length > 1 && run.every((m) => db.mnyPick.has(m))) s.add(`fam:${f}`);
      }
      return s;
    });

    /**
     * A family tick, expanded into its whole run — in BOTH directions.
     *
     * The panel hands back a Set; the family keys in it are not selection, they
     * are a gesture. Comparing what the run WAS against what the family key now
     * says is what distinguishes "the reader ticked ITM" from "the reader ticked
     * the last remaining ITM rung, which happens to complete the family", and the
     * second must not then re-add rungs the first would have.
     *
     * @param {Set<string>} next
     */
    function onMnyPick(next) {
      const out = new Set();
      for (const k of next) if (!k.startsWith('fam:')) out.add(k);
      for (const f of ['ITM', 'OTM']) {
        const run = ladderRungs.filter((m) => famOf(m) === f);
        if (run.length < 2) continue;
        const was = run.every((m) => db.mnyPick.has(m));
        const now = next.has(`fam:${f}`);
        if (now && !was) for (const m of run) out.add(m);
        else if (!now && was) for (const m of run) out.delete(m);
      }
      db.mnyPick = out;
    }

    /* WHAT EACH BUTTON SAYS. The count is the reader's own selection against what
       this chain supports — never a label and never a constant. An empty
       selection says what it MEANS rather than what it holds: no join at all, the
       same sentence the `Everything` universe row carries. */
    /* SINGLE SELECT, SO THERE IS NO PLURAL ARM.
     *
     * `Picker` has carried a `single` prop all along — it emits `new Set([key])`
     * and never grows — and these two call sites were the only ones on the page
     * that had not switched it on. The instrument picker already had it. So the
     * page offered single select on one control and multi on two, which is the
     * inconsistency rather than a design.
     *
     * The `size > 1` arm is gone rather than left standing: a branch no input can
     * reach is a branch no test can cover and a mutant that deletes it survives.
     * The Set stays as the storage because that is what `Picker` emits; it now
     * holds at most one key. */
    const strikeSummary = $derived(
      chainAll.length === 0
        ? 'No strike stored'
        : db.strikePick.size === 0
          ? `Any strike · ${fmt(chainAll.length)} listed`
          : `${strikeText(Number([...db.strikePick][0]))} only`
    );

    const mnySummary = $derived(
      ladderRungs.length === 0
        ? 'No ladder'
        : db.mnyPick.size === 0
          ? `Any rung · ${fmt(ladderRungs.length)} on the ladder`
          : `${[...db.mnyPick][0]} only`
    );

    /**
     * WHY THE STRIKE PANEL CANNOT BE OPENED, or `null` when it can.
     *
     * `CLAUDE.md` §4: degrade loudly and NAME THE REASON. "Not yet read" and "the
     * store holds no option" are opposite instructions — one is a wait and the
     * other is a fact about the pull — and one greyed control cannot say both.
     */
    const strikeRefusal = $derived.by(() => {
      if (chainAll.length > 0) return null;
      if (db.deco.length === 0)
        return `nothing has been read for ${db.feedDisplay(feeds.active)} yet, so no instrument name has been looked at — this is an unread store, not a store without options`;
      return `not one of the ${fmt(db.deco.length)} instrument-month(s) ${db.feedDisplay(feeds.active)} holds is named the way this repository names an option. crates/core/src/instrument.rs ends one “-<strike in paisa>-CE” or “-<strike in paisa>-PE”, and nothing in this store ends either way: it holds indices, equities and futures, whose names carry no strike at all. /store.json sends no strike field, so a strike here can only ever be READ OFF A NAME — never decoded from a spelling this page guessed at.`;
    });

    const moneyRefusal = $derived.by(() => {
      if (ladderRungs.length > 0) return null;
      if (chainAll.length === 0)
        return `there is no chain to place a rung on — ${strikeRefusal ?? 'no strike is stored'}. And there would be no ladder even with one, because ${NO_SPOT}`;
      return `${fmt(chainAll.length)} strike(s) are stored and not one of them can be named ITM or OTM, because ${NO_SPOT} The ladder is derived from this chain and nothing else, so it fills itself in the day a spot price reaches this page — no list of rung names is written down anywhere to be edited.`;
    });

    Object.defineProperties(db, {
      chainAll: { get: () => chainAll },
      chainAt: { get: () => chainAt },
      chainStep: { get: () => chainStep },
      spotPaisa: { get: () => spotPaisa },
      atmIndex: { get: () => atmIndex },
      ladderRungs: { get: () => ladderRungs },
      rungKeyOf: { get: () => rungKeyOf },
      strikeBars: { get: () => strikeBars },
      mnyBars: { get: () => mnyBars },
      barsDetail: { get: () => barsDetail },
      strikeOffered: { get: () => strikeOffered },
      strikeRows: { get: () => strikeRows },
      mnyOffered: { get: () => mnyOffered },
      mnyRows: { get: () => mnyRows },
      mnySelected: { get: () => mnySelected },
      onMnyPick: { get: () => onMnyPick },
      strikeSummary: { get: () => strikeSummary },
      mnySummary: { get: () => mnySummary },
      strikeRefusal: { get: () => strikeRefusal },
      moneyRefusal: { get: () => moneyRefusal }
    });
  }
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

<!-- MONEYNESS IS A SEPARATE CONTROL AND STAYS ONE. A strike is an
     absolute price and a rung is a distance from spot; the same tick
     means two different queries and neither is a view of the other. -->
{#if db.contractRungs.money}
  <div class="field mcell" class:off={Boolean(db.moneyRefusal)} title={db.moneyRefusal ?? undefined}>
    <span class="lab"
      title="A distance measured along that chain — the same 35,000 is ITM-2 for a call and OTM+2 for a put, which is why it is its own control."
      >Moneyness</span
    >
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
    <!-- THE REFUSAL IS SAID ONCE. When there is no chain the ladder's
         reason STARTS with the chain's, so the clause narrows to the part
         that is still true after a chain arrives. -->
    <span class="note" class:warn={Boolean(db.strikeRefusal || db.moneyRefusal)}>
      {#if db.strikeRefusal}
        No ladder — a rung is a distance measured along a chain, so there is nothing to place
        one on. There would be none with a chain either: {NO_SPOT}
      {:else if db.moneyRefusal}
        The ladder is not drawn — {db.moneyRefusal}
      {:else}
        {fmt(db.ladderRungs.length)} rung(s), {db.ladderRungs[0]} to {db.ladderRungs[
          db.ladderRungs.length - 1
        ]} — every position this chain reaches on both sides of the money
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

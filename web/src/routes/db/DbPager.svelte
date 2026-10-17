<script module>
  /**
   * /db — THE PAGER (R3-03)
   *
   * How a reader reaches the rows that are not on screen, under either grid:
   * the readout, Rows per page, the page box, Go to an instant and the page
   * buttons. Moved out of `DbBarsGrid.svelte`, whose `pager` snippet now
   * renders this, so later packages do not queue on the 3,667-line grid. Its
   * arithmetic is `$lib/db/pager.js` and `$lib/db/goto.js`; this file draws.
   *
   * Owner: R3-03.
   */
  import Picker from '$lib/Picker.svelte';
  import { fmt } from '$lib/db/notes-labels.js';
  import { PAGE_SIZES } from '$lib/db/bars-plan.js';
  import { BAR_COLS } from '$lib/db/notes-bars.js';
  import { pageItems, parsePageBox, stepServed, lastServed } from '$lib/db/pager.js';
</script>

<script>
  /** @type {{ db: any }} */
  let { db } = $props();

  /** What the page box refused, said under it until the next try or the next page. */
  let boxWhy = $state('');
  $effect(() => {
    void db.pageNow;
    boxWhy = '';
  });

  /** The page box's text, read when the reader commits it. @param {HTMLInputElement} el */
  function commitBox(el) {
    const got = parsePageBox(el.value, db.served);
    if (!got.ok) {
      boxWhy = got.why;
      return;
    }
    boxWhy = '';
    db.goPage(got.page);
  }

  const last = $derived(lastServed(db.served));
  const next = $derived(stepServed(db.served, db.pageNow, 1));
  const prev = $derived(stepServed(db.served, db.pageNow, -1));
  /** The page whose rows are drawn, and the one being opened when they differ. */
  const opening = $derived(db.view === 'bars' && db.barState?.loading && db.pageNow !== db.pageShown ? db.pageNow : 0);
</script>

<!-- ==================================================================
     THE PAGER — how you reach the rows that are not on screen.

     IT GOVERNS BOTH GRIDS, so `1–50 of 8,250 rows` means the same thing
     under either one and the arithmetic is written once.

     THE READOUT IS THE ROWS ON SCREEN (R3-03). It printed the page ASKED
     for the moment Next was pressed, above the rows of the page before,
     until the answer came; it now prints the page the rows belong to, and
     says which page is opening while one is.

     IT OFFERS ONLY PAGES THE SERVER SERVES (R3-03). Under an order the
     server reads rather than indexes, pages past its depth are refused
     `sort_too_deep`; they are neither drawn, nor reached by Next or Last,
     nor accepted in the box, and the stretch says why.

     EVERY DISABLED BUTTON SAYS WHY, on its own title. A greyed control
     with no reason is the silent degradation §4 bans.
     ================================================================== -->
<div class="pgbar" role="group" aria-label="Paging">
  <span class="pgof">
    {#if db.pageTotal === 0}
      <b>0</b> rows — {db.view === 'bars'
        ? 'no bar was read, so there is no page to turn'
        : 'nothing matched, so there is no page to turn'}
    {:else}
      <b>{fmt(db.pageFrom)}–{fmt(db.pageTo)}</b> of <b>{fmt(db.pageTotal)}</b> rows · page
      <b>{fmt(db.pageShown)}</b> of <b>{fmt(db.pageCount)}</b>{#if opening}{` · opening page ${fmt(opening)}…`}{/if}
    {/if}
  </span>

  <!-- `Picker`, single, with each row's own sentence on its `title`; `why`
       is for a refusal and none of these is one. -->
  <div class="pgsize">
    <span>Rows per page</span>
    <!-- NO "Fit · 50" BEFORE "Fit · 9" (R3-03, DB-N02). The page opens on a
         declared 50 until the box is measured, and the summary printed that
         50 under the word Fit for the moment between — a count Fit never
         chose. Until the measurement lands the summary says so. -->
    <Picker
      single
      label="page sizes"
      summary={db.sizeMode === 'fit' ? (db.fitLatched ? `Fit · ${fmt(db.pageSize)}` : 'Fit · measuring') : fmt(db.pageSize)}
      title="How many rows this page holds. The reader's place is kept across a change: the row you were standing on stays on screen."
      rows={[
        {
          key: 'fit',
          name: 'Fit',
          detail: db.fitLatched ? `${fmt(db.rowsThatFit)} row(s)` : 'measuring',
          title: db.fitLatched
            ? `As many rows as this window can SHOW — measured at ${fmt(db.rowsThatFit)} below the grid's own two heading rows. The grid then holds nothing the screen cannot, so the table does not scroll. Re-measured when the window changes.`
            : 'As many rows as this window can show — the grid box has not been measured yet, so no count is claimed.'
        },
        ...PAGE_SIZES.map((n) => ({
          key: String(n),
          name: fmt(n),
          detail: n >= 1000 ? `${fmt(n * BAR_COLS.length)} cells` : undefined,
          title:
            n >= 1000
              ? `${fmt(n)} rows in one document. On the bar grid that is ${fmt(n * BAR_COLS.length)} cells — legible, and slow to lay out.`
              : db.fitLatched && n > db.rowsThatFit
                ? `${fmt(n)} rows into a box that can show ${fmt(db.rowsThatFit)}. The remainder scrolls INSIDE the table, which is a second scrollbar — deliberate here, and the reason Fit exists.`
                : undefined
        }))
      ]}
      selected={new Set([db.sizeMode === 'fit' ? 'fit' : String(db.pageSize)])}
      onchange={(/** @type {Set<string>} */ sel) => {
        const key = [...sel][0];
        if (key === 'fit') {
          /* No `setPageSize` here: the effect owns the value in this mode. */
          db.sizeMode = 'fit';
          return;
        }
        const n = Number(key);
        if (Number.isFinite(n) && n > 0) {
          db.sizeMode = 'fixed';
          db.setPageSize(n);
        }
      }}
    />
  </div>

  <!-- THE PAGE BOX IS TEXT AND IS READ IN WORDS (R3-03). A number box hands
       over '' for `2.5e`, `-` and most nonsense and clamped the rest without
       a word; `0`, `-5`, `2.5`, `1e308`, an empty box and 6,000 digits are
       now each refused under the box, and a page past the end or inside a
       refused stretch is refused naming the pages there are. -->
  <label class="pgjump">
    <span>Go to page</span>
    <input
      type="text"
      inputmode="numeric"
      autocomplete="off"
      value={String(db.pageShown)}
      disabled={db.pageTotal === 0}
      aria-invalid={boxWhy ? 'true' : undefined}
      title={db.pageTotal === 0
        ? 'There is no page to go to: nothing is on this grid.'
        : `A page from 1 to ${fmt(last)}${db.served.tail !== null ? ` (pages ${fmt(db.served.front + 1)} to ${fmt(db.served.tail - 1)} are not served)` : ''}. Press Enter.`}
      onkeydown={(e) => {
        if (e.key === 'Enter') commitBox(e.currentTarget);
      }}
      onchange={(e) => commitBox(e.currentTarget)}
    />
    <span class="pgof2">of {fmt(db.pageCount)}</span>
  </label>
  {#if boxWhy}<span class="pgof2 pgwhy" role="status">{boxWhy}</span>{/if}

  {#if db.view === 'bars'}
    <!-- GO TO AN INSTANT, ACROSS THE WHOLE QUERY (DB-005, R3-03). A day and
         a time, to the second, or a time alone for the day on screen; a
         minute lands on its FIRST bar in either direction. -->
    <label class="pgjump pgat">
      <span>Go to</span>
      <input
        type="text"
        inputmode="numeric"
        autocomplete="off"
        maxlength="64"
        placeholder={db.secondRung ? '2019-03-15 10:02:07' : '2019-03-15 10:02'}
        disabled={db.pageTotal === 0}
        title="An instant to open the page of: a day and a time, 2019-03-15 10:02 or 10:02:07, or a time alone for the day on screen. A minute lands on its first bar; an instant on the first bar at or after it — in either direction, across every month and series of the query — and marks it. Press Enter."
        onkeydown={(e) => {
          if (e.key === 'Enter') db.jumpToText(e.currentTarget.value);
        }}
      />
    </label>
    {#if db.jumpWhy}<span class="pgof2 pgwhy">{db.jumpWhy}</span>{/if}
  {/if}

  <span class="pgnav">
    <button
      class="pg"
      type="button"
      disabled={db.pageNow === 1}
      title={db.pageNow === 1 ? 'Already on the first page' : 'First page'}
      aria-label="First page"
      onclick={() => db.goPage(1)}>«</button
    >
    <button
      class="pg"
      type="button"
      disabled={prev === null}
      title={prev === null ? 'Already on the first page' : prev === db.pageNow - 1 ? `Page ${fmt(prev)}` : `Page ${fmt(prev)}, the last page served before the refused stretch`}
      aria-label="Previous page"
      onclick={() => prev !== null && db.goPage(prev)}>‹</button
    >
    {#each pageItems(db.pageNow, db.served) as item (item.k)}
      {#if item.dead}
        <!-- A STRETCH THE SERVER REFUSES: not a control, and its reason is
             on its title (R3-03). -->
        <span class="pg gap dead" title={db.served.why ?? ''} aria-label="Pages not served: {db.served.why ?? ''}">⋯</span>
      {:else if item.gap}
        <!-- ELIDED, AND IT IS NOT A CONTROL: the page box reaches the pages
             this row does not draw. -->
        <span class="pg gap" aria-hidden="true">…</span>
      {:else}
        <button
          class="pg"
          type="button"
          aria-current={item.p === db.pageNow ? 'page' : undefined}
          title="Page {fmt(item.p)} of {fmt(db.pageCount)}"
          onclick={() => db.goPage(item.p)}>{fmt(item.p)}</button
        >
      {/if}
    {/each}
    <button
      class="pg"
      type="button"
      disabled={next === null}
      title={next === null
        ? db.pageCount === 1
          ? 'There is only one page'
          : db.pageNow >= last && last < db.pageCount
            ? `No later page is served: ${db.served.why}`
            : 'Already on the last page'
        : next === db.pageNow + 1
          ? `Page ${fmt(next)}`
          : `Page ${fmt(next)}, the first page served after the refused stretch`}
      aria-label="Next page"
      onclick={() => next !== null && db.goPage(next)}>›</button
    >
    <button
      class="pg"
      type="button"
      disabled={db.pageNow === last}
      title={db.pageNow === last
        ? db.pageCount === 1
          ? 'There is only one page'
          : last < db.pageCount
            ? `This is the last page served: ${db.served.why}`
            : 'Already on the last page'
        : last < db.pageCount
          ? `Page ${fmt(last)}, the last page served — ${db.served.why}`
          : `Last page, ${fmt(last)}`}
      aria-label="Last page"
      onclick={() => db.goPage(last)}>»</button
    >
  </span>
  {#if db.view === 'bars' && db.served.why && db.pageTotal > 0}
    <span class="pgof2 pgwhy">Pages {fmt(db.served.front + 1)} to {fmt(db.served.tail === null ? db.pageCount : db.served.tail - 1)} are not offered: {db.served.why}</span>
  {/if}
</div>

<style>
  /* ---- THE PAGER ---- */
  .pgbar {
    flex: none;
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s3) var(--s5);
    padding: var(--s4) var(--s5);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    background: var(--panel-2);
    /* ---------------------------------------------------------------------
       NOT STICKY, AND THE STICKY VERSION IS WHY THE PAGE SHOOK.

       It was pinned here — `position: sticky; bottom: calc(-1 * var(--s5))` —
       to answer a real complaint: 25 rows in a fixed box put this bar 553px
       below the fold, and pinning it meant the reader never had to travel to
       reach the next page. It worked, and it was the wrong layer to fix it at.

       A STICKY ELEMENT INSIDE A PADDED SCROLLER ALWAYS SNAPS BY THE PADDING.
       `.board` carries 12px of it. Stuck, this bar held at the scrollport's
       edge; released at the end of the scroll, it settled 12px higher.
       MEASURED on a 980px window with a 56px scroll range: the bar sat still
       through 55 pixels of scrolling and then JUMPED TWELVE at the last one.
       Cross that threshold with the wheel and the whole bottom of the page
       shakes, which is exactly what the operator reported and exactly what a
       control the eye uses as a fixed landmark must never do.

       AND IT WAS ALREADY REDUNDANT. `Fit` sizes the grid to what the window can
       show, so the page fits and this bar is on screen without help — the
       problem the pin was hired for no longer exists. What remained was its
       side effect. A workaround kept after its cause is fixed is strictly worse
       than no workaround, because it now only does the harm.

       WHAT HAPPENS ON A WINDOW TOO SHORT FOR `Fit`, said plainly: the table
       reaches its 320px floor, the page overflows by a few dozen pixels, and
       this bar scrolls with everything else. That is a short, honest scroll to
       a bar that does not move relative to the thing above it, and it is the
       better of the two failures. `Compact` recovers ~180px and removes even
       that. */
  }
  .pgof,
  .pgof2 {
    font-size: var(--fs-xs);
    color: var(--faint);
    font-variant-numeric: tabular-nums;
  }
  .pgof b {
    color: var(--ink-2);
    font-family: var(--mono);
    font-weight: var(--w-semi);
  }
  .pgsize,
  .pgjump {
    display: inline-flex;
    align-items: center;
    gap: var(--s3);
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  /* BIG ENOUGH TO TYPE IN. A 46px box with 11px digits is a control that
     technically accepts a page number; this one is legible at arm's length
     and its own spinner is not the only way to reach page 37. */
  .pgjump input {
    width: 92px;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    color: var(--ink);
    font: inherit;
    font-family: var(--mono);
    font-size: 15px;
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    padding: 10px 12px;
    text-align: right;
  }
  .pgat input {
    width: 200px;
    text-align: left;
  }
  .pgwhy {
    flex-basis: 100%;
  }
  .pgjump input:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .pgnav {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
    margin-left: auto;
  }
  .pg {
    appearance: none;
    min-width: 34px;
    height: 32px;
    padding: 0 var(--s3);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    background: var(--panel);
    color: var(--dim);
    font: inherit;
    font-family: var(--mono);
    font-size: var(--fs-sm);
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    cursor: pointer;
  }
  .pg:hover:not(:disabled) {
    color: var(--ink);
    border-color: var(--dim);
  }
  .pg:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .pg:disabled {
    opacity: 0.34;
    cursor: not-allowed;
  }
  .pg[aria-current='page'] {
    background: var(--acc);
    border-color: var(--acc);
    color: var(--on-acc);
  }
  .pg.gap {
    border: 0;
    background: none;
    cursor: default;
    color: var(--faint);
    min-width: 18px;
    padding: 0;
  }
  .pg.gap.dead {
    color: var(--warn);
    cursor: help;
  }
</style>

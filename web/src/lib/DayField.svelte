<script module>
  /**
   * WHICH CALENDAR IS OPEN — one number for the whole document.
   *
   * The identical problem `$lib/Picker.svelte` solves at the top of its own
   * file, solved the identical way on purpose: two grids could be open at once
   * because each owned its own flag and closed on an outside click, and a
   * click on ANOTHER field's button is not outside `.dcell` — it is inside
   * that one.
   *
   * Holding the identity of the single open field makes "two are open" a state
   * this component cannot reach, rather than a rule every call site has to
   * remember. O(1), and nothing to forget.
   */
  let openId = $state(0);
  let issued = 0;
  const nextId = () => ++issued;
</script>

<script>
  /**
   * A DAY, TYPED OR PICKED, IN THE PRODUCT'S OWN FORMAT.
   *
   * # Why this exists rather than `<input type="date">`
   *
   * /db drew its day window with the platform control and /ingest drew its
   * own, so one field read `31/07/2026` on one page and `31 Jul 2026` on the
   * other. That is not only an inconsistency. `<input type="date">` renders in
   * the OS locale, so `02/09/2024` is the 2nd of September to one reader and
   * the 9th of February to another, and no rule in either page can reach its
   * text or its popup to say which. /db's own source has carried a comment
   * saying exactly that — "The product's OWN calendar, never the platform's
   * ... There is not one on this page and there must never be" — for longer
   * than the `<input type="date">` sitting twenty lines above it.
   *
   * `dd Mon yyyy` cannot be misread in any locale, which is why
   * `$lib/dates.js` renders every other date in this product that way. This
   * makes the EDITABLE day agree with the displayed one.
   *
   * # The value is ISO and the text is not
   *
   * `value` in and out is always `yyyy-mm-dd` or `''`. The `dd Mon yyyy` form
   * is a rendering held in local state while it is being typed, so a redraw
   * driven by the selection cannot overwrite what is being typed — the second
   * of the two rules `Picker` states, and the same hazard.
   *
   * # What it never does
   *
   * It does not clamp. A day outside `min`/`max` is REFUSED and the field says
   * so; it is not quietly moved to the nearest legal one, because a control
   * that silently answers a different question than the one asked is the
   * `CLAUDE.md` §4 fallback that hides a failure. Out-of-range days are drawn
   * struck through and are not clickable, and typing one leaves `aria-invalid`
   * on the input with the bound stated under the grid.
   */
  import { MON, istMonth } from '$lib/dates.js';

  let {
    /** `yyyy-mm-dd`, or `''` for "no bound". The caller owns it. */
    value = '',
    /** Called with the next `yyyy-mm-dd`, or `''` when the field is cleared. */
    onchange,
    /** Visible label above the field. */
    label = 'Date',
    /** Earliest selectable day, `yyyy-mm-dd`. `''` means no floor. */
    min = '',
    /** Latest selectable day, `yyyy-mm-dd`. `''` means no ceiling. */
    max = '',
    /**
     * Why `min`/`max` are where they are — on the opener's `title` and under
     * the grid. The BOUND is mechanical; the REASON is the caller's, because
     * the caller is the only thing that knows it.
     */
    boundsReason = '',
    disabled = false
  } = $props();

  const id = nextId();
  const open = $derived(openId === id);

  /* ---- ISO helpers -----------------------------------------------------
     All arithmetic is on the calendar, never on a `Date`. A `Date` carries a
     timezone and every day in this product is an IST calendar day with no
     instant attached; `new Date(y, m, d)` would shift the day for any reader
     west of IST, which is the whole class of bug this component removes. */

  /** @param {string} iso @returns {[number, number, number]} */
  const partsOf = (iso) => [+iso.slice(0, 4), +iso.slice(5, 7), +iso.slice(8, 10)];
  /** @param {number} y @param {number} m @param {number} d */
  const isoOf = (y, m, d) =>
    `${String(y).padStart(4, '0')}-${String(m).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
  /** @param {number} y */
  const leap = (y) => (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0;
  /** @param {number} y @param {number} m Days in month `m` of year `y`, 1-based. */
  const daysIn = (y, m) => [31, leap(y) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m - 1];
  /** @param {string} iso */
  const isIso = (iso) =>
    /^\d{4}-\d{2}-\d{2}$/.test(iso) && iso === isoOf(partsOf(iso)[0], partsOf(iso)[1], partsOf(iso)[2]);

  /**
   * DAY OF WEEK BY SAKAMOTO, 0 = Sunday. Chosen over `new Date(...).getDay()`
   * for the reason above: pure calendar arithmetic, unmovable by the reader's
   * zone.
   */
  const SAK = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
  /** @param {number} y @param {number} m @param {number} d */
  function dow(y, m, d) {
    const yy = m < 3 ? y - 1 : y;
    return (
      (yy + Math.floor(yy / 4) - Math.floor(yy / 100) + Math.floor(yy / 400) + SAK[m - 1] + d) % 7
    );
  }

  /** `dd Mon yyyy` for an ISO day, or `''`. @param {string} iso */
  function render(iso) {
    if (!isIso(iso)) return '';
    const [y, m, d] = partsOf(iso);
    return `${String(d).padStart(2, '0')} ${MON[m - 1]} ${y}`;
  }

  /**
   * PARSE WHAT A HUMAN ACTUALLY TYPES, AND REFUSE THE REST.
   *
   * `01 Jul 2026`, `1 jul 2026`, `1/7/2026` and `2026-07-01` all mean the same
   * day and all four get typed. The slash form is read DAY FIRST, matching the
   * label the field renders and the placeholder it shows — reading it as US
   * order would walk the ambiguity this component exists to remove straight
   * back in.
   *
   * @param {string} text @returns {string} ISO, or `''` when it is not a day.
   */
  function parse(text) {
    const s = text.trim();
    if (s === '') return '';
    let m = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(s);
    if (m) return valid(+m[1], +m[2], +m[3]);
    m = /^(\d{1,2})[ \-/.]+([A-Za-z]{3,})[ \-/.,]+(\d{4})$/.exec(s);
    if (m) {
      const mo = MON.findIndex((x) => x.toLowerCase() === m[2].slice(0, 3).toLowerCase()) + 1;
      return mo === 0 ? '' : valid(+m[3], mo, +m[1]);
    }
    m = /^(\d{1,2})[/\-.](\d{1,2})[/\-.](\d{4})$/.exec(s);
    if (m) return valid(+m[3], +m[2], +m[1]);
    return '';
  }
  /** @param {number} y @param {number} mo @param {number} d */
  const valid = (y, mo, d) =>
    mo >= 1 && mo <= 12 && d >= 1 && d <= daysIn(y, mo) && y >= 1900 && y <= 2999
      ? isoOf(y, mo, d)
      : '';

  /** @param {string} iso Is this day inside `min`/`max`? */
  const inRange = (iso) => (!min || iso >= min) && (!max || iso <= max);

  /* ---- typing ---------------------------------------------------------- */

  /**
   * THE TEXT IS LOCAL WHILE IT IS BEING TYPED AND MIRRORS `value` OTHERWISE.
   * `typing` is the whole of that distinction: `null` means "nobody is
   * editing, show the caller's value"; a string means "this is what the reader
   * put in, leave it alone". Without it every keystroke that does not yet
   * parse would be erased by the next redraw.
   */
  let typing = $state(/** @type {string | null} */ (null));
  const text = $derived(typing ?? render(value));
  const bad = $derived(typing !== null && typing.trim() !== '' && parse(typing) === '');
  const outOfRange = $derived.by(() => {
    const iso = typing === null ? value : parse(typing);
    return isIso(iso) && !inRange(iso);
  });

  /** @param {string} next */
  function type(next) {
    typing = next;
    const iso = parse(next);
    if (next.trim() === '') onchange?.('');
    else if (iso !== '' && inRange(iso)) onchange?.(iso);
  }

  /** Typing ends at blur; the field re-renders whatever the caller holds. */
  const settle = () => (typing = null);

  /* ---- the grid -------------------------------------------------------- */

  /**
   * `yyyy-mm` the grid is showing. Follows `value`, then `max`, then `min`, and
   * finally TODAY IN IST.
   *
   * # The last fallback is not decoration — without it this control opens onto
   * nothing
   *
   * The chain used to end at `min`, on the unstated assumption that a caller
   * always has at least one bound. /ingest does: its fields carry the feed's
   * floor and a ceiling of today. /db does NOT — its day window is "any earlier
   * day" to "any later day", so `value`, `max` and `min` are all `''`, `shown`
   * resolved to `''`, `cells` returned the empty array its own guard produces,
   * and the popup drew a weekday header, two arrows and NO GRID.
   *
   * Found by opening it, not by reading it: every check in this repository
   * passed on the broken version, because an empty array is a legal value and
   * a control that renders nothing renders no error.
   *
   * `CLAUDE.md` §4 is why the guard in `cells` stays rather than being widened
   * to guess a month: a fallback that hides a failure is banned, so the empty
   * grid remains impossible-to-reach rather than papered over, and the month is
   * supplied here where "which month" is a question that has an answer.
   */
  let view = $state('');
  const shown = $derived(
    view || (isIso(value) ? value.slice(0, 7) : (max || min || '').slice(0, 7) || istMonth())
  );

  /** @param {string} ym @param {number} by */
  function addMonths(ym, by) {
    if (!/^\d{4}-\d{2}$/.test(ym)) return ym;
    const total = +ym.slice(0, 4) * 12 + (+ym.slice(5, 7) - 1) + by;
    return `${String(Math.floor(total / 12)).padStart(4, '0')}-${String((total % 12) + 1).padStart(2, '0')}`;
  }

  /**
   * SIX WEEKS, ALWAYS, STARTING MONDAY.
   *
   * A fixed 42 cells rather than "as many rows as this month needs": a grid
   * that is five rows in one month and six in the next changes height as the
   * reader pages through it, and everything under the popup jumps. The leading
   * and trailing cells are real days of the neighbouring months drawn faint —
   * not blanks, so clicking one is a legal way to reach the 1st.
   */
  const cells = $derived.by(() => {
    if (!/^\d{4}-\d{2}$/.test(shown)) return [];
    const y = +shown.slice(0, 4);
    const m = +shown.slice(5, 7);
    /* Monday-first: Sakamoto gives 0 = Sunday, so Sunday must land at index 6. */
    const lead = (dow(y, m, 1) + 6) % 7;
    const prev = addMonths(shown, -1);
    const prevLen = daysIn(+prev.slice(0, 4), +prev.slice(5, 7));
    const len = daysIn(y, m);
    const out = [];
    for (let i = 0; i < 42; i++) {
      const n = i - lead + 1;
      let iso;
      if (n < 1) iso = `${prev}-${String(prevLen + n).padStart(2, '0')}`;
      else if (n > len) iso = `${addMonths(shown, 1)}-${String(n - len).padStart(2, '0')}`;
      else iso = isoOf(y, m, n);
      out.push({ iso, day: +iso.slice(8, 10), other: n < 1 || n > len, ok: inRange(iso) });
    }
    return out;
  });

  /** Years the bounds allow — the year control's whole range. */
  const years = $derived.by(() => {
    const base = +(shown || '2000').slice(0, 4);
    const lo = min ? +min.slice(0, 4) : base - 10;
    const hi = max ? +max.slice(0, 4) : base + 10;
    return Array.from({ length: Math.max(1, hi - lo + 1) }, (_, i) => lo + i);
  });

  /**
   * WHICH FACE THE PANEL IS SHOWING.
   *
   * The month and the year were two `Picker`s in the header, and `Picker` is
   * the STRIP's control — 42px rows, an 18px radio, a 380px scroll box, and a
   * panel that hangs `position: absolute` off the button it belongs to. Inside
   * a 320px calendar that panel has nowhere to hang but OVER THE GRID: opening
   * the year drew a scrolling list of radio buttons across the forty-two days
   * it was supposed to be steering.
   *
   * `routes/ingest` fixed its own copy of this first; this is that fix brought
   * to the component /db uses, so the two calendars in this product are one
   * design rather than two. Days, then the twelve months, then the years — each
   * REPLACING the grid in place and drilling back down to it. Nothing overlaps
   * anything and `.calbody` holds the day face's height so the note and the
   * footer do not move when the face changes.
   *
   * @type {'day' | 'month' | 'year'}
   */
  let pad = $state('day');

  /** The twelve months of the year on screen, each carrying its own refusal. */
  const months = $derived(
    MON.map((label, i) => {
      const ym = `${shown.slice(0, 4)}-${String(i + 1).padStart(2, '0')}`;
      return {
        ym,
        label,
        off: (Boolean(min) && ym < min.slice(0, 7)) || (Boolean(max) && ym > max.slice(0, 7))
      };
    })
  );

  /** What the caption says, which is always the unit the arrows move. */
  const caption = $derived(
    pad === 'day'
      ? `${MON[Number(shown.slice(5, 7)) - 1] ?? ''} ${shown.slice(0, 4)}`
      : pad === 'month'
        ? shown.slice(0, 4)
        : `${years[0]} – ${years[years.length - 1]}`
  );
  /** What the caption OPENS, named rather than left to be guessed. */
  const captionWhy = $derived(
    pad === 'day'
      ? 'Pick a month instead of paging one at a time.'
      : pad === 'month'
        ? 'Pick a year.'
        : 'Back to the days.'
  );

  const atFloor = $derived(
    pad === 'day'
      ? Boolean(min) && shown <= min.slice(0, 7)
      : pad === 'month'
        ? Number(shown.slice(0, 4)) <= years[0]
        : true
  );
  const atCeil = $derived(
    pad === 'day'
      ? Boolean(max) && shown >= max.slice(0, 7)
      : pad === 'month'
        ? Number(shown.slice(0, 4)) >= years[years.length - 1]
        : true
  );

  /** The arrows move the unit the caption names, whichever face is up. */
  /** @param {number} by */
  function step(by) {
    if (pad === 'day') view = addMonths(shown, by);
    else if (pad === 'month') view = `${String(Number(shown.slice(0, 4)) + by).padStart(4, '0')}-${shown.slice(5, 7)}`;
  }
  /* THE CAPTION DRILLS UP AND A CHOICE DRILLS BACK DOWN — days to months to
     years, and a picked year lands on the months of that year rather than
     jumping straight to a grid no month has been chosen for. */
  function zoom() {
    pad = pad === 'day' ? 'month' : pad === 'month' ? 'year' : 'day';
  }

  /** @type {HTMLDivElement | null} */
  let panel = $state(null);

  function toggle() {
    if (disabled) return;
    if (open) {
      openId = 0;
      return;
    }
    view = '';
    // ALWAYS ON THE DAYS. A panel that reopened on the month pad because that
    // is where it was left would answer a question the reader did not ask
    // twice in a row.
    pad = 'day';
    openId = id;
  }

  /** @param {string} iso */
  function pick(iso) {
    if (iso !== '' && !inRange(iso)) return;
    typing = null;
    onchange?.(iso);
    openId = 0;
  }

  /** @param {KeyboardEvent} e */
  function onKey(e) {
    if (e.key !== 'Escape') return;
    e.stopPropagation();
    openId = 0;
  }

  /**
   * CLOSE ON AN OUTSIDE PRESS — `pointerdown` rather than `click`, because a
   * press that starts inside the grid and ends outside it is not an outside
   * click, and dragging off the panel would otherwise close it mid-gesture.
   * @param {MouseEvent} e
   */
  function onDocDown(e) {
    if (!open) return;
    const t = /** @type {HTMLElement} */ (e.target);
    if (panel?.contains(t)) return;
    if (t.closest?.(`[data-dayfield="${id}"]`)) return;
    openId = 0;
  }
</script>

<svelte:document onpointerdown={onDocDown} />

<div class="dcell" data-dayfield={id}>
  <span class="dlbl">{label}</span>
  <div class="dwrap">
    <input
      class="din"
      type="text"
      autocomplete="off"
      spellcheck="false"
      placeholder="dd Mon yyyy"
      aria-label={label}
      aria-invalid={bad || outOfRange}
      {disabled}
      value={text}
      oninput={(e) => type(e.currentTarget.value)}
      onblur={settle}
    />
    <button
      type="button"
      class="dbtn"
      aria-haspopup="dialog"
      aria-expanded={open}
      aria-label={`Choose ${label.toLowerCase()}`}
      title={boundsReason || undefined}
      {disabled}
      onclick={toggle}>▦</button
    >
  </div>

  {#if open}
    <div
      class="cal"
      role="dialog"
      aria-label={`Choose ${label.toLowerCase()}`}
      bind:this={panel}
      onkeydown={onKey}
    >
      <!-- ONE CAPTION BETWEEN TWO ARROWS, and the caption is the control. It
           was two `Picker`s; see `pad` for why a strip control inside a 320px
           popover could only ever draw over the grid it steers. The arrows
           always move the unit the caption names, so the pair reads as one
           instrument on all three faces. -->
      <div class="cal-h">
        <button
          type="button"
          class="nav"
          aria-label="Back"
          disabled={atFloor}
          onclick={() => step(-1)}>&lsaquo;</button
        >
        <button
          type="button"
          class="calcap"
          aria-label={`${caption}. ${captionWhy}`}
          title={captionWhy}
          aria-expanded={pad !== 'day'}
          onclick={zoom}
        >
          <span class="capt">{caption}</span>
          <span class="capc" class:on={pad !== 'day'} aria-hidden="true">&#9662;</span>
        </button>
        <button
          type="button"
          class="nav"
          aria-label="Forward"
          disabled={atCeil}
          onclick={() => step(1)}>&rsaquo;</button
        >
      </div>

      <!-- ONE BOX, THREE FACES, ONE HEIGHT. The days, the twelve months and the
           years each fill the same block, so the bounds note and the footer
           under it do not move when the reader changes face. -->
      <div class="calbody">
        {#if pad === 'month'}
          <div class="calpad" role="group" aria-label="Months of {shown.slice(0, 4)}">
            {#each months as m (m.ym)}
              <button
                type="button"
                class="padc"
                disabled={m.off}
                class:on={m.ym === shown}
                title={m.off
                  ? `${m.label} ${shown.slice(0, 4)} is outside what this field may take.`
                  : `Show ${m.label} ${shown.slice(0, 4)}.`}
                onclick={() => {
                  view = m.ym;
                  pad = 'day';
                }}
              >
                {m.label}
              </button>
            {/each}
          </div>
        {:else if pad === 'year'}
          <div class="calpad" role="group" aria-label="Years this field may take">
            {#each years as y (y)}
              <button
                type="button"
                class="padc"
                class:on={String(y) === shown.slice(0, 4)}
                title={`Show the months of ${y}.`}
                onclick={() => {
                  view = `${String(y).padStart(4, '0')}-${shown.slice(5, 7)}`;
                  pad = 'month';
                }}
              >
                {y}
              </button>
            {/each}
          </div>
        {:else}
          <div class="cal-w" aria-hidden="true">
            {#each ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'] as d (d)}<span>{d}</span>{/each}
          </div>

          <div class="cal-g">
            {#each cells as c (c.iso)}
              <button
                type="button"
                class="day"
                class:other={c.other}
                class:on={c.iso === value}
                class:no={!c.ok}
                disabled={!c.ok}
                aria-current={c.iso === value ? 'date' : undefined}
                title={c.ok ? undefined : boundsReason || 'Outside what this field may take.'}
                onclick={() => pick(c.iso)}>{c.day}</button
              >
            {/each}
          </div>
        {/if}
      </div>

      <!-- THE BOUNDS SAY THEMSELVES, UNDER THE GRID THEY BOUND. A struck cell
           states that a day cannot be picked; only this line states why, and a
           refusal a reader has to hover to find is one most readers never
           meet. -->
      {#if min || max}
        <p class="cal-n">
          {render(min) || 'any earlier day'} – {render(max) || 'any later day'} can be picked.{boundsReason
            ? ` ${boundsReason}`
            : ''}
        </p>
      {/if}

      <div class="cal-f">
        <button type="button" class="clear" onclick={() => pick('')} disabled={!value}>Clear</button>
        <button type="button" class="clear" onclick={() => (openId = 0)}>Close</button>
      </div>
    </div>
  {/if}
</div>

<style>
  /* Every metric here is `$lib/Picker.svelte`'s, so a day field and a dropdown
     standing side by side in one strip are the same object at the same height:
     15px semibold mono, 11px padding, 9px radius, the same panel and shadow. */
  .dcell {
    position: relative;
    flex: 1 1 190px;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .dlbl {
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    white-space: nowrap;
  }
  .dwrap {
    display: flex;
    align-items: center;
    gap: var(--s3);
    min-width: 0;
  }
  .din {
    flex: 1 1 auto;
    min-width: 0;
    appearance: none;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 9px;
    color: var(--ink);
    font-family: var(--mono);
    font-size: var(--fs-base);
    font-weight: var(--w-semi);
    /* TABULAR FIGURES SO FROM AND TO ARE THE SAME WIDTH. `1` and `0` are not
       the same width in a proportional face, which is what makes one end of a
       window look like a different control from the other. */
    font-variant-numeric: tabular-nums;
    padding: 11px 13px;
    /* MEASURED AGAINST /ingest, WHICH IS THE FIELD THIS HAS TO MATCH. Every
       other metric already agreed to the pixel — 16px, 600, 11px/13px, 9px
       radius, mono — and the two still rendered 48px and 43px tall, because
       this rule set no line-height and inherited a smaller one than the page's.
       A five-pixel disagreement between the same control on two pages is the
       drift that makes one of them look like a different product. */
    line-height: var(--lh-base);
  }
  .din::placeholder {
    color: var(--faint);
    font-weight: var(--w-reg);
  }
  .din:focus-visible,
  .dbtn:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: 2px;
  }
  .din[aria-invalid='true'] {
    border-color: var(--warn);
  }
  .din:disabled,
  .dbtn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .dbtn {
    flex: none;
    appearance: none;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: 9px;
    color: var(--acc);
    font-size: var(--fs-base);
    line-height: 1;
    padding: 11px 10px;
    cursor: pointer;
  }
  .dbtn:hover:not(:disabled),
  .din:hover:not(:disabled) {
    border-color: var(--dim);
  }

  .cal {
    position: absolute;
    left: 0;
    top: calc(100% + 7px);
    z-index: 40;
    width: max-content;
    max-width: min(92vw, 320px);
    background: var(--raise);
    border: 1px solid var(--line);
    border-radius: var(--r3);
    padding: var(--s4);
    box-shadow: var(--e3);
  }
  .cal-h {
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin-bottom: var(--s3);
  }
  .nav {
    appearance: none;
    background: none;
    border: 1px solid var(--line);
    border-radius: var(--r1);
    color: var(--ink);
    font-size: var(--fs-base);
    line-height: 1;
    padding: 4px 9px;
    cursor: pointer;
  }
  .nav:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  /* THE CAPTION IS THE CONTROL, and it takes the width the two menus took. One
     target between the arrows, so the header reads as a single instrument
     rather than as four things that happen to sit on a line. */
  .calcap {
    flex: 1 1 auto;
    min-width: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--s3);
    padding: 4px var(--s4);
    appearance: none;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--r1);
    color: var(--ink);
    font-family: var(--mono);
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    letter-spacing: var(--track-caps);
    cursor: pointer;
  }
  .calcap:hover {
    background: var(--panel-2);
    border-color: var(--line);
    color: var(--acc);
  }
  .calcap:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: 1px;
  }
  .capt {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* THE CARET SAYS WHICH WAY THE PANEL IS ABOUT TO GO — down while the days
     show, up while a pad does. 11px, not 9: below about ten a triangle in a
     mono face reads as punctuation rather than as a direction. */
  .capc {
    flex: 0 0 auto;
    font-size: 11px;
    line-height: 1;
    color: var(--dim);
  }
  .calcap:hover .capc {
    color: var(--acc);
  }
  .capc.on {
    transform: rotate(180deg);
    color: var(--acc);
  }
  /* ONE HEIGHT FOR THREE FACES. Six 6px-padded rows of `--fs-xs` plus the
     weekday strip is the day face; without a floor here the panel would shrink
     when the months face opened and the bounds note, the disclosure and the
     footer would all jump up under the pointer. */
  .calbody {
    min-height: 196px;
    display: flex;
    flex-direction: column;
  }
  /* THE MONTHS AND THE YEARS. Three columns filling the block the days leave,
     so twelve chips are one glance rather than a scroll. `grid-auto-rows`
     rather than a fixed four: the year span comes from the caller's bounds and
     is not always twelve. */
  .calpad {
    flex: 1 1 auto;
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-auto-rows: minmax(34px, 1fr);
    gap: var(--s2);
    align-content: stretch;
    overflow-y: auto;
  }
  .padc {
    display: flex;
    align-items: center;
    justify-content: center;
    appearance: none;
    font-family: var(--mono);
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
    border: 1px solid transparent;
    border-radius: var(--r1);
    background: none;
    color: var(--ink);
    cursor: pointer;
  }
  .padc:hover:not(:disabled) {
    background: var(--panel-2);
    border-color: var(--line);
    color: var(--acc);
  }
  /* STRUCK, NOT ABSENT — the same rank the day grid gives a refused day, so a
     month outside the span reads the same way on both faces. */
  .padc:disabled {
    color: var(--faint);
    cursor: not-allowed;
    text-decoration: line-through;
  }
  .padc.on {
    background: var(--acc);
    border-color: var(--acc);
    color: var(--panel);
    font-weight: var(--w-bold);
  }
  .padc:focus-visible {
    outline: 2px solid var(--acc);
    outline-offset: 1px;
  }
  .cal-w,
  .cal-g {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
  }
  .cal-w span {
    text-align: center;
    font-family: var(--mono);
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    color: var(--faint);
    padding-bottom: 3px;
  }
  .day {
    appearance: none;
    background: none;
    border: 0;
    border-radius: var(--r1);
    color: var(--ink);
    font-family: var(--mono);
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
    padding: 6px 0;
    cursor: pointer;
  }
  .day:hover:not(:disabled) {
    background: var(--panel-2);
  }
  .day.other {
    color: var(--faint);
  }
  /* A DAY OUTSIDE THE BOUNDS IS STRUCK, NOT ABSENT. It keeps its place in the
     month so the grid's shape never changes, and the strike says "this day
     exists and this field cannot take it", which an empty square does not. */
  .day.no {
    color: var(--faint);
    text-decoration: line-through;
    cursor: not-allowed;
  }
  .day.on {
    background: var(--acc);
    color: var(--panel);
    font-weight: var(--w-bold);
  }
  .cal-n {
    margin: var(--s3) 0 0;
    font-family: var(--mono);
    font-size: var(--fs-micro);
    line-height: 1.35;
    color: var(--faint);
  }
  .cal-f {
    display: flex;
    justify-content: space-between;
    gap: var(--s3);
    margin-top: var(--s3);
    padding-top: var(--s3);
    border-top: 1px solid var(--line-soft);
  }
  .clear {
    appearance: none;
    background: none;
    border: 0;
    color: var(--acc);
    font-family: var(--mono);
    font-size: var(--fs-micro);
    cursor: pointer;
    padding: 2px 4px;
  }
  .clear:disabled {
    color: var(--faint);
    cursor: not-allowed;
  }
  @media (prefers-reduced-motion: no-preference) {
    .capc {
      transition:
        transform var(--d-state, 180ms) var(--ease-out),
        color var(--d-hover) var(--ease-out);
    }
    .day,
    .nav,
    .padc,
    .calcap,
    .din,
    .dbtn {
      transition:
        background-color var(--d-hover) var(--ease-out),
        border-color var(--d-hover) var(--ease-out),
        color var(--d-hover) var(--ease-out);
    }
  }
</style>

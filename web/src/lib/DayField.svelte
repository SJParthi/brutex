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
  import { untrack } from 'svelte';
  import { MON, istMonth } from '$lib/dates.js';
  import { parseDay } from '$lib/db/input-check.js';
  import { cutText, dayFieldWhy, dayTakes, dayStruckWhy, calendarNote, DAY_TEXT_MAX } from '$lib/db/notes-controls.js';

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
    /**
     * Which days the exchange did not trade, by the caller's calendar:
     * `(iso) => true` closed, `false` a session, `null` unknown. A closed day
     * is STRUCK in the grid and stays pickable — a window may start on a
     * Sunday — so the struck days match the sentence that says closed days
     * are struck (DB-N04). Omitted, nothing is marked closed.
     * @type {((iso: string) => boolean | null) | null}
     */
    closed = null,
    /**
     * Which days carry a day flag (WP-54, DB-025): `(iso) => text` names
     * why — refused or incomplete at import, a degraded vendor day, a halt —
     * and `null` leaves the day unmarked. Only days a band has read are
     * marked; an unmarked day is unread or ordinary, never "clean".
     * @type {((iso: string) => string | null) | null}
     */
    flagged = null,
    /**
     * THE OTHER BOX'S DAY (R3-02; DB-045). A To box takes `after` (its From
     * day) and a From box `before` (its To day): a day on the wrong side is
     * refused in the window's order words, and struck in the grid with its own
     * clause under it — never folded into `min`/`max`, whose reason is the
     * held span's. `''` means no such bound.
     */
    after = '',
    afterLabel = 'From date',
    before = '',
    beforeLabel = 'To date',
    /**
     * THE DAY AN EMPTIED BOX TAKES (R3-02), `''` for none. /db passes the held
     * span's end, so a To box always holds a day: emptied, it reads the last
     * day held rather than a blank the page read as "no bound".
     */
    empty = '',
    /**
     * Called with `YYYY-MM` whenever the open grid shows a month, so the
     * caller can read that month's flags (DB-025). Omitted, nothing is asked.
     * @type {((ym: string) => void) | null}
     */
    onshow = null,
    /**
     * Why a shown month's days are not marked, or `''` (DB-025): said under
     * the grid, so an unmarked month is never read as clean.
     * @type {((ym: string) => string) | null}
     */
    flagWhy = null,
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
   * ONE PARSER (R3-02): `parseDay` in `$lib/db/input-check.js` is the rule a
   * link's `from=` is read with, written from this component's own `parse` and
   * `valid` (years 1900 to 2999) by R3-01 for this box to move onto. Two
   * parsers for one question answer differently on the same input.
   *
   * @param {string} text @returns {string} ISO, or `''` when it is not a day.
   */
  const parse = (/** @type {string} */ text) => parseDay(text);

  /**
   * Is this day one the field takes: inside `min`/`max` (the held span) and on
   * the right side of the other box (`after`/`before`)? The same predicate the
   * grid strikes by, so a struck day and a refused typed day are one rule.
   * @param {string} iso
   */
  const inRange = (iso) => dayTakes(iso, { min, max, after, before });

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

  /**
   * THE LAST PASTE'S CUT (R3-02). More than `DAY_TEXT_MAX` characters are cut
   * to that many as they arrive — 6,000 pasted characters were kept whole and
   * echoed back inside the refusal, a paragraph wider than the page — and this
   * holds the one sentence that says so, `''` otherwise.
   */
  let cutWhy = $state('');

  /**
   * WHAT THE BOX SAYS UNDER ITSELF, or `null` (R3-02; DB-045): the cut, the
   * shape, the held span or the window's order, each in its own words — see
   * `dayFieldWhy`. The reason is the caller's (`boundsReason`), cut to its first
   * sentence and punctuated once (`reasonTail` in `input-check.js`).
   */
  const why = $derived(
    dayFieldWhy({
      typed: typing,
      value,
      label,
      min,
      max,
      after,
      before,
      reason: boundsReason,
      cut: typing !== null ? cutWhy : ''
    })
  );

  /** @param {string} next */
  function type(next) {
    const cut = cutText(next, DAY_TEXT_MAX, label);
    typing = cut.text;
    cutWhy = cut.why;
    if (cut.why) return;
    const iso = parse(cut.text);
    /* AN EMPTIED BOX TAKES `empty` (the held span's end on /db), so the
       window never silently loses a side; with no `empty` it is unbounded. */
    if (cut.text.trim() === '') onchange?.(empty);
    else if (iso !== '' && inRange(iso)) onchange?.(iso);
  }

  /**
   * TYPING ENDS AT BLUR — BUT TEXT THAT WAS NOT APPLIED IS NEVER SILENTLY
   * THROWN AWAY (DB-N03, DB-N13).
   *
   * Blur used to drop whatever was typed and re-render the old value: text
   * appended to "01 Jan 2015" (or a day outside the bounds) vanished with no
   * word, and a 56-month query went on running when one month was asked for.
   * Now a blur keeps unread or out-of-range text in the box, with
   * `aria-invalid` and the sentence under it saying it was NOT applied and
   * what the field still holds. Escape puts the held value back; text that
   * did apply simply re-renders in the field's own format — an emptied box
   * re-renders the day it took.
   */
  function settle() {
    if (typing === null) return;
    const iso = parse(typing);
    if (typing.trim() === '' || (iso !== '' && inRange(iso))) typing = null;
  }

  /* A VALUE SET FROM OUTSIDE (a reset, a link, the other field) replaces
     unapplied text: the box shows what the window now is. Text that parses to
     the new value is the reader's own typing and is left alone — and so is an
     EMPTIED box whose emptying moved the value to `empty`: the reader is about
     to type the next day, and refilling the box under the caret would undo
     the clearing they just did. */
  let lastValue = /** @type {string | null} */ (null);
  $effect(() => {
    const v = value;
    untrack(() => {
      if (v === lastValue) return;
      lastValue = v;
      if (typing !== null && typing.trim() !== '' && parse(typing) !== v) typing = null;
    });
  });

  /** Enter applies what is typed (or says why not); Escape restores. */
  /** @param {KeyboardEvent} e */
  function onInputKey(e) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (typing !== null) {
        type(typing);
        settle();
      }
    } else if (e.key === 'Escape' && typing !== null) {
      e.preventDefault();
      e.stopPropagation();
      typing = null;
      cutWhy = '';
    }
  }

  /**
   * A CLICK INTO THE FIELD SELECTS ITS TEXT, so typing a date replaces the
   * one shown instead of appending to it ("01 Jan 201502 Mar 2020", the
   * first try in DB-N03). The mouseup that follows a focusing press would put
   * the caret back and undo the selection, so that one mouseup is told not to.
   */
  let selectOnUp = false;
  /** @param {FocusEvent & {currentTarget: HTMLInputElement}} e */
  function onFocus(e) {
    e.currentTarget.select();
    selectOnUp = true;
  }
  /** @param {MouseEvent} e */
  function onUp(e) {
    if (selectOnUp) e.preventDefault();
    selectOnUp = false;
  }

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
      out.push({
        iso,
        day: +iso.slice(8, 10),
        other: n < 1 || n > len,
        ok: inRange(iso),
        struck: dayStruckWhy(iso, { min, max, after, afterLabel, before, beforeLabel, reason: boundsReason }),
        shut: closed ? closed(iso) === true : false,
        flag: flagged ? flagged(iso) : null
      });
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

  /**
   * WHICH EDGE THE CALENDAR HANGS FROM (DB-N04). Anchored `left: 0` it ran
   * past the window on the right-hand field — measured x 1223..1543 on a
   * 1440px window, with Saturday, Sunday, the Forward arrow and Close
   * off-screen. Measured when it opens, the same test `Picker` uses: it hangs
   * from the right edge when the left would overflow.
   */
  let alignRight = $state(false);
  $effect(() => {
    if (!open || !panel) return;
    const host = panel.parentElement?.getBoundingClientRect();
    if (!host) return;
    alignRight = host.left + panel.offsetWidth > window.innerWidth - 12;
  });

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

  /** @type {HTMLButtonElement | null} */
  let opener = $state(null);

  /**
   * Escape inside the panel closes it AND gives focus back to the opener: the
   * panel had focus and is gone once it closes, so without this focus fell to
   * the page's body and the next Tab started again from the top (R3-02 repair
   * round 1). It stops the key here, so `onFieldKey` never hears it.
   * @param {KeyboardEvent} e
   */
  function onKey(e) {
    if (e.key !== 'Escape') return;
    e.stopPropagation();
    openId = 0;
    opener?.focus();
  }

  /**
   * ESCAPE CLOSES A CALENDAR OPENED WITH THE MOUSE (R3-02). A click on the
   * opener left focus on the opener, outside the panel, so the panel's own
   * Escape handler never heard the key and the calendar stayed open. The
   * field's root hears it now from anywhere in the field, and the opener
   * takes focus back, the way `Picker` returns focus to its button.
   * @param {KeyboardEvent} e
   */
  function onFieldKey(e) {
    if (e.key !== 'Escape' || !open) return;
    e.stopPropagation();
    openId = 0;
    opener?.focus();
  }

  /* THE PANEL TAKES FOCUS AS IT OPENS, so keys reach it whichever way it was
     opened — the mouse included. */
  $effect(() => {
    if (open && panel) panel.focus({ preventScroll: true });
  });

  /* THE MONTH ON SHOW IS TOLD TO THE CALLER (DB-025), once per month the
     open grid shows, so it can read that month's day flags. */
  $effect(() => {
    if (open && pad === 'day' && /^\d{4}-\d{2}$/.test(shown)) onshow?.(shown);
  });

  /** What the grid's footer says it strikes — every kind it strikes (DB-045). */
  const note = $derived(calendarNote({ min, max, after, afterLabel, before, beforeLabel, reason: boundsReason }));

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

<div class="dcell" data-dayfield={id} onkeydown={onFieldKey} role="presentation">
  <span class="dlbl">{label}</span>
  <div class="dwrap">
    <input
      class="din"
      type="text"
      autocomplete="off"
      spellcheck="false"
      placeholder="dd Mon yyyy"
      aria-label={label}
      aria-invalid={why !== null}
      aria-describedby={why !== null ? `${id}-why` : undefined}
      {disabled}
      value={text}
      oninput={(e) => type(e.currentTarget.value)}
      onblur={settle}
      onkeydown={onInputKey}
      onfocus={onFocus}
      onmouseup={onUp}
    />
    <button
      type="button"
      class="dbtn"
      aria-haspopup="dialog"
      aria-expanded={open}
      aria-label={`Choose ${label.toLowerCase()}`}
      title={boundsReason || undefined}
      bind:this={opener}
      {disabled}
      onclick={toggle}>▦</button
    >
  </div>

  <!-- ==================================================================
       THE REASON, WHERE THE FIELD IS — NOT ONLY INSIDE THE CALENDAR.

       The header above this component already promises that a refused day
       "leaves `aria-invalid` on the input with the bound stated under the
       grid". That grid is the CALENDAR POPOVER, and it is shut almost all of
       the time.

       MEASURED on /db by typing a from-date after the to-date: the input took
       `aria-invalid="true"` and an amber border, and that was the whole of it —
       no `aria-describedby`, no title, no text anywhere on the page. Colour
       alone is not a reason: it dies in greyscale, it is nothing to a screen
       reader beyond a bare "invalid", and it does not say WHICH bound was
       crossed or what to do. Fifty rows went on being drawn underneath it.

       `CLAUDE.md` §4 — degrade loudly and NAME the reason.

       `role="alert"` because it appears in response to what the reader just
       typed, and `aria-describedby` ties it to the input so the two are one
       control to anything reading rather than looking.
       ================================================================== -->
  <!-- ONE SENTENCE PER REFUSAL, NAMING WHAT WAS CROSSED (R3-02): the cut, the
       shape, the held span, or the window's order — `dayFieldWhy` — and never
       the typed text echoed whole (a 6,000-character paste was). After a cut
       the sentence already says it was not applied. -->
  {#if why !== null}
    <p class="dbad" id={`${id}-why`} role="alert">
      {why}
      {#if typing !== null && !cutWhy}
        Not applied: the field still holds <b>{render(value) || 'no bound'}</b>. Escape restores it.
      {/if}
    </p>
  {/if}

  {#if open}
    <!-- `tabindex="-1"` BECAUSE THE PANEL ITSELF TAKES KEYS. `onkeydown` is
         bound here, and a key event only reaches this element if focus can
         rest on it -- without a tabindex the handler depends entirely on a
         focused DESCENDANT, so a press with focus on the panel (which is where
         opening puts it) reached nothing.
         `-1` and not `0`: reachable by script, never a stop on the Tab ring,
         which is what a dialog wants. This clears
         `a11y_interactive_supports_focus`, the last warning `svelte-check`
         reported against this tree. -->
    <div
      class="cal"
      class:right={alignRight}
      role="dialog"
      tabindex="-1"
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
                class:shut={c.ok && c.shut}
                class:flag={c.flag !== null}
                disabled={!c.ok}
                aria-current={c.iso === value ? 'date' : undefined}
                title={c.ok
                  ? c.flag !== null
                    ? `Flagged: ${c.flag}`
                    : c.shut
                      ? 'A closed day on the exchange calendar of record: no session, so no bar. It can still be picked; the window then starts or ends on it.'
                      : undefined
                  : c.struck}
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
      <!-- EVERY KIND OF STRUCK DAY IS NAMED (DB-045): the held span, and the
           other box's day when it strikes more — `calendarNote`. -->
      {#if note}
        <p class="cal-n">{note}</p>
      {/if}
      <!-- AN UNMARKED MONTH SAYS WHY (DB-025): unread, refused, or a
           selection spanning instruments — never left to read as clean. -->
      {#if flagged && flagWhy && pad === 'day' && flagWhy(shown)}
        <p class="cal-n" role="status">{flagWhy(shown)}</p>
      {/if}

      <div class="cal-f">
        <!-- WITH `empty`, CLEARING TAKES THAT DAY, AND SAYS SO ON ITS FACE. -->
        <button
          type="button"
          class="clear"
          onclick={() => pick(empty)}
          disabled={!value || value === empty}
          title={empty ? `Back to ${render(empty)}, the end of what this field may take.` : undefined}
          >{empty ? `Back to ${render(empty)}` : 'Clear'}</button
        >
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
  /* THE REJECTED-DAY LINE. Warn tone, matching the border the input already
     takes — one state, one colour, said twice: once as a shape a glance
     catches and once as a sentence that survives greyscale and reaches a
     screen reader. */
  .dbad {
    margin: var(--s1) 0 0;
    font-size: var(--fs-micro);
    line-height: 1.45;
    color: var(--warn);
    max-width: 34ch;
  }
  .dbad b {
    font-weight: var(--w-semi);
    font-variant-numeric: tabular-nums;
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
  /* HANGS FROM THE RIGHT EDGE WHEN THE LEFT WOULD OVERFLOW (DB-N04). */
  .cal.right {
    left: auto;
    right: 0;
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
  /* A CLOSED DAY: struck like an out-of-range day, and still pickable. */
  /* A FLAGGED DAY (WP-54): a dot under the number, the reason on hover. */
  .day.flag {
    box-shadow: inset 0 -3px 0 var(--warn);
  }
  .day.shut {
    color: var(--faint);
    text-decoration: line-through;
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

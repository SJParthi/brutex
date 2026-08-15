<script module>
  /**
   * WHICH PICKER IS OPEN — one number for the whole document.
   *
   * Two menus were open at once and drew on top of each other. The cause was
   * that each picker owned its own `open` flag and closed on an outside click,
   * and a click on ANOTHER picker's button is not outside `.picker` — it is
   * inside that one. So opening the second never closed the first.
   *
   * Closing the others on open would work and would be wrong: it is a rule
   * that has to be obeyed at every call site that ever opens a menu. This
   * holds the identity of the single open picker instead, so "two are open" is
   * not a state the component can reach. O(1), and no bookkeeping to forget.
   */
  let openId = $state(0);
  let issued = 0;
  const nextId = () => ++issued;
</script>

<script>
  /**
   * THE ONE DROPDOWN.
   *
   * Every list-of-things control on every page is this component. Markets, DB
   * and Ingest each had their own, and they drifted to three widths, three row
   * heights and three ideas of where the bulk actions go. One component means
   * that cannot happen again — a change to the look lands everywhere at once.
   *
   * The shape, top to bottom, never varies:
   *
   *   [ filter box ]        only when the list can be long enough to need it
   *   [ Select all ][ Clear all ]
   *   [ + Add … ]           only when the caller can grow the list
   *   ─────────────────
   *   ☑ NAME                                              detail
   *
   * Two rules that are not cosmetic:
   *
   *   1. THE HEADER IS BUILT ONCE. Redrawing it on every keystroke detaches the
   *      focused input, and a detached element is blurred — the caret vanishes
   *      and the next character is dropped. Svelte keeps the node across
   *      updates as long as it is not inside a keyed block that re-creates it.
   *
   *   2. THE FILTER IS LOCAL STATE, the selection is the caller's. Typing must
   *      never mutate what is selected, and a redraw driven by the selection
   *      must never overwrite what is being typed.
   */

  let {
    /**
     * `{ key, name, detail, disabled, why, title }` — `name` shows left,
     * `detail` right, and `why` is a full-width sentence beneath both.
     *
     * `disabled` IS NOT A HIDDEN ROW. A row drawn dead in its own place, in the
     * ladder's order, with the reason on it, says "this exists and you cannot
     * have it"; the same row omitted says "this does not exist", and where the
     * caller's own vendors sell the thing on an invoice that second sentence is
     * a lie the reader has no way to catch. So the row stays and it carries its
     * refusal — the same shape the caller uses for a universe it cannot spell a
     * target for.
     *
     * `why` is the short sentence, VISIBLE — a refusal only a hover reveals is
     * a refusal most readers never meet. `title` is the long form with the
     * provenance, for the reader who asks.
     */
    rows = [],
    /** A Set of selected keys, owned by the caller. */
    selected = new Set(),
    /** Called with the next Set. The caller decides what selection means. */
    onchange,
    /** Button text when nothing is chosen, and the noun for the counts. */
    label = 'items',
    /** Summary shown on the closed button. Falls back to "n of m selected". */
    summary = null,
    /** Show the filter box. Worth it past a screenful; noise below that. */
    filter = false,
    /** `{ text, title, run }` — appended under the bulk actions when present. */
    add = null,
    /** Single-choice mode: ticking one unticks the rest. */
    single = false,
    disabled = false,
    /**
     * TUCK THE ROWS THIS FEED CAN NEVER REACH BEHIND ONE LINE.
     *
     * # Why this is not "hide it"
     *
     * `CLAUDE.md` §4 bans the silent version and this repository already
     * decided, for the FEED picker, that a dead row is "NOT HIDDEN, DISABLED,
     * WITH THE REASON — a feed that vanishes teaches the operator nothing".
     * That decision stands and this does not repeal it: nothing is removed, no
     * reason is deleted, and one click still shows every refusal in full.
     *
     * What it fixes is a real defect the rule did not anticipate. On Groww the
     * timeframe menu opens with THREE struck-through rows above the first
     * usable one — tick, 1s and 5s, none of which any amount of operator action
     * can turn on, because the vendor does not serve them. The live choice
     * ends up below the fold, and a list whose first screenful is entirely
     * unusable teaches the operator no more than a hidden row does. It teaches
     * less: it reads as a broken build.
     *
     * So the rows are COLLAPSED, not dropped, and the collapsed line states its
     * own count so the reader can see something is there before deciding
     * whether to care.
     *
     * Off by default. `/db` does not pass it, so nothing about that page's
     * menus changes.
     */
    tuck = false
  } = $props();

  const id = nextId();
  const open = $derived(openId === id);

  let q = $state('');
  let btn = $state(null);
  let menu = $state(null);
  /**
   * Which edge the panel hangs from.
   *
   * A left-anchored menu on the rightmost control in a row runs past the window
   * and clips its own actions — the reader sees half a "Clear all". The anchor
   * is decided from where the button actually sits when it opens, not from
   * which control it is: the row reflows, so any hardcoded rule is wrong at
   * some width.
   *
   * MEASURED, NOT ASSUMED. This used a hardcoded 440 back when every menu was
   * 440 wide. Menus are content-sized now, so the guess is wrong for most of
   * them — it reads the rendered box instead.
   */
  let flip = $state(false);

  $effect(() => {
    if (!open || !menu || !btn) return;
    const b = btn.getBoundingClientRect();
    flip = b.left + menu.offsetWidth > window.innerWidth - 12;
  });

  /**
   * Whether the header holds anything at all.
   *
   * It used to render unconditionally: a single-choice picker with no filter
   * and no add action drew an empty bar with padding and a divider above the
   * first row — a blank strip that looked like content had failed to load.
   * A header with nothing in it is not a header.
   */
  const hasHead = $derived(Boolean(filter) || !single || Boolean(add));

  const shown = $derived.by(() => {
    const needle = q.trim().toUpperCase();
    if (!needle) return rows;
    return rows.filter(
      (r) =>
        r.key.toUpperCase().includes(needle) ||
        String(r.name ?? '').toUpperCase().includes(needle) ||
        String(r.detail ?? '').toUpperCase().includes(needle)
    );
  });

  /**
   * THE KEYS A SELECTION MAY NOT REACH, built once per `rows` and read O(1).
   *
   * A `disabled` attribute on the input stops a MOUSE. It does not stop
   * `selectAll`, and it does not stop a caller that calls `toggle` itself — so
   * the rule lives at the one place that mutates rather than at the one place
   * that draws.
   */
  const blocked = $derived(new Set(rows.filter((r) => r.disabled).map((r) => r.key)));
  /** How many rows a bulk action can actually reach, in the whole list and in what is shown. */
  const freeAll = $derived(rows.length - blocked.size);
  const freeShown = $derived(shown.filter((r) => !r.disabled).length);

  /**
   * THE TWO HALVES OF WHAT IS SHOWN, and they are only two when `tuck` is on.
   *
   * `live` is what a click can reach. `dead` is what it cannot. With `tuck`
   * false the split is not made at all and `live` is the whole list, so the
   * default rendering is byte-for-byte the one every caller already gets.
   *
   * A FILTERED SEARCH OPENS THE DRAWER. Typing a query means the reader is
   * looking for something by name, and a match that stayed folded away under a
   * count would read as "no such rung" — the silent answer §4 bans. So while
   * `q` is non-empty every match is listed, dead ones included.
   */
  const live = $derived(tuck && !q.trim() ? shown.filter((r) => !r.disabled) : shown);
  const dead = $derived(tuck && !q.trim() ? shown.filter((r) => r.disabled) : []);
  let openTuck = $state(false);

  const head = $derived(
    summary ??
      (selected.size === 0
        ? `None of ${rows.length} ${label}`
        : selected.size === rows.length
          ? `All ${rows.length} ${label}`
          : `${selected.size} of ${rows.length} ${label}`)
  );

  function emit(next) {
    onchange?.(next);
  }

  function toggle(key) {
    // The refusal is here and not only on the input, because this is the door
    // every other path goes through as well.
    if (blocked.has(key)) return;
    if (single) return emit(new Set([key]));
    const next = new Set(selected);
    next.has(key) ? next.delete(key) : next.add(key);
    emit(next);
  }

  /**
   * Bulk actions act on WHAT IS SHOWN, so a filter narrows them too — and
   * "Select all" never selects a row a click could not.
   */
  function selectAll() {
    const next = new Set(selected);
    // AND IT NEVER SELECTS A ROW THAT IS KNOWN TO REFUSE.
    //
    // `disabled` is "nothing could ever make this work". `skipBulk` is weaker
    // and is the one that was missing: this row CAN be ticked, a person may
    // have a reason to, and picking it FOR them is choosing a request that will
    // be refused. Measured on Dhan: the feed declares one rung, the control
    // offers two, "Select all" ticked both, and the run came back
    // "Instruments refused 1 — Dhan does not serve 1min bars".
    //
    // A bulk action is a convenience. A convenience that manufactures a refusal
    // is worse than no bulk action, and the row stays individually clickable so
    // nothing is taken away from someone who means it.
    for (const r of shown) if (!r.disabled && !r.skipBulk) next.add(r.key);
    emit(next);
  }
  function clearAll() {
    const next = new Set(selected);
    for (const r of shown) next.delete(r.key);
    emit(next);
  }

  /** Split a name around the matched letters so the narrowing is legible. */
  function parts(text) {
    const needle = q.trim().toUpperCase();
    const i = needle ? String(text).toUpperCase().indexOf(needle) : -1;
    if (i < 0) return [text, '', ''];
    return [String(text).slice(0, i), String(text).slice(i, i + needle.length), String(text).slice(i + needle.length)];
  }

  function onKey(e) {
    if (e.key === 'Escape' && open) {
      openId = 0;
      btn?.focus();
    }
  }
</script>

<svelte:window
  onclick={(e) => {
    // Outside THIS picker, not outside pickers in general — a click on another
    // picker's button has to close this one, and that is the same click.
    if (open && e.target.closest?.('.picker') !== btn?.closest?.('.picker')) openId = 0;
  }}
/>

<div class="picker" onkeydown={onKey} role="presentation">
  <button
    class="pbtn"
    type="button"
    bind:this={btn}
    {disabled}
    aria-expanded={open}
    aria-haspopup="listbox"
    onclick={(e) => {
      e.stopPropagation();
      openId = open ? 0 : id;
    }}>{head}</button
  >

  {#if open}
    <div class="pmenu" class:flip class:wide={filter} bind:this={menu} role="group">
      {#if hasHead}
        <div class="phead">
          {#if filter}
            <!-- Not inside any keyed block, so Svelte keeps this exact node
                 across updates and the caret survives every keystroke. -->
            <input
              class="pq"
              type="text"
              placeholder="Search {label}"
              aria-label="Filter {label}"
              bind:value={q}
              onclick={(e) => e.stopPropagation()}
            />
            <p class="phint">Matches name or detail</p>
          {/if}
          {#if !single}
            <div class="pact">
              <!-- THE COUNT IS WHAT THE BUTTON WILL ACTUALLY DO. Naming the
                   row count while some of those rows can never be ticked is a
                   button that reports a number it is about to miss. -->
              <button type="button" disabled={freeShown === 0} onclick={selectAll}>
                Select all{q.trim() ? ` ${freeShown} shown` : ` ${freeAll}`}
              </button>
              <button type="button" onclick={clearAll}>
                Clear all{q.trim() ? ' shown' : ''}
              </button>
            </div>
          {/if}
          {#if add}
            <div class="pact">
              <button type="button" class="awide" title={add.title} onclick={() => add.run()}
                >{add.text}</button
              >
            </div>
          {/if}
        </div>
      {/if}

      <div class="plist">
        {#each live as r (r.key)}
          {@const p = parts(r.name ?? r.key)}
          <label class:pdis={r.disabled} title={r.title ?? r.why ?? undefined}>
            <!-- A RADIO WHEN THE PICKER IS SINGLE. A checkbox is a promise that
                 you may tick a second one, and in `single` mode `toggle` emits
                 `new Set([key])` — the first tick silently vanishes. The
                 control has to look like what it does. `name` groups the radios
                 per picker instance so two on one page cannot fight. -->
            <input
              type={single ? 'radio' : 'checkbox'}
              name={single ? `pk${id}` : undefined}
              checked={selected.has(r.key)}
              disabled={Boolean(r.disabled)}
              onchange={() => toggle(r.key)}
              onclick={(e) => e.stopPropagation()}
            />
            <span class="pnm">{p[0]}{#if p[1]}<mark>{p[1]}</mark>{/if}{p[2]}</span>
            {#if r.detail}<span class="pct">{r.detail}</span>{/if}
            {#if r.why}<span class="pwhy">{r.why}</span>{/if}
          </label>
        {:else}
          <p class="pnone">
            {q.trim() ? `Nothing matches “${q}”` : `No ${label} here yet`}
          </p>
        {/each}

        <!-- THE DRAWER. One line, its own count, and every reason one click
             away. It is a `button` and not a `summary` because the menu is
             already a listbox-shaped thing and a nested disclosure widget
             inside it confuses the keyboard order more than it helps. -->
        {#if dead.length > 0}
          <button
            type="button"
            class="ptuck"
            aria-expanded={openTuck}
            onclick={(e) => {
              e.stopPropagation();
              openTuck = !openTuck;
            }}
          >
            <span class="pcar" class:on={openTuck}>▸</span>
            {dead.length}
            {dead.length === 1 ? 'row' : 'rows'} this feed cannot serve
            <span class="pmore">{openTuck ? 'hide' : 'show why'}</span>
          </button>
          {#if openTuck}
            {#each dead as r (r.key)}
              {@const p = parts(r.name ?? r.key)}
              <label class="pdis" title={r.title ?? r.why ?? undefined}>
                <input
                  type={single ? 'radio' : 'checkbox'}
                  checked={false}
                  disabled
                  onclick={(e) => e.stopPropagation()}
                />
                <span class="pnm">{p[0]}{#if p[1]}<mark>{p[1]}</mark>{/if}{p[2]}</span>
                {#if r.detail}<span class="pct">{r.detail}</span>{/if}
                {#if r.why}<span class="pwhy">{r.why}</span>{/if}
              </label>
            {/each}
          {/if}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .picker {
    position: relative;
  }
  .pbtn {
    appearance: none;
    width: 100%;
    text-align: left;
    background: var(--panel, #0f1724);
    border: 1px solid var(--line, #26334a);
    border-radius: 9px;
    color: var(--ink, #f0f4fb);
    font: inherit;
    font-size: var(--fs-base);
    font-weight: 600;
    font-family: var(--mono, ui-monospace, Menlo, monospace);
    padding: 11px 36px 11px 14px;
    cursor: pointer;
    background-image: linear-gradient(45deg, transparent 50%, var(--acc, #22d3ee) 50%),
      linear-gradient(135deg, var(--acc, #22d3ee) 50%, transparent 50%);
    background-position: calc(100% - 17px) 55%, calc(100% - 12px) 55%;
    background-size: 5px 5px, 5px 5px;
    background-repeat: no-repeat;
  }
  .pbtn:hover:not(:disabled) {
    border-color: var(--dim, #68738a);
  }
  .pbtn:focus-visible {
    outline: 2px solid var(--acc, #22d3ee);
    outline-offset: 2px;
  }
  .pbtn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .pmenu {
    position: absolute;
    left: 0;
    top: calc(100% + 7px);
    z-index: 40;
    /* SIZED BY WHAT IS IN IT. Every menu was pinned to 440px, so a two-row
       "1min / 1day" choice opened a box wide enough for a 750-name search
       result — mostly empty, and it read as data that had failed to arrive.
       `max-content` fits the widest row; the floor keeps it at least as wide
       as the button it hangs off, so it never looks detached. */
    width: max-content;
    min-width: 100%;
    max-width: min(92vw, 560px);
    max-height: 380px;
    overflow-y: auto;
    background: var(--raise, #243046);
    border: 1px solid var(--line, #26334a);
    border-radius: 10px;
    padding: 5px;
    box-shadow: 0 18px 50px rgba(0, 0, 0, 0.7);
  }
  /* A filterable list is the one case that needs the room: the search box, the
     two bulk actions and a long name all have to fit on one line. */
  .pmenu.wide {
    min-width: 440px;
  }
  .pmenu.flip {
    left: auto;
    right: 0;
  }
  /* Pinned, so the controls are reachable at row 1 and at row 750 alike. */
  .phead {
    position: sticky;
    top: -5px;
    z-index: 2;
    background: var(--raise, #243046);
    padding: 8px 6px 10px;
    border-bottom: 1px solid var(--line, #26334a);
    margin-bottom: 5px;
  }
  .pq {
    width: 100%;
    background: var(--panel, #0f1724);
    border: 1px solid var(--line, #26334a);
    border-radius: 9px;
    color: var(--ink, #f0f4fb);
    font: inherit;
    font-size: var(--fs-base);
    font-weight: 600;
    font-family: var(--mono, ui-monospace, Menlo, monospace);
    font-size: var(--fs-lg);
    line-height: 1.2;
    padding: 18px 16px;
    outline: none;
  }
  .pq:focus {
    border-color: var(--acc, #22d3ee);
    box-shadow: 0 0 0 3px rgba(34, 211, 238, 0.18);
  }
  .phint {
    margin: 6px 5px 2px;
    font-size: var(--fs-mini);
    color: var(--faint, #414b60);
  }
  .pact {
    display: flex;
    gap: 7px;
    padding: 5px 4px 0;
  }
  .pact button {
    flex: 1;
    appearance: none;
    border: 1px solid var(--line, #26334a);
    background: var(--panel, #0f1724);
    color: var(--dim, #95a0b6);
    font: inherit;
    font-size: var(--fs-xs);
    font-weight: 600;
    padding: 8px;
    border-radius: 7px;
    cursor: pointer;
  }
  .pact button:hover {
    color: var(--ink, #f0f4fb);
    border-color: var(--dim, #68738a);
  }

  .plist label {
    display: flex;
    /* So a row's own reason can take a line of its own beneath the two
       columns. A row without one never wraps: the name flexes and ellipses
       before it ever pushes the detail down. */
    flex-wrap: wrap;
    align-items: center;
    /* Wide enough that the name and its detail read as two columns. At 11px,
       a content-sized menu ran them together into one line. The ROW gap is
       small and separate — 28px between a name and the sentence under it would
       read as two rows. */
    gap: 3px 28px;
    padding: 9px 12px;
    min-height: 38px;
    border-radius: 7px;
    cursor: pointer;
    font-size: var(--fs-sm);
    transition: background 0.12s;
  }
  .plist label:hover {
    background: var(--panel2, #161f30);
  }
  /* DRAWN, NOT DROPPED, AND IT LOOKS LIKE WHAT IT IS. Struck through and dimmed
     so no reader mistakes it for a choice they merely have not made yet — and
     still legible, because the row is on screen in order to be READ. */
  .plist label.pdis {
    cursor: not-allowed;
  }
  .plist label.pdis:hover {
    background: transparent;
  }
  .plist label.pdis .pnm {
    color: var(--dim, #95a0b6);
    text-decoration: line-through;
    text-decoration-thickness: 1px;
  }
  .plist label.pdis .pct {
    color: var(--faint, #68738a);
  }
  .plist input:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
  /* The reason, on its own line, indented past the checkbox so it reads as
     belonging to the name above it rather than to the list. */
  .pwhy {
    flex: 0 0 100%;
    padding-left: 45px;
    font-size: var(--fs-mini);
    line-height: 1.45;
    color: var(--dim, #95a0b6);
    white-space: normal;
  }
  .plist input {
    accent-color: var(--acc, #22d3ee);
    width: 17px;
    height: 17px;
    cursor: pointer;
    flex: 0 0 auto;
  }
  /* Name takes the space and clips; detail keeps its own width on the right.
     Wrapping either one breaks the single row height the list depends on. */
  .pnm {
    flex: 1;
    min-width: 0;
    font-family: var(--mono, ui-monospace, Menlo, monospace);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pnm :global(mark) {
    background: var(--accdeep, #0e4f5e);
    color: var(--acc, #22d3ee);
    border-radius: 2px;
    padding: 0 1px;
  }
  .pct {
    flex: 0 0 auto;
    /* Widened from 190px when the detail became a STATUS rather than a
       measurement — `not fetched · no store dir` is the longest true thing a
       row can say about itself and it has to fit without an ellipsis, because
       an ellipsed refusal is a refusal nobody reads. */
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: right;
    /* The detail is the reason to pick the row — "375 bars per session",
       "derived from 1min". At --faint it was barely legible on the panel
       ground, which is how a real answer ends up looking like a placeholder. */
    color: var(--dim, #95a0b6);
    font-size: var(--fs-xs);
    font-family: var(--mono, ui-monospace, Menlo, monospace);
  }
  .pnone {
    padding: 16px 12px;
    margin: 0;
    text-align: center;
    color: var(--faint, #68738a);
    font-size: var(--fs-xs);
  }
  /* THE DRAWER LINE. Quiet on purpose — it is a signpost, not a choice, and it
     must not compete with the rows above it that a click can actually reach. */
  .ptuck {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 8px 12px;
    border: 0;
    border-top: 1px solid var(--line-soft, #edf0f6);
    background: transparent;
    color: var(--faint, #68738a);
    font-family: var(--mono, ui-monospace, Menlo, monospace);
    font-size: var(--fs-micro);
    text-align: left;
    cursor: pointer;
  }
  .ptuck:hover {
    color: var(--ink, #0d1220);
    background: var(--panel-2, #eef1f7);
  }
  .pcar {
    display: inline-block;
    font-size: 8px;
    transition: transform 140ms cubic-bezier(0.22, 0.61, 0.36, 1);
  }
  .pcar.on {
    transform: rotate(90deg);
  }
  .pmore {
    margin-left: auto;
    color: var(--acc, #0e7f96);
  }
  @media (prefers-reduced-motion: reduce) {
    * {
      transition: none !important;
    }
  }
</style>

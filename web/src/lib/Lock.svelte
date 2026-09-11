<script>
  /**
   * A neutral unavailable value.
   *
   * This component keeps the historical filename so existing call sites stay
   * small, but it deliberately renders no lock. A padlock says permission,
   * paywall or ownership; the states on this page are missing evidence,
   * inapplicable metrics and undefined arithmetic. Those are data semantics,
   * not access control.
   *
   * The compact table form is an em dash. The ordinary form says
   * "Unavailable" in visible text. Both retain the exact reason in `title`
   * and `aria-label`; keyboard focus also reveals that reason inline, so it is
   * not evidence available only to a pointer.
   */
  let {
    /** Why the value is unavailable. Shown on hover/focus and to a screen reader. */
    why = '',
    /** Inline in a dense table cell. */
    small = false,
    /** Leading a panel that explains itself in prose beside it. */
    big = false,
    /** Missing evidence, undefined arithmetic, or a metric outside this model. */
    kind = 'unavailable'
  } = $props();

  const word = $derived(
    kind === 'undefined' ? 'Undefined' : kind === 'not-applicable' ? 'Not applicable' : 'Unavailable'
  );
  const label = $derived(why ? `${word}: ${why}` : `${word} for this recorded run`);
  let expanded = $state(false);
</script>

<button
  type="button"
  class="lk"
  class:sm={small}
  class:bg={big}
  title={label}
  aria-label={label}
  aria-expanded={expanded}
  onclick={() => (expanded = !expanded)}
>
  {small ? '—' : word}
  {#if expanded}<span class="lk-detail" aria-hidden="true">{label}</span>{/if}
</button>

<style>
  .lk {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 0;
    background: transparent;
    font-family: inherit;
    color: var(--n8);
    cursor: help;
    font-size: max(12px, 0.78em);
    font-weight: 500;
    line-height: 1.2;
    text-decoration: underline dotted;
    text-underline-offset: 0.2em;
  }
  .lk:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .lk-detail {
    margin: 0 0 0 0.4rem;
    white-space: normal;
    color: var(--n10);
    font-size: 12px;
    text-decoration: none;
  }
  .lk.sm {
    min-width: 1ch;
    font-size: inherit;
    text-decoration: none;
  }
  .lk.bg {
    font-size: 0.82rem;
  }
  .lk:hover {
    color: var(--n10);
  }
</style>

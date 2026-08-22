<script>
  /**
   * THE PADLOCK — a metric that exists and cannot be shown.
   *
   * # Why this borrows TradingView's glyph
   *
   * The Strategy Tester renders a metric it cannot show as a LOCK in the cell:
   * not a blank, not `N/A`, not a dropped row. The row keeps its place and its
   * order, and the cell says the value is unavailable rather than pretending
   * the metric does not exist. That is exactly the discipline `CLAUDE.md` §4
   * demands — degrade loudly and name the reason, never a fallback that hides
   * a failure — and a reader who already knows the Strategy Tester reads it
   * without learning anything.
   *
   * # Where the two meanings differ, and why ours is better
   *
   * TradingView's lock means *"your plan does not include this"*. Ours means
   * *"the sweep never wrote this"*. Theirs is a paywall and carries no
   * explanation; ours carries [`why`], and a lock without one is a lock that
   * teaches an operator to stop asking. So `why` is not decoration — it is the
   * whole difference between documenting a gap and merely having one.
   *
   * Rendered as a `<span>` with the reason in `title` AND in `aria-label`, so
   * the fact reaches a pointer and a screen reader by the same route. Purely
   * decorative on its own: the metric's NAME is in the row beside it, so this
   * never has to carry it.
   */
  let {
    /** Why the value is unavailable. Shown on hover and to a screen reader. */
    why = '',
    /** Inline in a dense table cell. */
    small = false,
    /** Leading a panel that explains itself in prose beside it. */
    big = false
  } = $props();

  const label = $derived(why ? `Not recorded: ${why}` : 'Not recorded by the sweep');
</script>

<span class="lk" class:sm={small} class:bg={big} title={label} aria-label={label} role="img">
  <svg viewBox="0 0 14 14" aria-hidden="true">
    <!-- A shackle and a body. Drawn rather than taken from an icon font: this
         repository ships no font it does not own, and one glyph is not worth a
         dependency. -->
    <path
      d="M4.2 6.2V4.6a2.8 2.8 0 0 1 5.6 0v1.6"
      fill="none"
      stroke="currentColor"
      stroke-width="1.3"
      stroke-linecap="round"
    />
    <rect x="2.8" y="6.2" width="8.4" height="5.6" rx="1.2" fill="currentColor" opacity="0.85" />
  </svg>
</span>

<style>
  .lk {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: var(--n8);
    cursor: help;
    vertical-align: -1px;
  }
  .lk svg {
    width: 13px;
    height: 13px;
  }
  .lk.sm svg {
    width: 11px;
    height: 11px;
  }
  .lk.bg svg {
    width: 20px;
    height: 20px;
  }
  .lk:hover {
    color: var(--n10);
  }
</style>

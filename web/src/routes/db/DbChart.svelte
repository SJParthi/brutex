<script>
  /**
   * /db — THE CHART PANEL — A PLACEHOLDER, MOUNTED IN ADVANCE.
   *
   * Will draw candles, OI, IV and greeks for the selection, paged through
   * /bars/window.json.
   *
   * WP-00 created this file empty and mounted it in the bar view above the grid, so its owner fills
   * it without touching the file that mounts it. It renders NOTHING until then:
   * no element, no text, no request.
   *
   * Owner after WP-00: WP-57.
   */
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

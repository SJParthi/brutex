<script>
  /**
   * /db — THE PRICE-BASIS BADGE — A PLACEHOLDER, MOUNTED IN ADVANCE.
   *
   * Will say whether prices are back-adjusted (Zerodha) or raw as traded (GDFL).
   *
   * WP-00 created this file empty and mounted it in the anchor beside the instrument, so its owner fills
   * it without touching the file that mounts it. It renders NOTHING until then:
   * no element, no text, no request.
   *
   * Owner after WP-00: WP-55.
   */
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

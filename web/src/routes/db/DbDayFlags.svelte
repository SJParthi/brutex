<script>
  /**
   * /db — THE DAY BAND — A PLACEHOLDER, MOUNTED IN ADVANCE.
   *
   * Will show refused or incomplete days, degraded vendor days, halts, the CAS
   * phase and gaps, from /dayflags.json.
   *
   * WP-00 created this file empty and mounted it in the bar view above the grid, so its owner fills
   * it without touching the file that mounts it. It renders NOTHING until then:
   * no element, no text, no request.
   *
   * Owner after WP-00: WP-54.
   */
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

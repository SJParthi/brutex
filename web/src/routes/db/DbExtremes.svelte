<script>
  /**
   * /db — THE BAR SELECTION'S EXTREMES — A PLACEHOLDER, MOUNTED IN ADVANCE.
   *
   * Will show max/min close, max high, min low, the widest and the heaviest bar,
   * each with its IST time and a jump link.
   *
   * WP-00 created this file empty and mounted it in the bar view above the grid, so its owner fills
   * it without touching the file that mounts it. It renders NOTHING until then:
   * no element, no text, no request.
   *
   * Owner after WP-00: WP-52.
   */
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

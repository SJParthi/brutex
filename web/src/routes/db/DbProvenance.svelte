<script>
  /**
   * /db — THE SOURCE COLUMN'S DRAWER — A PLACEHOLDER, MOUNTED IN ADVANCE.
   *
   * Will show where a bar came from, from /provenance.json.
   *
   * WP-00 created this file empty and mounted it in the bar view under the grid, so its owner fills
   * it without touching the file that mounts it. It renders NOTHING until then:
   * no element, no text, no request.
   *
   * Owner after WP-00: WP-55.
   */
  /** @type {{ db: any }} */
  let { db } = $props();
</script>

<script>
  /**
   * AUDIT — every request, including the refused ones.
   *
   * The Rust side renders this page today and renders it well; embedding it is
   * honest about that rather than reimplementing a journal reader that would
   * drift from the one that writes it.
   */
</script>

<div class="pane">
  <div class="pane-head"><span class="pane-title">Audit · every run, including refusals</span></div>
  <iframe
    title="Audit journal"
    src="/audit"
    style="flex:1; width:100%; border:0; background:#fff"
  ></iframe>
</div>

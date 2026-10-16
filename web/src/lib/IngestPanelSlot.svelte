<!--
  ONE /ingest PANEL, INSIDE ITS OWN ERROR BOUNDARY (R4-01, repair round 1).

  The page mounts every `routes/ingest/panels/*.svelte` through this slot. A
  panel that throws while it mounts, or in an effect afterwards, is replaced by
  ONE visible line naming its file and its error (`panelFailedLine`) and is
  named on the console with the error itself (`reportPanelFailure`), so the
  stack is kept. Every other panel and the rest of /ingest keep working.

  Without it, one panel's throw at initialisation blanked the whole page:
  `document.body.innerText` was empty at 6 s and at 15 s, none of the page's
  own fetches started, and nothing named the panel — the silent failure
  CLAUDE.md §4 bans. `web/tests/ingest-panels.test.js` renders this file with a
  throwing panel and pins the line.

  An error thrown in an event handler or a fetch callback is not a render
  error and no boundary sees it; a panel answers those itself (`answerLine`).
-->
<script>
  import { panelFailedLine, reportPanelFailure } from './ingest-panels.js';

  /** @type {{ panel: { file: string, component: any }, ctx: any }} */
  let { panel, ctx } = $props();
  const Panel = $derived(panel.component);
</script>

<svelte:boundary onerror={(error) => reportPanelFailure(panel.file, error)}>
  <Panel {ctx} />
  {#snippet failed(error)}
    <p class="warn mono panel-failed" role="alert" data-panel={panel.file}>{panelFailedLine(panel.file, error)}</p>
  {/snippet}
</svelte:boundary>

<style>
  .panel-failed {
    margin: 0;
    overflow-wrap: anywhere;
  }
</style>

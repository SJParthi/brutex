<script>
 let {metadata=/** @type {any} */(null)}=$props();
 const settings=[['max_loss_points','Research loss limit'],['batch_programs','Expressions in each batch'],['node_allowance','Search work per batch'],['batch_allowance','Batches in this run']];
</script>

{#if metadata&&!metadata.ready}
 <section class="readiness" aria-label="Why the sweep cannot start">
  <h4>Setup is incomplete. The sweep has not started.</h4>
  <p>The server has not yet accepted every required setting. These checks leave the fixed candle stop and the 15:10 exit unchanged.</p>
  <table>
   <caption>What still needs to be ready</caption>
   <thead><tr><th>Check</th><th>Configuration status</th></tr></thead>
   <tbody>
    {#each settings as [key,label]}<tr><th>{label}</th><td>{metadata.configured[key]?'Value supplied':'Not configured'}</td></tr>{/each}
    <tr><th>Institutional acceptance settings</th><td>{metadata.policy.ready?'Resolved':'Not resolved'}</td></tr>
    <tr><th>Stored-data and result-reading limits</th><td>{metadata.limits.physical?'Resolved':'Not resolved'}</td></tr>
   </tbody>
  </table>
  <p>Values that are supplied still have to pass the server’s checks together. This table does not certify the historical candles or clear a run for launch.</p>
  <details><summary>Exact reason from the server</summary><p role="alert">{metadata.refusal}</p></details>
 </section>
{/if}

<style>
 .readiness{border:1px solid var(--line,#cbd6df);border-left:3px solid var(--down,#ad4944);border-radius:5px;padding:.85rem;margin:.85rem 0}h4{font-size:.9rem;margin:0 0 .5rem}p,summary,table{font-size:.8rem;line-height:1.55}p{margin:.6rem 0}table{width:100%;border-collapse:collapse;text-align:left}caption{text-align:left;font-weight:600;margin-bottom:.5rem}th,td{padding:.55rem .4rem;border-bottom:1px solid var(--line,#cbd6df);vertical-align:top}tbody th{font-weight:500}td{font-variant-numeric:tabular-nums}summary{cursor:pointer}summary:focus-visible{outline:2px solid var(--acc,#34849e);outline-offset:2px}[role=alert]{overflow-wrap:anywhere;color:var(--down,#ad4944)}
</style>

<script>
  import { indexConsistencyRules } from './index-consistency.js';
  let { policy = /** @type {any} */ (null) } = $props();
</script>

<section class="index-policy" aria-label="Index day and week acceptance rule">
  <h4>Daily and weekly consistency</h4>
  {#if policy}
    <p><b>Applies to NIFTY and BANKNIFTY.</b> Training, the later evaluation and the full date span
      must each satisfy these checks. Cash stocks keep their existing institutional checks.</p>
    <table><thead><tr><th>What is checked</th><th>Required result</th></tr></thead><tbody>
      <!-- READ FROM THE POLICY, NEVER WRITTEN HERE. These rows once read "60%",
           "3 winning days" and "no more than 2" as literal prose while the
           `policy` prop beside them was used only for its digest, so the table
           stated the old rule beside records judged by the new one. The text now
           comes from `indexConsistencyRules`, one function for every page. -->
      {#each indexConsistencyRules(policy) as [what, rule]}<tr><th>{what}</th><td>{rule}</td></tr>{/each}
    </tbody></table>
    <p>Multiple trades per day are allowed; a day uses the sum of its trades, per unit.
      {policy.ratio_basis === 'decided_days' ? 'Flat days and days without trades are neither wins nor losses and stay out of the ratio.' : 'Flat days and days without trades remain separate and stay in the denominator.'}
      Short calendar weeks and partial boundary weeks are shown separately. A missing expected day,
      including a gap between periods, withholds the result. Costs are excluded.</p>
    <details><summary>Exact policy identity</summary><code>{policy.policy_digest}</code></details>
  {:else}
    <p>The server has not reported the new index day/week rule. Older results remain unassessed under it.</p>
  {/if}
</section>

<style>
  .index-policy{border:1px solid var(--line,#c9d4df);border-radius:8px;padding:.8rem 1rem;margin:1rem 0;max-width:100%}h4{margin:.1rem 0 .6rem;font-size:.95rem}p{font-size:.82rem;line-height:1.6}table{border-collapse:collapse;width:100%;font-size:.8rem;text-align:left}th,td{padding:.6rem;border-bottom:1px solid var(--line,#c9d4df);vertical-align:top}thead{background:var(--subtle,#f5f8fb)}summary{cursor:pointer;font-size:.8rem}details{margin-top:.7rem}code{display:block;overflow-wrap:anywhere;margin:.6rem 0;font-size:.72rem}@media(max-width:520px){th,td{padding:.5rem .3rem}.index-policy{padding:.65rem}}
</style>

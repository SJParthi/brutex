<script>
  let { policy = /** @type {any} */ (null) } = $props();
</script>

<section class="index-policy" aria-label="Index day and week acceptance rule">
  <h4>Daily and weekly consistency</h4>
  {#if policy}
    <p><b>Applies to NIFTY and BANKNIFTY.</b> Training, the later evaluation and the full date span
      must each satisfy these checks. Cash stocks keep their existing institutional checks.</p>
    <table><thead><tr><th>What is checked</th><th>Required result</th></tr></thead><tbody>
      <!-- READ FROM THE POLICY, NEVER WRITTEN HERE. These three rows read "60%",
           "3 winning days", "no more than 2 losing days" and "no more than 2"
           as literal prose while the `policy` prop beside them was used only for
           its digest. When the served policy moved, this table went on stating
           the old rule next to records judged by the new one -- a page
           advertising a two-day streak cap beside a record judged at ten. -->
      <tr><th>Winning trading days</th><td>At least {policy.minimum_winning_day_numerator} in every {policy.minimum_winning_day_denominator} eligible trading days</td></tr>
      <tr><th>Every complete five-session week</th><td>At least {policy.minimum_week_winning_days} winning {Number(policy.minimum_week_winning_days) === 1 ? 'day' : 'days'} and no more than {policy.maximum_week_losing_days} losing {Number(policy.maximum_week_losing_days) === 1 ? 'day' : 'days'}</td></tr>
      <tr><th>Losing-day streak across weeks</th><td>No more than {policy.maximum_losing_day_streak} losing {Number(policy.maximum_losing_day_streak) === 1 ? 'day' : 'days'}; only a winning day resets the streak</td></tr>
    </tbody></table>
    <p>Multiple trades per day are allowed. A day uses the sum of its pessimistic gross profit and
      loss per unit. Flat days and days without trades remain separate and stay in the denominator.
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

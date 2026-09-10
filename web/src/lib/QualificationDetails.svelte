<script>
  import BooleanLater from './BooleanLater.svelte';
  import {candidateMoney} from './candidate-trades.js';
  import {catalogDay} from './boolean-catalog.js';
  import IndexConsistency from './IndexConsistency.svelte';
  import {indexConsistencyContext} from './index-consistency.js';
  let {body,row=/** @type {any} */ (null),consistency=/** @type {any} */ (undefined),institutionalStatus=/** @type {string|undefined} */ (undefined)}=$props();
  let opened=$state(false);
  $effect(()=>{void row;void body;opened=false;});
  /** @param {string} value */
  const label=value=>value.replaceAll('-',' ');
</script>
{#if row===null}
  {@const q=body.qualification}
  {@const a=q.allocation}
  <div class="notice">
    <p><b>Fixed training settings, later observations.</b> Every declared coordinate is retained, including losses, zero trades and refusals. PBO remains a training diagnostic; the checks below use the separately recorded later execution and profitability. These are cost-excluded research observations, with no promotion or future-return guarantee.</p>
    <table><caption>Declared search and statistical resolution</caption><tbody>
      <tr><th>Fixed scope</th><td>{a.batches} batches × {a.rungs} rungs. This record: batch {a.batch}, rung {a.rung} (zero-based).</td></tr>
      <tr><th>Exact per-family threshold</th><td>{a.threshold.numerator} / {a.threshold.denominator}; total alpha {a.alpha_ppm} ppm.</td></tr>
      <tr><th>Later bootstrap procedure</th><td>{q.procedure.draws} draws; seed {q.procedure.seed}; block length {q.procedure.block_length}.</td></tr>
      <tr><th>Draw resolution</th><td>{a.minimum_draws===null?'Zero alpha cannot be reached by a positive bootstrap probability':`Minimum ${a.minimum_draws} draws to reach this threshold`}. <b>{a.draw_resolution_met?'Numerically reachable; not evidence of a passing strategy':'Insufficient resolution for a rejection'}</b>.</td></tr>
      <tr><th>Later folds</th><td>{q.fold_count} predeclared windows; missing per-setting fold evidence stays unavailable.</td></tr>
    </tbody></table>
    <details><summary>Exact original, later and allocation receipts</summary><pre>{JSON.stringify(q,null,2)}</pre></details>
  </div>
{:else}
  {@const q=row.qualification}
  {@const r=q.romano}
  <div class="notice">
    <p><b>Later-period evidence.</b> Romano–Wolf classification: {label(r.classification)}. Exact conservative probability {r.probability.numerator} / {r.probability.denominator}.</p>
    <IndexConsistency value={consistency === undefined ? row.index_consistency : consistency}
      context={indexConsistencyContext(body,row,institutionalStatus ?? row.status)} />
    {#if r.shared}<p>Nonconstant-subfamily position {r.shared.strategy}, stepdown rank {r.shared.stepdown_rank}. Statistic {r.shared.statistic.decimal} (exact IEEE bits {r.shared.statistic.bits}); {r.shared.strict_exceedances} strict exceedances. Original adjusted fraction {r.shared.adjusted.numerator} / {r.shared.adjusted.denominator}.</p>{:else}<p>No nonconstant-subfamily bootstrap record for this zero-return observation.</p>{/if}
    <button onclick={()=>{opened=!opened;}}>This exact setting’s later trades and sessions</button>
    {#if opened}<BooleanLater initialIdentity={q.source.identity} initialCompletion={q.source.completion} initialCandidate={q.coordinate} autoLoad={true} />{/if}
    {#if q.folds}
      <div class="scroll"><table><caption>Every fixed-training later fold</caption><thead><tr><th>Window</th><th>Actual sessions</th><th>Trades / wins</th><th>Cost-excluded return per unit</th></tr></thead><tbody>{#each q.folds.rows as fold (fold.index)}<tr><th>{catalogDay(fold.first_day)} to {catalogDay(fold.last_day)}</th><td>{fold.sessions}</td><td>{fold.trades} / {fold.wins}</td><td>{candidateMoney(fold.return_paisa)}</td></tr>{/each}</tbody></table></div>
      <p>{q.folds.profitable} profitable windows of {q.folds.decided}; aggregate {candidateMoney(q.folds.return_paisa)} per unit. Execution refusal bits: {q.folds.execution_refusal_bits}.</p>
    {:else}<p><b>Fixed-training fold evidence unavailable for this setting.</b> No zero values or passing outcome are substituted.</p>{/if}
    <details><summary>Complete later identities, folds and unrounded statistical facts</summary><pre>{JSON.stringify(q,null,2)}</pre></details>
  </div>
{/if}
<style>
  .notice{border-left:3px solid #68adba;padding:.6rem 1rem;margin:.8rem 0}p{font-size:.84rem;line-height:1.55}table{border-collapse:collapse;width:100%;font-size:.8rem;text-align:left}th,td{border-bottom:1px solid #61717e44;padding:.55rem;vertical-align:top}caption{text-align:left;font-weight:650;margin:.5rem 0}.scroll{overflow:auto}button{background:transparent;color:inherit;border:1px solid #647985;border-radius:5px;padding:.55rem;cursor:pointer}button:focus-visible{outline:2px solid #79cad8}summary{cursor:pointer}pre{max-height:30rem;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;font-size:.75rem}
</style>

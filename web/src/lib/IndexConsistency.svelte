<script>
  import { onDestroy } from 'svelte';
  import { candidateMoney } from './candidate-trades.js';
  import { indexStopPoints } from './index-stop-results.js';
  import { catalogDay } from './boolean-catalog.js';
  import { validateIndexConsistency, indexConsistencyLabel, indexConsistencyReasons, indexCombinedLabel, indexConsistencyRules, createIndexConsistencyPages } from './index-consistency.js';
  let { value = /** @type {any} */ (undefined), context = /** @type {any} */ (null), model = 'qualification' } = $props();
  let period = $state('full');
  let page = $state.raw(/** @type {any} */ ({ phase: 'idle', body: null, why: '' }));
  const pages = createIndexConsistencyPages(state => { page = state; }, undefined, () => model);
  const checked = $derived.by(() => {
    try { return { value: validateIndexConsistency(value, context), why: '' }; }
    catch (error) { return { value: null, why: error instanceof Error ? error.message : String(error) }; }
  });
  const selected = $derived(checked.value?.periods?.[period] ?? null);
  const periods = [['training', 'Training'], ['later', 'Later evaluation'], ['full', 'Full date span']];
  $effect(() => { void value; void context; void model; period = 'full'; pages.close(); });
  onDestroy(() => pages.dispose());
  /** @param {string} kind @param {string} [offset] */
  const open = (kind, offset = '0') => pages.open($state.snapshot(checked.value), kind, offset, period);
  /** @param {string} key */
  function selectPeriod(key) { period = key; if (page.phase !== 'idle') void open(page.kind ?? 'index-weeks'); }
  /** @param {any} day */
  // The server classifies each day under the record's own day rule; the sign of
  // the pessimistic total alone was V1's rule and mislabelled every later one.
  const dayLabel = day => ({ winning: 'Winning day', losing: 'Losing day', scratch: 'Neither: flat, or depends on fill order', no_trade: 'No trades' })[/** @type {'winning'|'losing'|'scratch'|'no_trade'} */ (day.class)] ?? 'Unclassified';
  /** @param {any} s */
  const ratioBase = s => checked.value?.policy?.ratio_basis === 'decided_days' ? String(BigInt(s.winning_days) + BigInt(s.losing_days)) : s.eligible_days;
  /** @param {string} kind */
  const weekLabel = kind => ({ complete: 'Five-session week', short: 'Short calendar week', partial: 'Partial boundary week', unmeasured: 'Calendar unconfirmed' })[/** @type {'complete'|'short'|'partial'|'unmeasured'} */ (kind)];
  /** @param {string} value */
  const amount = value => model === 'index-stop-qualification' ? indexStopPoints(value) : candidateMoney(value);
  const institutional = $derived(context?.institutional === 'admitted' ? 'Passed' : context?.institutional === 'rejected' ? 'Failed' : indexConsistencyLabel(context?.institutional));
</script>

<section class="consistency" aria-label="Daily and weekly consistency comparison">
  <h4>Daily and weekly consistency</h4>
  {#if checked.why}<p role="alert">This assessment cannot be displayed: {checked.why}</p>
  {:else}
    <div class="verdicts">
      <span><b>Existing institutional checks</b>{institutional}</span>
      <span><b>Index day/week rule</b>{indexConsistencyLabel(checked.value?.state ?? 'not_assessed')}</span>
      <span><b>Combined result</b>{indexCombinedLabel(checked.value)}</span>
    </div>
    {#each indexConsistencyReasons(checked.value) as why}<p class="reason">{why}</p>{/each}
    {#if checked.value?.periods && checked.value.state !== 'not_applicable'}
      <!-- The rule is the record's own, carried with it and tied to it by its
           digest; this paragraph once stated V1's numbers beside V3 verdicts. -->
      <p>Required in <b>each period</b>. Multiple trades are summed each day.</p>
      {#each indexConsistencyRules(checked.value.policy) as [what, rule]}<p><b>{what}.</b> {rule}</p>{/each}
      <div class="scroll" role="region" aria-label="Compare training, later and full-span consistency">
        <table><caption>Separate period checks; a full-span pass alone is insufficient</caption>
          <thead><tr><th>Period</th><th>Winning / {checked.value.policy.ratio_basis === 'decided_days' ? 'decided' : 'eligible'} days</th><th>Passed / complete weeks</th><th>Failed weeks</th><th>Longest losing streak</th><th>Result</th></tr></thead>
          <tbody>{#each periods as [key, label]}
            {@const p = checked.value.periods[key]}{@const s = p.summary}
            <tr><th>{label}<small>{catalogDay(p.first_day)} – {catalogDay(p.last_day)}</small></th>
              <td>{s.winning_days} / {ratioBase(s)}</td><td>{s.passing_weeks} / {s.complete_weeks}</td>
              <td>{s.failing_weeks}</td><td>{s.longest_losing_streak}</td><td>{indexConsistencyLabel(p.state)}</td></tr>
          {/each}</tbody>
        </table>
      </div>
      <div class="controls" aria-label="Choose the evidence period">
        {#each periods as [key, label]}<button class:active={period === key} aria-pressed={period === key} onclick={() => selectPeriod(key)}>{label}</button>{/each}
      </div>
      {#if selected}
        {@const s = selected.summary}
        <div class="facts"><span><b>Winning</b>{s.winning_days}</span><span><b>Losing</b>{s.losing_days}</span>
          <span><b>Flat, with trades</b>{s.zero_days}</span><span><b>No trades</b>{s.no_trade_days}</span>
          <span><b>Missing expected days</b>{s.missing_days}</span><span><b>Calendar unconfirmed days</b>{s.unmeasured_days}</span>
          <span><b>Trades</b>{s.trades}</span><span><b>Pessimistic gross / unit</b>{amount(s.pessimistic_paisa)}</span></div>
        <p>{checked.value.policy.ratio_basis === 'decided_days' ? 'Flat and no-trade days stay out of the ratio.' : 'Flat and no-trade days are included in the eligible-day denominator.'} Short calendar weeks:
          {s.short_weeks}; partial boundary weeks: {s.partial_weeks}; unconfirmed weeks: {s.unmeasured_weeks}.
          These are not counted as passed five-session weeks. Eligible weekend sessions: {s.weekend_sessions};
          they still affect the overall day ratio and losing streak.</p>
        {#if selected.first_issue_day !== null}<p>First recorded issue: {catalogDay(selected.first_issue_day)}.</p>{/if}
        <div class="controls"><button onclick={() => open('index-weeks')}>Inspect saved weeks</button><button onclick={() => open('index-days')}>Inspect saved days</button></div>
      {/if}
      {#if page.phase === 'loading'}<p role="status">Reading one saved {page.kind === 'index-days' ? 'day' : 'week'} page…</p>
      {:else if page.phase === 'failed'}<p role="alert">Saved evidence is unavailable: {page.why}</p><button onclick={() => open(page.kind, page.offset)}>Retry this exact page</button>
      {:else if page.phase === 'ready'}
        {@const b = page.body}
        <div class="scroll"><table><caption>{page.kind === 'index-days' ? 'Each observed day: sum of all saved pessimistic gross trade returns' : 'Weekday results; weekend sessions remain separate'}</caption>
          {#if page.kind === 'index-days'}
            <thead><tr><th>Day</th><th>Trades</th><th>Gross result / unit</th><th>Day outcome</th></tr></thead>
            <tbody>{#each b.rows as day (day.index)}<tr><th>{catalogDay(day.day)}</th><td>{day.trades}</td><td>{amount(day.pessimistic_paisa)}</td><td>{dayLabel(day)}</td></tr>{/each}</tbody>
          {:else}
            <thead><tr><th>Week starting</th><th>Kind</th><th>Wins</th><th>Losses</th><th>Flat</th><th>No trades</th><th>Missing</th><th>Gross / unit</th><th>Result</th></tr></thead>
            <tbody>{#each b.rows as week (week.index)}<tr><th>{catalogDay(week.monday)}</th><td>{weekLabel(week.kind)}</td><td>{week.winning_days}</td><td>{week.losing_days}</td><td>{week.zero_days}</td><td>{week.no_trade_days}</td><td>{week.missing_days}</td><td>{amount(week.pessimistic_paisa)}</td><td>{week.state === 'not_applicable' ? 'Weekly rule does not apply' : indexConsistencyLabel(week.state)}</td></tr>{/each}</tbody>
          {/if}
        </table></div>
        {#if b.total === '0'}<p>This receipt contains no saved {page.kind === 'index-days' ? 'day' : 'week'} rows for this period. Its assessment above remains unchanged.</p>{/if}
        <div class="controls pager"><span>{b.rows.length} of {b.total} rows; page starts at {b.offset}</span>
          <button disabled={b.offset === '0'} onclick={() => open(page.kind, String(BigInt(b.offset) > BigInt(b.limit) ? BigInt(b.offset) - BigInt(b.limit) : 0n))}>Previous</button>
          <button disabled={b.next === null} onclick={() => open(page.kind, b.next)}>Next</button></div>
      {/if}
    {/if}
    {#if checked.value?.receipt}<details><summary>Exact saved assessment and source identities</summary>
      <dl><dt>Policy</dt><dd>{checked.value.policy_digest}</dd><dt>Assessment</dt><dd>{checked.value.receipt.identity}</dd>
        <dt>Completion</dt><dd>{checked.value.receipt.completion}</dd><dt>Qualification</dt><dd>{checked.value.qualification.identity}</dd><dt>Setting</dt><dd>{checked.value.setting_index}</dd></dl></details>{/if}
    <p class="foot">Returns exclude costs and are per unit, not a return on an assumed account balance. The original saved trades remain available in this setting’s trade viewer.</p>
  {/if}
</section>

<style>
  .consistency{border:1px solid var(--line,#c9d4df);border-radius:8px;margin:1rem 0;padding:1rem;color:var(--n11,#23374c)}h4{margin:0 0 .7rem;font-size:1rem}p{font-size:.82rem;line-height:1.6;margin:.65rem 0}.verdicts,.facts,.controls{display:flex;flex-wrap:wrap;gap:.65rem;margin:.8rem 0}.verdicts span,.facts span{padding:.6rem .75rem;border:1px solid var(--line,#c9d4df);border-radius:5px;font-size:.8rem}.verdicts b,.facts b{display:block;margin-bottom:.35rem}.scroll{overflow:auto;max-width:100%}table{border-collapse:collapse;width:100%;font-size:.8rem;text-align:left;font-variant-numeric:tabular-nums}th,td{padding:.65rem;border-bottom:1px solid var(--line,#c9d4df);vertical-align:top;white-space:nowrap}thead{background:var(--subtle,#f5f8fb)}caption{text-align:left;padding:.7rem 0;font-weight:600;white-space:normal}small{display:block;font-weight:400;font-size:.72rem;margin-top:.3rem}button{font:inherit;font-size:.8rem;color:inherit;background:transparent;border:1px solid var(--line,#c9d4df);border-radius:5px;padding:.5rem .75rem;cursor:pointer}button.active{background:var(--acc-soft,#e8f2f7);border-color:var(--acc,#33839b)}button:disabled{opacity:.4;cursor:default}button:focus-visible,summary:focus-visible{outline:2px solid var(--acc,#33839b);outline-offset:2px}.pager{align-items:center;font-size:.8rem}.pager span{margin-right:auto}.reason{border-left:3px solid var(--acc,#33839b);padding-left:.7rem}[role=alert]{color:var(--down,#b14a3c)}summary{cursor:pointer;font-size:.8rem}dl{display:grid;grid-template-columns:100px minmax(0,1fr);gap:.5rem;font-size:.75rem}dd{margin:0;overflow-wrap:anywhere}.foot{color:var(--n8,#597086)}@media(max-width:520px){.consistency{padding:.65rem}.verdicts span{width:100%}dl{grid-template-columns:1fr}}
</style>

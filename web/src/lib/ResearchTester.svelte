<script>
  import { tick } from 'svelte';
  import { createResearchTester, researchRule } from './research-tester.js';
  import { candidateTime, candidateMoney } from './candidate-trades.js';
  import { catalogDay } from './boolean-catalog.js';
  import { CAMPAIGN_RUNGS } from './boolean-campaign.js';
  import IndexConsistency from './IndexConsistency.svelte';
  import { decisionExplanation, periodComparison, exitExplanation, checkExplanation, formatSavedCount, RETURN_EXPLANATION } from '../../saved-backtest/explanations.js';

  let { selection = /** @type {any} */ (null), vocabulary = /** @type {any} */ (null) } = $props();
  let report = $state(/** @type {any} */ ({ phase: 'idle', context: null, fact: null, body: null, why: '' }));
  let view = $state('overview');
  let panel = $state(/** @type {HTMLElement|null} */ (null));
  const tester = createResearchTester(value => { report = value; });
  $effect(() => {
    const current = selection;
    view = 'overview';
    if (current) {
      void tester.open($state.snapshot(current.detail), current.index);
      void tick().then(() => {
        if (current !== selection) return;
        panel?.focus({ preventScroll: true });
        panel?.scrollIntoView({ block: 'start' });
      });
    }
    return () => tester.close();
  });
  /** @param {any} body */
  const prior = body => String(BigInt(body.offset) > BigInt(body.limit) ? BigInt(body.offset) - BigInt(body.limit) : 0n);
  const measures = /** @type {const} */ ([['Simulated trades','trades'],['Winning trades','wins'],['Pessimistic total / unit','pessimistic'],['Optimistic total / unit','optimistic'],['Sessions with signals','supportSessions'],['Unknown signal bars','unknown']]);
</script>

{#if selection}
  <section class="research-tester" aria-label="Saved research strategy tester" tabindex="-1" bind:this={panel}>
    <header><div><span class="eyebrow">STRATEGY TESTER · SAVED RESEARCH</span><h3>{report.context ? `${report.context.row.statistics.source.instrument} · setting ${report.context.row.index}` : 'Opening the exact setting'}</h3></div><span class="costs">Costs excluded</span></header>
    {#if report.context && report.fact}
      {@const c = report.context}{@const f = report.fact}{@const periods = periodComparison(f.training, f.later)}
      <div class="subject"><b>{c.row.statistics.side === 'short' ? 'Short' : 'Long'} · {CAMPAIGN_RUNGS[Number(c.rung)]} signal · 1min execution</b><span>Existing institutional checks: {decisionExplanation(c.comparison.status).label}</span><p><b>Entry rule: {researchRule(f, vocabulary)}</b></p><p>Intraday only, exit by 15:10 IST. Setting numbers are saved addresses, not a profitability ranking.</p></div>
      <nav aria-label="Saved research report views">
        {#each [['overview','Overview'],['trades','List of trades'],['sessions','Trading days'],['consistency','Daily / weekly rule'],['checks','All checks']] as [key,name]}<button class:active={view === key} aria-pressed={view === key} onclick={() => { view = key; if (key === 'trades' || key === 'sessions') void tester.page(report.period, key); }}>{name}</button>{/each}
      </nav>
      {#if view === 'overview'}
        <div class="columns"><div class="table-wrap"><table><caption>Same rules, two historical periods</caption><thead><tr><th>Measure</th><th>Training</th><th>Later data</th></tr></thead><tbody>
          {#each measures as [name,key]}<tr><th>{name}</th><td>{periods.training[key]}</td><td>{periods.later[key]}</td></tr>{/each}
        </tbody></table></div><dl>{#each exitExplanation(f.training, f.grid) as exit}<div><dt>{exit.label}</dt><dd>{exit.value}</dd></div>{/each}</dl></div>
        <p>{RETURN_EXPLANATION}</p><p>Later data helped evaluate these settings. It is not a separate untouched final test. This report does not establish a net profitable edge or a top-0.1% rank across the complete search.</p>
      {:else if view === 'consistency'}
        <IndexConsistency value={c.consistency ?? undefined} context={c.consistencyContext} />
      {:else if view === 'checks'}
        <div class="table-wrap"><table><caption>Every saved search-wide acceptance check</caption><thead><tr><th>Check</th><th>Observed</th><th>Required</th><th>Outcome</th></tr></thead><tbody>{#each c.comparison.checks as check (check.index)}{@const reason = checkExplanation(check, c.comparison.values, c.policy)}<tr><th>{reason.label}</th><td>{reason.observed}</td><td>{reason.required}</td><td>{reason.outcome}</td></tr>{/each}</tbody></table></div>
      {:else}
        <div class="periods" role="group" aria-label="Trade evidence period">{#each ['training','later'] as period}<button class:active={report.period === period} aria-pressed={report.period === period} onclick={() => tester.page(period, view)}>{period === 'training' ? 'Training period' : 'Later period'}</button>{/each}</div>
        {#if report.phase === 'ready' && report.kind === view}
          {@const body = report.body}
          <div class="table-wrap"><table><caption>{view === 'trades' ? 'Exact saved trades' : 'Every saved trading day, including zero-trade days'} · {report.period}</caption>
            {#if view === 'trades'}<thead><tr><th>Trade</th><th>Entry · IST</th><th>Exit bar · IST</th><th>Pessimistic / unit</th><th>Optimistic / unit</th><th>Adverse excursion</th><th>Favourable excursion</th></tr></thead><tbody>{#each body.rows as trade (trade.index)}<tr><th>{trade.index}</th><td>{candidateTime(trade.entry_micros)}</td><td>{candidateTime(trade.exit_micros)}</td><td>{candidateMoney(trade.worst)}</td><td>{candidateMoney(trade.best)}</td><td>{candidateMoney(trade.adverse_paisa)}</td><td>{candidateMoney(trade.favourable_paisa)}</td></tr>{/each}</tbody>
            {:else}<thead><tr><th>Trading day</th><th>Trades</th><th>Wins</th><th>Pessimistic / unit</th></tr></thead><tbody>{#each body.rows as day (day.index)}<tr><th>{catalogDay(day.day)}</th><td>{day.trades}</td><td>{day.wins}</td><td>{candidateMoney(day.return_paisa)}</td></tr>{/each}</tbody>{/if}
          </table></div>
          {#if body.total === '0'}<p>No saved {view === 'trades' ? 'trades' : 'trading days'} for this exact setting and period. This is a recorded zero, not a read failure.</p>{/if}
          <div class="pager"><span>{body.rows.length} rows on this page · {formatSavedCount(body.total)} saved in this period</span><button disabled={body.offset === '0'} onclick={() => tester.page(report.period, view, prior(body))}>Previous</button><button disabled={body.next === null} onclick={() => tester.page(report.period, view, body.next)}>Next</button></div>
          <p>Exit times identify the saved 1-minute bar. A 15:09 bar closes at the 15:10 deadline. No indicator or entry/exit marker is drawn over an unrelated current-store chart.</p>
        {/if}
      {/if}
      <details><summary>Exact saved identity and period</summary><dl class="identities"><dt>Search</dt><dd>{c.search}</dd><dt>Checkpoint</dt><dd>{c.pin}</dd><dt>Completion</dt><dd>{c.completion}</dd><dt>Training</dt><dd>{candidateTime(f.grid.first_micros)} – {candidateTime(f.grid.last_micros)}</dd><dt>Later period</dt><dd>{f.laterPeriod.from} – {f.laterPeriod.to}</dd></dl></details>
    {/if}
    {#if !report.context || view === report.kind}
      {#if report.phase === 'loading'}<p class="notice" role="status">Reading the pinned setting and one bounded evidence page…</p>{:else if report.phase === 'failed'}<p class="notice failed" role="alert">{report.context ? `Saved ${report.period} ${report.kind} page unavailable at row ${report.offset}` : 'Saved setting unavailable'}: {report.why}</p><button onclick={() => report.context ? tester.page(report.period, report.kind, report.offset) : tester.open($state.snapshot(selection.detail), selection.index)}>Retry this exact selection</button>{/if}
    {/if}
  </section>
{/if}

<style>
  .research-tester{margin:1.2rem 0;border:1px solid var(--line,#c9d4df);border-radius:10px;background:var(--panel,#fff);color:var(--n11,#23374c);overflow:hidden}header,.subject,nav,.periods,.pager,details,p{padding:.85rem 1.1rem}header{display:flex;align-items:center;justify-content:space-between;border-bottom:1px solid var(--line,#dce3eb)}h3{margin:.3rem 0;font-size:1.1rem}.eyebrow{font-size:.68rem;letter-spacing:.08em;color:var(--n8,#597086)}.costs{font-size:.75rem;border:1px solid var(--line,#c9d4df);border-radius:20px;padding:.35rem .65rem}.subject{background:var(--subtle,#f5f8fb)}.subject span{display:block;margin:.5rem 0}.subject p{padding:0;margin:0}p{font-size:.82rem;line-height:1.6;margin:.2rem 0}nav,.periods,.pager{display:flex;flex-wrap:wrap;gap:.4rem;align-items:center}nav{border-bottom:1px solid var(--line,#dce3eb)}button{color:inherit;background:transparent;border:1px solid var(--line,#c9d4df);border-radius:5px;cursor:pointer;font-size:.8rem;padding:.5rem .75rem}button.active{background:var(--acc-soft,#e8f2f7);border-color:var(--acc,#33839b)}button:disabled{opacity:.4;cursor:default}button:focus-visible,summary:focus-visible{outline:2px solid var(--acc,#33839b);outline-offset:2px}.columns{display:grid;grid-template-columns:minmax(0,1.3fr) minmax(220px,1fr);gap:1rem;padding:1rem}.table-wrap{overflow:auto;max-width:100%}table{border-collapse:collapse;width:100%;font-size:.8rem;text-align:left;font-variant-numeric:tabular-nums}caption{text-align:left;padding:.65rem 1rem;font-weight:600}th,td{padding:.7rem 1rem;border-bottom:1px solid var(--line,#dce3eb);white-space:nowrap}th{font-weight:600}thead{background:var(--subtle,#f5f8fb)}dl{margin:0;font-size:.82rem}dl div{display:flex;justify-content:space-between;border-bottom:1px solid var(--line,#dce3eb);padding:.7rem}dd{margin:0}.identities{display:grid;grid-template-columns:110px minmax(0,1fr);gap:.6rem;margin-top:1rem}.identities dd{overflow-wrap:anywhere}.notice{border-left:3px solid var(--acc,#33839b)}.failed{border-color:var(--down,#be4a42)}summary{cursor:pointer;font-size:.8rem}.pager span{margin-right:auto;font-size:.8rem}@media(max-width:700px){.columns{grid-template-columns:1fr}header{gap:.6rem}.identities{grid-template-columns:1fr}.costs{white-space:nowrap}}
</style>

<script>
  import { onDestroy } from 'svelte';
  import { ask } from './ask.js';
  import { booleanLaunchPlan, booleanPlanTimeframes, createBooleanLaunch, validateBooleanLaunchMetadata } from './boolean-launch.js';
  import IndexConsistencyPolicy from './IndexConsistencyPolicy.svelte';

  let {
    active = false, feed = '', symbols = /** @type {string[]} */ ([]), rungs = /** @type {string[]} */ ([]), from = '', to = '',
    initialRun = /** @type {any} */ (null),
    blockedReason = '', onbusy = /** @type {(busy:boolean)=>void} */ (() => {}),
    onTimeframes = /** @type {(values:string[])=>void} */ (() => {}),
    onSearch = /** @type {(identity:string)=>void} */ (() => {})
  } = $props();
  let laterFrom = $state(''), laterTo = $state('');
  let settings = $state({ horizonBars: '', maxPoints: '', batchPrograms: '', nodeAllowance: '', batchAllowance: '' });
  let config = $state.raw(/** @type {any} */ ({ phase: 'idle', body: null, why: '' }));
  let run = $state.raw(/** @type {any} */ ({ phase: 'idle', attempt: null, searchIdentity: null, report: null, why: '', exhausted: null, completedBatches: null }));
  let localWhy = $state('');
  let restoredRun = /** @type {any} */ (null);
  let configGeneration = 0, configAbort = /** @type {AbortController|null} */ (null);
  const controller = createBooleanLaunch({ request: ask, changed: state => {
    run = state;
    onbusy(['starting', 'running', 'unknown'].includes(state.phase));
  }, onSearch: identity => onSearch(identity) });
  const fields = /** @type {const} */ ([
    ['horizonBars', 'horizon_bars', 'Maximum holding period (bars)', 'The 15:10 IST exit still applies.'],
    ['maxPoints', 'max_points', 'Price-movement ceiling (points)', 'Used by the recorded research acceptance policy.'],
    ['batchPrograms', 'batch_programs', 'Expressions per saved batch', 'A work allowance; it does not claim the entire grammar is exhausted.'],
    ['nodeAllowance', 'node_allowance', 'Grammar choices per batch', 'Checked against the server’s replay and storage limits.'],
    ['batchAllowance', 'batch_allowance', 'Batches in this work allowance', 'The server saves progress before continuing. A pause remains a pause.']
  ]);
  const busy = $derived(['starting', 'running', 'unknown'].includes(run.phase));
  /** @param {number} year @param {number} month */
  const period = (year, month) => `${String(year).padStart(4, '0')}-${String(month).padStart(2, '0')}`;
  const displayedScope = $derived(run.plan ? {
    from: period(run.plan.from_year, run.plan.from_month), to: period(run.plan.to_year, run.plan.to_month),
    feed: run.plan.feed, symbols: run.plan.symbols, timeframes: booleanPlanTimeframes(run.plan)
  } : { from, to, feed, symbols, timeframes: rungs });
  const prepared = $derived.by(() => {
    try {
      if (!config.body) throw new Error('Read the server’s research configuration first.');
      const timeframes = [...rungs].sort((a, b) => config.body.timeframes.indexOf(a) - config.body.timeframes.indexOf(b));
      return { plan: booleanLaunchPlan({ feed, symbols, timeframes, from, to, laterFrom, laterTo, bits: 'all', ...settings }, config.body), why: '' };
    } catch (error) { return { plan: null, why: error instanceof Error ? error.message : String(error) }; }
  });
  const serverWhy = $derived(config.phase === 'failed' ? config.why
    : config.phase === 'loading' ? 'Reading the server’s research configuration…'
    : config.body && !config.body.ready ? config.body.refusal || 'Required server configuration is unavailable.'
    : config.body && !config.body.index_consistency_policy && symbols.some((/** @type {string} */ symbol) => ['NIFTY', 'BANKNIFTY', 'NSE-NIFTY', 'NSE-BANKNIFTY'].includes(symbol))
      ? 'This server has not reported the required index day/week rule. Update the server before starting an index sweep.' : '');
  const titles = {
    idle: 'Brute-force sweep', starting: 'Submitting one sweep request',
    running: 'The sweep is running', done: 'This work allowance finished',
    refused: 'Research request refused', unknown: 'Execution status is unconfirmed'
  };

  $effect(() => { if (active && config.phase === 'idle') void readConfiguration(); });
  $effect(() => {
    if (!initialRun || initialRun === restoredRun) return;
    restoredRun = initialRun;
    void controller.restore(initialRun).catch(error => {
      localWhy = error instanceof Error ? error.message : String(error);
    });
  });
  onDestroy(() => { configGeneration += 1; configAbort?.abort(); controller.dispose(); onbusy(false); });
  async function readConfiguration() {
    const ticket = ++configGeneration;
    configAbort?.abort();
    const abort = new AbortController(); configAbort = abort;
    config = { phase: 'loading', body: null, why: '' };
    const points = settings.maxPoints;
    const query = /^[1-9]\d{0,19}$/.test(points) ? `?max_points=${encodeURIComponent(points)}` : '';
    try {
      const response = await ask(`/engine/boolean-launch.json${query}`, { signal: abort.signal, cache: 'no-store' });
      if (!response.ok) throw new Error(`Research configuration returned HTTP ${response.status}. No launch was attempted.`);
      const body = validateBooleanLaunchMetadata(await response.json());
      if (ticket !== configGeneration || abort.signal.aborted) return;
      config = { phase: 'ready', body, why: '' };
      onTimeframes([...body.timeframes]);
      for (const [key, wire] of fields) if (settings[key] === '' && body.configured[wire] !== null) settings[key] = body.configured[wire];
    } catch (error) {
      if (ticket === configGeneration && !abort.signal.aborted) config = { phase: 'failed', body: null, why: error instanceof Error ? error.message : String(error) };
    }
  }
  async function start() {
    if (!active || busy || blockedReason || serverWhy || !prepared.plan) return;
    localWhy = '';
    try { await controller.start(prepared.plan); }
    catch (error) { localWhy = error instanceof Error ? error.message : String(error); }
  }
  async function recheck() {
    localWhy = '';
    try { await controller.recheck(); }
    catch (error) { localWhy = error instanceof Error ? error.message : String(error); }
  }
</script>

<section class="boolean-launch" aria-label="Launch brute-force sweep">
  <h3>{titles[/** @type {keyof typeof titles} */ (run.phase)]}</h3>
  <p>Search eligible conditions together, as alternatives, and as exclusions (AND, OR and NOT) across
    <strong>your selected instruments and timeframes</strong>. The server validates real stored OHLCV and required
    checksum receipts before pricing. Trading stays intraday, with forced exit at <strong>15:10 IST</strong>.</p>
  <div class="scope" aria-label="Research scope">
    <span><b>{run.plan ? 'Submitted training period' : 'Training period'}</b> {displayedScope.from || 'Choose first month'} → {displayedScope.to || 'Choose last month'}</span>
    <span><b>{run.plan ? 'Submitted instruments' : 'Instruments'}</b> {displayedScope.symbols.length ? displayedScope.symbols.join(', ') : 'Choose at least one'}</span>
    <span><b>Feed</b> {displayedScope.feed || 'Choose a feed'}</span>
    <span><b>{run.plan ? 'Submitted timeframes' : 'Timeframes'}</b> {displayedScope.timeframes.length ? displayedScope.timeframes.join(' · ') : 'Choose at least one'}</span>
  </div>
  <IndexConsistencyPolicy policy={config.body?.index_consistency_policy ?? null} />
  {#if config.body?.work_model}
    {@const work = config.body.work_model}
    <details class="work-model"><summary>Search size and what a saved batch means</summary>
      {#if work.state === 'refused'}<p role="alert">Search size could not be established: {work.refusal}</p>
      {:else}
        <p>The all-condition search has at least <b class="identity">{work.conjunction_program_lower_bound}</b>
          distinct AND expressions per selected timeframe, before OR, NOT, repeated conditions or exit choices.
          This is a mathematical lower bound, not a count of viable trading setups.</p>
        <p>{work.live_conditions} available conditions. The current executor processes selected timeframes
          one after another. A batch allowance limits this request’s work; it is not the total search population.
          No completion time has been established.</p>
        {#if work.lower_bound_exceeds_cumulative_counter}<p><b>Complete exhaustion exceeds the current cumulative counter’s range.</b>
          Saved batches can record partial progress; they must not be presented as an exhausted search.</p>{/if}
      {/if}
    </details>
  {/if}
  <div class="controls">
    <label>Later evaluation starts <input type="month" bind:value={laterFrom} disabled={busy} /></label>
    <label>Later evaluation ends <input type="month" bind:value={laterTo} disabled={busy} /></label>
    <button onclick={readConfiguration} disabled={busy || config.phase === 'loading'}>Check sweep readiness</button>
  </div>
  <p>The two dates above must come after the training period selected in the main date controls.
    The later comparison is part of research selection; it is not an untouched final holdout.
    Costs are excluded. The settings saved below belong to this sweep.</p>
  <details>
    <summary>Advanced settings and acceptance checks</summary>
    <p>Available values below come from explicit server configuration. Blank values have no configured
      default. Values are checked again by the server and retained in the request and search identity.</p>
    <div class="settings">
      {#each fields as [key,wire,label,help] (key)}
        <label>{label}<input type="text" inputmode="numeric" autocomplete="off" bind:value={settings[key]}
          disabled={busy} onchange={() => { if (key === 'maxPoints') void readConfiguration(); }} /><small>{help}</small></label>
      {/each}
    </div>
    {#if config.body}
      <p>Input bytes: {config.body.limits.checksum_max_bytes ?? 'Unconfigured'} · input records:
        {config.body.limits.checksum_max_records ?? 'Unconfigured'} · observation bytes:
        {config.body.limits.observation_bytes ?? 'Unconfigured'} · replay choices:
        {config.body.limits.replay_nodes ?? 'Unconfigured'}.</p>
      {#if config.body.policy.ready}
        <div class="scroll"><table><caption>All server-resolved research policy values</caption>
          <thead><tr><th>Rule and unit</th><th>Required value</th></tr></thead><tbody>
            {#each config.body.policy.values as value (value.name)}<tr><th>{value.name.replaceAll('_', ' ')}</th><td>{typeof value.value === 'boolean' ? value.value ? 'Required' : 'Not required' : value.value}</td></tr>{/each}
          </tbody></table></div>
        <p class="identity">Policy identity: {config.body.policy.digest}</p>
      {:else}<p role="alert">Research policy unavailable: {config.body.policy.refusal || 'No validated policy was supplied.'}</p>{/if}
    {/if}
  </details>
  <div class="controls">
    <button class="launch" onclick={start} disabled={!active || busy || !!blockedReason || !!serverWhy || !prepared.plan}>Run brute-force sweep</button>
    {#if run.attempt && ['running','unknown'].includes(run.phase)}<button onclick={recheck}>Recheck this exact attempt</button>{/if}
  </div>
  {#if blockedReason}<p role="alert">{blockedReason}</p>{/if}
  {#if serverWhy}<p role={config.phase === 'loading' ? 'status' : 'alert'}>{serverWhy}</p>{/if}
  {#if !prepared.plan && !serverWhy}<p class="hint">{prepared.why}</p>{/if}
  {#if localWhy}<p role="alert">{localWhy}</p>{/if}
  {#if run.why}<p role="alert">{run.why}</p>{/if}
  {#if run.attempt}
    <p class="identity" role="status">Exact request: {run.attempt}
      {#if run.completedBatches !== null} · {run.completedBatches} saved batches{/if}</p>
  {/if}
  {#if run.searchIdentity}
    <p><a href={`#saved-boolean-research`} onclick={() => onSearch(run.searchIdentity)}>View this search’s saved progress, settings and trades</a>.</p>
    <p class="identity">Search: {run.searchIdentity}</p>
  {/if}
  {#if run.plan}
    <details><summary>Exact submitted scope</summary>
      <p>Feed: {run.plan.feed} · instruments: {run.plan.symbols.join(', ')} · timeframes: {booleanPlanTimeframes(run.plan).join(' · ')}.</p>
      <p>Training: {run.plan.from_year}-{String(run.plan.from_month).padStart(2, '0')} →
        {run.plan.to_year}-{String(run.plan.to_month).padStart(2, '0')}. Later evaluation:
        {run.plan.later_from_year}-{String(run.plan.later_from_month).padStart(2, '0')} →
        {run.plan.later_to_year}-{String(run.plan.later_to_month).padStart(2, '0')}.</p>
      <p>Holding period: {run.plan.horizon_bars} bars · price ceiling: {run.plan.max_points} points ·
        {run.plan.batch_programs} expressions and {run.plan.node_allowance} grammar choices per batch ·
        {run.plan.batch_allowance} batches in this request.</p>
      <p class="identity">Submitted policy: {run.plan.expected_policy_digest}</p>
      <p>Changing controls above does not change this captured request.</p>
    </details>
  {/if}
  {#if run.phase === 'done'}
    <p>{run.exhausted === true ? 'The server recorded that this declared grammar is exhausted.'
      : run.exhausted === false ? 'The saved search is paused after this work allowance. The complete grammar has not been exhausted.'
      : 'The command ended, but no grammar-exhaustion receipt was supplied.'}</p>
  {/if}
  {#if run.report}<details><summary>Saved command report</summary><pre>{run.report}</pre></details>{/if}
  <p class="hint">An uncertain submission is never automatically sent again. Closing this page stops
    observation; an accepted server job continues. Saved outcomes distinguish refusal, a paused allowance,
    complete grammar exhaustion and strategy admission. A recorded trade does not establish a profitable edge.</p>
</section>

<style>
  .boolean-launch{width:100%;padding:1rem 0;border-top:1px solid var(--line,#566d7944)}h3{font-size:1.05rem;margin:.2rem 0 .8rem}p{font-size:.85rem;line-height:1.6;max-width:110ch}.scope,.controls{display:flex;flex-wrap:wrap;gap:.8rem;align-items:end;margin:1rem 0}.scope span{padding:.6rem .8rem;border:1px solid var(--line,#566d7944);border-radius:6px;font-size:.8rem}.scope b{display:block;margin-bottom:.25rem}label{display:grid;gap:.4rem;font-size:.85rem}input,button{font:inherit;color:inherit;border:1px solid var(--line,#506879);background:transparent;border-radius:5px;padding:.6rem .8rem}button,summary{cursor:pointer}button:disabled,input:disabled{opacity:.5;cursor:not-allowed}.launch{background:var(--acc-soft,#e8f2f7);font-weight:650}input:focus-visible,button:focus-visible,summary:focus-visible,a:focus-visible{outline:2px solid var(--acc,#33839b);outline-offset:2px}.settings{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:1rem;margin:1rem 0}.settings small,.hint{opacity:.8}small{font-size:.75rem;line-height:1.5}details{padding:.8rem 0}summary{font-weight:600}.scroll{overflow:auto}table{width:100%;border-collapse:collapse;font-size:.82rem;text-align:left}caption{text-align:left;font-weight:600;padding:.75rem 0}th,td{padding:.65rem;border-bottom:1px solid var(--line,#566d7933)}.identity{overflow-wrap:anywhere;font-variant-numeric:tabular-nums}pre{white-space:pre-wrap;overflow-wrap:anywhere;max-height:26rem;overflow:auto;font-size:.78rem}[role=alert]{color:var(--down,#b14a3c)}
</style>

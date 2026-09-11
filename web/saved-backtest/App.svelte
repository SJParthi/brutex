<script>
  import { onMount, tick } from 'svelte';
  import { ask } from '../src/lib/ask.js';
  import { fetchQualifiedSearch } from '../src/lib/boolean-qualified-search.js';
  import { fetchBooleanLater } from '../src/lib/boolean-oos.js';
  import { fetchBooleanCatalog, catalogDay } from '../src/lib/boolean-catalog.js';
  import { candidateMoney, candidateTime } from '../src/lib/candidate-trades.js';
  import { CAMPAIGN_RUNGS } from '../src/lib/boolean-campaign.js';
  import { savedSelection, savedSettingIndex, latestRequest } from './selection.js';
  import { loadSettingFacts, assertTradeSetting } from './setting-pages.js';
  import { fetchWithBusyRetry } from './requests.js';
  import { decisionExplanation, resultReason, signalExplanation, exitExplanation, periodComparison, checkExplanation, formatPaisa, SETTING_EXPLANATION, RETURN_EXPLANATION, VWAP_EXPLANATION, UNKNOWN_EXPLANATION, timeframeExplanation } from './explanations.js';

  let overview = $state({ phase: 'loading', body: null, why: '' });
  let detail = $state({ phase: 'idle', body: null, why: '' });
  let trades = $state({ phase: 'idle', body: null, why: '' });
  let facts = $state({ phase: 'idle', body: null, why: '' });
  let vocabulary = $state(null);
  let busyMessage = $state(''), busyOwner = null;
  let viewer = $state(null), selected = $state(null), selectedComparison = $state(null);
  let period = $state('later'), batch = $state('0'), rung = $state('0'), jump = $state('0');
  let elapsed = $state(''), selectionError = $state(''), initial, selectionGeneration = 0;
  const overviewRequest = latestRequest(value => overview = value);
  const detailRequest = latestRequest(value => detail = value);
  const tradeRequest = latestRequest(value => trades = value);
  const factRequest = latestRequest(value => facts = value);
  const request = signal => async url => {
    const owner = Symbol();
    try {
      return await fetchWithBusyRetry(url, { signal, onBusy: retry => {
        busyOwner = owner;
        busyMessage = `A saved-evidence reader is busy. Retrying this read (${retry.nextAttempt}/3) in ${retry.delayMs} ms…`;
      } }, ask);
    } finally {
      if (busyOwner === owner) { busyMessage = ''; busyOwner = null; }
    }
  };
  const number = value => BigInt(value).toLocaleString('en-IN');
  const label = value => value.replaceAll('_', ' ').replaceAll('-', ' ');
  const sum = (body, key) => body.rows.reduce((total, row) => total + BigInt(key === 'count' ? row.count : row.counts[key]), 0n).toLocaleString('en-IN');

  onMount(() => {
    const startup = new AbortController();
    async function start() {
      try {
        initial = savedSelection(location.search);
        const response = await ask('/viewer.json', { signal: startup.signal });
        if (!response.ok) throw new Error('The read-only viewer identity could not be read.');
        viewer = await response.json();
        if (startup.signal.aborted) return;
        // Names are optional, but must belong to the exact saved engine build.
        try {
          const names = await ask('/vocab.json', { signal: startup.signal });
          if (names.ok) {
            const body = await names.json();
            if (body.api_commit === viewer.vocabulary_commit && /^[0-9a-f]{64}$/.test(body.commit_digest) &&
                Number.isInteger(body.vocab_version) && body.vocab_version > 0 && Array.isArray(body.bits) &&
                body.count === body.bits.length && body.bits.length > 0 && body.bits.length <= 65536 &&
                body.bits.every((row, i) => row.i === i && typeof row.name === 'string' && row.name.length > 0)) vocabulary = body;
          }
        } catch { /* The raw expression remains visible if names are unavailable. */ }
        if (startup.signal.aborted) return;
        await loadOverview(initial);
      } catch (why) {
        if (!startup.signal.aborted) overview = { phase: 'failed', body: null, why: String(why) };
      }
    }
    void start();
    return () => { startup.abort(); overviewRequest.stop(); detailRequest.stop(); tradeRequest.stop(); factRequest.stop(); };
  });
  async function loadOverview(selection) {
    const body = await overviewRequest.run(signal => fetchQualifiedSearch({ identity: selection.identity, pin: selection.pin }, request(signal)));
    if (!body) return;
    batch = selection.batch ?? body.batch;
    rung = selection.rung ?? '0';
    await openSettings(selection.offset ?? '0', selection.setting ?? null);
  }
  async function openSettings(offset = '0', setting = null, opened = null) {
    const generation = ++selectionGeneration;
    selectionError = ''; elapsed = '';
    factRequest.stop(); facts = { phase: 'idle', body: null, why: '' };
    tradeRequest.stop(); trades = { phase: 'idle', body: null, why: '' }; selected = null; selectedComparison = null;
    jump = offset;
    const parent = overview.body;
    const previousDetail = detail.body;
    const askedBatch = opened?.batch ?? batch, askedRung = opened?.rung ?? rung;
    const started = performance.now();
    const body = await detailRequest.run(async signal => {
      const base = { identity: parent.identity, pin: parent.pin, batch: askedBatch, rung: askedRung };
      let completion = parent.batch === askedBatch ? parent.rows[Number(askedRung)]?.qualification?.completion : null;
      if (!completion && previousDetail?.batch === askedBatch && previousDetail?.rung === askedRung && previousDetail?.pin === parent.pin) completion = previousDetail.completion;
      // Older direct links first authenticate that child's initial page. A
      // continued page always carries its exact completion, never a guessed pin.
      if (!completion && offset !== '0') {
        const first = await fetchQualifiedSearch({ ...base, offset: '0', limit: 1 }, request(signal));
        completion = first.completion;
      }
      return fetchQualifiedSearch({ ...base, completion: completion ?? null, offset, limit: 32 }, request(signal));
    });
    if (!body) return;
    let index = -1;
    try { index = savedSettingIndex(body.source.rows, setting); }
    catch (why) { selectionError = String(why); }
    const loadedFacts = await factRequest.run(signal => loadSettingFacts(body.source.rows, request(signal), signal));
    if (generation !== selectionGeneration) return;
    elapsed = ((performance.now() - started) / 1000).toFixed(2);
    if (loadedFacts && index >= 0) await openSetting(body.source.rows[index], body.rows[index].comparison);
  }
  async function openSetting(row, comparison) {
    if (facts.phase !== 'ready' || !facts.body?.has(row.index)) return;
    const generation = selectionGeneration;
    selectionError = '';
    selected = row; selectedComparison = comparison; period = 'later';
    const loaded = await loadTrades('0', 'trades');
    if (!loaded) return;
    await tick();
    if (generation === selectionGeneration && selected?.index === row.index && period === 'later' && trades.phase === 'ready') {
      document.getElementById('trade-section')?.scrollIntoView({ block: 'start' });
    }
  }
  function namedRule(fact) {
    const names = vocabulary && fact?.grids?.every(grid => grid.commit === vocabulary.commit_digest)
      ? vocabulary.bits.map(row => ({ index: row.i, name: row.name })) : [];
    return signalExplanation(fact?.training?.expression, names);
  }
  function decisionReason(comparison, fact) {
    return resultReason(comparison, fact, detail.body.search_policy);
  }
  async function loadTrades(offset = '0', kind = 'trades') {
    const row = selected, isLater = period === 'later';
    if (!row || facts.phase !== 'ready') return null;
    const fact = facts.body?.get(row.index);
    if (!fact) return null;
    const source = isLater ? row.qualification.source : row.statistics.source;
    const candidate = isLater ? row.qualification.coordinate : row.statistics.coordinate;
    return tradeRequest.run(async signal => {
      const body = await (isLater ? fetchBooleanLater : fetchBooleanCatalog)({ identity: source.identity, completion: source.completion, candidate, kind, offset, limit: 32 }, request(signal));
      return assertTradeSetting(body, row, isLater, fact);
    });
  }
  function previous(offset, limit) { const value = BigInt(offset) - BigInt(limit); return String(value > 0n ? value : 0n); }
  function shareLink() {
    if (!detail.body) return location.href;
    const d = detail.body;
    const q = new URLSearchParams({ identity: d.identity, pin: d.pin, batch: d.batch, rung: d.rung, offset: d.source.offset });
    if (selected) q.set('setting', selected.index);
    return '/backtest?' + q;
  }
</script>

<a class="skip" href="#content">Skip to results</a>
<header><a class="brand" href={location.href}>brutex<span>Research results</span></a><nav aria-label="Applications"><a href="http://127.0.0.1:8080/db">Main app · DB</a><span class="mode">Saved results · read only</span></nav></header>
<main id="content">
  {#if busyMessage}<p class="inline-status" role="status">{busyMessage}</p>{/if}
  <div class="intro"><div><p class="eyebrow">FROM TRADING RULE TO RECORDED OUTCOME</p><h1>Backtest results.</h1><p>See what was tested, how it performed, and why it passed or failed.</p></div><p class="scope">Real OHLCV · intraday only<br />Forced exit by 15:10 IST · costs excluded<br />Saved simulations, not actual exchange executions.</p></div>
  {#if overview.phase === 'loading'}<div class="message" role="status">Reading the saved search checkpoint…</div>
  {:else if overview.phase === 'failed'}<div class="message failure" role="alert"><h2>Saved results unavailable</h2><p>{overview.why}</p><p>No empty ledger or zero result is substituted for this failure.</p></div>
  {:else if overview.phase === 'ready'}
    {@const o = overview.body}
    <section class="verdict" aria-label="What these results mean"><div class="verdict-label">LATEST BATCH · {o.batch}</div><h2>{sum(o, 'admitted') === '0' ? 'No settings passed the saved research checks.' : sum(o, 'admitted') + ' settings passed the saved research checks.'}</h2><p>{sum(o, 'admitted') === '0' ? 'These results help you identify failed rules and missing evidence. They do not identify an approved strategy.' : 'Inspect the evidence and its limits before drawing conclusions from a saved pass.'} {o.exhausted ? 'The declared search is complete.' : 'The declared search is still incomplete.'}</p></section>
    <section class="overview" aria-label="Saved search summary">
      <div class="state"><b>{o.exhausted ? 'Declared search complete' : o.state === 'refused' ? 'Search refused' : 'Saved checkpoint'}</b><span>{o.completed_batches} batches recorded · {o.programs} programs</span><p>{o.exhausted ? 'The declared grammar is exhausted.' : 'The declared search is not exhausted. These results are a saved verification run.'}</p></div>
      <dl><div><dt>Settings tested</dt><dd>{sum(o, 'count')}</dd></div><div><dt>Passed checks</dt><dd>{sum(o, 'admitted')}</dd></div><div><dt>Failed measured limits</dt><dd>{sum(o, 'rejected')}</dd></div><div><dt>Evidence refused</dt><dd>{sum(o, 'refused')}</dd></div><div><dt>Evidence missing</dt><dd>{sum(o, 'unmeasured')}</dd></div></dl>
    </section>
    <div class="reading-guide"><div><b>1. Compare settings</b><p>Each row is one combination of entry and exit rules. The setting number is its saved address, not its rank.</p></div><div><b>2. Compare periods</b><p>Training found the setting. The later period replayed the same rules on subsequent data.</p></div><div><b>3. Inspect the evidence</b><p>Open a setting for exact trades, zero-trade days, and the required checks. A refusal can still retain trade observations.</p></div></div>
    <p class="explanation">Counts above cover the latest saved batch across all eight timeframes. Trade observations can repeat across settings; adding them does not count unique market opportunities.</p>
    {#if o.reason}<p class="message failure" role="alert">Recorded search refusal: {o.reason}</p>{/if}
    <div class="workspace">
      <aside><h2>Timeframes</h2><p>Latest-batch counts</p><nav aria-label="Timeframes">{#each o.rows as row}<button class:active={rung === row.rung_index} onclick={() => { rung = row.rung_index; void openSettings(); }}><span>{row.rung}</span><small>{number(row.count)} settings</small></button>{/each}</nav><label class="batch">Batch (starts at 0)<input bind:value={batch} inputmode="numeric" /></label><button class="secondary" onclick={() => openSettings()}>Read this batch</button></aside>
      <section class="settings" aria-label="Settings comparison">
        <div class="section-title"><div><h2>Settings comparison</h2><p>{CAMPAIGN_RUNGS[Number(detail.body?.rung ?? rung)]} · batch {detail.body?.batch ?? batch}</p></div>{#if detail.phase === 'ready' && facts.phase === 'ready' && elapsed}<span class="timing">Comparison loaded in {elapsed} s</span>{/if}</div>
        {#if detail.phase === 'loading'}<p class="message" role="status">Checking this timeframe’s saved evidence and loading 32 settings…</p>
        {:else if detail.phase === 'failed'}<p class="message failure" role="alert">{detail.why}</p>
        {:else if detail.phase === 'ready'}
          {@const d = detail.body}{@const s = d.source}
          {#if selectionError}<p class="message failure" role="alert">{selectionError}</p>{/if}
          <p class="table-note">{timeframeExplanation(CAMPAIGN_RUNGS[Number(d.rung)])} Returns below are pessimistic totals for one unit per trade, before costs.</p>
          {#if facts.phase === 'loading'}<p class="inline-status" role="status">Loading the exact entry rules, exits and later results for this page…</p>{:else if facts.phase === 'failed'}<p class="message failure" role="alert">Rule comparison could not be loaded: {facts.why} <button onclick={() => openSettings(s.offset, null, d)}>Retry this page</button></p>{/if}
          <div class="table-wrap"><table class="comparison-table"><caption>{SETTING_EXPLANATION}</caption><thead><tr><th>Setting &amp; entry rule</th><th>Exit rules</th><th>Training</th><th>Later period</th><th>Decision &amp; reason</th><th>Inspect</th></tr></thead><tbody>{#each s.rows as row, index}
            {@const fact = facts.body?.get(row.index)}{@const comparison = d.rows[index].comparison}{@const periods = periodComparison(fact?.training ?? row.statistics, fact?.later)}
            <tr class:selected={selected?.index === row.index}><th><span class="setting-id">#{row.index} · {row.statistics.side === 'short' ? 'Short' : 'Long'}</span><strong>{row.statistics.source.instrument}</strong><span class="rule-name">{fact ? namedRule(fact) : 'Entry rule not loaded'}</span></th><td class="exits">{#if fact}{#each exitExplanation(fact.training, fact.grid) as exit}<span><b>{exit.label}:</b> {exit.value}</span>{/each}{:else}Exit rules not loaded{/if}</td><td class="period-total"><strong>{periods.training.pessimistic}</strong><small>{periods.training.trades} trades</small></td><td class="period-total"><strong>{periods.later.pessimistic}</strong><small>{periods.later.trades} trades</small></td><td><span class={'badge ' + comparison.status}>{decisionExplanation(comparison.status).label}</span><small class="decision-reason">{decisionReason(comparison, fact)}</small></td><td><button class="row-action" disabled={facts.phase !== 'ready' || !fact} onclick={() => openSetting(row, comparison)}>Inspect setting</button></td></tr>
          {/each}</tbody></table></div>
          <div class="pager"><span>{#if s.rows.length}Settings {s.offset}–{String(BigInt(s.offset) + BigInt(s.rows.length) - 1n)} of {number(s.total)}{:else}No settings at offset {s.offset}; {number(s.total)} settings recorded{/if}</span><button disabled={s.offset === '0'} onclick={() => openSettings(previous(s.offset, s.limit), null, d)}>Previous settings</button><button disabled={s.next === null} onclick={() => openSettings(s.next, null, d)}>Next settings</button><form onsubmit={event => { event.preventDefault(); void openSettings(jump, null, d); }}><label>Start at setting <input bind:value={jump} inputmode="numeric" /></label><button>Go</button></form></div>
          <details class="audit"><summary>Policy, checks and saved identities</summary><p>Calculation version {d.projection_version}. The overview checks recorded history; opening this table also authenticates its saved child evidence. Current raw market files are not reread by this viewer.</p><dl class="identities"><dt>Search</dt><dd>{d.identity}</dd><dt>Checkpoint</dt><dd>{d.pin}</dd><dt>Child completion</dt><dd>{d.completion}</dd></dl><div class="table-wrap"><table><caption>Original and effective policy values</caption><thead><tr><th>Setting / unit</th><th>Original</th><th>Effective</th></tr></thead><tbody>{#each s.policy.values as value, index}<tr><th>{label(value.name)}</th><td>{value.value}</td><td>{d.search_policy.values[index].value}</td></tr>{/each}</tbody></table></div></details>
        {/if}
      </section>
    </div>
    {#if selected}
      {@const selectedFacts = facts.body?.get(selected.index)}
      {@const periods = periodComparison(selectedFacts?.training ?? selected.statistics, selectedFacts?.later)}
      <section class="trade-section" id="trade-section">
        <div class="section-title"><div><h2>{selected.statistics.source.instrument} · setting {selected.index}</h2><p>{selected.statistics.side === 'short' ? 'Short: benefits from a price fall' : 'Long: benefits from a price rise'} · <span class={'badge ' + selectedComparison.status}>{decisionExplanation(selectedComparison.status).label}</span></p></div><a href={shareLink()}>Link to this exact result</a></div>
        <div class="setting-summary">
          <div class="rule-card"><span class="eyebrow">ENTRY RULE</span><h3>{namedRule(selectedFacts)}</h3><p>{timeframeExplanation(CAMPAIGN_RUNGS[Number(detail.body.rung)])}</p>{#if selectedFacts}<dl class="exit-list">{#each exitExplanation(selectedFacts.training, selectedFacts.grid) as exit}<div><dt>{exit.label}</dt><dd>{exit.value}</dd></div>{/each}</dl><details><summary>How to read this rule</summary><p>{VWAP_EXPLANATION}</p><p>{UNKNOWN_EXPLANATION}</p><p>Saved expression: <code>{selectedFacts.training.expression}</code></p></details>{/if}</div>
          <div class="period-card">
            <h3>Did it hold up in the later period?</h3><table><caption>Same entry and exit settings in both periods</caption><thead><tr><th>Recorded measure</th><th>Training</th><th>Later period</th></tr></thead><tbody><tr><th>Simulated trades</th><td>{periods.training.trades}</td><td>{periods.later.trades}</td></tr><tr><th>Winning trades</th><td>{periods.training.wins}</td><td>{periods.later.wins}</td></tr><tr><th>Pessimistic total</th><td>{periods.training.pessimistic}</td><td>{periods.later.pessimistic}</td></tr><tr><th>Optimistic total</th><td>{periods.training.optimistic}</td><td>{periods.later.optimistic}</td></tr><tr><th>Sessions with signals</th><td>{periods.training.supportSessions}</td><td>{periods.later.supportSessions}</td></tr><tr><th>Bars with unknown rule</th><td>{periods.training.unknown}</td><td>{periods.later.unknown}</td></tr></tbody></table><p>{RETURN_EXPLANATION}</p>
            {#if selectedFacts}<p class="scope">Training: {candidateTime(selectedFacts.grid.first_micros)} – {candidateTime(selectedFacts.grid.last_micros)}.<br />Later: {candidateTime(selectedFacts.laterPeriod.first_micros)} – {candidateTime(selectedFacts.laterPeriod.last_micros)}.</p>{/if}
          </div>
        </div>
        <div class="decision-panel"><h3>Research decision</h3><p><b>{decisionReason(selectedComparison, selectedFacts)}.</b> {decisionExplanation(selectedComparison.status).detail}</p><details><summary>Compare all {selectedComparison.checks.length} checks with the saved requirements</summary><div class="table-wrap"><table><caption>Actual saved evidence and limits; no policy threshold is invented by this page.</caption><thead><tr><th>Check</th><th>Observed</th><th>Required</th><th>Outcome</th></tr></thead><tbody>{#each selectedComparison.checks as check}{@const explained = checkExplanation(check, selectedComparison.values, detail.body.search_policy)}<tr><th>{explained.label}</th><td>{explained.observed}</td><td>{explained.required}</td><td>{explained.outcome}</td></tr>{/each}</tbody></table></div></details></div>
        <div class="tabs" aria-label="Observation period"><button class:active={period === 'later'} onclick={() => { period = 'later'; void loadTrades(); }}>Later-period trades</button><button class:active={period === 'training'} onclick={() => { period = 'training'; void loadTrades(); }}>Training trades</button><button onclick={() => loadTrades('0', 'sessions')}>Trading days, including zero trades</button></div>
        {#if trades.phase === 'loading'}<p class="message" role="status">Authenticating this exact setting and reading its saved trade page…</p>
        {:else if trades.phase === 'failed'}<p class="message failure" role="alert">{trades.why}</p>
        {:else if trades.phase === 'ready'}
          {@const t = trades.body}{@const c = period === 'later' ? t.selected.later : t.selected}
          <div class="trade-facts"><strong>{t.kind === 'trades' ? number(t.total) + ' saved simulated trades' : number(t.total) + ' trading days'}</strong><span>{period === 'later' ? 'Later comparison' : 'Original training'}</span></div>
          <p class="scope">{candidateTime(period === 'later' ? t.later.first_micros : t.grids[0].first_micros)} to {candidateTime(period === 'later' ? t.later.last_micros : t.grids[0].last_micros)}. {c.execution_refusals.length ? 'Recorded execution refusals: ' + c.execution_refusals.join('; ') + '.' : 'No execution refusal recorded for this setting.'}</p>
          <div class="table-wrap"><table>
            {#if t.kind === 'trades'}<caption>Each row is one simulated round trip. Pessimistic and optimistic returns show the saved candle-ordering range.</caption><thead><tr><th>Trade</th><th>Entry bar (IST)</th><th>Exit bar (IST)</th><th>Pessimistic return</th><th>Optimistic return</th><th>Largest adverse / favourable move</th></tr></thead><tbody>{#each t.rows as row}<tr><th>{row.index}</th><td>{candidateTime(row.entry_micros)}</td><td>{candidateTime(row.exit_micros)}</td><td>{formatPaisa(row.worst)}</td><td>{formatPaisa(row.best)}</td><td>{formatPaisa(row.adverse_paisa)} / {formatPaisa(row.favourable_paisa)}</td></tr>{/each}</tbody>
            {:else}<caption>Every recorded session, including zero-trade days.</caption><thead><tr><th>Trading day</th><th>Trades</th><th>Wins</th><th>Return per unit</th></tr></thead><tbody>{#each t.rows as row}<tr><th>{catalogDay(row.day)}</th><td>{row.trades}</td><td>{row.wins}</td><td>{candidateMoney(row.return_paisa)}</td></tr>{/each}</tbody>{/if}
          </table></div>
          {#if t.total === '0'}<p class="message">No trades were recorded for this setting in this period. This is a saved zero result, not a missing page.</p>{/if}
          <div class="pager"><span>{t.rows.length} rows shown, starting at {t.offset}</span><button disabled={t.offset === '0'} onclick={() => loadTrades(previous(t.offset, t.limit), t.kind)}>Previous rows</button><button disabled={t.next === null} onclick={() => loadTrades(t.next, t.kind)}>Next rows</button></div>
          <p class="scope">Times identify bar opens. The 15:09 minute bar closes at the 15:10 intraday deadline. Stored source is the projected execution-fill index, not the original completed signal index. OHLCV cannot identify the order of movements inside a candle. Trade observations can repeat across tested settings.</p>
          <details class="audit"><summary>Frozen exits, source indices and exact trade receipt</summary><pre>{JSON.stringify({ identity: t.identity, completion: t.completion, setting: c, rows: t.rows }, null, 2)}</pre></details>
        {/if}
      </section>
    {/if}
  {/if}
  <footer><b>Separate saved-research store</b><p>This viewer only reads saved evidence. It cannot pull data, start a sweep or change the production ledger.</p>{#if viewer}<details><summary>Store and viewer configuration</summary><pre>{JSON.stringify(viewer, null, 2)}</pre></details>{/if}</footer>
</main>

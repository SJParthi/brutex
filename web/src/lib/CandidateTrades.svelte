<script>
  import { ask } from './ask.js';
  import { fetchCandidatePage, candidateMoney, candidateTotal, candidatePricingDetails, candidateConditions, candidateTime } from './candidate-trades.js';
  let { identity, attempt, model = 'and-mask', initialDigest = /** @type {string | null} */ (null), vocabulary = /** @type {any} */ (null) } = $props();
  let loaded = $state(/** @type {any} */ ({ phase: 'loading', body: null, why: '' }));
  let trades = $state(/** @type {any} */ ({ phase: 'idle', body: null, why: '' }));
  let tier = $state('0');
  let generation = 0;
  let tradeGeneration = 0;

  /** @param {any} selection */
  async function load(selection) {
    const ticket = ++generation;
    tradeGeneration += 1;
    trades = { phase: 'idle', body: null, why: '' };
    loaded = { phase: 'loading', body: null, why: '' };
    try {
      const body = await fetchCandidatePage(selection, (url) => ask(url, { cache: 'no-store' }));
      if (ticket === generation) loaded = { phase: 'ready', body, why: '' };
    } catch (why) {
      if (ticket === generation) loaded = { phase: 'failed', body: null, why: why instanceof Error ? why.message : String(why) };
    }
  }
  $effect(() => {
    const id = identity, token = attempt, predicateModel = model, digest = initialDigest;
    tier = '0';
    void load({ identity: id, attempt: token, model: predicateModel, digest });
    return () => { generation += 1; tradeGeneration += 1; };
  });
  /** @param {string} offset */
  function candidates(offset) {
    void load({ identity, attempt, model, digest: loaded.body.catalog_digest, tier: loaded.body.tier.index, offset });
  }
  function chooseTier() {
    void load({ identity, attempt, model, digest: loaded.body.catalog_digest, tier, offset: '0' });
  }
  /** @param {any} candidate @param {string} offset */
  async function showTrades(candidate, offset = '0') {
    const ticket = ++tradeGeneration;
    trades = { phase: 'loading', body: null, why: '' };
    try {
      const body = await fetchCandidatePage({ identity, attempt, model, digest: loaded.body.catalog_digest, tier: candidate.tier, rank: candidate.rank, direction: candidate.direction, offset }, (url) => ask(url, { cache: 'no-store' }));
      if (ticket === tradeGeneration) trades = { phase: 'ready', body, why: '' };
    } catch (why) {
      if (ticket === tradeGeneration) trades = { phase: 'failed', body: null, why: why instanceof Error ? why.message : String(why) };
    }
  }
  /** @param {string} offset */
  const previous = (offset) => String(BigInt(offset) > 256n ? BigInt(offset) - 256n : 0n);
</script>

<section class="candidate-detail" aria-label="Exact candidate trades">
  <div class="heading"><div><span class="eyebrow">Every priced candidate</span><h3>Explore the exact trades behind a result</h3></div><button onclick={() => { tier = '0'; void load({ identity, attempt, model, digest: initialDigest }); }}>Refresh capture</button></div>
  <p class="scope">Both directions, for each candidate actually priced in each visited policy pass. Each row opens its own selected exit cell. Pricing evidence and whole-audit completion are separate facts.</p>
  {#if loaded.phase === 'loading'}<p role="status">Verifying the saved candidate catalog…</p>
  {:else if loaded.phase === 'failed'}<p role="alert" class="problem">Candidate evidence unavailable: {loaded.why}</p>
  {:else if loaded.body.status === 'missing'}<p class="notice">{loaded.body.why}</p>
  {:else}
    {@const body = loaded.body}
    {#if body.expression}<p class="notice">Expression pricing uses the complete saved predicate. The condition names below show referenced bits only; they do not imply AND. <b>{body.expression.source ?? 'Versioned predicate saved'}</b></p><details><summary>Exact versioned predicate bytes</summary><code class="predicate">{body.expression.encoded_hex}</code></details>{/if}
    <p class="facts">Saved candidate-side outcomes <b>{body.candidate_side_count}</b> · Visited policy passes <b>{body.tier_count}</b> · Audit state <b>{body.audit_completion ?? 'unknown'}</b></p>
    {#if body.tier}
      <div class="chooser"><label>Policy pass (starts at 0) <input inputmode="numeric" bind:value={tier} /></label><button onclick={chooseTier}>Open pass</button></div>
      <p class="notice">This pass priced <b>{body.tier.evaluated}</b> of <b>{body.tier.eligible}</b> retained signal candidates in both directions. Hold: {body.tier.horizon} execution bars. A cell-rule pass does not establish calendar consistency or institutional admission.</p>
      <details><summary>Applied policy and exact grid inputs</summary><dl>{#each Object.entries(body.tier.rules) as [name, value]}<dt>{name.replaceAll('_', ' ')}</dt><dd>{String(value)}</dd>{/each}<dt>Grid rungs</dt><dd>{body.tier.rungs}</dd><dt>Forced stop, ppm</dt><dd>{body.tier.forced_ppm ?? 'none'}</dd><dt>Grid step, ppm</dt><dd>{body.tier.step_ppm ?? 'derived'}</dd></dl></details>
      <div class="scroll"><table><caption>Exact candidate-side results · {body.total_count} in this pass</caption><thead><tr><th>Signal rank</th><th>Direction</th><th>Conditions</th><th>Trades</th><th>Worst-fill total</th><th>Best-fill total</th><th>Cell rules</th><th>Detail</th></tr></thead><tbody>
        {#each body.rows as row (`${row.tier}-${row.rank}-${row.direction}`)}
          {@const saved = candidatePricingDetails(row)}
          <tr><td>{row.rank}</td><td>{row.direction}</td><td class="conditions">{candidateConditions(row, vocabulary)}</td><td>{row.cell?.trades ?? 'No selected cell'}</td><td>{candidateTotal(row, 'pessimistic')}</td><td>{candidateTotal(row, 'optimistic')}</td><td>{row.cell ? (row.cell_rules_pass ? 'Pass' : 'Does not pass') : 'No selected cell'}</td><td><button disabled={!row.cell} onclick={() => showTrades(row)}>Exact trades</button>
            {#if saved.length > 0}<details class="saved-details"><summary>Signals and exit settings</summary><p>Saved values only. Choice numbers start at 0; ppm means parts per million. A refused-path count does not identify its individual causes.</p><dl>{#each saved as field (field.label)}<dt>{field.label}</dt><dd>{field.value}</dd>{/each}</dl></details>{/if}
          </td></tr>
        {/each}
      </tbody></table></div>
      {#if body.rows.length === 0}<p class="notice">This pass explicitly recorded no candidate-side rows.</p>{/if}
      <div class="paging"><button disabled={body.offset === '0'} onclick={() => candidates(previous(body.offset))}>Previous candidates</button><span>Rows from {body.offset}</span><button disabled={body.next_offset === null} onclick={() => candidates(body.next_offset)}>Next candidates</button></div>
    {:else}<p class="notice">The capture is sealed with no visited pricing pass. This does not imply a completed profitable search.</p>{/if}
  {/if}
  {#if trades.phase === 'loading'}<p role="status">Verifying this candidate’s exact trade file…</p>
  {:else if trades.phase === 'failed'}<p role="alert" class="problem">Exact trades unavailable: {trades.why} No other candidate’s trades are substituted.</p>
  {:else if trades.phase === 'ready'}
    {@const detail = trades.body}
    <h4>Signal rank {detail.selected.rank} · {detail.selected.direction} · policy pass {detail.selected.tier}</h4>
    <p class="scope">{candidateConditions(detail.selected, vocabulary)}</p>
    <div class="scroll"><table><caption>Exact selected-cell trades · {detail.total_count} recorded</caption><thead><tr><th>Trade</th><th>Entry (IST)</th><th>Exit (IST)</th><th>Entry bar</th><th>Exit bar</th><th>Worst fill</th><th>Best fill</th><th>Adverse move</th><th>Favourable move</th></tr></thead><tbody>{#each detail.rows as row (row.seq)}<tr><td>{String(BigInt(row.seq) + 1n)}</td><td title={row.entry_micros + ' µs'}>{candidateTime(row.entry_micros)}</td><td title={row.exit_micros + ' µs'}>{candidateTime(row.exit_micros)}</td><td>{row.entry_bar}</td><td>{row.exit_bar}</td><td>{candidateMoney(row.worst)}</td><td>{candidateMoney(row.best)}</td><td>{candidateMoney(row.adverse_paisa)}</td><td>{candidateMoney(row.favourable_paisa)}</td></tr>{/each}</tbody></table></div>
    {#if detail.rows.length === 0}<p class="notice">This exact cell recorded zero trades.</p>{/if}
    <div class="paging"><button disabled={detail.offset === '0'} onclick={() => showTrades(detail.selected, previous(detail.offset))}>Previous trades</button><span>Rows from {detail.offset}</span><button disabled={detail.next_offset === null} onclick={() => showTrades(detail.selected, detail.next_offset)}>Next trades</button></div>
  {/if}
</section>

<style>
  .saved-details{margin-top:.65rem;min-width:18rem;max-width:34rem;white-space:normal}.saved-details p{font-size:.75rem;line-height:1.45;opacity:.8}.saved-details dl{grid-template-columns:minmax(9rem,1fr) minmax(10rem,1.4fr)}.saved-details dd{overflow-wrap:anywhere}
  .predicate{display:block;overflow-wrap:anywhere;font-size:.72rem;max-height:12rem;overflow:auto}
  .candidate-detail{margin:1.3rem 0 0;padding-top:1.2rem;border-top:1px solid #60798855}.heading,.chooser,.paging{display:flex;align-items:center;gap:.8rem;justify-content:space-between}.chooser{justify-content:flex-start;margin:.8rem 0}.eyebrow{font-size:.7rem;letter-spacing:.1em;text-transform:uppercase;color:#72b7c5}h3{margin:.3rem 0}.scope{font-size:.85rem;opacity:.8;max-width:90ch}.facts{font-size:.85rem}.notice{border-left:3px solid #bfa45e;padding:.65rem;font-size:.85rem}.problem{color:#ef998e}button,input{background:transparent;border:1px solid #506879;color:inherit;padding:.5rem .7rem;border-radius:5px}button{cursor:pointer}button:disabled{opacity:.4;cursor:default}button:focus-visible,input:focus-visible{outline:2px solid #8cd8e5;outline-offset:3px}input{width:6rem;margin-left:.5rem}.scroll{overflow:auto}table{width:100%;border-collapse:collapse;font-size:.8rem;font-variant-numeric:tabular-nums;text-align:left}caption{text-align:left;font-weight:650;padding:.7rem 0}th,td{padding:.65rem;border-bottom:1px solid #65708033;white-space:nowrap;vertical-align:top}.conditions{min-width:20rem;max-width:35rem;white-space:normal;line-height:1.45}.paging{justify-content:flex-start;margin:.8rem 0;font-size:.8rem}dl{display:grid;grid-template-columns:minmax(16rem,1fr) 1fr;font-size:.8rem;gap:.35rem}dt{opacity:.8}dd{margin:0}summary{cursor:pointer;font-size:.85rem}@media(max-width:600px){.heading,.paging{align-items:flex-start;flex-wrap:wrap}.chooser{flex-wrap:wrap}dl{grid-template-columns:1fr 1fr}}
</style>

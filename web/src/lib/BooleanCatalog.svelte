<script>
  import { ask } from './ask.js';
  import { fetchBooleanCatalog, catalogDay } from './boolean-catalog.js';
  import { candidateMoney, candidateTime } from './candidate-trades.js';
  let { initialIdentity = '', initialCompletion = /** @type {string|null} */ (null), initialCandidate = /** @type {string|null} */ (null), autoLoad = false } = $props();
  let input = $state('');
  let loaded = $state(/** @type {any} */ ({phase:'idle',body:null,why:''}));
  let generation = 0;
  $effect(() => {
    input = initialIdentity; generation += 1; loaded = {phase:'idle',body:null,why:''};
    const completion=initialCompletion, candidate=initialCandidate;
    if(autoLoad&&initialIdentity)void load({identity:initialIdentity,completion,kind:candidate===null?'coordinates':'trades',candidate});
  });
  /** @param {any} selection */
  async function load(selection) {
    const ticket = ++generation;
    loaded = {phase:'loading',body:null,why:''};
    try {
      const body = await fetchBooleanCatalog(selection,(url)=>ask(url,{cache:'no-store'}));
      if(ticket===generation)loaded={phase:'ready',body,why:''};
    } catch(why) {
      if(ticket===generation)loaded={phase:'failed',body:null,why:why instanceof Error?why.message:String(why)};
    }
  }
  /** @param {any} body @param {any} changes */
  const select = (body,changes) => ({identity:body.identity,completion:body.completion,kind:body.kind,candidate:body.candidate,side:body.side,axis:body.axis,offset:'0',limit:body.limit,...changes});
  /** @param {string|null} ppm */
  const level = ppm => ppm === null ? 'Off' : ppm+' ppm';
</script>

<details class="catalog" open={autoLoad}>
  <summary>Explore a saved Boolean program catalog</summary>
  <p>Read the exact supplied programs, every saved exit setting, and each setting’s own trades and session totals. This is a finite catalog, not an exhaustive search of every Boolean expression.</p>
  <form onsubmit={(event)=>{event.preventDefault();void load({identity:input.trim()});}}>
    <label>Candidate catalog ID <input bind:value={input} placeholder="64-character candidate catalog identity" autocomplete="off" spellcheck="false" /></label>
    <button>Read saved catalog</button>
  </form>
  <p class="scope">Uses this server’s configured evidence folder. A command saved to another output folder is unavailable here. The saved statistics and research-check viewer authenticates their separate receipts and linked catalog completions.</p>
  {#if loaded.phase==='loading'}<p role="status">Checking the saved catalog and requested page…</p>
  {:else if loaded.phase==='failed'}<p role="alert" class="problem"><b>Detail unavailable.</b> {loaded.why} No partial or replacement page is displayed.</p>
  {:else if loaded.phase==='ready'}
    {@const body=loaded.body}
    <div class="facts"><strong>{body.instrument}</strong><span>{body.program_count} supplied programs</span><span>{body.coordinate_count} priced settings</span><span>{body.session_count} accepted sessions</span></div>
    <p class="scope">Authenticated saved observations. A priced setting can have zero trades or execution refusals. This receipt does not establish institutional approval, Selection V6, later out-of-sample returns or future profitability.</p>
    <details><summary>Exact saved identity and source settings</summary><p>Catalog <code>{body.identity}</code><br />Completion <code>{body.completion}</code><br />Common cohort <code>{body.cohort}</code></p>
      {#each body.grids as grid (grid.side)}
        <p><b>{grid.side === 'long'?'Long':'Short'}</b> · {grid.bars} real execution bars · one-minute execution · {grid.horizon_bars}-bar horizon · {grid.cells} exit settings · {candidateTime(grid.first_micros)} to {candidateTime(grid.last_micros)}</p>
        <p>Stop/target ratio {grid.ratio_min_hundredths}–{grid.ratio_max_hundredths} hundredths · range rounding: {grid.range_rounding} · selector: {grid.selector} · forced protective stop: {grid.forced_stop.kind}{grid.forced_stop.ppm===null?'':' at '+grid.forced_stop.ppm+' ppm'}. The separate intraday deadline is 15:10 IST.</p>
        <p>Ambiguous-bar ceiling {grid.max_ambiguous_bars}; gap-fill ceiling {grid.max_gap_fills}; maximum cells {grid.max_cells}; maximum ratio pairs {grid.max_pairs}; maximum levels per axis {grid.max_levels_per_axis}.</p>
        <p class="hashes">Execution <code>{grid.execution}</code><br />Feed <code>{grid.feed}</code><br />Build <code>{grid.commit}</code><br />Calendar <code>{grid.calendar}</code><br />Resolution <code>{grid.resolution}</code><br />Saved cost model <code>{grid.cost_model}</code></p>
        <div class="actions">{#each ['stop','target','trail','requested-stop','requested-target','requested-trail'] as axis}<button onclick={()=>load(select(body,{kind:'grid',candidate:null,side:grid.side,axis}))}>{axis.startsWith('requested-')?'Requested '+axis.slice(10)+' percentiles':'Resolved '+axis+' levels'}</button>{/each}</div>
      {/each}
    </details>
    <div class="actions"><button onclick={()=>load(select(body,{kind:'coordinates',candidate:null,side:null,axis:null}))}>All saved settings</button><button onclick={()=>load(select(body,{kind:'programs',candidate:null,side:null,axis:null}))}>Supplied programs</button></div>
    {#if body.selected}
      {@const row=body.selected}
      <div class="selected"><h4>Setting {row.index} · {row.side} · {row.expression}</h4>
        <p>Stop {level(row.levels.stop_ppm)} · target {level(row.levels.target_ppm)} · trailing stop {level(row.levels.tsl_ppm)} · profit trail arms at {level(row.levels.ttp_arm_ppm)}, trails by {level(row.levels.ttp_trail_ppm)}.</p>
        <p>{row.cell.trades} completed trades; {row.cell.wins} wins. Pessimistic {candidateMoney(row.cell.pessimistic)}, optimistic {candidateMoney(row.cell.optimistic)} per unit. True {row.truth.hits}, false {row.truth.misses}, unknown {row.truth.unknown}; definite signals in {row.support_sessions} sessions.</p>
        <p>Execution policy: {row.execution_refusals.length?row.execution_refusals.join('; '):'No execution refusal recorded; this is not institutional admission'}.</p>
        <div class="actions"><button onclick={()=>load(select(body,{kind:'trades'}))}>Exact trades</button><button onclick={()=>load(select(body,{kind:'sessions'}))}>Every accepted session</button></div>
        <details><summary>Exact candidate and parameter indices</summary><code>{row.identity}</code><p>Run <code>{row.run}</code></p><p>Program {row.program_index}; directional grid ordinal {row.ordinal}; stop {row.cell.stop??'off'}, target {row.cell.target??'off'}, trailing stop {row.cell.tsl??'off'}, profit-trail arm {row.cell.ttp?.arm??'off'}, trail {row.cell.ttp?.trail??'off'}. All indices are zero-based.</p></details>
      </div>
    {/if}
    <div class="scroll"><table>
      <caption>{body.kind==='coordinates'?'Every saved exit setting':body.kind==='programs'?'The complete supplied program list':body.kind==='trades'?'This exact setting’s trades':body.kind==='sessions'?'Accepted sessions, including zero-trade days':body.side+' '+body.axis+' inputs'}</caption>
      {#if body.kind==='coordinates'}
        <thead><tr><th>Setting</th><th>Program / direction</th><th>Signals true / false / unknown</th><th>Trades / wins</th><th>Pessimistic / optimistic per unit</th><th>Execution status</th><th>Details</th></tr></thead>
        <tbody>{#each body.rows as row (row.index)}<tr><td>{row.index}</td><td>{row.expression}<br />{row.side} · grid {row.ordinal}</td><td>{row.truth.hits} / {row.truth.misses} / {row.truth.unknown}</td><td>{row.cell.trades} / {row.cell.wins}</td><td>{candidateMoney(row.cell.pessimistic)} / {candidateMoney(row.cell.optimistic)}</td><td>{row.execution_refusals.length?row.execution_refusals.join('; '):'No execution refusal'}</td><td><button onclick={()=>load(select(body,{kind:'trades',candidate:row.index}))}>Trades and sessions</button></td></tr>{/each}</tbody>
      {:else if body.kind==='programs'}
        <thead><tr><th>Index</th><th>Exact Boolean program</th></tr></thead><tbody>{#each body.rows as row (row.index)}<tr><td>{row.index}</td><td>{row.expression}</td></tr>{/each}</tbody>
      {:else if body.kind==='trades'}
        <thead><tr><th>Trade</th><th>Stored source / entry / exit indices</th><th>Entry bar IST</th><th>Exit bar IST</th><th>Worst / best per unit</th><th>Adverse / favourable</th></tr></thead><tbody>{#each body.rows as row (row.index)}<tr><td>{row.index}</td><td>{row.signal_bar} / {row.entry_bar} / {row.exit_bar}</td><td title={row.entry_micros+' microseconds since epoch'}>{candidateTime(row.entry_micros)}</td><td title={row.exit_micros+' microseconds since epoch'}>{candidateTime(row.exit_micros)}</td><td>{candidateMoney(row.worst)} / {candidateMoney(row.best)}</td><td>{row.adverse_ppm} / {row.favourable_ppm} ppm<br />{candidateMoney(row.adverse_paisa)} / {candidateMoney(row.favourable_paisa)}</td></tr>{/each}</tbody>
      {:else if body.kind==='sessions'}
        <thead><tr><th>Session index</th><th>Session date IST</th><th>Trades</th><th>Wins</th><th>Pessimistic per unit</th></tr></thead><tbody>{#each body.rows as row (row.index)}<tr><td>{row.index}</td><td>{catalogDay(row.day)}<br />Day {row.day} since epoch</td><td>{row.trades}</td><td>{row.wins}</td><td>{candidateMoney(row.return_paisa)}</td></tr>{/each}</tbody>
      {:else}
        <thead><tr><th>Axis index</th><th>{body.axis.startsWith('requested-')?'Exact requested percentile fraction':'Resolved distance in ppm'}</th></tr></thead><tbody>{#each body.rows as row (row.index)}<tr><td>{row.index}</td><td>{body.axis.startsWith('requested-')?row.numerator+' / '+row.denominator:row.ppm}</td></tr>{/each}</tbody>
      {/if}
    </table></div>
    {#if body.kind==='trades'}<p class="scope">Stored source is the projected execution-fill index. The original completed signal bar is earlier; this field is not its source-series index. Times identify the saved bar opens. The exact 15:09 one-minute bar closes at the 15:10 intraday deadline; OHLCV does not reveal the order of prices within that bar.</p>{/if}
    {#if body.rows.length===0}<p>No rows at this saved extent. {body.kind==='trades'?'This setting has no completed trades; no winner trades are substituted.':''}</p>{/if}
    <div class="actions"><span>{body.rows.length} of {body.total} rows on this page</span><button disabled={body.offset==='0'} onclick={()=>load(select(body,{offset:String(BigInt(body.offset)>BigInt(body.limit)?BigInt(body.offset)-BigInt(body.limit):0n)}))}>Previous</button><button disabled={body.next===null} onclick={()=>load(select(body,{offset:body.next}))}>Next</button></div>
  {/if}
</details>

<style>
  .catalog{border:1px solid #566d7955;border-radius:10px;margin:1rem 0;padding:1rem}summary{font-weight:650;cursor:pointer}p{font-size:.85rem;line-height:1.55}.scope{opacity:.75;max-width:100ch}form,.facts,.actions{display:flex;flex-wrap:wrap;align-items:center;gap:.6rem;margin:1rem 0}label{display:flex;flex:1;align-items:center;gap:.6rem;min-width:250px}input{flex:1;min-width:150px;background:transparent;color:inherit;border:1px solid #647985;border-radius:5px;padding:.55rem}button{background:transparent;color:inherit;border:1px solid #647985;border-radius:5px;padding:.5rem .7rem;cursor:pointer}button:disabled{opacity:.4;cursor:default}button:focus-visible,input:focus-visible{outline:2px solid #79cad8;outline-offset:3px}.problem{color:#e99587}.selected{border-left:3px solid #6bb8c4;padding:.1rem 1rem;margin:1rem 0}.scroll{overflow:auto}table{border-collapse:collapse;width:100%;font-size:.8rem;text-align:left;font-variant-numeric:tabular-nums}caption{text-align:left;font-weight:650;margin:.6rem 0}th,td{border-bottom:1px solid #61717e44;padding:.65rem;vertical-align:top}td{min-width:8ch}code{overflow-wrap:anywhere;font-size:.75rem}.hashes{opacity:.8}h4{margin:.7rem 0}
</style>

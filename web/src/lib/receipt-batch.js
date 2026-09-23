import { sweepOutcome } from './sweep.js';

// This bounds browser bookkeeping, not the engine's search or support policy.
export const MAX_RECEIPT_JOBS = 4096;
export const STRICT_COMMAND = 'audit-audited-range';
export const STRICT_KNOBS = Object.freeze([
  'ceiling', 'screen_cap', 'screen_budget_ms', 'top', 'validate', 'horizon_bars',
  'grid_rungs', 'grid_resolution', 'min_rr_bp', 'min_win_rate_bp', 'min_trades',
  'min_ret_over_dd_bp', 'min_weakest_bp', 'max_mae_ppm'
]);

/** @param {unknown} value */
function positiveU64(value) {
  if (typeof value !== 'string' || !/^[1-9]\d{0,19}$/.test(value)) return null;
  return BigInt(value) <= 18446744073709551615n ? value : null;
}

/** @param {string} value @param {string} name */
function month(value, name) {
  const match = /^(\d{4})-(\d{2})$/.exec(value);
  if (!match || Number(match[1]) === 0 || Number(match[2]) < 1 || Number(match[2]) > 12) {
    throw new Error(`${name} must be a month in YYYY-MM form.`);
  }
  return { year: Number(match[1]), month: Number(match[2]) };
}

/**
 * Freeze exactly what the operator selected before the first request. The
 * server remains the authority for instrument eligibility and knob bounds.
 * @param {{feed:string,symbols:string[],rungs:string[],supportedRungs:string[],from:string,to:string,minHits:string,knobs:Record<string,string>}} input
 */
export function receiptPlan(input) {
  const from = month(input.from, 'Start');
  const to = month(input.to, 'End');
  if (input.from > input.to) throw new Error('The end month precedes the start month.');
  if (!input.feed) throw new Error('Choose a feed before starting receipt-checked research.');
  const minHits = positiveU64(input.minHits.trim());
  if (!minHits) throw new Error('Minimum hits must be an explicit positive whole number no greater than 18446744073709551615.');
  const supported = input.supportedRungs;
  if (!Array.isArray(supported) || supported.length === 0 ||
      supported.some(rung => !/^\d+min$/.test(rung)) || new Set(supported).size !== supported.length) {
    throw new Error('The server’s intraday timeframe list is unavailable or malformed. Re-read the ledger.');
  }
  const symbols = [...new Set(input.symbols)];
  const selected = new Set(input.rungs);
  if (symbols.length === 0 || symbols.some(symbol => typeof symbol !== 'string' || !symbol.trim())) {
    throw new Error('Choose at least one instrument.');
  }
  if (selected.size === 0 || [...selected].some(rung => !supported.includes(rung))) {
    throw new Error('Select only intraday timeframes published by the server. Daily bars are reference context only.');
  }
  const rungs = supported.filter(rung => selected.has(rung));
  if (symbols.length > Math.floor(MAX_RECEIPT_JOBS / rungs.length)) {
    throw new Error(`This browser queue supports at most ${MAX_RECEIPT_JOBS} instrument/timeframe jobs. Select a smaller batch; nothing was truncated or submitted.`);
  }
  /** @type {Record<string,string>} */
  const knobs = {};
  for (const [key, raw] of Object.entries(input.knobs)) {
    const value = raw.trim();
    if (!value) continue;
    if (!STRICT_KNOBS.includes(key)) throw new Error(`${key} is not a receipt-checked setting. Clear it; this mode requires an absolute minimum hit count.`);
    knobs[key] = value;
  }
  return Object.freeze(symbols.flatMap(underlying => rungs.map(rung => Object.freeze({
    ...knobs, command: STRICT_COMMAND, feed: input.feed, underlying, rung,
    from_year: from.year, from_month: from.month, to_year: to.year, to_month: to.month,
    min_hits: minHits
  }))));
}

/** @typedef {ReturnType<typeof receiptPlan>[number]} Request */
/** @typedef {'queued'|'starting'|'running'|'done'|'failed'|'unknown'|'not-started'} JobPhase */
/** @typedef {{request:Request,phase:JobPhase,attempt:string|null,report:string|null,why:string}} Job */
/** @typedef {{phase:'idle'|'running'|'done'|'failed'|'unknown'|'stopped',jobs:Job[],why:string}} BatchState */

/**
 * No "latest command" is evidence for a browser request. Match the accepted
 * exact token and the complete browser context before interpreting its status.
 * @param {unknown} value @param {Job} job
 */
export function receiptObservation(value, job) {
  const run = /** @type {import('./sweep.js').Running & {where?:string,attempt_key?:string}} */ (value);
  if (!run || run.where !== 'browser' || run.kind !== 'command' ||
      positiveU64(run.attempt_key) !== job.attempt || !job.attempt ||
      run.feed !== job.request.feed || run.underlying !== job.request.underlying ||
      run.from_year !== job.request.from_year || run.from_month !== job.request.from_month ||
      run.to_year !== job.request.to_year || run.to_month !== job.request.to_month) {
    throw new Error('The status does not identify this exact browser attempt. No other job will start and no completion is inferred.');
  }
  const outcome = sweepOutcome(run);
  if (outcome.phase !== 'running' && outcome.phase !== 'done' && outcome.phase !== 'failed') {
    throw new Error(outcome.why || 'This exact attempt has no confirmed terminal result.');
  }
  return { phase: outcome.phase, why: outcome.why, run: outcome.run };
}

/**
 * One POST at a time, then exact-attempt observation before the next POST.
 * A network failure never retries a POST: it may already have started work.
 * @param {{request:(url:string,options?:RequestInit)=>Promise<Response>,changed:(state:BatchState)=>void,wait?:()=>Promise<void>}} options
 */
export function createReceiptBatch({ request, changed, wait = () => new Promise(resolve => setTimeout(resolve, 2000)) }) {
  /** @type {BatchState} */
  let state = { phase: 'idle', jobs: [], why: '' };
  let disposed = false;
  let stopRequested = false;
  let active = false;
  const publish = () => { if (!disposed) changed({ ...state, jobs: state.jobs.map(job => ({ ...job })) }); };
  const stopQueued = () => {
    for (const job of state.jobs) if (job.phase === 'queued') job.phase = 'not-started';
  };
  /** @param {Job} job */
  async function observe(job) {
    const attempt = positiveU64(job.attempt);
    if (!attempt) throw new Error('An exact accepted attempt is required before checking status.');
    const response = await request(`/backtest/run.json?attempt=${encodeURIComponent(attempt)}`, { cache: 'no-store' });
    if (disposed) return;
    if (!response.ok) throw new Error(`Attempt status is unavailable (HTTP ${response.status}). No other job will start.`);
    const body = await response.json();
    if (disposed) return;
    const outcome = receiptObservation(body.running, job);
    job.phase = outcome.phase;
    job.why = outcome.why;
    job.report = outcome.run?.report ?? null;
  }
  /** @param {Job} job @param {unknown} error */
  function unknown(job, error) {
    stopRequested = true;
    stopQueued();
    job.phase = 'unknown';
    job.why = error instanceof Error ? error.message : String(error);
    state.phase = 'unknown';
    state.why = job.why;
    publish();
  }
  return {
    /** @param {ReturnType<typeof receiptPlan>} plan */
    async start(plan) {
      if (disposed || active || state.jobs.some(job => ['starting', 'running', 'unknown'].includes(job.phase))) {
        throw new Error('The current attempt must be resolved before starting another batch.');
      }
      if (!plan.length || plan.length > MAX_RECEIPT_JOBS) throw new Error('No bounded batch was prepared.');
      active = true;
      stopRequested = false;
      state = { phase: 'running', why: '', jobs: plan.map(job => ({ request: job, phase: 'queued', attempt: null, report: null, why: '' })) };
      publish();
      try {
        for (const job of state.jobs) {
          if (disposed || stopRequested) break;
          job.phase = 'starting';
          publish();
          try {
            const response = await request('/engine/command', {
              method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(job.request)
            });
            if (disposed) return;
            const body = await response.json();
            if (disposed) return;
            if (!response.ok || body.accepted !== true) {
              if (body.accepted === false && typeof body.refusal === 'string' && body.refusal.length > 0) {
                job.phase = 'failed'; job.why = body.refusal;
                state.phase = 'failed'; state.why = body.refusal;
                stopRequested = true; stopQueued(); publish(); break;
              }
              throw new Error(`The server did not confirm acceptance or refusal (HTTP ${response.status}). The request may have started; it will not be resent.`);
            }
            job.attempt = positiveU64(body.attempt);
            if (!job.attempt) throw new Error('The server accepted the request without an exact attempt string. The request will not be resent; update the server before submitting another batch.');
            job.phase = 'running'; publish();
            while (!disposed && job.phase === 'running') {
              await wait();
              if (disposed) return;
              await observe(job);
              publish();
            }
            if (/** @type {JobPhase} */ (job.phase) === 'failed') {
              state.phase = 'failed'; state.why = job.why;
              stopRequested = true; stopQueued(); publish(); break;
            }
          } catch (error) {
            if (!disposed) unknown(job, error);
            break;
          }
        }
        if (disposed) return;
        stopQueued();
        if (state.phase === 'running') {
          state.phase = state.jobs.every(job => job.phase === 'done') ? 'done' : 'stopped';
        }
        publish();
      } finally { active = false; }
    },
    stop() {
      stopRequested = true;
      stopQueued();
      state.why = 'No further job will start. An accepted server attempt continues until it finishes or refuses.';
      publish();
    },
    async recheck() {
      if (disposed || active) return;
      const job = state.jobs.find(item => item.phase === 'unknown' || item.phase === 'running');
      if (!job?.attempt) return;
      active = true;
      try {
        await observe(job);
        if (disposed) return;
        state.phase = job.phase === 'failed' ? 'failed' : job.phase === 'done' ? 'stopped' : 'unknown';
        state.why = job.phase === 'failed' ? job.why : 'Status refreshed for this attempt only. The remaining queue stays stopped.';
        publish();
      } catch (error) { if (!disposed) unknown(job, error); }
      finally { active = false; }
    },
    dispose() { disposed = true; stopRequested = true; }
  };
}

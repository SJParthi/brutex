import { ask } from './ask.js';
import { sweepOutcome } from './sweep.js';
import { validateIndexConsistencyPolicy } from './index-consistency.js';
import { validateBooleanWorkModel } from './boolean-work-model.js';
import { NATIVE_POLICY_FIELDS, NATIVE_POLICY_NAMES } from './native-policy-schema.js';
// @ts-expect-error Node 24 strips erasable TypeScript; SvelteKit resolves this source too.
import { liveAttemptKey } from './live-progress.ts';

export const BOOLEAN_SEARCH_COMMAND = 'boolean-qualified-search-stored';
const U64 = (1n << 64n) - 1n, U32 = (1n << 32n) - 1n;
const REQUEST_BYTES = 16_384, SYMBOL_BYTES = 64, BITS_BYTES = 4096;
const RUNGS = Object.freeze(['1min', '2min', '3min', '5min', '10min', '15min', '30min', '60min']);
const KNOBS = Object.freeze(['horizon_bars', 'max_points', 'batch_programs', 'node_allowance', 'batch_allowance']);
const LIMITS = Object.freeze(['max_symbols', 'request_bytes', 'horizon_bars_max', 'max_points_max',
  'checksum_max_bytes', 'checksum_max_records', 'observation_bytes', 'replay_nodes']);
const INPUTS = Object.freeze(['feed', 'symbols', 'timeframes', 'from', 'to', 'laterFrom', 'laterTo', 'bits',
  'horizonBars', 'maxPoints', 'batchPrograms', 'nodeAllowance', 'batchAllowance']);
// The frontend gate derives required names/types from Rust; the live server
// supplies their ordering and all effective threshold values.
const metadataReceipts = new WeakSet(), plans = new WeakSet(), observationPlans = new WeakSet();
const utf8 = new TextEncoder();
/** @param {unknown} value @returns {value is Record<string, any>} */
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
/** @param {unknown} value @param {boolean} [positive] */
function integer(value, positive = true) {
  return typeof value === 'string' && /^(0|[1-9]\d{0,19})$/.test(value) &&
    (!positive || value !== '0') && BigInt(value) <= U64;
}
/** @param {unknown} value */
const identity = value => typeof value === 'string' && /^[0-9a-f]{64}$/.test(value) && value !== '0'.repeat(64);
/** @param {unknown} value @param {number} [max] */
const text = (value, max = 4096) => typeof value === 'string' && value.trim().length > 0 && utf8.encode(value).length <= max;
/** @param {unknown} value @param {readonly string[]} names @returns {value is Record<string, any>} */
function keys(value, names) {
  return object(value) && Object.keys(value).length === names.length &&
    names.every(name => Object.hasOwn(value, name));
}
/** @param {any} value */
function freeze(value) {
  if (value && typeof value === 'object') {
    for (const child of Object.values(value)) freeze(child);
    Object.freeze(value);
  }
  return value;
}
/** @param {unknown} value */
function bytes(value) { return utf8.encode(JSON.stringify(value)).length; }
/** @param {string} name @param {unknown} value */
function knob(name, value) {
  return integer(value) && (name !== 'horizon_bars' || BigInt(/** @type {string} */ (value)) <= U32) &&
    (name !== 'max_points' || BigInt(/** @type {string} */ (value)) <= U64 / 100n);
}
/** @param {string} name @param {unknown} value */
function policyValue(name, value) {
  const type = NATIVE_POLICY_FIELDS[name];
  if (type === 'bool') return typeof value === 'boolean';
  if (type === 'u64') return integer(value, false);
  if (type !== 'i64') return false;
  return typeof value === 'string' && /^(0|-?[1-9]\d{0,18})$/.test(value) &&
    BigInt(value) >= -(1n << 63n) && BigInt(value) < (1n << 63n);
}

/** Validate recorded native acceptance thresholds without choosing replacements.
 * Shared by the grid and signal-candle execution models.
 * @param {any} policy */
export function validateNativeResearchPolicy(policy) {
  if (!keys(policy, ['ready', 'digest', 'values', 'refusal']) || typeof policy.ready !== 'boolean' ||
      !Array.isArray(policy.values) || !(policy.refusal === null || text(policy.refusal))) {
    throw new Error('The native research policy contract is malformed.');
  }
  if (policy.ready) {
    if (!identity(policy.digest) || policy.refusal !== null || policy.values.length !== NATIVE_POLICY_NAMES.length) {
      throw new Error('A resolved policy requires its digest and all 39 native policy values.');
    }
    const names = new Set();
    for (const value of policy.values) {
      if (!keys(value, ['name', 'value']) || !text(value.name, 64) || !/^[a-z][a-z0-9_]*$/.test(value.name) ||
          !Object.hasOwn(NATIVE_POLICY_FIELDS, value.name) || !policyValue(value.name, value.value) ||
          names.has(value.name)) throw new Error('The native policy values are missing, unknown, malformed or duplicated.');
      names.add(value.name);
    }
    if (!NATIVE_POLICY_NAMES.every(name => names.has(name))) throw new Error('Required native policy fields are missing.');
  } else if (policy.digest !== null || policy.values.length !== 0 || !text(policy.refusal)) {
    throw new Error('An unresolved policy must retain its refusal and absent digest/values.');
  }
  return policy;
}

/** Validate the server's transport/configuration contract without choosing a
 * policy, data split, universe size or missing default in the browser.
 * @param {unknown} body */
export function validateBooleanLaunchMetadata(body) {
  const fields = ['schema_version', 'model', 'command', 'timeframes', 'ready', 'refusal', 'limits', 'configured', 'policy'];
  for (const name of ['selected_timeframes_supported', 'index_consistency_policy', 'work_model']) {
    if (object(body) && Object.hasOwn(body, name)) fields.push(name);
  }
  if (!keys(body, fields) ||
      body.schema_version !== 1 || body.model !== 'boolean-qualified-search-launch' || body.command !== BOOLEAN_SEARCH_COMMAND ||
      !Array.isArray(body.timeframes) || body.timeframes.length !== RUNGS.length ||
      !RUNGS.every((rung, index) => body.timeframes[index] === rung) || typeof body.ready !== 'boolean' ||
      (Object.hasOwn(body, 'selected_timeframes_supported') && typeof body.selected_timeframes_supported !== 'boolean') ||
      !(body.refusal === null || text(body.refusal)) || !keys(body.limits, LIMITS) || !keys(body.configured, KNOBS) ||
      !keys(body.policy, ['ready', 'digest', 'values', 'refusal'])) {
    throw new Error('The Boolean launch metadata schema is unavailable or malformed.');
  }
  const limits = body.limits, policy = body.policy;
  if (Object.hasOwn(body, 'index_consistency_policy')) validateIndexConsistencyPolicy(body.index_consistency_policy);
  if (Object.hasOwn(body, 'work_model')) validateBooleanWorkModel(body.work_model);
  if (!Number.isSafeInteger(limits.max_symbols) || limits.max_symbols < 1 || limits.request_bytes !== REQUEST_BYTES ||
      limits.horizon_bars_max !== String(U32) || limits.max_points_max !== String(U64 / 100n) ||
      !LIMITS.slice(4).every(name => limits[name] === null || integer(limits[name])) ||
      !KNOBS.every(name => body.configured[name] === null || knob(name, body.configured[name])) ||
      typeof policy.ready !== 'boolean' || !Array.isArray(policy.values) ||
      !(policy.refusal === null || text(policy.refusal))) {
    throw new Error('The Boolean launch limits, configured values or policy contract is malformed.');
  }
  validateNativeResearchPolicy(policy);
  if ((body.ready && (body.refusal !== null || !policy.ready || !LIMITS.slice(4).every(name => integer(limits[name])))) ||
      (!body.ready && !text(body.refusal)) || bytes(body) > REQUEST_BYTES) {
    throw new Error('The Boolean launch readiness claim is contradictory or oversized.');
  }
  const checked = freeze(structuredClone(body));
  metadataReceipts.add(checked);
  return checked;
}

/** @param {unknown} value @param {string} label */
function month(value, label) {
  if (typeof value !== 'string' || !/^[0-9]{4}-(0[1-9]|1[0-2])$/.test(value) || value.startsWith('0000')) {
    throw new Error(`${label} must be an explicit month in YYYY-MM form.`);
  }
  return { year: Number(value.slice(0, 4)), month: Number(value.slice(5)), key: value };
}
/** @param {unknown} value */
function bits(value) {
  if (value === undefined || value === 'all') return 'all';
  if (typeof value !== 'string' || value.length > BITS_BYTES) throw new Error('Choose all conditions or an increasing list of canonical condition IDs within the 4,096-byte protocol limit.');
  let previous = -1n;
  for (const item of value.split(',')) {
    if (!integer(item, false) || BigInt(item) > U32 || BigInt(item) <= previous) {
      throw new Error('Condition IDs must be canonical increasing u32 values; live eligibility is checked by the server.');
    }
    previous = BigInt(item);
  }
  return value;
}

/** The wire order is canonical; an omitted legacy field denotes all eight.
 * An explicit empty, duplicate, unknown or reordered array never widens scope.
 * @param {unknown} value @returns {string[]} */
function selectedTimeframes(value) {
  if (value === undefined) return [...RUNGS];
  if (!Array.isArray(value) || value.length === 0 || value.length > RUNGS.length)
    throw new Error('Choose at least one supported intraday timeframe. An empty selection cannot run.');
  let previous = -1;
  for (const rung of value) {
    const index = typeof rung === 'string' ? RUNGS.indexOf(rung) : -1;
    if (index <= previous) throw new Error('Selected timeframes must be unique supported intraday values in the server’s order.');
    previous = index;
  }
  return [...value];
}

/** @param {unknown} value @param {readonly string[]} expected */
function sameTimeframes(value, expected) {
  return Array.isArray(value) && value.length === expected.length && expected.every((rung, index) => value[index] === rung);
}

/** Compare validated declarations across the additive all-eight compatibility
 * field. Only its historical omission and explicit all-eight value are equal;
 * selected subsets remain part of the exact request binding.
 * @param {any} left @param {any} right */
function sameDeclaration(left, right) {
  const fields = Object.keys(left).filter(name => name !== 'timeframes');
  return fields.length === Object.keys(right).filter(name => name !== 'timeframes').length &&
    fields.every(name => JSON.stringify(left[name]) === JSON.stringify(right[name])) &&
    sameTimeframes(selectedTimeframes(left.timeframes), selectedTimeframes(right.timeframes));
}

/** Display the scope retained by an exact saved declaration. Older records
 * without this additive field explicitly retain the historical all-eight scope.
 * @param {any} plan @returns {string[]} */
export function booleanPlanTimeframes(plan) { return selectedTimeframes(plan?.timeframes); }

/** Validate the common transport declaration. Observing an old request does
 * not require today's configured universe or admission policy. The byte and
 * scalar ceilings remain the same fixed wire contract in either direction.
 * @param {unknown} input @param {number|null} maxSymbols @param {unknown} policyDigest
 * @param {boolean} [supportsSelection] */
function declaration(input, maxSymbols, policyDigest, supportsSelection = false) {
  if (!object(input) || Object.keys(input).some(name => !INPUTS.includes(name))) {
    throw new Error('Only the Boolean campaign declaration is accepted; paths and ordinary sweep settings are not launch fields.');
  }
  if (!identity(policyDigest)) throw new Error('The exact native research policy digest is missing or malformed.');
  const timeframes = selectedTimeframes(input.timeframes);
  if (!supportsSelection && !sameTimeframes(timeframes, RUNGS))
    throw new Error('This server cannot bind a selected timeframe subset. No sweep was submitted; update the server or explicitly select all eight timeframes.');
  const from = month(input.from, 'Training start'), to = month(input.to, 'Training end');
  const laterFrom = month(input.laterFrom, 'Later start'), laterTo = month(input.laterTo, 'Later end');
  if (from.key > to.key || laterFrom.key > laterTo.key || to.key >= laterFrom.key) {
    throw new Error('Use ordered, nonoverlapping training and later periods; the later period must start after training ends.');
  }
  if (typeof input.feed !== 'string' || !/^[a-z][a-z0-9_-]{0,63}$/.test(input.feed)) throw new Error('Choose a canonical server feed.');
  if (!Array.isArray(input.symbols) || input.symbols.length === 0 || (maxSymbols !== null && input.symbols.length > maxSymbols)) {
    throw new Error(maxSymbols === null ? 'A recorded Boolean declaration must name its instruments.' :
      `Select 1–${maxSymbols} instruments from the server's current research universe.`);
  }
  const selected = new Set();
  for (const symbol of input.symbols) {
    if (typeof symbol !== 'string' || !/^[A-Z0-9][A-Z0-9&._-]*$/.test(symbol) ||
        utf8.encode(symbol).length > SYMBOL_BYTES || selected.has(symbol)) {
      throw new Error('Instrument symbols must be unique canonical names of at most 64 UTF-8 bytes. Nothing was deduplicated or truncated.');
    }
    selected.add(symbol);
  }
  const asked = {
    command: BOOLEAN_SEARCH_COMMAND, feed: input.feed, symbols: [...selected], expected_policy_digest: policyDigest,
    ...(supportsSelection ? { timeframes } : {}),
    from_year: from.year, from_month: from.month, to_year: to.year, to_month: to.month,
    later_from_year: laterFrom.year, later_from_month: laterFrom.month,
    later_to_year: laterTo.year, later_to_month: laterTo.month, bits: bits(input.bits),
    horizon_bars: input.horizonBars, max_points: input.maxPoints, batch_programs: input.batchPrograms,
    node_allowance: input.nodeAllowance, batch_allowance: input.batchAllowance
  };
  if (!KNOBS.every(name => knob(name, /** @type {Record<string,unknown>} */ (asked)[name]))) {
    throw new Error('Every budget must be an explicit positive canonical integer string within the protocol limits.');
  }
  if (bytes(asked) > REQUEST_BYTES) throw new Error('The complete Boolean declaration exceeds the 16,384-byte request limit. Nothing was submitted.');
  return freeze(asked);
}

/** Freeze the exact declaration before admission. Membership, live conditions,
 * available OHLCV and further runtime resource limits remain server checks.
 * @param {unknown} input @param {unknown} metadata */
export function booleanLaunchPlan(input, metadata) {
  if (!object(metadata) || !metadataReceipts.has(metadata) || !metadata.ready) {
    throw new Error('Read valid, ready Boolean launch metadata before preparing a launch.');
  }
  const plan = declaration(input, metadata.limits.max_symbols, metadata.policy.digest, metadata.selected_timeframes_supported === true);
  if (plan.symbols.some((/** @type {string} */ symbol) => ['NIFTY', 'BANKNIFTY', 'NSE-NIFTY', 'NSE-BANKNIFTY'].includes(symbol)) && !metadata.index_consistency_policy) {
    throw new Error('This server has not reported the required index day/week rule. Update the server before starting an index sweep.');
  }
  if (plan.max_points !== metadata.configured.max_points) {
    throw new Error('Refresh the server policy for this exact max-points value before starting.');
  }
  plans.add(plan);
  return plan;
}

/** A native status can authorize observation, never submission. Do not use
 * current form values or current-policy defaults to reconstruct old work.
 * @param {unknown} value */
function recordedPlan(value) {
  if (!object(value) || value.command !== BOOLEAN_SEARCH_COMMAND) throw new Error('The saved status does not identify a Boolean search request.');
  /** @param {string} prefix */
  const date = prefix => {
    const year = value[`${prefix}_year`], month = value[`${prefix}_month`];
    if (!Number.isInteger(year) || year < 1 || year > 9999 || !Number.isInteger(month) || month < 1 || month > 12) {
      throw new Error('The recorded request does not contain exact numeric training/later months.');
    }
    return `${String(year).padStart(4, '0')}-${String(month).padStart(2, '0')}`;
  };
  const plan = declaration({ feed: value.feed, symbols: value.symbols, timeframes: value.timeframes,
    from: date('from'), to: date('to'), laterFrom: date('later_from'), laterTo: date('later_to'),
    bits: value.bits, horizonBars: value.horizon_bars, maxPoints: value.max_points,
    batchPrograms: value.batch_programs, nodeAllowance: value.node_allowance, batchAllowance: value.batch_allowance
  }, null, value.expected_policy_digest, Object.hasOwn(value, 'timeframes'));
  observationPlans.add(plan);
  return plan;
}

/** @typedef {'idle'|'starting'|'running'|'done'|'refused'|'unknown'} LaunchPhase */
/** @typedef {{phase:LaunchPhase,attempt:string|null,searchIdentity:string|null,report:string|null,why:string,exhausted:boolean|null,completedBatches:string|null,plan:any}} LaunchState */

/** Join the complete declaration and exact attempt before reading a lifecycle.
 * Identity/exhaustion come only from the structured saved-evidence fields.
 * @param {unknown} value @param {any} plan @param {string} attempt @param {LaunchState|null} [previous] */
export function booleanLaunchObservation(value, plan, attempt, previous = null) {
  if (!object(plan) || (!plans.has(plan) && !observationPlans.has(plan)) || !object(value) || !integer(attempt) || value.where !== 'browser' || value.kind !== 'command' ||
      liveAttemptKey(value) !== attempt || !Array.isArray(value.symbols) || value.symbols.length !== plan.symbols.length ||
      !plan.symbols.every((/** @type {string} */ symbol, /** @type {number} */ index) => value.symbols[index] === symbol) ||
      !(Object.hasOwn(plan, 'timeframes') ? sameTimeframes(value.timeframes, plan.timeframes)
        : value.timeframes === undefined || sameTimeframes(value.timeframes, RUNGS)) ||
      !Object.keys(plan).filter(name => name !== 'symbols' && name !== 'timeframes').every(name => value[name] === plan[name])) {
    throw new Error('The status does not match this exact Boolean attempt and complete declaration. No completion is inferred.');
  }
  if (!(value.search_identity === null || identity(value.search_identity)) ||
      !(value.exhausted === null || typeof value.exhausted === 'boolean') ||
      !(value.completed_batches === null || integer(value.completed_batches, false)) ||
      (value.search_identity === null && (value.exhausted !== null || value.completed_batches !== null)) ||
      ((value.exhausted === null) !== (value.completed_batches === null)) ||
      (previous?.searchIdentity && value.search_identity !== previous.searchIdentity) ||
      (previous?.exhausted === true && value.exhausted !== true) ||
      (previous?.completedBatches !== null && previous?.completedBatches !== undefined &&
        (value.completed_batches === null || BigInt(value.completed_batches) < BigInt(previous.completedBatches)))) {
    throw new Error('Saved search identity or progress is malformed, contradictory, or moved backwards.');
  }
  if (!(value.report === null || text(value.report, Number.MAX_SAFE_INTEGER)) ||
      !(value.refusal === null || text(value.refusal)) || (value.report !== null && value.refusal !== null) ||
      (value.status !== undefined && !['running', 'completed', 'refused', 'failed', 'cancelled'].includes(value.status)) ||
      (value.finished_micros !== undefined && value.finished_micros !== null &&
        (typeof value.finished_micros !== 'number' || !Number.isInteger(value.finished_micros))) ||
      (value.in_flight === false && value.finished_micros === null) ||
      (value.in_flight === true && (value.report !== null || value.refusal !== null ||
        (value.finished_micros !== undefined && value.finished_micros !== null)))) {
    throw new Error(value.why || 'This exact attempt has no consistent running or terminal evidence.');
  }
  const outcome = sweepOutcome(/** @type {import('./sweep.js').Running} */ (value));
  if (!['running', 'done', 'failed'].includes(outcome.phase) ||
      (outcome.phase === 'done' && !identity(value.search_identity)) ||
      (value.status === 'running' && outcome.phase !== 'running') ||
      (value.status === 'completed' && outcome.phase !== 'done') ||
      (previous?.phase === 'done' && outcome.phase !== 'done') ||
      (previous?.phase === 'refused' && outcome.phase !== 'failed') ||
      (['refused', 'failed', 'cancelled'].includes(value.status) && outcome.phase !== 'failed')) {
    throw new Error(outcome.why || 'The exact attempt has not supplied a consistent terminal result.');
  }
  return { phase: /** @type {LaunchPhase} */ (outcome.phase === 'failed' ? 'refused' : outcome.phase),
    attempt, searchIdentity: value.search_identity, report: value.report, why: outcome.why,
    exhausted: value.exhausted, completedBatches: value.completed_batches, plan };
}

/** @param {AbortSignal} signal */
function pause(signal) {
  return new Promise((resolve, reject) => {
    const aborted = () => { clearTimeout(timer); reject(signal.reason); };
    const timer = setTimeout(() => { signal.removeEventListener('abort', aborted); resolve(undefined); }, 2000);
    if (signal.aborted) aborted(); else signal.addEventListener('abort', aborted, { once: true });
  });
}

/** One POST followed only by exact-attempt GETs. Cancelling observation cannot
 * cancel the server's computation or make an ambiguous POST safe to repeat.
 * @param {{request?:typeof ask,changed:(state:LaunchState)=>void,onSearch?:(identity:string)=>void,wait?:(signal:AbortSignal)=>Promise<unknown>}} options */
export function createBooleanLaunch({ request = ask, changed, onSearch, wait = pause }) {
  /** @type {LaunchState} */
  let state = { phase: 'idle', attempt: null, searchIdentity: null, report: null, why: '', exhausted: null, completedBatches: null, plan: null };
  let epoch = 0, disposed = false, busy = /** @type {number|null} */ (null), notified = /** @type {string|null} */ (null);
  let abort = /** @type {AbortController|null} */ (null);
  const publish = () => { if (!disposed) changed(Object.freeze({ ...state })); };
  const cancel = () => { epoch += 1; abort?.abort(); abort = null; busy = null; };
  /** @param {number} ticket */
  const current = ticket => !disposed && ticket === epoch && !abort?.signal.aborted;
  /** @param {any} plan */
  const prepared = plan => { if (!object(plan) || !plans.has(plan)) throw new Error('Prepare a validated Boolean declaration before starting or resuming.'); };
  /** @param {unknown} why */
  const unknown = why => { state = { ...state, phase: 'unknown', why: why instanceof Error ? why.message : String(why) }; publish(); };
  function ticket() { cancel(); abort = new AbortController(); busy = epoch; return { id: epoch, signal: abort.signal }; }
  /** @param {number} id @param {AbortSignal} signal */
  async function observe(id, signal) {
    if (!current(id)) return;
    const response = await request(`/backtest/run.json?attempt=${encodeURIComponent(/** @type {string} */ (state.attempt))}`, { cache: 'no-store', signal });
    if (!current(id)) return;
    if (!response.ok) throw new Error(`Exact attempt status is unavailable (HTTP ${response.status}). The launch will not be resent.`);
    const body = await response.json();
    if (!current(id)) return;
    const observed = booleanLaunchObservation(body?.running, state.plan ?? recordedPlan(body?.running), /** @type {string} */ (state.attempt), state);
    state = observed; publish();
    if (current(id) && state.searchIdentity && state.searchIdentity !== notified) { notified = state.searchIdentity; onSearch?.(state.searchIdentity); }
  }
  /** @param {number} id @param {AbortSignal} signal */
  async function poll(id, signal) {
    try {
      do {
        await observe(id, signal);
        if (!current(id) || state.phase !== 'running') return;
        await wait(signal);
      } while (current(id));
    } catch (why) { if (current(id)) unknown(why); }
    finally { if (busy === id) busy = null; }
  }
  return {
    /** @param {any} plan */
    async start(plan) {
      prepared(plan);
      if (disposed || busy !== null || ['starting', 'running', 'unknown'].includes(state.phase)) throw new Error('Resolve the current attempt before submitting another launch.');
      const work = ticket(); notified = null;
      state = { phase: 'starting', attempt: null, searchIdentity: null, report: null, why: '', exhausted: null, completedBatches: null, plan }; publish();
      try {
        if (!current(work.id)) return;
        const response = await request('/engine/command', { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(plan), signal: work.signal });
        if (!current(work.id)) return;
        const body = await response.json();
        if (!current(work.id)) return;
        if (body?.accepted === false && text(body.refusal) && response.status !== 202 &&
            (body.attempt === undefined || body.attempt === null) &&
            (body.attempt_key === undefined || body.attempt_key === null) && body.started !== true) {
          state = { ...state, phase: 'refused', why: body.refusal }; publish(); return;
        }
        if (body?.accepted === true && integer(body.attempt)) state = { ...state, attempt: body.attempt };
        if (!response.ok || response.status !== 202 || body?.accepted !== true || body.refusal !== null || !state.attempt ||
            (body.attempt_key !== undefined && body.attempt_key !== state.attempt) || body.started === false) {
          throw new Error('The server did not unambiguously accept or refuse this launch with an exact attempt string. It may have started; it will not be resent.');
        }
        state = { ...state, phase: 'running' }; publish();
        await poll(work.id, work.signal);
      } catch (why) { if (current(work.id)) unknown(why); }
      finally { if (busy === work.id) busy = null; }
    },
    async recheck() {
      if (disposed || busy !== null || !integer(state.attempt)) return;
      const work = ticket();
      await poll(work.id, work.signal);
    },
    /** @param {any} plan @param {unknown} attempt */
    async resume(plan, attempt) {
      prepared(plan);
      if (disposed || busy !== null || !integer(attempt) ||
          (state.attempt !== null && state.attempt !== attempt) ||
          (state.plan !== null && !sameDeclaration(state.plan, plan))) {
        throw new Error('Resume requires the same complete declaration and exact attempt, with no other request active.');
      }
      const work = ticket();
      state = { ...state, phase: 'unknown', attempt: /** @type {string} */ (attempt), plan, why: 'Checking this exact saved attempt; no launch is submitted.' }; publish();
      await poll(work.id, work.signal);
    },
    /** Restore only the request actually reported by the native status route.
     * No POST-authorized plan or current admission configuration is created.
     * @param {unknown} running */
    async restore(running) {
      if (disposed || busy !== null) return;
      const work = ticket();
      try {
        const attempt = liveAttemptKey(running);
        if (state.attempt !== null && attempt !== state.attempt) throw new Error('The restored status belongs to a different exact attempt.');
        if (state.attempt === null && attempt !== null) state = { ...state, attempt };
        if (!integer(attempt)) throw new Error('The restored Boolean status has no valid exact attempt identity.');
        const plan = recordedPlan(running);
        if (state.plan !== null && !sameDeclaration(state.plan, plan)) throw new Error('The restored attempt changed its complete recorded declaration.');
        state = booleanLaunchObservation(running, plan, /** @type {string} */ (attempt), state);
        publish();
        if (current(work.id) && state.searchIdentity && state.searchIdentity !== notified) {
          notified = state.searchIdentity; onSearch?.(state.searchIdentity);
        }
        if (state.phase === 'running') await poll(work.id, work.signal);
      } catch (why) { if (current(work.id)) unknown(why); }
      finally { if (busy === work.id) busy = null; }
    },
    stop() {
      if (disposed) return;
      cancel();
      if (['starting', 'running', 'unknown'].includes(state.phase)) unknown('Observation stopped. The server attempt may continue; checking again will not resend the launch.');
    },
    dispose() { cancel(); disposed = true; }
  };
}

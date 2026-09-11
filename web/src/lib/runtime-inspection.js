import { savedSelection } from '../../saved-backtest/selection.js';

/** The frontend reports the server's declared mode; it grants no release clearance. */
export const INSPECTION_MESSAGE = 'Inspection mode: real saved data. Sweep and data-pull controls are disabled while release checks remain unresolved.';

/** @typedef {{phase:'idle'|'reading'|'ready'|'legacy'|'failed',mode:'application'|'read-only-main-app'|'legacy'|null,canSweep:boolean,canPull:boolean,savedResultsUrl:string|null,why:string}} InspectionState */
/** @type {InspectionState} */
export const INITIAL_INSPECTION = Object.freeze({
  phase: 'idle', mode: null, canSweep: false, canPull: false, savedResultsUrl: null,
  why: 'Checking this server’s execution mode. Sweep controls stay disabled until it answers.'
});

/** A configured saved-results link must stay on a literal loopback address and
 * select one authenticated checkpoint through the same contract as its viewer.
 * @param {unknown} value @returns {string|null} */
export function savedResultsUrl(value) {
  if (value === undefined || value === null) return null;
  if (typeof value !== 'string' || value.length > 1024) throw new Error('The saved-results link is invalid.');
  const url = new URL(value);
  if (url.href !== value || url.protocol !== 'http:' || !['127.0.0.1', '[::1]'].includes(url.hostname) ||
      !url.port || url.username || url.password || url.pathname !== '/backtest' || url.hash ||
      url.searchParams.toString() !== url.search.slice(1)) {
    throw new Error('The saved-results link must name a canonical local viewer.');
  }
  const selection = savedSelection(url.search);
  if (selection.pin === null || selection.batch === null || !url.searchParams.has('rung') || !url.searchParams.has('offset')) {
    throw new Error('The saved-results link must pin its checkpoint, batch, timeframe and page.');
  }
  return url.href;
}

/** Validate explicit restrictions before describing this server as read-only.
 * @param {unknown} value @returns {InspectionState} */
export function inspectionState(value) {
  const body = /** @type {Record<string,unknown>|null} */ (value);
  if (body && typeof body === 'object' && !Array.isArray(body) && body.schema === 2 && body.mode === 'application') {
    const stamped = typeof body.commit === 'string' && /^[a-f0-9]{40}$/.test(body.commit);
    if ((!stamped && body.commit !== null) || body.can_sweep !== stamped ||
        body.can_pull !== true || body.can_write !== true || body.release_cleared !== false) {
      throw new Error('The application capability declaration is inconsistent. Sweep controls remain disabled.');
    }
    return { phase: 'ready', mode: 'application', canSweep: stamped, canPull: true,
      savedResultsUrl: null, why: stamped
        ? 'Connected to the Brutex application. Each run is checked before it starts.'
        : 'The application is connected, but this engine build has no verified source identity. Sweeps are unavailable.' };
  }
  if (!body || typeof body !== 'object' || Array.isArray(body) || body.schema !== 1 ||
      body.mode !== 'read-only-main-app' || body.can_sweep !== false || body.can_pull !== false ||
      body.can_write !== false || body.release_cleared !== false ||
      body.automatic_recovery !== false || body.automatic_acquisition !== false) {
    throw new Error('The server did not provide a recognised inspection-mode declaration. Sweep controls remain disabled.');
  }
  return { phase: 'ready', mode: 'read-only-main-app', canSweep: false, canPull: false,
    savedResultsUrl: savedResultsUrl(body.saved_results_url), why: INSPECTION_MESSAGE };
}

/** One shared, bounded request per page lifetime; failed reads may be retried.
 * A 404 is an explicitly labelled legacy-server compatibility path, never an
 * inspection declaration or a release-readiness result.
 * @param {(url:string,options?:RequestInit)=>Promise<Response>} request
 * @param {(state:InspectionState)=>void} publish */
export function createInspectionReader(request, publish) {
  /** @type {Promise<InspectionState>|null} */ let pending = null;
  /** @type {InspectionState|null} */ let settled = null;
  function load() {
    if (pending) return pending;
    if (settled && settled.phase !== 'failed') return Promise.resolve(settled);
    publish({ ...INITIAL_INSPECTION, phase: 'reading' });
    pending = (async () => {
      try {
        const response = await request('/inspection.json', { cache: 'no-store' });
        if (response.status === 404) {
          settled = { phase: 'legacy', mode: 'legacy', canSweep: true, canPull: true, savedResultsUrl: null,
            why: 'Legacy server: inspection capabilities are not reported. Existing controls are available; this is not sweep-readiness clearance.' };
        } else {
          if (!response.ok) throw new Error(`Inspection-mode check answered HTTP ${response.status}. Sweep controls remain disabled.`);
          if ((response.headers.get('content-type') ?? '').split(';')[0].trim().toLowerCase() !== 'application/json') {
            throw new Error('Inspection-mode check did not answer JSON. Sweep controls remain disabled.');
          }
          settled = inspectionState(await response.json());
        }
      } catch (error) {
        settled = { phase: 'failed', mode: null, canSweep: false, canPull: false, savedResultsUrl: null,
          why: `Execution mode could not be verified: ${error instanceof Error ? error.message : String(error)}` };
      }
      publish(settled);
      return settled;
    })().finally(() => { pending = null; });
    return pending;
  }
  return { load };
}

/** Saved evidence is a separate, attempt-bound contract. Never turn absence into zero. */
const U64_MAX = (1n << 64n) - 1n;
const I64_MIN = -(1n << 63n);
const I64_MAX = (1n << 63n) - 1n;
const ROW_CAP = 4096;
const PAGE_ROWS = 256;
const unsigned = (/** @type {any} */ value) =>
  typeof value === 'string' && /^(0|[1-9][0-9]*)$/.test(value) && value.length <= 20 && BigInt(value) <= U64_MAX;
const signed = (/** @type {any} */ value) =>
  typeof value === 'string' && /^(0|-?[1-9][0-9]*)$/.test(value) && value.length <= 20 &&
  BigInt(value) >= I64_MIN && BigInt(value) <= I64_MAX;
const object = (/** @type {any} */ value) => !!value && typeof value === 'object' && !Array.isArray(value);
const fields = ['k', 'generated', 'duplicates', 'excluded', 'pruned', 'infrequent', 'frequent', 'admitted', 'pairs'];

/** @param {any} evidence */
function validateSummary(evidence) {
  if (!object(evidence) || !unsigned(evidence.attempt) || evidence.attempt === '0' ||
    !['sweep', 'audit', 'auto-search', 'auto-probe', 'expression', 'expression-search', 'preparation', 'global-replay', 'global-replay-stream', 'checksum-audit', 'boolean-candidates', 'boolean-statistics', 'boolean-admission', 'boolean-oos', 'boolean-qualification', 'index-consistency', 'index-stop', 'index-stop-qualification'].includes(evidence.operation) ||
    !['running', 'completed', 'halted', 'refused'].includes(evidence.completion) ||
    !signed(evidence.started_micros) || !signed(evidence.updated_micros) ||
    !unsigned(evidence.depth_rows) || !unsigned(evidence.ranked_rows) ||
    BigInt(evidence.depth_rows) > BigInt(ROW_CAP) ||
    typeof evidence.ranked_available !== 'boolean' ||
    (evidence.validation_requested !== null && typeof evidence.validation_requested !== 'boolean')) {
    throw new Error('Saved attempt metadata is missing, invalid, or over its row bound.');
  }
  if (!evidence.ranked_available && evidence.ranked_rows !== '0') {
    throw new Error('Unavailable rankings cannot carry a measured row count.');
  }
}

/** @param {string} identity @param {(url:string)=>Promise<any>} request */
export async function fetchSweepEvidence(identity, request) {
  if (!/^[0-9a-f]{64}$/.test(identity)) throw new Error('A canonical run identity is required.');
  /** @type {any} */
  let summary = null;
  /** @type {any[]} */
  const rows = [];
  for (let page = 0; page < ROW_CAP / PAGE_ROWS; page += 1) {
    const url = '/sweep-evidence.json?identity=' + identity + '&kind=depth&page=' + page +
      '&limit=' + PAGE_ROWS + (summary ? '&attempt=' + summary.attempt : '');
    const response = await request(url);
    if (!response?.ok) throw new Error('Saved evidence request failed (HTTP ' + String(response?.status) + ').');
    const body = await response.json();
    if (!object(body) || body.schema_version !== 1 || body.identity !== identity || body.refusal !== null) {
      throw new Error('Saved evidence is refused or belongs to another run.');
    }
    if (body.status === 'missing') {
      if (page !== 0 || body.evidence !== null || !Array.isArray(body.rows) || body.rows.length !== 0 ||
        typeof body.why !== 'string' || !body.why.trim()) {
        throw new Error('Missing evidence must be explicit, empty, and explained.');
      }
      return { status: 'missing', evidence: null, rows: [], why: body.why };
    }
    if (body.status !== 'saved') throw new Error('Unknown saved evidence state.');
    validateSummary(body.evidence);
    if (summary && Object.keys(summary).some((key) => summary[key] !== body.evidence[key])) {
      throw new Error('Saved attempt changed between pages; no mixed result is shown.');
    }
    summary = body.evidence;
    const total = Number(summary.depth_rows);
    const expected = Math.min(PAGE_ROWS, total - rows.length);
    if (body.kind !== 'depth' || body.page !== page || body.limit !== PAGE_ROWS ||
      body.page_complete !== true || body.total_count !== summary.depth_rows ||
      !Array.isArray(body.rows) || expected < 0 || body.rows.length !== expected) {
      throw new Error('Saved level page is incomplete or has inconsistent counts.');
    }
    for (const row of body.rows) {
      if (!object(row) || fields.some((field) => !unsigned(row[field])) ||
        BigInt(row.k) !== BigInt(rows.length + 1) || BigInt(row.k) > 384n ||
        typeof row.reconciles !== 'boolean') {
        throw new Error('Saved level has invalid counters or a missing/repeated depth.');
      }
      const sum = ['duplicates', 'excluded', 'pruned', 'infrequent', 'frequent']
        .reduce((total, field) => total + BigInt(row[field]), 0n);
      if (row.reconciles !== (sum === BigInt(row.generated))) {
        throw new Error('Saved level accounting flag contradicts its exact counters.');
      }
      rows.push(row);
    }
    const next = rows.length < total ? page + 1 : null;
    if (body.next_page !== next) throw new Error('Saved level cursor loops, skips, or ends early.');
    if (next === null) return { status: 'saved', evidence: summary, rows, why: '' };
  }
  throw new Error('Saved evidence exceeded the bounded page count.');
}

/** Whether this summary can offer the separate candidate-detail reader.
 * This is a navigation capability, never proof that a priced capture exists.
 * @param {any} result */
export function canInspectCandidateTrades(result) {
  return result?.status === 'saved' && ['audit', 'expression'].includes(result.evidence?.operation);
}

/** Human labels speak only for evidence actually received. @param {any} result */
export function evidenceComparison(result) {
  const e = result?.status === 'saved' ? result.evidence : null;
  const absent = result?.status === 'missing' ? 'Not recorded' : 'Not established';
  const complete = e?.completion === 'completed';
  const integrityOnly = e?.operation === 'checksum-audit';
  const laterOnly = e?.operation === 'boolean-oos';
  const catalogOnly = ['boolean-candidates', 'boolean-statistics', 'boolean-admission', 'boolean-qualification', 'index-consistency', 'index-stop', 'index-stop-qualification'].includes(e?.operation);
  const catalogStatus = e?.operation === 'index-stop' ? 'Recorded single-stop executions only' : e?.operation === 'index-stop-qualification' ? 'Recorded single-stop qualification only' : e?.operation === 'index-consistency' ? 'Recorded index day/week assessment only' : e?.operation === 'boolean-statistics' ? 'Recorded statistics only'
    : e?.operation === 'boolean-admission' ? 'Recorded research admission only' : e?.operation === 'boolean-qualification' ? 'Recorded fixed-training later qualification' : 'Explicit program catalog only';
  const candidateDetail = canInspectCandidateTrades(result);
  const catalogDetail = e?.operation === 'boolean-candidates';
  const linkedCatalogDetail = ['boolean-statistics','boolean-admission','boolean-qualification','index-consistency','index-stop','index-stop-qualification'].includes(e?.operation);
  const tradeStatus = !e ? absent : integrityOnly ? 'No trades in checksum audit'
    : candidateDetail || catalogDetail || linkedCatalogDetail || laterOnly ? 'Not verified in this summary' : 'No candidate capture in this view';
  const tradeDetail = !e
    ? 'No candidate trade evidence has been verified for this view. Missing or failed summary reads cannot establish trades.'
    : integrityOnly
      ? 'A checksum audit establishes input integrity only. It does not price a strategy or capture trades.'
      : laterOnly
        ? 'This lifecycle records later-period replay of fixed training settings. Its terminal state does not authenticate a trade page, establish full-campaign completion or promote a strategy.'
      : linkedCatalogDetail
        ? 'Open the saved statistics or research-check viewer above to authenticate its own receipt and linked catalog completions, then open an exact setting’s trades and sessions. This lifecycle summary does not verify those pages or grant Selection V6 authority.'
      : catalogDetail
        ? 'Open the saved Boolean catalog above to authenticate its own completion and each exact exit setting’s trades and sessions. This summary does not verify those pages or grant statistical or institutional admission.'
      : candidateDetail
        ? 'Open candidate detail above to verify this attempt’s sealed pricing capture, then its exact selected-cell trade page. Captures cover both directions of each actually priced candidate. Missing older captures and unpriced signal rows remain explicit; no winner trace is substituted.'
        : 'This view has no candidate-trade reader for the recorded operation. A completed signal search, preparation or replay summary does not establish a captured candidate trace here.';
  const accounting = e && result.rows.length > 0
    ? result.rows.every((/** @type {any} */ row) => row.reconciles) ? 'Reconciled' : 'Mismatch'
    : e ? 'Not recorded' : absent;
  return [
    { area: 'AND combination search', status: integrityOnly ? 'Input integrity only' : laterOnly ? 'Later-period comparison only' : catalogOnly ? catalogStatus : e ? ['expression', 'expression-search'].includes(e.operation) ? 'Different candidate model' : e.operation === 'global-replay-stream' ? 'One strategy replay only' : e.operation === 'global-replay' ? 'Separate chronological replay plan' : e.operation === 'preparation' ? 'Preparation only' : e.operation === 'auto-search' ? 'Orchestration only' : complete ? 'Attempt completed' : e.completion === 'running' ? 'No terminal receipt' : e.completion : absent,
      detail: 'A completed attempt is evidence for its recorded inputs and resource bounds. It does not prove every possible strategy or future outcome.' },
    { area: 'Every saved search depth', status: accounting,
      detail: 'Generated candidates must equal duplicates + excluded + pruned + below-support + frequent. Cumulative admissions and pair work are shown separately.' },
    { area: 'Historical financial rules', status: integrityOnly ? 'Not a pricing operation' : catalogOnly || laterOnly ? 'Not established in this summary' : 'Five checks only',
      detail: catalogOnly || laterOnly ? 'This operation records a stage of the explicit program catalog research pipeline. Its summary does not certify exhaustive Boolean grammar search, the legacy five-check ranking or a passing institutional strategy. Read its separate sealed evidence and individual decisions.' : 'The combination table checks win rate, worst win/loss ratio, return/drawdown, trade count and Wilson lower bound against saved thresholds.' },
    { area: 'Remaining selection checks', status: 'Incomplete in this view',
      detail: 'Worst adverse excursion, protective exits, fill headroom, average payoff and period consistency are not all proven by the five-check verdict.' },
    { area: 'Walk-forward and overfitting checks', status: e?.validation_requested === false ? 'Explicitly disabled' : e?.validation_requested === true ? 'Requested; outcome not stored here' : e ? 'Not recorded' : absent,
      detail: 'A requested validation stack is not a passed validation. This evidence does not carry sealed walk-forward, PBO, White, SPA or Romano–Wolf decisions.' },
    { area: 'Institutional admission', status: 'Authority not exposed here',
      detail: 'The separate admission authority is not bound to this legacy sweep view. Historical rule passes do not establish institutional admission or identify institutional order flow.' },
    { area: 'Exact trade replay', status: tradeStatus, detail: tradeDetail },
    { area: 'Boolean expressions: AND / OR / NOT', status: 'Separate recorded predicates',
      detail: 'Signal evaluation, versioned expression search and priced-expression research carry separate identities and explicit truth availability. Exact trades require a sealed expression capture; signal evidence alone does not establish trades or institutional admission.' }
  ];
}

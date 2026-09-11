import { decodeMaskWords } from './mask.js';
import { detailRefusal } from './detail-refusal.js';
const MAX_U64 = (1n << 64n) - 1n;
const MIN_I64 = -(1n << 63n);
const MAX_I64 = (1n << 63n) - 1n;
const hex = (/** @type {any} */ value) => typeof value === 'string' && /^[0-9a-f]{64}$/.test(value);
const unsigned = (/** @type {any} */ value) => typeof value === 'string' && /^(0|[1-9][0-9]*)$/.test(value) && value.length <= 20 && BigInt(value) <= MAX_U64;
const signed = (/** @type {any} */ value) => typeof value === 'string' && /^(0|-?[1-9][0-9]*)$/.test(value) && value.length <= 20 && BigInt(value) >= MIN_I64 && BigInt(value) <= MAX_I64;
const object = (/** @type {any} */ value) => value !== null && typeof value === 'object' && !Array.isArray(value);
const side = (/** @type {any} */ value) => value === 'long' || value === 'short';
const list = (/** @type {any} */ value) => Array.isArray(value) && value.every(signed);

/** @param {any} candidate @param {string} tier */
function validateCandidate(candidate, tier) {
  if (!object(candidate) || candidate.tier !== tier || !unsigned(candidate.rank) || candidate.rank === '0' ||
      !side(candidate.direction) || !decodeMaskWords(candidate.mask_words).ok || !hex(candidate.trade_identity) ||
      typeof candidate.cell_rules_pass !== 'boolean' || !unsigned(candidate.signals) || !unsigned(candidate.refused_paths) ||
      !['stops_ppm', 'targets_ppm', 'trails_ppm'].every((key) => list(candidate[key]))) {
    throw new Error('Candidate metadata has an invalid identity, mask, side or grid.');
  }
  const cell = candidate.cell;
  if (cell === null) {
    if (candidate.cell_rules_pass) throw new Error('A missing cell cannot pass its rules.');
    return;
  }
  if (!object(cell) || !unsigned(cell.trades) || !unsigned(cell.wins) || BigInt(cell.wins) > BigInt(cell.trades) ||
      !['pessimistic', 'optimistic', 'max_drawdown', 'worst_trade'].every((key) => signed(cell[key])) ||
      BigInt(cell.pessimistic) > BigInt(cell.optimistic)) throw new Error('Candidate cell statistics are invalid.');
  for (const [key, ladder] of [['stop', 'stops_ppm'], ['target', 'targets_ppm'], ['tsl', 'trails_ppm']]) {
    if (cell[key] !== null && (!unsigned(cell[key]) || BigInt(cell[key]) >= BigInt(candidate[ladder].length))) throw new Error('Selected exit does not belong to its saved ladder.');
  }
  if (cell.ttp !== null && (!object(cell.ttp) || !unsigned(cell.ttp.arm) || !unsigned(cell.ttp.trail) ||
      BigInt(cell.ttp.arm) >= BigInt(candidate.targets_ppm.length) || BigInt(cell.ttp.trail) >= BigInt(candidate.trails_ppm.length))) {
    throw new Error('Selected trailing profit does not belong to its saved ladder.');
  }
}
/** @param {any} tier */
function validateTier(tier) {
  if (!object(tier) || !['index', 'eligible', 'evaluated', 'horizon', 'rungs'].every((key) => unsigned(tier[key])) ||
      BigInt(tier.evaluated) > BigInt(tier.eligible) || tier.horizon === '0' || typeof tier.ratios !== 'boolean' ||
      !list(tier.stops_ppm) || (tier.step_ppm !== null && !signed(tier.step_ppm)) ||
      (tier.forced_ppm !== null && !signed(tier.forced_ppm)) || !object(tier.rules)) throw new Error('Saved pricing pass metadata is invalid.');
  if (!['max_mae_ppm', 'min_rr_bp', 'min_win_rate_bp', 'min_assurance_bp', 'min_weakest_bp', 'min_ret_over_dd_bp', 'min_fill_headroom_bp', 'min_avg_rr_bp'].every((key) => signed(tier.rules[key])) ||
      !unsigned(tier.rules.min_trades) || !unsigned(tier.rules.top) || typeof tier.rules.require_protective_exits !== 'boolean') throw new Error('Saved pricing policy is incomplete.');
}

/** Loads one bounded page only. Callers keep the returned catalog digest for every continuation.
 * @param {any} selection @param {(url:string)=>Promise<any>} request */
export async function fetchCandidatePage(selection, request) {
  const { identity, attempt, model = 'and-mask', digest = null, tier = '0', offset = '0', rank = null, direction = null } = selection;
  if (!['and-mask', 'expression'].includes(model) || !hex(identity) || !unsigned(attempt) || attempt === '0' || !unsigned(tier) || !unsigned(offset) ||
      (digest !== null && !hex(digest)) || ((rank !== null) !== (direction !== null)) ||
      (rank !== null && (!unsigned(rank) || rank === '0' || !side(direction))) ||
      ((tier !== '0' || offset !== '0' || rank !== null) && digest === null)) throw new Error('An exact run, attempt and catalog are required for candidate pages.');
  const query = new URLSearchParams({ identity, attempt, model, tier, offset, limit: '256' });
  if (digest) query.set('digest', digest);
  if (rank !== null) { query.set('rank', rank); query.set('direction', direction); }
  const response = await request('/candidate-trades.json?' + query);
  if (!response?.ok) throw new Error(await detailRefusal(response, 'Candidate detail request failed (HTTP ' + String(response?.status) + ').'));
  const body = await response.json();
  if (!object(body) || body.schema_version !== 1 || body.model !== model || body.identity !== identity || body.attempt !== attempt || body.refusal !== null || !Array.isArray(body.rows)) throw new Error('Candidate evidence is refused or belongs to another attempt or predicate model.');
  if (body.status === 'missing') {
    if (digest !== null || offset !== '0' || tier !== '0' || rank !== null || body.rows.length !== 0 || typeof body.why !== 'string' || !body.why.trim()) throw new Error('Missing candidate evidence must be explicit and cannot replace a pinned page.');
    return body;
  }
  if (body.status !== 'saved' || !hex(body.catalog_digest) || (digest !== null && body.catalog_digest !== digest) ||
      !hex(body.execution_digest) || body.capture !== 'sealed-pricing-evidence' ||
      ![null, 'running', 'completed', 'halted', 'refused'].includes(body.audit_completion) ||
      !unsigned(body.tier_count) || !unsigned(body.candidate_side_count) || !unsigned(body.total_count) ||
      body.offset !== offset || body.limit !== 256 || body.page_complete !== true ||
      body.kind !== (rank === null ? 'candidates' : 'trades')) throw new Error('Candidate catalog changed or the page is incomplete.');
  if (model === 'and-mask' ? body.expression !== null : !object(body.expression) || body.expression.version !== 1 ||
      typeof body.expression.source !== 'string' || body.expression.source.length === 0 || body.expression.source.length > 131072 ||
      typeof body.expression.encoded_hex !== 'string' || body.expression.encoded_hex.length === 0 ||
      body.expression.encoded_hex.length > 65536 || body.expression.encoded_hex.length % 2 !== 0 ||
      !/^[0-9a-f]+$/.test(body.expression.encoded_hex) || !decodeMaskWords(body.expression.referenced_mask_words).ok) {
    throw new Error('The complete expression descriptor is missing or the predicate model changed.');
  }
  const total = BigInt(body.total_count), start = BigInt(offset);
  if ((total > 0n && start >= total) || (total === 0n && start !== 0n)) throw new Error('Candidate page is outside the saved extent.');
  const count = Number(total - start > 256n ? 256n : total - start);
  const end = start + BigInt(count);
  if (body.rows.length !== count || body.next_offset !== (end < total ? String(end) : null)) throw new Error('Candidate page cardinality or continuation is inconsistent.');
  if (body.tier_count === '0') {
    if (body.tier !== null || body.candidate_side_count !== '0' || total !== 0n || rank !== null) throw new Error('An empty capture cannot carry selected trades.');
    return body;
  }
  validateTier(body.tier);
  if (body.tier.index !== tier || BigInt(tier) >= BigInt(body.tier_count)) throw new Error('Candidate policy pass changed.');
  if (rank === null) {
    if (body.selected !== null || total !== BigInt(body.tier.evaluated) * 2n) throw new Error('Candidate sides do not match the evaluated subset.');
    body.rows.forEach((/** @type {any} */ row, /** @type {number} */ index) => {
      validateCandidate(row, tier);
      if (model === 'expression' && row.mask_words.some((/** @type {string} */ word, /** @type {number} */ index) => word !== body.expression.referenced_mask_words[index])) throw new Error('Expression candidate referenced bits disagree with its complete predicate.');
      const position = start + BigInt(index);
      if (row.rank !== String(position / 2n + 1n) || row.direction !== (position % 2n === 0n ? 'long' : 'short')) throw new Error('Candidate order disagrees with its exact key.');
    });
  } else {
    validateCandidate(body.selected, tier);
    if (model === 'expression' && body.selected.mask_words.some((/** @type {string} */ word, /** @type {number} */ index) => word !== body.expression.referenced_mask_words[index])) throw new Error('Selected expression referenced bits changed.');
    if (body.selected.rank !== rank || body.selected.direction !== direction || total !== BigInt(body.selected.cell?.trades ?? '0')) throw new Error('Trades belong to a different selected candidate.');
    body.rows.forEach((/** @type {any} */ row, /** @type {number} */ index) => {
      if (!object(row) || row.identity !== body.selected.trade_identity || row.direction !== direction || row.seq !== String(start + BigInt(index)) ||
          !['signal_bar', 'entry_bar', 'exit_bar'].every((key) => unsigned(row[key])) ||
          !['best', 'worst', 'entry_micros', 'exit_micros', 'adverse_ppm', 'adverse_paisa', 'favourable_ppm', 'favourable_paisa'].every((key) => signed(row[key])) ||
          BigInt(row.entry_bar) > BigInt(row.exit_bar) || BigInt(row.entry_micros) > BigInt(row.exit_micros) || BigInt(row.worst) > BigInt(row.best)) throw new Error('Trade identity, sequence, direction or financial bounds disagree.');
    });
  }
  return body;
}

/** Exact paisa formatting without floating-point rounding. @param {string} value */
export function candidateMoney(value) {
  if (!signed(value)) return 'unavailable';
  const amount = BigInt(value), magnitude = amount < 0n ? -amount : amount;
  return (amount < 0n ? '−₹' : '₹') + String(magnitude / 100n) + '.' + String(magnitude % 100n).padStart(2, '0');
}

/** A priced outcome can have no selected cell; that is not an unpriced row.
 * @param {any} candidate @param {'pessimistic'|'optimistic'} field */
export function candidateTotal(candidate, field) {
  return candidate?.cell === null ? 'No selected cell' : candidateMoney(candidate?.cell?.[field]);
}

/** Saved counters and exit settings only; missing fields are never defaulted.
 * Choice numbers are the API's zero-based indices into its exact ppm ladders.
 * @param {any} candidate @returns {{label:string,value:string}[]} */
export function candidatePricingDetails(candidate) {
  if (!object(candidate)) return [];
  const rows = /** @type {{label:string,value:string}[]} */ ([]);
  for (const [key, label] of [['signals', 'Observed signals'], ['refused_paths', 'Refused execution paths']]) {
    if (unsigned(candidate[key])) rows.push({ label, value: candidate[key] });
  }
  if (candidate.cell === null) rows.push({ label: 'Pricing outcome', value: 'No selected cell' });
  /** @param {string} label @param {any} index @param {any} ladder */
  const selected = (label, index, ladder) => {
    if (index === null) rows.push({ label, value: 'No setting selected' });
    else if (unsigned(index) && list(ladder) && BigInt(index) < BigInt(ladder.length)) {
      rows.push({ label, value: 'Choice ' + index + ' · ' + ladder[Number(index)] + ' ppm' });
    }
  };
  if (object(candidate.cell)) {
    selected('Selected stop', candidate.cell.stop, candidate.stops_ppm);
    selected('Selected target', candidate.cell.target, candidate.targets_ppm);
    selected('Selected trailing stop', candidate.cell.tsl, candidate.trails_ppm);
    if (candidate.cell.ttp === null) rows.push({ label: 'Selected trailing profit', value: 'No setting selected' });
    else if (object(candidate.cell.ttp)) {
      selected('Trailing profit activation', candidate.cell.ttp.arm, candidate.targets_ppm);
      selected('Trailing profit distance', candidate.cell.ttp.trail, candidate.trails_ppm);
    }
  }
  for (const [key, label] of [['stops_ppm', 'Saved stop choices'], ['targets_ppm', 'Saved target choices'], ['trails_ppm', 'Saved trailing choices']]) {
    if (list(candidate[key])) rows.push({ label, value: candidate[key].length === 0 ? 'Empty saved ladder'
      : candidate[key].map((/** @type {string} */ value, /** @type {number} */ index) => index + ': ' + value + ' ppm').join(' · ') });
  }
  return rows;
}

/** @param {any} candidate @param {any} vocabulary */
export function candidateConditions(candidate, vocabulary) {
  const decoded = decodeMaskWords(candidate.mask_words);
  if (!decoded.ok) return 'Condition mask unavailable';
  if (decoded.positions.length === 0) return 'Empty condition mask';
  return decoded.positions.map((index) => vocabulary?.bits?.get(index)?.name ?? ('Condition bit ' + index)).join(' · ');
}

/** Human-readable IST time, retaining raw microseconds when outside Date's range.
 * @param {string} value */
export function candidateTime(value) {
  if (!signed(value)) return 'unavailable';
  const millis = BigInt(value) / 1000n;
  if (millis < -8640000000000000n || millis > 8640000000000000n) return value + ' µs';
  return new Date(Number(millis)).toLocaleString('en-IN', { timeZone: 'Asia/Kolkata', year: 'numeric', month: 'short', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false });
}

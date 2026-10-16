import { detailRefusal } from './detail-refusal.js';

/* THE SELECTION V6 READER. `/selection-v6.json` serves the sealed blocks
   `ledger-v6` committed, one list per intraday rung. This module asks for
   them and refuses any payload it cannot account for, field by field, so the
   page never draws a winner the server did not send in full. D-1578. */

const U64 = 18446744073709551615n;
const I64 = 9223372036854775807n;
export const RUNGS = ['1min', '2min', '3min', '5min', '10min', '15min', '30min', '60min'];
export const FAMILIES = ['NIFTY', 'BANKNIFTY'];
const TERMINALS = ['evaluated', 'insufficient-for-cscv', 'naturally-extinct'];

/** @param {any} v */
const uint = (v) => typeof v === 'string' && /^(0|[1-9][0-9]*)$/.test(v) && BigInt(v) <= U64;
/** @param {any} v */
const sint = (v) =>
  typeof v === 'string' && /^(0|-?[1-9][0-9]*)$/.test(v) && BigInt(v) >= -I64 - 1n && BigInt(v) <= I64;
/** @param {any} v */
const hex = (v) => typeof v === 'string' && /^[0-9a-f]{64}$/.test(v);
/** @param {any} v */
const object = (v) => v !== null && typeof v === 'object' && !Array.isArray(v);
/** @param {any} v */
const ratio = (v) => v === null || uint(v);

/**
 * The one selector the route takes. Anything but no family, NIFTY or
 * BANKNIFTY is refused here before a request is made: Selection V6 holds the
 * two index families only, and no equity result may enter it (CLAUDE.md §1).
 * @param {string|null} family
 */
export function selectionV6Query(family) {
  if (family === null || family === '') return '';
  if (!FAMILIES.includes(family)) {
    throw new Error(
      `${family} is not a Selection V6 family. Selection V6 holds NIFTY and BANKNIFTY only; ` +
        'no equity result may enter it until a charter-sourced equity charge stack exists.'
    );
  }
  return '?' + new URLSearchParams({ family });
}

/** @param {any} w @param {string|null} family */
function winner(w, family) {
  return (
    object(w) &&
    Number.isInteger(w.rank) &&
    w.rank >= 0 &&
    w.rank < 25 &&
    w.top_ten === w.rank < 10 &&
    FAMILIES.includes(w.family) &&
    (family === null || w.family === family) &&
    (w.direction === 'long' || w.direction === 'short') &&
    ['strategy', 'disposition', 'selected_exit'].every((k) => hex(w[k])) &&
    [
      'global_sequence',
      'family_sequence',
      'score',
      'drawdown',
      'worst_loss',
      'losing_rate_ppm',
      'losing_trades',
      'winning_trades',
      'win_rate_ppm',
      'average_win',
      'average_loss',
      'assurance_ppm'
    ].every((k) => uint(w[k])) &&
    sint(w.pessimistic_profit) &&
    ratio(w.loss_ratio_ppm) &&
    ratio(w.reward_to_risk_ppm) &&
    Array.isArray(w.mask_words) &&
    w.mask_words.length === 6 &&
    w.mask_words.every(uint)
  );
}

/** @param {any} r @param {string|null} family */
function record(r, family) {
  if (
    !object(r) ||
    !['identity', 'population', 'execution_completion', 'policy'].every((k) => hex(r[k])) ||
    !['rung_seconds', 'horizon_bars', 'rows', 'authorized', 'policy_refused', 'considered', 'admitted', 'refused', 'unmeasured'].every(
      (k) => uint(r[k])
    ) ||
    !Number.isInteger(r.winner_count) ||
    r.winner_count < 0 ||
    r.winner_count > 25 ||
    !Array.isArray(r.families) ||
    r.families.length !== 2 ||
    !r.families.every(
      (/** @type {any} */ f, /** @type {number} */ i) =>
        object(f) &&
        f.family === FAMILIES[i] &&
        TERMINALS.includes(f.terminal) &&
        ['candidates', 'evaluated', 'decisions'].every((k) => uint(f[k]))
    ) ||
    !Array.isArray(r.winners) ||
    r.winners.length > r.winner_count ||
    (family === null && r.winners.length !== r.winner_count) ||
    !r.winners.every((/** @type {any} */ w) => winner(w, family))
  ) {
    return false;
  }
  /* RANKS ASCEND AND, UNNARROWED, ARE EXACTLY 0..n-1: the stored Top-25. */
  return r.winners.every(
    (/** @type {any} */ w, /** @type {number} */ i) =>
      (family === null ? w.rank === i : i === 0 || w.rank > r.winners[i - 1].rank)
  );
}

/**
 * Fetch and check the committed Selection V6 records.
 * @param {string|null} family
 * @param {(url:string)=>Promise<any>} request
 */
export async function fetchSelectionV6(family, request) {
  const query = selectionV6Query(family);
  const asked = query === '' ? null : family;
  const response = await request('/selection-v6.json' + query);
  if (!response?.ok) {
    throw new Error(
      await detailRefusal(response, 'Selection V6 unavailable (HTTP ' + String(response?.status) + ').')
    );
  }
  const body = await response.json();
  if (
    !object(body) ||
    body.schema_version !== 1 ||
    (body.status !== 'saved' && body.status !== 'absent') ||
    body.authority !== 'sealed-stored-selection-v6-record' ||
    body.family !== asked ||
    typeof body.scope !== 'string' ||
    typeof body.equities !== 'string' ||
    !body.equities.includes('No equity result may enter Selection V6') ||
    body.refusal !== null ||
    !Array.isArray(body.rungs) ||
    body.rungs.length !== RUNGS.length
  ) {
    throw new Error('Selection V6 payload is not the shape this page reads; nothing is drawn.');
  }
  let saved = false;
  body.rungs.forEach((/** @type {any} */ rung, /** @type {number} */ i) => {
    const ok =
      object(rung) &&
      rung.rung === RUNGS[i] &&
      Array.isArray(rung.records) &&
      ((rung.status === 'absent' && typeof rung.path === 'string' && rung.refusal === null && rung.records.length === 0) ||
        (rung.status === 'refused' && typeof rung.refusal === 'string' && rung.refusal !== '' && rung.records.length === 0) ||
        (rung.status === 'saved' &&
          rung.refusal === null &&
          rung.records.length > 0 &&
          rung.records.every((/** @type {any} */ r) => record(r, asked))));
    if (!ok) throw new Error(`Selection V6 rung ${RUNGS[i]} is malformed; nothing is drawn.`);
    if (rung.status === 'saved') saved = true;
  });
  if ((body.status === 'saved') !== saved) {
    throw new Error('Selection V6 status disagrees with its rungs; nothing is drawn.');
  }
  return body;
}

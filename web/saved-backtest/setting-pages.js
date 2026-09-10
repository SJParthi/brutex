import { fetchBooleanLater } from '../src/lib/boolean-oos.js';

const U64 = (1n << 64n) - 1n, I64 = (1n << 63n) - 1n;
const VISIBLE_LIMIT = 32;
/** @param {any} value */
const uint = value => typeof value === 'string' && value.length <= 20 && /^(0|[1-9]\d*)$/.test(value) && BigInt(value) <= U64;
/** @param {any} value */
const sint = value => typeof value === 'string' && value.length <= 20 && /^(0|-?[1-9]\d*)$/.test(value) && BigInt(value) >= -I64 - 1n && BigInt(value) <= I64;
/** @param {any} value */
const hex = value => typeof value === 'string' && /^[0-9a-f]{64}$/.test(value);
/** @param {any} value */
const link = value => value && !Array.isArray(value) && ['identity', 'completion'].every(key => hex(value[key]));
/** Copy the bounded join fields before the first await. A changed caller row
 * cannot rename a setting while its response is in flight. @param {any} row */
function snapshot(row) {
  const original = row?.statistics, later = row?.qualification, source = original?.source;
  return {
    index: row?.index,
    statistics: {
      index: original?.index, coordinate: original?.coordinate, identity: original?.identity,
      side: original?.side, run: original?.run, program_index: original?.program_index,
      ordinal: original?.ordinal, trades: original?.trades, wins: original?.wins,
      return_paisa: original?.return_paisa, execution_refusal_bits: original?.execution_refusal_bits,
      source: {
        identity: source?.identity, completion: source?.completion, instrument: source?.instrument,
        cash: source?.cash, index: source?.index, coordinates: source?.coordinates,
        membership_digest: source?.membership_digest
      }
    },
    qualification: {
      coordinate: later?.coordinate, identity: later?.identity,
      source: { identity: later?.source?.identity, completion: later?.source?.completion }
    }
  };
}

/**
 * Fill only the visible settings from exact, authenticated later-coordinate
 * pages, which also contain the frozen training coordinates. No extra catalog
 * download and no requests for gaps. Return one complete map or refuse it all.
 *
 * `request` should already carry `signal` to the transport (as App's request
 * factory does). Checks here also stop stale results from publishing when a
 * request implementation resolves despite cancellation.
 *
 * @param {any[]} rows Validated qualified-search source rows, at most 32.
 * @param {(url:string)=>Promise<any>} request
 * @param {AbortSignal} signal
 * @param {typeof fetchBooleanLater} [fetcher] Test seam; production uses the shared validator.
 * @returns {Promise<Map<string, {training:any,later:any,grid:any,laterPeriod:any,grids:any[]}>>}
 */
export async function loadSettingFacts(rows, request, signal, fetcher = fetchBooleanLater) {
  signal.throwIfAborted();
  if (!Array.isArray(rows) || rows.length > VISIBLE_LIMIT) throw new Error('Visible setting detail is limited to 32 rows.');
  /** @type {Map<string, any[]>} */
  const groups = new Map();
  const global = new Set(), coordinates = new Set();
  for (const input of rows) {
    const row = snapshot(input);
    const original = row?.statistics, later = row?.qualification;
    if (!uint(row?.index) || global.has(row.index) || !link(original?.source) || !link(later?.source) ||
        original.index !== row.index || !hex(original.identity) || !hex(original.run) || !hex(later.identity) ||
        !['coordinate', 'program_index', 'ordinal', 'trades', 'wins', 'execution_refusal_bits'].every(key => uint(original[/** @type {keyof typeof original} */(key)])) ||
        !sint(original.return_paisa) || BigInt(original.wins) > BigInt(original.trades) || BigInt(original.execution_refusal_bits) > 63n ||
        original.coordinate !== later.coordinate || !['long', 'short'].includes(original.side) ||
        typeof original.source.instrument !== 'string' || !original.source.instrument || typeof original.source.cash !== 'boolean' ||
        !uint(original.source.index) || !uint(original.source.coordinates) || !hex(original.source.membership_digest) ||
        BigInt(original.coordinate) >= BigInt(original.source.coordinates)) {
      throw new Error('A visible setting has ambiguous or incomplete original/later linkage.');
    }
    global.add(row.index);
    const key = later.source.identity + ':' + later.source.completion;
    const group = groups.get(key) ?? [];
    const coordinate = key + ':' + original.coordinate;
    if (coordinates.has(coordinate)) throw new Error('Two visible settings claim the same later coordinate.');
    coordinates.add(coordinate);
    if (group.length && Object.keys(original.source).some(field =>
      original.source[/** @type {keyof typeof original.source} */(field)] !== group[0].statistics.source[field])) {
      throw new Error('One pinned later source claims conflicting original ancestors or instruments.');
    }
    group.push(row); groups.set(key, group);
  }
  /** @type {Map<string, {training:any,later:any,grid:any,laterPeriod:any,grids:any[]}>} */
  const result = new Map();
  for (const group of groups.values()) {
    group.sort((a, b) => BigInt(a.statistics.coordinate) < BigInt(b.statistics.coordinate) ? -1 : BigInt(a.statistics.coordinate) > BigInt(b.statistics.coordinate) ? 1 : 0);
    for (let start = 0; start < group.length;) {
      let end = start + 1;
      while (end < group.length) {
        const previous = BigInt(group[end - 1].statistics.coordinate), next = BigInt(group[end].statistics.coordinate);
        if (next !== previous + 1n) break;
        end += 1;
      }
      const visible = group.slice(start, end), source = visible[0].qualification.source;
      const offset = visible[0].statistics.coordinate, limit = visible.length;
      signal.throwIfAborted();
      const body = await fetcher({ identity: source.identity, completion: source.completion, kind: 'coordinates', offset, limit }, request);
      signal.throwIfAborted();
      if (body?.identity !== source.identity || body.completion !== source.completion || body.kind !== 'coordinates' ||
          body.offset !== offset || body.limit !== limit || !Array.isArray(body.rows) || body.rows.length !== limit || !Array.isArray(body.grids)) {
        throw new Error('A saved setting page differs from its exact requested source or span.');
      }
      for (const [index, row] of visible.entries()) {
        const pair = body.rows[index], original = row.statistics, later = row.qualification;
        const grids = body.grids.filter((/** @type {any} */ grid) => grid.side === original.side);
        if (body.parent?.identity !== original.source.identity || body.parent?.completion !== original.source.completion ||
            body.instrument !== original.source.instrument || body.cash !== original.source.cash ||
            body.membership_digest !== original.source.membership_digest || body.coordinate_count !== original.source.coordinates || grids.length !== 1 ||
            pair?.index !== original.coordinate || pair.index !== later.coordinate ||
            pair.training?.index !== pair.index || pair.later?.index !== pair.index ||
            pair.training?.identity !== original.identity || pair.later?.identity !== later.identity ||
            pair.training?.side !== original.side || pair.later?.side !== original.side ||
            pair.training?.run !== original.run || pair.training?.program_index !== original.program_index || pair.training?.ordinal !== original.ordinal ||
            pair.training?.execution_refusal_bits !== original.execution_refusal_bits ||
            pair.training?.cell?.trades !== original.trades || pair.training?.cell?.wins !== original.wins ||
            pair.training?.cell?.pessimistic !== original.return_paisa) {
          throw new Error('Saved setting detail does not match its original source, coordinate, direction or measured training result.');
        }
        result.set(row.index, { training: pair.training, later: pair.later, grid: grids[0], laterPeriod: body.later, grids: body.grids });
      }
      start = end;
    }
  }
  signal.throwIfAborted();
  return result;
}

/** Compare the fixed coordinate projection already authenticated for a visible
 * setting. Neither response key order nor row order is used as an identity.
 * @param {any} actual @param {any} expected */
function sameCoordinate(actual, expected) {
  if (!actual || !expected || typeof expected.expression !== 'string' || !expected.expression) return false;
  const scalars = ['index', 'identity', 'run', 'program_index', 'ordinal', 'side', 'expression', 'support_sessions', 'execution_refusal_bits'];
  const counts = ['trades', 'wins', 'pessimistic', 'optimistic', 'fill_cost', 'stopped', 'targeted', 'trailed_stop', 'trailed_profit', 'timed_out', 'ambiguous_bars', 'gapped'];
  if (!scalars.every(key => typeof expected[key] === 'string' && actual[key] === expected[key]) ||
      !counts.every(key => typeof expected.cell?.[key] === 'string' && actual.cell?.[key] === expected.cell[key]) ||
      !['evaluated', 'hits', 'misses', 'unknown'].every(key => typeof expected.truth?.[key] === 'string' && actual.truth?.[key] === expected.truth[key]) ||
      !['stop_ppm', 'target_ppm', 'tsl_ppm', 'ttp_arm_ppm', 'ttp_trail_ppm'].every(key =>
        (expected.levels?.[key] === null || typeof expected.levels?.[key] === 'string') && actual.levels?.[key] === expected.levels[key]) ||
      !['stop', 'target', 'tsl'].every(key => (expected.cell[key] === null || uint(expected.cell[key])) && actual.cell?.[key] === expected.cell[key])) return false;
  const expectedProfit = expected.cell.ttp, actualProfit = actual.cell?.ttp;
  if (expectedProfit === null ? actualProfit !== null : !expectedProfit || !['arm', 'trail'].every(key => uint(expectedProfit[key]) && actualProfit?.[key] === expectedProfit[key])) return false;
  return Array.isArray(expected.execution_refusals) && Array.isArray(actual.execution_refusals) &&
    expected.execution_refusals.length === actual.execution_refusals.length &&
    expected.execution_refusals.every((/** @type {any} */ reason) => typeof reason === 'string' && actual.execution_refusals.includes(reason));
}

/** Match the already validated grid without depending on JSON property order.
 * @param {any} actual @param {any} expected */
function sameGrid(actual, expected) {
  return actual && expected && Object.keys(expected).every(key => key === 'forced_stop'
    ? actual[key]?.kind === expected[key]?.kind && actual[key]?.ppm === expected[key]?.ppm
    : actual[key] === expected[key]);
}

/** Final selected-setting join, after the shared trade/session wire validator
 * but before the detail response is published. Missing comparison facts are a
 * refusal; they cannot be replaced by a matching page number or source label.
 * @param {any} body @param {any} row @param {boolean} isLater @param {any} fact
 * @returns {any} The same validated body, unchanged.
 */
export function assertTradeSetting(body, row, isLater, fact) {
  if (typeof isLater !== 'boolean' || !fact?.training || !fact?.later || !Array.isArray(fact.grids) || fact.grids.length !== 2) {
    throw new Error('Load the exact setting comparison before opening its trade or session pages.');
  }
  const original = row?.statistics, later = row?.qualification;
  if (!uint(row?.index) || !original || original.index !== row.index || !link(original.source) || !link(later?.source) ||
      !hex(original.identity) || !hex(later.identity) || !uint(original.coordinate) || original.coordinate !== later.coordinate ||
      fact.training.identity !== original.identity || fact.later.identity !== later.identity ||
      fact.training.index !== original.coordinate || fact.later.index !== later.coordinate ||
      fact.training.side !== original.side || fact.later.side !== original.side ||
      fact.training.run !== original.run || fact.training.program_index !== original.program_index || fact.training.ordinal !== original.ordinal ||
      fact.training.cell?.trades !== original.trades || fact.training.cell?.wins !== original.wins || fact.training.cell?.pessimistic !== original.return_paisa ||
      fact.training.execution_refusal_bits !== original.execution_refusal_bits) {
    throw new Error('The opened setting differs from its visible training/later comparison.');
  }
  const source = isLater ? later.source : original.source;
  if (!body || body.identity !== source.identity || body.completion !== source.completion ||
      !['trades', 'sessions'].includes(body.kind) || body.candidate !== original.coordinate ||
      body.instrument !== original.source.instrument || body.cash !== original.source.cash ||
      body.membership_digest !== original.source.membership_digest || body.coordinate_count !== original.source.coordinates ||
      !Array.isArray(body.grids) || body.grids.length !== 2 || !body.grids.every((/** @type {any} */ grid, /** @type {number} */ index) => sameGrid(grid, fact.grids[index]))) {
    throw new Error('The trade or session page belongs to a different saved source, grid or setting.');
  }
  if (isLater) {
    if (body.parent?.identity !== original.source.identity || body.parent?.completion !== original.source.completion ||
        body.selected?.index !== original.coordinate || !sameCoordinate(body.selected?.training, fact.training) ||
        !sameCoordinate(body.selected?.later, fact.later) || !fact.laterPeriod ||
        !['source', 'execution', 'first_micros', 'last_micros', 'bars', 'from', 'to'].every(key => typeof fact.laterPeriod[key] === 'string' && body.later?.[key] === fact.laterPeriod[key])) {
      throw new Error('The later trade or session page lost its exact original setting or later-period identity.');
    }
  } else if (!sameCoordinate(body.selected, fact.training)) {
    throw new Error('The training trade or session page differs from the visible original setting.');
  }
  return body;
}

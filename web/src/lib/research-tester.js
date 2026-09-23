import { ask } from './ask.js';
import { fetchBooleanCatalog } from './boolean-catalog.js';
import { fetchBooleanLater } from './boolean-oos.js';
import { CAMPAIGN_RUNGS } from './boolean-campaign.js';
import { savedProjectionTimeframes } from './saved-timeframes.js';
import { indexConsistencyContext, validateSearchIndexConsistency } from './index-consistency.js';
import { loadSettingFacts, assertTradeSetting } from '../../saved-backtest/setting-pages.js';
import { fetchWithBusyRetry } from '../../saved-backtest/requests.js';
import { signalExplanation } from '../../saved-backtest/explanations.js';

/** @param {any} value */
const uint = value => typeof value === 'string' && value.length <= 20 && /^(0|[1-9]\d*)$/.test(value) && BigInt(value) <= (1n << 64n) - 1n;
/** @param {any} value */
const hex = value => typeof value === 'string' && /^[0-9a-f]{64}$/.test(value) && value !== '0'.repeat(64);

/** Condition names must belong to the saved indicator build. A live vocabulary
 * from another build cannot silently rename historical expressions.
 * @param {any} fact @param {any} vocabulary */
export function researchRule(fact, vocabulary) {
  const matching = vocabulary?.phase === 'ready' && hex(vocabulary.commitDigest) &&
    vocabulary.bits instanceof Map && Array.isArray(fact?.grids) && fact.grids.length === 2 &&
    fact.grids[0]?.commit === vocabulary.commitDigest && fact.grids[1]?.commit === vocabulary.commitDigest;
  const names = matching ? Array.from(vocabulary.bits, ([index, value]) => ({ index, name: value.name })) : [];
  return signalExplanation(fact?.training?.expression, names);
}

/** Select from a previously authenticated search page, never by display rank.
 * The bounded copy prevents a refreshed overview from relabelling an open detail.
 * @param {any} detail @param {string} index */
export function researchSetting(detail, index) {
  if (detail?.authority !== 'authenticated-search-projection' || detail.kind !== 'rung' ||
      detail.child_bodies_checked !== true || ![1, 2, 3, 4].includes(detail.projection_version) ||
      !hex(detail.identity) || !hex(detail.pin) || !hex(detail.completion) || !uint(detail.batch) ||
      !uint(detail.rung) || BigInt(detail.rung) >= 8n || !uint(index) ||
      !Array.isArray(detail.source?.rows) || detail.source.rows.length > 256 ||
      !Array.isArray(detail.rows) || detail.rows.length !== detail.source.rows.length ||
      detail.source.completion !== detail.completion || !hex(detail.search_policy?.digest)) {
    throw new Error('Open an authenticated search comparison before inspecting a setting.');
  }
  const timeframes = savedProjectionTimeframes(detail);
  if (!timeframes.includes(CAMPAIGN_RUNGS[Number(detail.rung)])) {
    throw new Error('The selected setting is outside the exact saved timeframe scope.');
  }
  const matches = detail.source.rows.map((/** @type {any} */ row, /** @type {number} */ position) => ({ row, position })).filter((/** @type {any} */ value) => value.row.index === index);
  if (matches.length !== 1) throw new Error('The selected setting is absent or duplicated in this exact page.');
  const { row, position } = matches[0], comparison = detail.rows[position]?.comparison;
  if (comparison?.index !== row.index || comparison.identity !== row.identity ||
      comparison.policy_digest !== detail.search_policy.digest ||
      !['admitted', 'rejected', 'refused', 'unmeasured'].includes(comparison.status)) {
    throw new Error('The setting and its search-wide decision do not have the same saved identity.');
  }
  const consistencyContext = indexConsistencyContext(detail.source, row, comparison.status);
  const consistency = validateSearchIndexConsistency(row.index_consistency, comparison.index_consistency,
    consistencyContext, detail.projection_version);
  return structuredClone({ search: detail.identity, pin: detail.pin, completion: detail.completion,
    batch: detail.batch, rung: detail.rung, timeframes, projection: detail.projection_version,
    pageOffset: detail.source.offset, row, comparison, policy: detail.search_policy, consistency, consistencyContext });
}

/** One bounded page at a time. No full-trade download, recalculated admission,
 * profit ranking, current-market overlay or synthetic candle is introduced.
 * @param {(state:any)=>void} publish
 * @param {typeof ask} [request]
 * @param {{facts?:typeof loadSettingFacts,training?:typeof fetchBooleanCatalog,later?:typeof fetchBooleanLater,join?:typeof assertTradeSetting}} [adapters]
 */
export function createResearchTester(publish, request = ask, adapters = {}) {
  const facts = adapters.facts ?? loadSettingFacts, training = adapters.training ?? fetchBooleanCatalog;
  const later = adapters.later ?? fetchBooleanLater, join = adapters.join ?? assertTradeSetting;
  let generation = 0, abort = /** @type {AbortController|null} */ (null);
  let context = /** @type {any} */ (null), fact = /** @type {any} */ (null);
  /** @param {AbortSignal} signal */
  const transport = signal => (/** @type {string} */ url) => fetchWithBusyRetry(url, { signal }, request);
  function cancel() { generation += 1; abort?.abort(); abort = null; }
  /** @param {string} period @param {string} kind @param {string} offset */
  async function page(period = 'later', kind = 'trades', offset = '0') {
    cancel();
    const ticket = generation, controller = new AbortController(); abort = controller;
    const opened = context, openedFact = fact;
    const base = { context: opened, fact: openedFact, period, kind, offset };
    publish({ ...base, phase: 'loading', body: null, why: '' });
    try {
      if (!opened || !openedFact || !['training', 'later'].includes(period) ||
          !['trades', 'sessions'].includes(kind) || !uint(offset)) throw new Error('Choose a saved setting, period and valid page.');
      const isLater = period === 'later', part = isLater ? opened.row.qualification : opened.row.statistics;
      const body = await (isLater ? later : training)({ identity: part.source.identity,
        completion: part.source.completion, candidate: part.coordinate, kind, offset, limit: 32 }, transport(controller.signal));
      controller.signal.throwIfAborted();
      const checked = join(body, opened.row, isLater, openedFact);
      if (ticket !== generation) return null;
      publish({ ...base, phase: 'ready', body: checked, why: '' });
      return checked;
    } catch (why) {
      if (ticket === generation) publish({ ...base, phase: 'failed', body: null, why: String(why) });
      return null;
    }
  }
  return {
    /** @param {any} detail @param {string} index */
    async open(detail, index) {
      cancel(); context = null; fact = null;
      const ticket = generation, controller = new AbortController(); abort = controller;
      publish({ phase: 'loading', context: null, fact: null, body: null, period: 'later', kind: 'trades', why: '' });
      try {
        const selected = researchSetting(detail, index);
        const loaded = await facts([selected.row], transport(controller.signal), controller.signal);
        controller.signal.throwIfAborted();
        if (ticket !== generation) return null;
        const selectedFact = loaded.get(index);
        if (!selectedFact) throw new Error('The selected setting has no authenticated entry, exit and later-period comparison.');
        context = selected; fact = selectedFact;
        return await page();
      } catch (why) {
        if (ticket === generation) publish({ phase: 'failed', context: null, fact: null, body: null, why: String(why) });
        return null;
      }
    },
    page,
    close() { cancel(); context = null; fact = null; publish({ phase: 'idle', context: null, fact: null, body: null, why: '' }); }
  };
}

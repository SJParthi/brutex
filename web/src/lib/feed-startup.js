import { pickDefaultFeed } from './pick.js';
import { surveyFeedHeaders } from './feed-summary.js';

export const FEED_PREFERENCE_KEY = 'brutex.feed.v1';

/**
 * A saved selection needs only the small descriptor response. On a first
 * visit, manifest headers preserve the data-backed default without fetching
 * every feed's full inventory.
 *
 * @template {{wire: string, ready: boolean}} F
 * @param {{all: F[], active: string | null, error: string | null}} state
 * @param {import('./ask.js').ask} request
 * @param {() => Pick<Storage, 'getItem' | 'setItem'>} storage
 */
export function createFeedStartup(state, request, storage) {
  let selectionRevision = 0;
  /** @type {Promise<void> | null} */
  let flight = null;

  /** @param {string | null} wire */
  function select(wire) {
    if (wire !== null && !state.all.some((feed) => feed.wire === wire)) {
      throw new Error('The selected feed is absent from /feeds.json');
    }
    selectionRevision += 1;
    state.active = wire;
    state.error = null;
    try {
      storage().setItem(FEED_PREFERENCE_KEY, JSON.stringify(wire));
    } catch {
      // Browser preferences are optional; the in-page selection still holds.
    }
  }

  /** @param {F[]} list */
  function initial(list) {
    if (state.active !== null && list.some((feed) => feed.wire === state.active)) {
      return state.active;
    }
    try {
      const saved = storage().getItem(FEED_PREFERENCE_KEY);
      if (saved !== null) {
        const wire = JSON.parse(saved);
        if (wire === null || (typeof wire === 'string' && list.some((f) => f.wire === wire))) {
          return wire;
        }
      }
    } catch {
      // An unavailable or malformed preference cannot invent a feed.
    }
    return undefined;
  }

  async function loadNow() {
    const revision = selectionRevision;
    try {
      const response = await request('/feeds.json');
      if (!response.ok) throw new Error(`HTTP ${response.status} from /feeds.json`);
      const list = await response.json();
      if (!Array.isArray(list) || list.some((feed) =>
        !feed || typeof feed.wire !== 'string' || feed.wire.length === 0 ||
        typeof feed.ready !== 'boolean'
      ) || new Set(list.map((feed) => feed.wire)).size !== list.length) {
        throw new Error('The body of /feeds.json is not a valid feed descriptor list');
      }
      state.all = list;
      state.error = null;
      // A user selection (including an explicit clear) outranks an older read.
      if (selectionRevision !== revision) return;
      const preferred = initial(list);
      if (preferred !== undefined) {
        state.active = preferred;
        return;
      }
      try {
        const held = await surveyFeedHeaders(list, request);
        if (selectionRevision === revision) state.active = pickDefaultFeed(held);
      } catch (why) {
        if (selectionRevision === revision) throw why;
      }
    } catch (why) {
      state.error = String(why);
    }
  }

  /** @param {boolean} [force] */
  function load(force = false) {
    if (flight) return flight;
    if (!force && state.all.length > 0 && !state.error) return Promise.resolve();
    flight = loadNow().finally(() => { flight = null; });
    return flight;
  }

  return { load, select };
}

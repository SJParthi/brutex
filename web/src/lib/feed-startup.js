import { pickDefaultFeed } from './pick.js';
import { surveyFeedHeaders } from './feed-summary.js';

export const FEED_PREFERENCE_KEY = 'brutex.feed.v1';

/**
 * THE FEED A PAGE'S ADDRESS NAMES, as `?feed=<wire>` (the first of a
 * comma-separated list), or `null` when it names none. Pure, so the startup can
 * resolve it BEFORE any census request (DB-B54).
 *
 * @param {string | null | undefined} search a `location.search`
 * @returns {string | null}
 */
export function urlFeedOf(search) {
  if (typeof search !== 'string' || search === '') return null;
  let named = null;
  try {
    named = new URLSearchParams(search).get('feed');
  } catch {
    return null;
  }
  const first = (named ?? '').split(',')[0].trim();
  return first === '' ? null : first;
}

/**
 * A saved selection needs only the small descriptor response. On a first
 * visit, manifest headers preserve the data-backed default without fetching
 * every feed's full inventory.
 *
 * THE ADDRESS OUTRANKS THE SAVED PREFERENCE, AND IS NEVER SAVED (DB-B54).
 * Opening `/ingest?feed=gdfl` with Zerodha saved used to apply the saved feed
 * first, start Zerodha's whole census download (15.7 MB), then switch to GDFL
 * (32.6 MB more) and WRITE `gdfl` to the shared preference, so every other
 * tab's next load opened on GDFL. `urlFeed` is read before the first
 * selection, and a feed taken from it or from any non-explicit source is
 * selected with `{ persist: false }`: only a picker choice is remembered.
 *
 * @template {{wire: string, ready: boolean}} F
 * @param {{all: F[], active: string | null, error: string | null}} state
 * @param {import('./ask.js').ask} request
 * @param {() => Pick<Storage, 'getItem' | 'setItem'>} storage
 * @param {() => string | null} [urlFeed] the feed the page's address names
 */
export function createFeedStartup(state, request, storage, urlFeed = () => null) {
  let selectionRevision = 0;
  /** @type {Promise<void> | null} */
  let flight = null;

  /**
   * @param {string | null} wire
   * @param {{ persist?: boolean }} [how] `persist: false` for a feed the page
   *   took from its address rather than from the operator's own pick.
   */
  function select(wire, how = {}) {
    if (wire !== null && !state.all.some((feed) => feed.wire === wire)) {
      throw new Error('The selected feed is absent from /feeds.json');
    }
    selectionRevision += 1;
    state.active = wire;
    state.error = null;
    if (how.persist === false) return;
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
    let addressed = null;
    try {
      addressed = urlFeed();
    } catch {
      addressed = null;
    }
    if (addressed !== null && list.some((feed) => feed.wire === addressed)) return addressed;
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
        // ONLY THE OFFERED FEEDS ARE SURVEYED (D-3338): a feed the operator
        // excluded is never the default, so its census is not asked for.
        const held = await surveyFeedHeaders(list.filter((feed) => /** @type {any} */ (feed).offered !== false), request);
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

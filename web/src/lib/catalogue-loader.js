import { build, probe } from './prefix.js';

/** One retained catalogue and one current read for the selected feed.
 * Duplicate callers share the same promise. Replaced requests are aborted and
 * their responses cannot publish, including an A → B → A selection sequence.
 * Parsing and indexing cost O(rows) once per successful read; a repeated load
 * of the held feed does no network work or indexing. A forced load revalidates.
 * @template {{symbol?: string}} Row
 * @param {{rows:Row[],ready:boolean,feed:string|null,error:string|null}} state
 * @param {import('./ask.js').ask} request
 */
export function createCatalogueLoader(state, request) {
  let heldFeed = /** @type {string|null} */ (null);
  let active = /** @type {{feed:string,controller:AbortController,promise:Promise<void>}|null} */ (null);
  let byPrefix = new Map();

  function clear() {
    heldFeed = null;
    byPrefix = new Map();
    state.rows = [];
    state.feed = null;
    state.ready = false;
  }

  /** @param {string|null|undefined} feed @param {boolean} [force] */
  function load(feed, force = false) {
    if (!feed) {
      active?.controller.abort();
      active = null;
      clear();
      state.error = 'No feed was named, so no instrument master was read. Choose a feed to read its catalogue.';
      return Promise.resolve();
    }
    if (active?.feed === feed) return active.promise;
    if (!force && heldFeed === feed && active === null) return Promise.resolve();
    active?.controller.abort();
    clear();
    state.error = null;
    const current = { feed, controller: new AbortController(), promise: Promise.resolve() };
    active = current;
    current.promise = (async () => {
      try {
        const response = await request(`/instruments.json?feed=${encodeURIComponent(feed)}`, {
          signal: current.controller.signal
        });
        if (active !== current) return;
        if (!response.ok) {
          const note = response.headers.get('x-brutex-master-note');
          const phase = response.headers.get('x-brutex-master-state');
          throw new Error(note ? (phase ? `${phase} — ${note}` : note) : `HTTP ${response.status}`);
        }
        const rows = await response.json();
        if (active !== current) return;
        if (!Array.isArray(rows) || rows.some((row) => !row || typeof row !== 'object' || Array.isArray(row))) {
          throw new Error('The body of /instruments.json is not an instrument row list');
        }
        byPrefix = build(rows);
        heldFeed = feed;
        state.rows = rows;
        state.feed = feed;
        state.ready = true;
        state.error = null;
      } catch (why) {
        if (active !== current) return;
        clear();
        state.error = String(why);
      } finally {
        if (active === current) active = null;
      }
    })();
    return current.promise;
  }

  return {
    load,
    /** @param {string} typed */
    search: (typed) => probe(byPrefix, state.rows, typed)
  };
}

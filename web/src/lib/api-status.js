/**
 * THE HEADER'S API BUTTON — when it checks, and what it says (DB-N01).
 *
 * Measured on the live page, 9 Oct 2026: the button read "Checking API…" for
 * at least 37 s after load, its title said "Checked 01 Jan 1970, 05:30", and
 * only a click settled it. The probe runs through `watchVisible`, which polls
 * only while the document is visible — so a page opened in a tab that was not
 * yet visible never ran its FIRST check, and the never-set stamp (0) was
 * rendered as a date.
 *
 * Two rules, pure, so `web/tests/api-status.test.js` drives them under
 * `node --test`:
 *
 *   1. The first check runs at load whatever the visibility; only the
 *      repeating poll waits for a visible page.
 *   2. A stamp that was never set is not a date: the title says the API has
 *      not been checked yet.
 *
 * # The check asks for headers only (R3-15)
 *
 * The probe was a `GET /feeds.json` every 8 s on every open tab: the whole
 * feeds answer on the wire, parsed and thrown away, and on the server a
 * census read, a settings stat and a held-span walk per probe, per tab, for a
 * question whose whole answer is "something that is the brutex API
 * answered". An idle tab now asks only {@link upCheck}: `HEAD
 * /inspection.json`, a constant answer the server builds from its commit
 * stamp alone, whose headers carry the content type and the build
 * (`x-brutex-build`, DB-B71) and no body. The dev-server trap the layout
 * records still holds — a static fallback answering 200 with HTML is caught
 * by the content type, which a HEAD answer carries.
 */
import { createPageRequests } from './page-requests.js';
import { buildOf } from './runtime-inspection.js';

/** What the header's button asks: a constant route, by its headers alone. */
export const UP_ROUTE = '/inspection.json';

/** How it asks. */
export const UP_METHOD = 'HEAD';

/**
 * @typedef {{state: 'checking' | 'up' | 'down', ms: number, why: string | null, at: number,
 *   route?: string, method?: string, build?: string | null}} ApiState
 */

/** The button's state before the first answer: checking, and what it asks. */
export const UP_CHECKING = Object.freeze({ state: 'checking', ms: 0, why: null, at: 0, route: UP_ROUTE, method: UP_METHOD, build: null });

/**
 * One header-only "server up" check: `HEAD /inspection.json`, through the
 * page's own `ask` and its ticket. Up when the answer is 2xx and JSON by its
 * content type; down with the reason otherwise. `null` when the ticket was
 * revoked first, so a late answer publishes nothing.
 *
 * @param {(url: string, init?: RequestInit & {ms?: number}) => Promise<Response>} ask
 * @param {{signal: AbortSignal, current: () => boolean}} ticket
 * @param {{elapsed?: () => number, clock?: () => number}} [time] a monotonic
 *   clock and a wall clock, ms
 * @returns {Promise<ApiState | null>}
 */
export async function upCheck(ask, ticket, time = {}) {
  const elapsed = time.elapsed ?? (() => performance.now());
  const clock = time.clock ?? Date.now;
  const t0 = elapsed();
  const ms = () => Math.round(elapsed() - t0);
  try {
    const r = await ask(UP_ROUTE, { method: UP_METHOD, cache: 'no-store', signal: ticket.signal });
    if (!ticket.current()) return null;
    if (!r.ok) throw new Error(`HTTP ${r.status}`);
    const ct = r.headers.get('content-type') ?? '';
    if (!ct.includes('json')) {
      throw new Error(`answered ${ct || 'no content-type'}, not JSON — the API is not behind this route`);
    }
    return { state: 'up', ms: ms(), why: null, at: clock(), route: UP_ROUTE, method: UP_METHOD, build: buildOf(r.headers) };
  } catch (why) {
    if (!ticket.current()) return null;
    // A caught value is `unknown`: read `.message` defensively and fall back
    // to the value itself, as the layout's probe always did.
    return {
      state: 'down',
      ms: ms(),
      why: String(/** @type {{ message?: unknown }} */ (why)?.message ?? why),
      at: clock(),
      route: UP_ROUTE,
      method: UP_METHOD,
      build: null
    };
  }
}

/**
 * The first check, for a page that is not visible when it loads.
 * `watchVisible` runs its first poll only on a visible page; this runs the
 * one check the button is waiting on through its own single-flight request,
 * so stopping it revokes a late answer. On a visible page it does nothing —
 * `watchVisible` already ran.
 *
 * @param {import('./page-requests.js').Work} probe
 * @param {{visible: () => boolean, schedule?: (work: () => void, delay: number) => any,
 *   cancel?: (timer: any) => void}} options
 * @returns {() => void} stop
 */
export function firstCheck(probe, options) {
  const first = createPageRequests(options);
  if (!options.visible()) void first.run(probe);
  return () => first.dispose();
}

/**
 * The button's title. It names the route the state says was asked, and how:
 * a state that names neither is the probe before R3-15, a `GET /feeds.json`.
 *
 * @param {{state: 'checking' | 'up' | 'down', why: string | null, at: number,
 *   route?: string, method?: string}} api
 * @param {(ms: number) => string} stamp a wall-clock formatter
 */
export function apiTitle(api, stamp) {
  const route = api.route ?? '/feeds.json';
  const trip = api.method === UP_METHOD ? `Header-only round trip (${UP_METHOD}) to ${route}` : `Round trip to ${route}`;
  if (api.state === 'down') return `Probing ${route} failed: ${api.why}`;
  if (!api.at) return `${trip}, not answered yet. Click to check now.`;
  return `${trip}. Checked ${stamp(api.at)}. Click to re-check.`;
}

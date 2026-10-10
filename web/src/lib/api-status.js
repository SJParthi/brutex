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
 */
import { createPageRequests } from './page-requests.js';

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
 * The button's title.
 *
 * @param {{state: 'checking' | 'up' | 'down', why: string | null, at: number}} api
 * @param {(ms: number) => string} stamp a wall-clock formatter
 */
export function apiTitle(api, stamp) {
  if (api.state === 'down') return `Probing /feeds.json failed: ${api.why}`;
  if (!api.at) return 'Round trip to /feeds.json, not answered yet. Click to check now.';
  return `Round trip to /feeds.json. Checked ${stamp(api.at)}. Click to re-check.`;
}

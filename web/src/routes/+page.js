/**
 * THE ROOT IS THE TERMINAL, AND THIS IS THE ONE LINE THAT SAYS SO.
 *
 * # Why a redirect and not the page itself
 *
 * `/terminal` could have been moved here instead, and it deliberately was not.
 * It is a real URL that is already in the nav, already bookmarkable, already
 * named in `BARE` in `routes/+layout.svelte`, and already the subject of a
 * dozen comments in `$lib/terminal.svelte.js` that would all become lies about
 * a path that no longer existed. Moving a 1,200-line component to rename a
 * route is a large edit whose only product is churn.
 *
 * So the root points AT it. `/` and `/terminal` are one page with one
 * canonical address, and the address is the one every existing reference
 * already uses.
 *
 * # Why the Markets page moved rather than being displaced
 *
 * `routes/+page.svelte` held the Markets console — 4,235 lines of it — and a
 * redirect on this path would have made it unrenderable. That is exactly the
 * defect this tree keeps finding and this session already fixed once: *a
 * surface that exists and cannot be reached is the same as absent*. It moved
 * to `routes/markets/+page.svelte` with `git mv`, so its history follows it,
 * and the nav entry that used to read `/` now reads `/markets`. Nothing was
 * displaced; one page gained a path and another gave one up.
 *
 * # Why this runs in the browser and not at build time
 *
 * `routes/+layout.js` sets `ssr = false`, so no load function on any route
 * runs on a server — there is no server that runs JavaScript, and D-0053
 * forbids there ever being one. `adapter-static` is configured with
 * `fallback: 'index.html'`, and the build's own log says it overwrites
 * `build/index.html` with that fallback, so the root has always been served as
 * the generic shell rather than as prerendered output. The shell hydrates, this
 * load runs, and the client router is already the thing that puts every other
 * page on screen.
 *
 * `replaceState` is not available to a load redirect and is not wanted here:
 * the entry stays in history, so the browser's back button leaves the console
 * the way it always has rather than bouncing between `/` and `/terminal`.
 */

import { redirect } from '@sveltejs/kit';

/**
 * 307, NOT 301 OR 308.
 *
 * The permanent codes are cached by the browser, sometimes indefinitely and
 * across restarts, and this is a product decision rather than a fact about the
 * URL — the day the operator wants the console back on the root, a cached 308
 * would keep sending them to the terminal from a store no edit can reach. 307
 * is temporary and re-asked every time, which costs nothing on localhost and
 * keeps the choice reversible.
 */
export function load() {
  redirect(307, '/terminal');
}

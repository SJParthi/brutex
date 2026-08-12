import { sveltekit } from '@sveltejs/kit/vite';

/**
 * `/api` proxies to the Rust server in development so the browser talks to one
 * origin. In production the Rust binary serves both the built assets and the
 * JSON, so there is no proxy and no CORS.
 */

// THE PORT IS WRITTEN ONCE.
//
// It used to be typed into seven proxy entries, and it drifted: every entry
// said 8731 while the release binary listens on 8080. The result was that
// EVERY json route returned 500, so the feed picker read "feeds unavailable",
// /db read "Nothing stored for" against a store holding 11.5 million bars, and
// the whole front end looked like a design failure when it was a one-number
// configuration failure. Seven copies of a number is seven chances to be
// wrong; this is one.
//
// `BRUTEX_API_PORT` overrides it, so an operator running the debug binary on
// another port changes an environment variable rather than editing this file
// and risking the same drift again.
const API = `http://127.0.0.1:${process.env.BRUTEX_API_PORT ?? 8080}`;

// EVERY JSON ROUTE THE RUST SIDE SERVES. Listing them one by one is how
// `/feeds.json` got missed and the feed picker showed a bare 404 — loudly,
// which is right, but it should not have been possible. A prefix would be
// tidier; these routes are not under one.
const ROUTES = [
	'/instruments.json',
	'/feeds.json',
	'/bars.json',
	'/store.json',
	'/audit.json',
	// `/audit` IS NOT HERE, AND ITS ABSENCE IS THE POINT.
	//
	// It used to be, and it made one URL serve two different applications:
	// clicking "Audit" in the nav rendered `src/routes/audit/+page.svelte`
	// (SvelteKit routes in the browser and never consults this proxy), while a
	// reload, a bookmark or a typed address hit the proxy and got the
	// Rust-rendered page instead — different nav, no feed picker, no theme, and
	// two of its own links 404 in development. The console was unreachable by
	// every route except a click.
	//
	// The collision was real: both surfaces wanted the same path, and the
	// Svelte one had no other source of data. `/audit.json` is what removed
	// that — see `crates/api/src/audit_json.rs`. The page now owns `/audit`
	// here, the Rust page still answers `/audit` on the API's own port, and
	// nothing renders differently depending on how the operator arrived.
	'/pull',
	// THE AUTOPILOT. Four entries and NOT the `/autopilot` prefix, deliberately:
	// `/autopilot` is a PAGE this app renders, and proxying that prefix would
	// hand the page itself to Rust and the route would 404 in development only.
	// `/autopilot/` (with the slash) covers the three controls without covering
	// the page.
	//
	// `/autopilot/control` IS THE ONE THE PAGE PRESSES, and it was the entry that
	// was missing. It is the only control route that answers
	// `{action, accepted, why, status}` for BOTH words, and that `why` is the
	// sentence /autopilot prints on its receipt verbatim — including "started,
	// and it is NOT a full recovery". Without this entry the POST is answered by
	// the dev server's own HTML fallback with a 200; the page catches that on the
	// content type and prints it as a refusal, which is loud and still wrong. The
	// control has to reach Rust.
	'/autopilot.json',
	'/autopilot/pause',
	'/autopilot/resume',
	'/autopilot/control'
];

export default {
	plugins: [sveltekit()],
	server: {
		proxy: Object.fromEntries(ROUTES.map((route) => [route, API]))
	}
};

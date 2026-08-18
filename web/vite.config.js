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
	'/autopilot/control',
	// THE THIRD AND FOURTH TIME THIS LIST DRIFTED, found by diffing it against
	// what `web/src` actually calls. Both are real routes -- `server.rs:6844`
	// and `server.rs:6891` -- and both were missing here, so in `npm run dev`
	// they were answered by the dev server's own HTML fallback with a 200.
	//
	// A 200 is the worst possible answer: `r.ok` is true, the page proceeds,
	// and `r.json()` throws on `<!doctype html>` -- so `/ingest` reported a
	// JSON parse error for what was a routing gap, and the folder picker
	// reported the same. Neither of these two has a content-type guard, which
	// is exactly why they were the two that hurt.
	//
	// `web/tests/proxy.test.js` now diffs this list against every `fetch` in
	// `web/src` on every run, so a fifth drift fails a test rather than
	// producing a plausible error message about JSON.
	'/folder.json',
	'/ingest/status.json'
];

export default {
	plugins: [sveltekit()],
	// SOURCE MAPS ARE BUILT AND NOT COMMITTED, AND `hidden` IS WHY THAT WORKS.
	//
	// The bundle is minified and `web/build` is committed, so the artifact in
	// this repository is the thing that actually runs -- and debugging it meant
	// reading single-letter identifiers. Building maps fixes that for anyone
	// who runs `npm run build`.
	//
	// Committing them does not. Measured: 26 map files, 3.0 MB, against a 940
	// kB artifact -- 4.2x the tracked tree, rewritten in full on every rebuild,
	// in a repository that already commits its build output under D-0068. That
	// is a large permanent cost for a file only a developer opens, and a
	// developer can rebuild.
	//
	// So they are generated, gitignored, and `hidden` rather than `true`.
	// `true` appends a `//# sourceMappingURL=` comment; with the maps absent
	// from a fresh clone every chunk would request one, take a 404 from
	// `assets.rs`, and enter its missing-asset log -- turning a deliberate
	// omission into a recurring false alarm about a broken build. `hidden`
	// emits the same maps and omits the comment, so nothing asks for what is
	// not there.
	build: { sourcemap: 'hidden' },
	server: {
		proxy: Object.fromEntries(ROUTES.map((route) => [route, API]))
	}
};

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
	'/audit',
	'/pull'
];

export default {
	plugins: [sveltekit()],
	server: {
		proxy: Object.fromEntries(ROUTES.map((route) => [route, API]))
	}
};

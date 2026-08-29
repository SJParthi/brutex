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
	// THE SORTED/PAGED WINDOW OVER ONE INSTRUMENT'S BARS, and it shipped
	// missing from this list. `/db` calls it whenever the reader sorts by a
	// column the census cannot answer from its prefix sums, so the gap was not
	// an edge case — it was every sort on the page. Caught by
	// `tests/proxy.test.js`, which walks what `web/src` fetches and asserts a
	// ROUTES entry covers each: without it `npm run dev` answers the request
	// with the dev server's HTML fallback at 200 and the grid empties under a
	// pager still reporting twelve thousand pages. Production was fine — the
	// binary serves both — which is exactly how a dev-only gap survives review.
	'/bars/window.json',
	// THE INDEX-SYMBOL JOIN `/mapping` READS. Registered in
	// `crates/api/src/server.rs` and missing from this list, so the page works
	// against the binary and fails against `npm run dev` — the same dev-only
	// shape as the entry above it, arriving by the same route: a feature added
	// end to end without this file being the third place it had to land.
	// Two consecutive misses is what makes `tests/proxy.test.js` worth having;
	// it reports one gap per run, so a green suite after adding an entry is not
	// evidence the list is complete.
	'/indexmap.json',
	'/store.json',
	'/audit.json',
	// THE RESULTS LEDGER. Its page at `/backtest` is a SvelteKit route and is
	// deliberately NOT listed here, for the reason the `/audit` note below
	// gives at length: a path that appears in this list is answered by Rust in
	// development and by the client router on a click, which is one URL serving
	// two applications. `crates/api/src/server.rs` registers no `/backtest`
	// page at all, so here there is only the JSON.
	'/backtest.json',
	// THE RUN CONTROL'S TWO. `/backtest/run` starts a sweep and
	// `/backtest/run.json` is what the page polls while it goes. Listed with
	// the slash, so neither covers `/backtest` itself — that path is the
	// SvelteKit page and proxying it would hand it to Rust and 404 the route
	// in development only, which is the trap the `/audit` note above records.
	'/backtest/run',
	'/backtest/run.json',
	// THE RANKED COMBINATIONS OF A FINISHED SWEEP.
	//
	// `record_frontier` writes up to `rules.top` rows per rung into
	// `results/frontier.bin` — twenty-five per rung, two hundred for an
	// eight-rung press — and `/engine/top.json` reads them back with the
	// condition names already resolved. Both halves shipped and worked; nothing
	// in `web/src` ever fetched it, so twenty-four of every twenty-five results
	// were written correctly and never seen by anyone.
	//
	// Listed with the `.json` suffix rather than as `/engine`, for the same
	// reason `/backtest/run` is: `/engine/command` is a POST that no page calls
	// and proxying the whole prefix would claim it too.
	'/engine/top.json',
	// ONE RUN'S ROUND TRIPS. The per-trade table on the backtest page rendered
	// a padlock in every cell, because nothing wrote the file it reads and no
	// route served it. `cli::trades` writes it now and `/trades.json` serves it,
	// keyed on the identity `/backtest.json` already prints on every row.
	'/trades.json',
	// THE EVENT FEED, WHICH IS THE ONLY LIVE PROGRESS THAT EXISTS.
	//
	// `GET /backtest/run.json` reads a struct written exactly twice — once when
	// a sweep is accepted and once when it returns — so a multi-hour eight-rung
	// run is a boolean to the page. MEASURED: two overnight runs held thirteen
	// cores for seven hours each and the operator could not tell a working sweep
	// from a hung one.
	//
	// The events carry what the struct does not: a span load per rung with its
	// bar count and derived `min_hits`, and now a `rung finished` per rung with
	// its outcome and, on a refusal, the reason. `/logs.json` has served them
	// all along and nothing in `web/src` fetched it — the Rust-rendered `/logs`
	// page linked to it and the console never did.
	'/logs.json',
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
	'/ingest/status.json',
	// THE FIFTH DRIFT, and the test above called its shot: it said a fifth
	// would fail a test rather than produce a plausible error about JSON, and
	// this is it. `/mapping` reads `/masters/status.json` to learn which
	// masters are on disk, and the route was never listed here.
	//
	// `/masters/` WITH THE TRAILING SLASH, for the reason the autopilot block
	// gives. `/masters` on its own is a PAGE, rendered by Rust at
	// `server.rs:12047` -- but the slash form is what the two data routes this
	// app actually calls both start with (`/masters/status.json`, and
	// `/masters/refresh` at `+page.svelte:120`), so one entry covers both
	// without this list making a claim about the page.
	//
	// It failed quietly rather than loudly: `readMasters` swallows the throw
	// on purpose -- a status read is not that page's subject -- so in
	// `npm run dev` the HTML fallback came back 200, `r.json()` threw,
	// `onDisk` stayed `[]`, and the join simply showed nothing on disk. No
	// banner, no console error, a page that looked answered.
	'/masters/',
	// AND THE SIXTH, FOUND IN THE SAME PASS. `/universe/resolve` is registered
	// in `server.rs` and called from `web/src`, and was missing here too.
	//
	// Both of these were found by diffing the WHOLE of what `web/src` fetches
	// against this list in one go, rather than by re-running the test and
	// taking its next single complaint. The test reports one gap per run and
	// says so in its own comment; that makes a green suite after one addition
	// evidence of nothing, and it is how this list reached six drifts. The
	// diff is ten fetched paths against eighteen entries -- it is cheap, and
	// it is the only form of the check that terminates.
	'/universe/resolve',
	// DRIFTS SEVEN, EIGHT AND NINE — and the reason all three sat here unseen
	// through a green suite is now fixed in `web/tests/proxy.test.js` itself.
	//
	// That scanner matched `fetch(` and `ask(`. Two files import the helper
	// under a local alias — `ask as request` in `ingest/+page.svelte`, `ask as
	// ask_` in `backtest/+page.svelte` — which hid 15 of the tree's 28 call
	// sites. The suite read 145/145 while these three were each called, each
	// served by the binary, and each absent from this list. A checker that
	// cannot see half the call sites reports the list clean by construction.
	//
	// `/calendar.json` is the expensive one. `/ingest` reads it for the NSE
	// holiday table; under `npm run dev` the HTML fallback answers 200, `.ok`
	// is true, `.json()` throws, the `owed` map stays empty, and
	// `holidaysKnownFor()` is false for every day — so `isSession()` falls back
	// to "weekday = session". Eight Saturdays that traded and every holiday are
	// then wrong, and the page's whole shortfall arithmetic is computed against
	// a calendar nobody supplied. No banner: the catch writes `why` and the
	// numbers carry on.
	//
	// `/vocab.json` — `/backtest` renders stored masks as raw 64-bit words
	// instead of condition names. `/universes.json` — `/backtest` fails
	// SILENTLY (`if (!response.ok) { sweptSurface = null; return; }` and a bare
	// `catch {}`), so the swept-target sentence just vanishes.
	'/calendar.json',
	'/vocab.json',
	'/universes.json'
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

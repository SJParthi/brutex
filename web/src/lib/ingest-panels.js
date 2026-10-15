/**
 * THE /ingest PANEL REGISTRY (R4-01).
 *
 * Every later /ingest panel — the GDFL import (IA-12a,
 * `panels/GdflImport.svelte`), the storage band (SC-8, `panels/Storage.svelte`),
 * the Zerodha repair (`panels/ZerodhaRepair.svelte`) — is a file in
 * `web/src/routes/ingest/panels/`. The page mounts every module
 * `import.meta.glob('./panels/*.svelte', { eager: true })` finds, ordered by
 * the module's exported `order` (ties by file name), so a new panel is a new
 * file and never an edit of the 13,000-line page.
 *
 * # THE CONTRACT A PANEL KEEPS
 *
 *   `<script module>` exports `order` (a finite number) and `title` (a
 *   non-empty string); the default export is the component. The component
 *   receives one prop, `ctx` (`{ feed, display, archive }`, the selected
 *   feed), owns its own fetches after mount, answers a 404 or a 503 with one
 *   named line (`answerLine`), titles every control, fits 375 px, and polls at
 *   most once a second and never while the page is hidden (`createPoller`).
 *
 * # A PANEL THAT BREAKS THE CONTRACT IS NAMED, NEVER DROPPED
 *
 * One without `order` or `title` is still mounted — last, in file order — and
 * named in one visible line and on the console. Dropping it would hide the
 * very panel someone just added, which is the silent fallback `CLAUDE.md` §4
 * bans. A module with no component cannot be mounted at all; it is named the
 * same way.
 */

/**
 * @typedef {{
 *   file: string,
 *   order: number,
 *   title: string,
 *   component: any,
 *   problems: string[]
 * }} Panel
 */

/** The file name of a glob key: `./panels/ZerodhaRepair.svelte` -> `ZerodhaRepair.svelte`. @param {string} path */
export const panelFile = (path) => String(path).split('/').pop() ?? String(path);

/**
 * THE PANELS, ORDERED, AND EVERY BREACH OF THE CONTRACT NAMED.
 *
 * @param {Record<string, any>} modules what `import.meta.glob(…, { eager: true })` returned
 * @returns {{ panels: Panel[], refused: { file: string, why: string }[] }}
 *   `panels` are every module with a component, valid ones first by
 *   (`order`, file) and then the ones missing `order` or `title` by file;
 *   `refused` names each breach once, in file order.
 */
export function panelRegistry(modules) {
  /** @type {Panel[]} */
  const ok = [];
  /** @type {Panel[]} */
  const flawed = [];
  /** @type {{ file: string, why: string }[]} */
  const refused = [];
  const entries = Object.entries(modules ?? {}).sort(([a], [b]) => (panelFile(a) < panelFile(b) ? -1 : panelFile(a) > panelFile(b) ? 1 : 0));
  for (const [path, mod] of entries) {
    const file = panelFile(path);
    /** @type {string[]} */
    const problems = [];
    const order = mod?.order;
    const title = mod?.title;
    if (typeof order !== 'number' || !Number.isFinite(order)) problems.push('exports no finite `order`');
    if (typeof title !== 'string' || title.trim() === '') problems.push('exports no `title`');
    const component = mod?.default;
    if (typeof component !== 'function') {
      problems.push('exports no component, so it cannot be mounted');
      refused.push({ file, why: problems.join('; ') });
      continue;
    }
    const panel = {
      file,
      order: problems.length === 0 ? /** @type {number} */ (order) : Number.POSITIVE_INFINITY,
      title: typeof title === 'string' && title.trim() !== '' ? title.trim() : file,
      component,
      problems
    };
    if (problems.length === 0) ok.push(panel);
    else {
      flawed.push(panel);
      refused.push({ file, why: problems.join('; ') });
    }
  }
  ok.sort((a, b) => a.order - b.order || (a.file < b.file ? -1 : a.file > b.file ? 1 : 0));
  return { panels: [...ok, ...flawed], refused };
}

/**
 * The ONE visible line naming every panel that breaks the contract, or `null`.
 * @param {{ file: string, why: string }[]} refused
 */
export function refusedLine(refused) {
  if (!Array.isArray(refused) || refused.length === 0) return null;
  const each = refused.map((r) => `${r.file} ${r.why}`).join('; ');
  return `${refused.length} /ingest panel(s) break the panel contract (web/src/lib/ingest-panels.js): ${each}. A panel with a component is still mounted, last; fix the export, not the page.`;
}

/**
 * ONE NAMED LINE FOR AN ANSWER THAT IS NOT A 200 — the words every panel uses
 * for a route this build does not serve (404) or cannot serve now (503).
 *
 * @param {string} route e.g. `GET /pull/repair.json`
 * @param {number} status the HTTP status, 0 when the request never answered
 * @param {string} [reason] the server's own words, verbatim, when it sent any
 */
export function answerLine(route, status, reason = '') {
  const said = typeof reason === 'string' && reason.trim() !== '' ? `: ${reason.trim()}` : '';
  if (status === 404) return `${route} is not served by this build (HTTP 404)${said}. The panel waits for a server that has it.`;
  if (status === 503) return `${route} cannot answer now (HTTP 503)${said}. Nothing here is a count until it does.`;
  if (status === 0) return `${route} did not answer${said}.`;
  return `${route} answered HTTP ${status}${said}.`;
}

/**
 * THE LINE A PANEL THAT FAILED IS SHOWN AS, in place of the panel.
 *
 * A panel that throws while it mounts or while it updates is caught by the
 * page's `<svelte:boundary>` around it (`$lib/IngestPanelSlot.svelte`): it is
 * replaced by this one visible line naming its file and its error, and every
 * other panel and the rest of /ingest keep working. Without the boundary one
 * panel's throw blanked the whole page with no word (R4-01 repair round 1:
 * `document.body.innerText` was empty at 6 s and 15 s).
 *
 * @param {string} file the panel's file name
 * @param {unknown} error what it threw
 */
export function panelFailedLine(file, error) {
  const what = error instanceof Error ? error.message : String(error);
  const said = what.trim() === '' ? 'an error with no message' : what.trim();
  return `/ingest panel ${file} failed and is not shown: ${said}. Every other panel and the rest of /ingest keep working; the browser console holds the stack.`;
}

/**
 * Name a failed panel on the console (with the error, so its stack is kept)
 * and answer its visible line. `log` is the console's by default.
 *
 * @param {string} file
 * @param {unknown} error
 * @param {(...a: unknown[]) => void} [log]
 */
export function reportPanelFailure(file, error, log = console.error) {
  const line = panelFailedLine(file, error);
  log(line, error);
  return line;
}

/** The fastest a panel may poll, in milliseconds: once a second. */
export const MIN_POLL_MS = 1000;

/**
 * A POLL THAT NEVER RUNS FASTER THAN ONCE A SECOND AND NEVER WHILE HIDDEN.
 *
 * `run` is awaited before the next one is scheduled, so a slow answer is never
 * stacked. While `hidden()` answers true the poll waits without asking;
 * `wake()` (the page's `visibilitychange`) runs one at once when shown again.
 *
 * A RUN THAT THROWS DOES NOT END THE POLL. It is caught, kept as `failed`
 * (`{ message, at }`, until a later run succeeds), counted in `failures`,
 * handed to `onError`, and the next run is scheduled as usual. Before this a
 * throw escaped the timer as an unhandled rejection and the poll stopped for
 * good with no word (R4-01 repair round 1).
 *
 * @param {{
 *   every: number,
 *   run: () => unknown,
 *   hidden?: () => boolean,
 *   setTimer?: (fn: () => void, ms: number) => unknown,
 *   clearTimer?: (id: any) => void,
 *   now?: () => number,
 *   onError?: (error: unknown) => void
 * }} o
 */
export function createPoller({ every, run, hidden = () => false, setTimer = setTimeout, clearTimer = clearTimeout, now = () => Date.now(), onError = (e) => console.error('a panel poll failed', e) }) {
  const ms = Math.max(MIN_POLL_MS, Number.isFinite(every) ? every : MIN_POLL_MS);
  /** @type {any} */
  let timer = null;
  let stopped = true;
  let busy = false;
  let asked = 0;
  let last = Number.NEGATIVE_INFINITY;
  let failures = 0;
  /** @type {{ message: string, at: number } | null} */
  let failed = null;
  const schedule = () => {
    if (stopped) return;
    timer = setTimer(tick, ms);
  };
  const tick = async () => {
    timer = null;
    if (stopped) return;
    if (hidden() || busy) return schedule();
    busy = true;
    asked += 1;
    last = now();
    try {
      await run();
      failed = null;
    } catch (why) {
      failures += 1;
      failed = { message: why instanceof Error ? why.message : String(why), at: now() };
      try {
        onError(why);
      } catch {
        // The reporter itself failing must not end the poll either; `failed`
        // still holds the run's own error.
      }
    } finally {
      busy = false;
    }
    schedule();
  };
  return {
    /** The interval this poll actually keeps, after the clamp. */
    ms,
    /** How many times `run` was called. */
    get asked() { return asked; },
    /** How many runs threw. */
    get failures() { return failures; },
    /** The last run's error and when, or null once a run succeeds again. */
    get failed() { return failed; },
    start() {
      if (!stopped) return;
      stopped = false;
      schedule();
    },
    stop() {
      stopped = true;
      if (timer !== null) clearTimer(timer);
      timer = null;
    },
    /** Shown again: ask now rather than wait out the interval — but never
     *  sooner than `ms` after the last ask, however often it is woken. */
    wake() {
      if (stopped || hidden() || busy || now() - last < ms) return;
      if (timer !== null) clearTimer(timer);
      void tick();
    }
  };
}

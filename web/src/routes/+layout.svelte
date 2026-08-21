<script>
  /**
   * THE APP SHELL — brand, nav, feed, theme, and the answer to "is anything
   * happening".
   *
   * Everything in here is cross-cutting: it is the same on every route, so it
   * lives above every route. Three things earn their place in a 46-pixel bar.
   *
   * **The feed is a SELECTION, never a comparison.** One feed is chosen here
   * and every page shows that one. No surface in this product puts one feed's
   * numbers beside another's; the picker names the alternatives, which is a
   * different thing from rendering them.
   *
   * **The feed picker is built from `/feeds.json`.** Nothing here names a
   * vendor, and a fifth broker appears the day its descriptor row exists. It
   * is a listbox rather than a `<select>` because an option has to carry the
   * server's REASON when a feed cannot be used — a native option renders one
   * string, so "unavailable" arrived without the why, and a failure named
   * halfway is the shape `CLAUDE.md` section 4 bans.
   *
   * **The status strip exists because a 32-minute pull looked exactly like a
   * hang.** It answers two questions continuously: can this page reach the API
   * at all, and is a pull in flight right now. Both are MEASURED — the API
   * figure is a real request with its real round-trip, and the pull indicator
   * is driven by the actual in-flight `fetch`, not by a timer that would keep
   * animating over a dead request.
   */
  import '$lib/theme.css';
  import { page } from '$app/state';
  import { feeds, loadFeeds } from '$lib/feeds.svelte.js';
  import { loadCatalogue } from '$lib/index.svelte.js';
  // THE ZONE IS NAMED, BECAUSE THE MACHINE'S ZONE IS NOT THE PRODUCT'S.
  //
  // A bare `toLocaleTimeString()` printed `15:29:04` in whatever zone the host
  // happens to sit in, and named neither the zone nor the day -- so a stamp
  // from yesterday's session was indistinguishable from one taken a minute
  // ago, which is the single fact this line exists to report. `/db` diagnosed
  // this and fixed itself; `stampLabel` is what it fixed itself with, and this
  // is the third and last spelling of the rule joining the other two.
  import { stampLabel } from '$lib/dates.js';
  // A REQUEST THAT CANNOT END IS A SPINNER THAT LIES. `ask` is `fetch` with a
  // ceiling; see `$lib/ask.js` for why the wrapper exists rather than a signal
  // threaded through every call site.
  import { ask } from '$lib/ask.js';

  /** ONE DEFINITION OF A FEED, AND IT IS NOT THIS FILE'S.
   *
   * `$lib/feeds.svelte.js` owns the shape because it owns the fetch — the
   * annotation there was written against a live `/feeds.json` body. Restating
   * the fields here would put a second copy of the server's contract in the
   * tree, and two copies of one shape drift the day a field is added: the
   * fetcher would learn the new field and the picker would keep insisting the
   * old set is all there is. Imported, so there is exactly one place to change.
   *
   * @typedef {import('$lib/feeds.svelte.js').Feed} Feed
   */

  let { children } = $props();

  // The feed list loads once. The INSTRUMENT list reloads whenever the feed
  // changes, because the two brokers do not list the same instruments —
  // GIFTNIFTY is Dhan-only — and a page showing the merge offers symbols the
  // selected feed cannot be asked for.
  let feedsTried = $state(false);
  $effect(() => {
    loadFeeds().finally(() => (feedsTried = true));
  });
  $effect(() => {
    if (feeds.active) loadCatalogue(feeds.active);
  });

  const NAV = [
    { href: '/', label: 'Markets' },
    { href: '/ingest', label: 'Ingest' },
    { href: '/autopilot', label: 'Autopilot' },
    { href: '/db', label: 'DB' },
    { href: '/audit', label: 'Audit' },
    // THE LOG, REACHABLE BY CLICKING RATHER THAN BY KNOWING THE URL.
    //
    // `crates/api/src/logs.rs` is a complete bounded reader -- 1,717 lines,
    // with its own `/logs.json` beside it -- written because "the first
    // question an operator asks after a failed pull (what did it say?) had no
    // answer inside the application that wrote it". `render.rs`'s own nav
    // gained a Logs entry for exactly that reason, in those words.
    //
    // THIS nav never did. Measured: `/logs` appeared in all of `web/src`
    // exactly once, inside the <noscript> block of `app.html` -- so with script
    // ON, which is every real session, the page was unreachable by clicking. A
    // surface that exists and cannot be reached is the same as absent.
    //
    // `reload` IS LOAD-BEARING AND NOT A PREFERENCE. There is no
    // `src/routes/logs/`, so the client router has nothing to resolve and the
    // browser has to leave the app for axum to answer. Adding a Svelte route
    // here instead would put a SECOND application on this path -- the defect
    // `vite.config.js` records against `/audit`, where a click renders one page
    // and a reload renders another with different nav, no feed picker and no
    // theme. One `/logs`, and it is the server's.
    {
      href: '/logs',
      label: 'Logs',
      reload: true,
      why: 'The rolling event log — every event a run wrote, newest first. This one leaves the console: it is rendered by the API itself, carries its own navigation, and the browser back button is the way back.'
    }
  ];

  // `/db/anything` still lights DB. An exact match alone leaves the operator
  // on a sub-route with no nav item lit and no idea where they are.
  /** @param {string} href */
  function current(href) {
    const p = page.url.pathname;
    return p === href || (href !== '/' && p.startsWith(href + '/'));
  }

  /* ====================================================================
     WHICH ROUTES CARRY THEIR OWN FEED CONTROL
     --------------------------------------------------------------------
     EXACTLY ONE CONTROL FOR ONE VALUE, ON EVERY ROUTE. That is the rule,
     and it has two failure modes, not one. Two controls writing the same
     `feeds.active` teach the reader they are independent when they are
     not — that is the one this bar was protecting against. ZERO controls
     is the other, and it is worse: a page that shows one feed's answer
     and offers no way to change it is a dead end, and the operator's only
     move is the browser's back button.

     `/ingest` and `/db` each carry the feed as the FIRST control of their
     own control strip, because on both pages the feed is not chrome — it
     is the first rung of the cascade, and everything below it is literally
     that feed's answer. Drawing the picker up here as well would be the
     two-control defect, which is what stood on `/db` until the feed rung
     landed there: a `.scopeline` READOUT in the strip and the only real
     control in this bar, two rows apart, both naming one value.

     `/db` DRAWS ITS RUNG IN EVERY STATE, not only the one with rows —
     the failed read, the unchosen feed, the loading store and the empty
     store all render the same snippet. That is the condition of joining
     this list: a page whose control disappears with its data would hand
     back the dead end this comment is about, at exactly the moment the
     operator needs another feed.

     SO THE BAR YIELDS, ROUTE BY ROUTE, RATHER THAN GLOBALLY. Removing it
     outright would leave `/`, `/autopilot` and `/audit` with no way to
     change the feed at all — none of the three has ever had its own. A
     route joins this list on the day it grows a control of its own, and
     the count stays at one in both directions.

     Prefix-matched with `current()` so `/ingest/anything` and `/db/anything`
     are covered by the same entries, for the same reason the nav is. ==== */
  const FEED_OWNED = ['/ingest', '/db'];
  const feedInBar = $derived(!FEED_OWNED.some(current));

  /* ====================================================================
     THEME
     --------------------------------------------------------------------
     Three states, and the third one is the point: `auto` REMOVES the
     attribute rather than writing a value, which is what hands the decision
     back to `prefers-color-scheme`. Writing "dark" for auto would look
     identical on a dark machine and be wrong on a light one, silently.
     ==================================================================== */
  const THEME_KEY = 'brutex.theme';
  const MODES = [
    { id: 'auto', label: 'Match the system theme' },
    { id: 'light', label: 'Force the light theme' },
    { id: 'dark', label: 'Force the dark theme' }
  ];

  let mode = $state(readMode());
  let systemDark = $state(true);

  function readMode() {
    try {
      const v = localStorage.getItem(THEME_KEY);
      return v === 'dark' || v === 'light' || v === 'auto' ? v : 'auto';
    } catch {
      // Private browsing, or storage disabled. Not an error worth a banner —
      // the theme simply does not persist, and everything else still works.
      return 'auto';
    }
  }

  $effect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    systemDark = mq.matches;
    /** @param {MediaQueryListEvent} e */
    const onChange = (e) => (systemDark = e.matches);
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  });

  $effect(() => {
    const el = document.documentElement;
    if (mode === 'auto') el.removeAttribute('data-theme');
    else el.setAttribute('data-theme', mode);
    el.setAttribute('data-theme-mode', mode);
    try {
      localStorage.setItem(THEME_KEY, mode);
    } catch {
      /* see readMode */
    }
  });

  const resolved = $derived(mode === 'auto' ? (systemDark ? 'dark' : 'light') : mode);

  /* ====================================================================
     THE FEED LISTBOX
     ==================================================================== */
  let open = $state(false);
  let hi = $state(-1);

  /* THE THREE `bind:this` HOLDERS, TYPED AT THE DECLARATION AND NOT AT THE USE.
   *
   * `$state(null)` on its own infers the type `null` and nothing else, so the
   * binding that later writes the real element is the error — the checker
   * reports "HTMLDivElement is not assignable to null" down in the markup,
   * pointing at the one line that is correct. `| null` is not decoration
   * either: null is the value these genuinely hold before the header mounts,
   * and it is what Svelte writes back when the conditionally-rendered combo is
   * torn down on a route that carries its own feed control. Every read below
   * already guards with `?.` for exactly that reason.
   *
   * `optEls` is sparse by construction: the array is filled by `bind:this` in
   * an `{#each}`, so an index exists only once its option has mounted, which
   * is why the scroll calls reach it through `?.` too.
   */
  /** @type {HTMLDivElement | null} */
  let comboEl = $state(null);
  /** @type {HTMLDivElement | null} */
  let triggerEl = $state(null);
  /** @type {Array<HTMLDivElement | null>} */
  let optEls = $state([]);

  const options = $derived(feeds.all ?? []);
  const active = $derived(options.find((f) => f.wire === feeds.active) ?? null);
  const activeIndex = $derived(options.findIndex((f) => f.wire === feeds.active));

  /* `Feed | undefined` AND NOT `Feed`, because the undefined case is the whole
     point of the `?.`: `commit()` hands this whatever sits at an index that a
     keyboard or a pointer named, and the list can have been refetched shorter
     since. A feed that is not there is not usable, which is the same answer as
     a feed the server marked unready. */
  /** @param {Feed | undefined} f */
  function usable(f) {
    return f?.ready === true;
  }

  function openList() {
    if (!options.length) return;
    open = true;
    hi = activeIndex >= 0 ? activeIndex : firstUsable();
  }

  function close({ refocus = true } = {}) {
    open = false;
    hi = -1;
    if (refocus) triggerEl?.focus();
  }

  function firstUsable() {
    const i = options.findIndex(usable);
    return i >= 0 ? i : 0;
  }

  /**
   * Move the highlight, skipping feeds the server says cannot be used.
   *
   * The parameter is a step OR an end of the list, and the union says so
   * rather than smuggling the ends in as sentinel numbers: `'first'` and
   * `'last'` seed the cursor one place outside the array and then walk inward
   * by `dir`, which is what makes a disabled first or last feed skip correctly
   * instead of landing on it.
   *
   * @param {1 | -1 | 'first' | 'last'} step
   */
  function move(step) {
    if (!options.length) return;
    let next;
    if (step === 'first') next = -1;
    else if (step === 'last') next = options.length;
    else next = hi;

    const dir = step === 'first' ? 1 : step === 'last' ? -1 : step;
    for (let n = 0; n < options.length; n += 1) {
      next = (next + dir + options.length) % options.length;
      if (usable(options[next])) {
        hi = next;
        optEls[next]?.scrollIntoView({ block: 'nearest' });
        return;
      }
    }
    // Every feed is unusable. Highlight nothing rather than pretend.
    hi = -1;
  }

  // Type-to-select, the one behaviour a custom listbox usually drops and the
  // one an operator with four brokers actually uses.
  let typed = '';
  let typedAt = 0;
  /** @param {string} ch a single printable character, as `onTriggerKey` filters it */
  function typeahead(ch) {
    const now = Date.now();
    typed = now - typedAt > 800 ? ch.toLowerCase() : typed + ch.toLowerCase();
    typedAt = now;
    const i = options.findIndex((f) => usable(f) && f.display.toLowerCase().startsWith(typed));
    if (i >= 0) {
      hi = i;
      optEls[i]?.scrollIntoView({ block: 'nearest' });
    }
  }

  /** @param {number} i an index into `options`, or `-1` when nothing is highlighted */
  function commit(i) {
    const f = options[i];
    if (!f || !usable(f)) return;
    feeds.active = f.wire;
    close();
  }

  /** @param {KeyboardEvent} e */
  function onTriggerKey(e) {
    const k = e.key;
    if (!open) {
      if (k === 'ArrowDown' || k === 'ArrowUp' || k === 'Enter' || k === ' ') {
        e.preventDefault();
        openList();
      } else if (k === 'Home' || k === 'End') {
        e.preventDefault();
        openList();
        move(k === 'Home' ? 'first' : 'last');
      }
      return;
    }
    switch (k) {
      case 'Escape':
        e.preventDefault();
        close();
        break;
      case 'Enter':
      case ' ':
        e.preventDefault();
        commit(hi);
        break;
      case 'ArrowDown':
        e.preventDefault();
        move(1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        move(-1);
        break;
      case 'Home':
        e.preventDefault();
        move('first');
        break;
      case 'End':
        e.preventDefault();
        move('last');
        break;
      case 'Tab':
        close({ refocus: false });
        break;
      default:
        if (k.length === 1 && /\S/.test(k)) {
          e.preventDefault();
          typeahead(k);
        }
    }
  }

  $effect(() => {
    if (!open) return;
    /* `e.target` is an `EventTarget`, which is the wider thing: a window, a
       `fetch` XHR or a media element are all event targets and none of them is
       in the document, so `contains()` will not take one. The narrowing is a
       cast rather than an `instanceof` guard because a pointer event on this
       page always originates at a node, and turning "not a node" into a fourth
       branch would invent a case to handle that the platform cannot deliver
       here. A node outside the combo and a node inside it are the only two
       outcomes, which is exactly what the line already says. */
    /** @param {PointerEvent} e */
    const away = (e) => {
      if (comboEl && !comboEl.contains(/** @type {Node | null} */ (e.target))) {
        close({ refocus: false });
      }
    };
    window.addEventListener('pointerdown', away, true);
    return () => window.removeEventListener('pointerdown', away, true);
  });

  /* AN UNMOUNTED MENU IS NOT A CLOSED ONE. `open` is state, and the combo it
     belongs to is now conditionally rendered: leaving a route with the list
     down takes the markup away and leaves the flag standing, so the next route
     that DOES draw the bar's picker would mount it already open, under a
     pointer that never asked for it. Closed on the way out, where the leaving
     happens, rather than repaired on the way back in.

     `refocus: false` — there is nothing left to focus, and calling `focus()` on
     a detached trigger would move the caret to the body mid-navigation. */
  $effect(() => {
    if (!feedInBar && open) close({ refocus: false });
  });

  function retryFeeds() {
    feeds.error = null;
    feedsTried = false;
    loadFeeds(true).finally(() => (feedsTried = true));
  }

  /* ====================================================================
     THE STATUS STRIP — reachability
     --------------------------------------------------------------------
     Probed against `/feeds.json` and NOT against `/health`, for a reason
     worth writing down: in development the Vite proxy forwards a fixed list
     of routes and `/health` is not on it, so the request never reaches Rust.
     A probe that treats "the dev server answered instead" as "the API is up"
     is a green light wired to nothing.
     So the probe also checks the CONTENT TYPE. A static-fallback HTML page
     with a 200 is precisely the failure this indicator exists to catch, and
     `r.ok` alone cannot see it.
     ==================================================================== */
  const PROBE_MS = 8000;

  /* THREE STATES, NAMED, AND `why` IS A STRING THAT HAPPENS TO START NULL.
   *
   * Inferred from the initialiser alone this reads as `state: string, why:
   * null`, which gets both facts backwards: the state is not any string — a
   * typo'd `'ok'` would satisfy `string` and light nothing, because every
   * branch in the markup tests against these three exact words — and `why` is
   * null only until the first failure, so the assignment that finally carries
   * the reason is what the checker rejected.
   *
   * `at` is a wall-clock stamp and `ms` a measured round trip; they are not
   * interchangeable and are kept apart for that reason. `at` is 0 before the
   * first probe returns, and nothing renders it while the state is `checking`.
   */
  /** @type {{ state: 'checking' | 'up' | 'down', ms: number, why: string | null, at: number }} */
  let api = $state({ state: 'checking', ms: 0, why: null, at: 0 });

  async function probe() {
    const t0 = performance.now();
    try {
      const r = await ask('/feeds.json', { cache: 'no-store' });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const ct = r.headers.get('content-type') ?? '';
      if (!ct.includes('json')) {
        throw new Error(`answered ${ct || 'no content-type'}, not JSON — the API is not behind this route`);
      }
      await r.json();
      api = { state: 'up', ms: Math.round(performance.now() - t0), why: null, at: Date.now() };
    } catch (why) {
      /* A CAUGHT VALUE IS `unknown`, AND THAT IS THE TRUTH ABOUT `throw`.
         Anything can be thrown, so nothing about `.message` is guaranteed —
         which is precisely why this line reads it defensively and falls back
         to the value itself. The cast states the shape the read assumes and
         leaves the runtime expression untouched: a `TypeError` from a refused
         connection has a message, a `DOMException` from an aborted fetch has
         one too, and a bare string thrown by anything else has none and is
         printed whole. Narrowing with `instanceof Error` instead would be a
         behaviour change — `DOMException` does not inherit from `Error` in the
         browser, so the abort case would lose its message and print
         `[object DOMException]`. */
      api = {
        state: 'down',
        ms: Math.round(performance.now() - t0),
        why: String(/** @type {{ message?: unknown }} */ (why)?.message ?? why),
        at: Date.now()
      };
    }
  }

  $effect(() => {
    probe();
    const id = setInterval(probe, PROBE_MS);
    const wake = () => document.visibilityState === 'visible' && probe();
    document.addEventListener('visibilitychange', wake);
    return () => {
      clearInterval(id);
      document.removeEventListener('visibilitychange', wake);
    };
  });

  /* ====================================================================
     THE STATUS STRIP — is a pull running
     --------------------------------------------------------------------
     A pull is ONE SYNCHRONOUS POST. There is no server-side job to ask about
     and inventing a `/pull/status` endpoint here would be inventing a fact
     (`CLAUDE.md` section 3 rule 1). What is genuinely knowable in the browser
     is whether that POST is still open, so that is what is reported — one
     `fetch` wrapper, installed once, counting requests under `/pull`.
     The wrapper is a pass-through: it changes no argument, returns the same
     Response and rethrows the same error. The ingest page does not know it is
     there and does not have to.
     ==================================================================== */
  const PULL_PATH = /^\/pull(\/|$)/;

  /** THE EVENT IS TWO EVENTS, AND THE DIFFERENCE IS NOT DECORATION.
   *
   * A start says a request left; an end says what came back. They are written
   * as separate types joined by `kind` rather than as one type with five
   * optional fields, because the strip reads `ms`, `ok` and `status` off the
   * finished one WITHOUT a guard — `Last pull HTTP {status}` and
   * `{clock(lastPull.ms)}` have no branch for a missing value and should not
   * grow one. One loose type with `ms?: number` would push that impossible
   * case into the markup and make the honest rendering of it somebody's job.
   * Discriminating on `kind` gets the same narrowing the runtime already does
   * one line into the hook.
   *
   * `why` is present only on a failed end: a rejected `fetch` never produced a
   * response, so `status` is 0 and the reason is all there is to show.
   *
   * @typedef {{ kind: 'start', id: number, path: string, started: number }} PullStart
   * @typedef {{ kind: 'end', id: number, path: string, started: number,
   *             ms: number, ok: boolean, status: number, why?: string }} PullEnd
   * @typedef {PullStart | PullEnd} PullEvent
   */

  /** THE HOOK LIVES ON `window` BECAUSE THE WRAPPER OUTLIVES THE COMPONENT.
   *
   * `window.fetch` is replaced ONCE per page and the flag below is what keeps
   * it that way across hot reloads; the closure it captures therefore cannot
   * hold this component's state directly, or a reload would leave the live
   * wrapper reporting into a dead instance. So the wrapper reports through a
   * property that the current instance re-points, and both halves of that
   * arrangement are declared here rather than in an ambient global: nothing
   * else in the product reads or writes either name — checked — so widening
   * `Window` for the whole app would advertise a contract that exists in one
   * file and invite a second writer to it.
   *
   * @typedef {Window & typeof globalThis & {
   *   __brutexPull?: ((evt: PullEvent) => void) | null,
   *   __brutexPullWrapped?: boolean
   * }} PullWindow
   */

  /** @type {PullStart[]} */
  let inflight = $state([]);
  /** @type {PullEnd | null} */
  let lastPull = $state(null);
  let now = $state(Date.now());
  let seq = 0;

  $effect(() => {
    // The same `window`, named once with the two properties this file owns on
    // it, so the wrapper below and the teardown agree about what they are.
    const w = /** @type {PullWindow} */ (window);
    // Re-pointing a hook rather than re-wrapping keeps hot reload from
    // stacking wrappers, and keeps THIS instance's state the one that updates.
    w.__brutexPull = (evt) => {
      if (evt.kind === 'start') inflight = [...inflight, evt];
      else {
        inflight = inflight.filter((p) => p.id !== evt.id);
        lastPull = evt;
      }
    };
    if (!w.__brutexPullWrapped) {
      w.__brutexPullWrapped = true;
      const real = w.fetch.bind(window);
      w.fetch = async (input, init) => {
        let path = '';
        /* `fetch` TAKES THREE THINGS, AND ONLY TWO OF THEM CARRY `.url`: a
           string is the URL, a `Request` exposes it as `.url`, and a bare
           `URL` object exposes it as `.href` and has no `.url` at all. The
           third has never been tracked here — reading `.url` off it yielded
           undefined, the path matched no pull, and the request went straight
           through — and naming the case keeps that exactly rather than
           quietly starting to track it. Making a `URL` argument trackable is
           a behaviour change, and no call site in this app passes one. */
        if (!(input instanceof URL)) {
          try {
            path = new URL(typeof input === 'string' ? input : input.url, location.href).pathname;
          } catch {
            /* a Request built from something exotic — not a pull, then */
          }
        }
        if (!PULL_PATH.test(path)) return real(input, init);
        const id = (seq += 1);
        const started = Date.now();
        w.__brutexPull?.({ kind: 'start', id, path, started });
        try {
          const r = await real(input, init);
          w.__brutexPull?.({
            kind: 'end',
            id,
            path,
            started,
            ms: Date.now() - started,
            ok: r.ok,
            status: r.status
          });
          return r;
        } catch (e) {
          w.__brutexPull?.({
            kind: 'end',
            id,
            path,
            started,
            ms: Date.now() - started,
            ok: false,
            status: 0,
            // See the probe's catch above for why this is a cast and not an
            // `instanceof Error` narrowing.
            why: String(/** @type {{ message?: unknown }} */ (e)?.message ?? e)
          });
          throw e;
        }
      };
    }
    return () => {
      w.__brutexPull = null;
    };
  });

  // The clock only ticks while something is actually in flight. A permanent
  // one-second interval is a permanent re-render for a number nobody is
  // reading on the other twenty-three hours of the day.
  $effect(() => {
    if (!inflight.length) return;
    now = Date.now();
    const id = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(id);
  });

  const running = $derived(inflight.length > 0);
  const elapsed = $derived(running ? now - Math.min(...inflight.map((p) => p.started)) : 0);

  /** @param {number} ms */
  function clock(ms) {
    const s = Math.max(0, Math.floor(ms / 1000));
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const r = s % 60;
    /** @param {number} n */
    const pad = (n) => String(n).padStart(2, '0');
    return h ? `${h}:${pad(m)}:${pad(r)}` : `${pad(m)}:${pad(r)}`;
  }
</script>

<a class="skip" href="#main">Skip to content</a>

<div class="shell">
  <header class="topbar">
    <span class="brand">bru<b>tex</b></span>

    <nav aria-label="Primary">
      {#each NAV as t (t.href)}
        <!-- `data-sveltekit-reload` ONLY WHERE THE ENTRY ASKS FOR IT. An empty
             string renders the attribute; `undefined` omits it entirely, so the
             five in-app tabs keep client-side routing and only `/logs` leaves.
             `title` carries the reason a tab behaves differently from its
             neighbours, which is the one thing a label cannot say. -->
        <a
          class="tab"
          href={t.href}
          data-sveltekit-reload={t.reload ? '' : undefined}
          title={t.why}
          aria-current={current(t.href) ? 'page' : undefined}>{t.label}</a
        >
      {/each}
    </nav>

    <span class="spacer"></span>

    <!-- IS ANYTHING HAPPENING. Two independent facts, never merged into one
         green light: a reachable API says nothing about a running pull, and a
         running pull says nothing about the next request succeeding. -->
    {#if running}
      <span class="status busy" role="status" aria-live="polite">
        <span class="dot acc live"></span>
        <span class="txt">Pull running · {inflight[0].path}</span>
        <span class="ms">{clock(elapsed)}</span>
      </span>
    {:else if lastPull}
      <span class="status" title={lastPull.why ?? `${lastPull.path} finished`}>
        <span class="dot" class:up={lastPull.ok} class:down={!lastPull.ok}></span>
        <span class="txt">Last pull HTTP {lastPull.status}</span>
        <span class="ms">{clock(lastPull.ms)}</span>
      </span>
    {/if}

    <button
      class="status"
      class:bad={api.state === 'down'}
      type="button"
      onclick={probe}
      title={api.state === 'down'
        ? `Probing /feeds.json failed: ${api.why}`
        : `Round trip to /feeds.json. Checked ${stampLabel(api.at)}. Click to re-check.`}
    >
      <span
        class="dot"
        class:up={api.state === 'up'}
        class:down={api.state === 'down'}
        class:warn={api.state === 'checking'}
      ></span>
      {#if api.state === 'down'}
        <span class="txt">API unreachable — {api.why}</span>
      {:else if api.state === 'checking'}
        <span class="txt">Checking API…</span>
      {:else}
        <span class="txt">API</span>
        <span class="ms">{api.ms} ms</span>
      {/if}
    </button>

    <!-- THE FEED. One is selected; the others are named, not rendered.

         NOT DRAWN ON A ROUTE THAT CARRIES ITS OWN. See FEED_OWNED above: the
         rule is one control per value per route, and it is broken by a second
         copy exactly as surely as by none at all. `/ingest` puts the feed where
         it belongs on that page — first in the row of controls it governs — so
         the bar stands down there and nowhere else. Everything else in this
         header is genuinely cross-cutting and is always drawn. -->
    {#if feedInBar}
    <div class="combo" bind:this={comboEl}>
      <span class="lbl" id="feed-label">Feed</span>
      <div
        class="combo-btn"
        role="combobox"
        tabindex="0"
        id="feed-trigger"
        aria-controls="feed-listbox"
        aria-expanded={open}
        aria-haspopup="listbox"
        aria-labelledby="feed-label feed-value"
        aria-activedescendant={open && hi >= 0 ? `feed-opt-${hi}` : undefined}
        bind:this={triggerEl}
        onclick={() => (open ? close() : openList())}
        onkeydown={onTriggerKey}
      >
        <span class="val" id="feed-value">
          {#if feeds.error}
            Feeds unavailable
          {:else if !feedsTried}
            Loading…
          {:else if active}
            {active.display}
          {:else if options.length}
            Select a feed
          {:else}
            No feeds
          {/if}
        </span>
        {#if active}
          <!-- `pull::vendor::SourceKind::label`, off `/feeds.json`. This read
               `transport`, whose two words (`broker`, `archive`) were minted in
               the API handler by a second `match` on the same split `kind`
               already carried — one fact, emitted twice, in two vocabularies.
               The field is gone; the label is the type's own. -->
          <span class="sub">{active.kind_label ?? 'kind not stated'}</span>
        {/if}
        <span class="caret"></span>
      </div>

      {#if open}
        <div class="combo-pop" role="listbox" id="feed-listbox" aria-label="Feed">
          {#each options as f, i (f.wire)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="combo-opt"
              role="option"
              id="feed-opt-{i}"
              tabindex="-1"
              bind:this={optEls[i]}
              aria-selected={f.wire === feeds.active}
              aria-disabled={!usable(f)}
              data-hi={hi === i}
              onclick={() => commit(i)}
              onpointerdown={(e) => e.preventDefault()}
              onpointerenter={() => usable(f) && (hi = i)}
            >
              <span class="tick" aria-hidden="true">{f.wire === feeds.active ? '✓' : ''}</span>
              <span class="name">{f.display}</span>
              <span class="side"
                >{f.kind_label ?? 'kind not stated'}{usable(f) ? '' : ' · unavailable'}</span
              >
              {#if f.why}
                <span class="why">{f.why}</span>
              {/if}
            </div>
          {/each}

          {#if feeds.error}
            <div class="combo-note bad" role="alert">
              <strong>The feed list could not be read.</strong>
              {feeds.error}
              <br />
              Nothing on any page can be trusted to be complete until this
              answers. <button class="link" onclick={retryFeeds}>Try again</button>
            </div>
          {:else if !feedsTried}
            <div class="combo-note" role="status">
              <span class="skel line" style="width:60%"></span>
              <span class="skel line" style="width:40%"></span>
              Reading /feeds.json…
            </div>
          {:else if !options.length}
            <div class="combo-note" role="status">
              <strong>No feeds are configured.</strong>
              The server answered /feeds.json with an empty list, so there is no
              vendor to select and nothing to pull. A feed appears here when its
              descriptor row exists on the Rust side.
            </div>
          {/if}
        </div>
      {/if}
    </div>
    {/if}

    <!-- THEME. Three explicit states, because "auto" is a real choice and a
         two-way switch cannot express it — it can only silently pin whatever
         the machine happened to be on the day it was first clicked. -->
    <div class="themer" role="group" aria-label="Theme">
      {#each MODES as m (m.id)}
        <button
          type="button"
          aria-pressed={mode === m.id}
          title={m.label}
          aria-label={m.label}
          onclick={() => (mode = m.id)}
        >
          {#if m.id === 'auto'}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
              <circle cx="12" cy="12" r="9" />
              <path d="M12 3a9 9 0 0 0 0 18z" fill="currentColor" stroke="none" />
            </svg>
          {:else if m.id === 'light'}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
              <circle cx="12" cy="12" r="4.2" />
              <path d="M12 2.5v2.2M12 19.3v2.2M4.2 4.2l1.6 1.6M18.2 18.2l1.6 1.6M2.5 12h2.2M19.3 12h2.2M4.2 19.8l1.6-1.6M18.2 5.8l1.6-1.6" />
            </svg>
          {:else}
            <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
              <path d="M20.5 14.6A8.5 8.5 0 1 1 9.4 3.5a7 7 0 0 0 11.1 11.1z" />
            </svg>
          {/if}
        </button>
      {/each}
    </div>
    <span class="sr-only" aria-live="polite">Theme: {mode === 'auto' ? `auto (${resolved})` : mode}</span>

    {#if running}
      <span class="progress" aria-hidden="true"><i></i></span>
    {/if}
  </header>

  <main class="main" id="main" tabindex="-1">
    {@render children()}
  </main>
</div>

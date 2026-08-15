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
    { href: '/audit', label: 'Audit' }
  ];

  // `/db/anything` still lights DB. An exact match alone leaves the operator
  // on a sub-route with no nav item lit and no idea where they are.
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
  let comboEl = $state(null);
  let triggerEl = $state(null);
  let optEls = $state([]);

  const options = $derived(feeds.all ?? []);
  const active = $derived(options.find((f) => f.wire === feeds.active) ?? null);
  const activeIndex = $derived(options.findIndex((f) => f.wire === feeds.active));

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

  /** Move the highlight, skipping feeds the server says cannot be used. */
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

  function commit(i) {
    const f = options[i];
    if (!f || !usable(f)) return;
    feeds.active = f.wire;
    close();
  }

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
    const away = (e) => {
      if (comboEl && !comboEl.contains(e.target)) close({ refocus: false });
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
  let api = $state({ state: 'checking', ms: 0, why: null, at: 0 });

  async function probe() {
    const t0 = performance.now();
    try {
      const r = await fetch('/feeds.json', { cache: 'no-store' });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const ct = r.headers.get('content-type') ?? '';
      if (!ct.includes('json')) {
        throw new Error(`answered ${ct || 'no content-type'}, not JSON — the API is not behind this route`);
      }
      await r.json();
      api = { state: 'up', ms: Math.round(performance.now() - t0), why: null, at: Date.now() };
    } catch (why) {
      api = {
        state: 'down',
        ms: Math.round(performance.now() - t0),
        why: String(why?.message ?? why),
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
  let inflight = $state([]);
  let lastPull = $state(null);
  let now = $state(Date.now());
  let seq = 0;

  $effect(() => {
    // Re-pointing a hook rather than re-wrapping keeps hot reload from
    // stacking wrappers, and keeps THIS instance's state the one that updates.
    window.__brutexPull = (evt) => {
      if (evt.kind === 'start') inflight = [...inflight, evt];
      else {
        inflight = inflight.filter((p) => p.id !== evt.id);
        lastPull = evt;
      }
    };
    if (!window.__brutexPullWrapped) {
      window.__brutexPullWrapped = true;
      const real = window.fetch.bind(window);
      window.fetch = async (input, init) => {
        let path = '';
        try {
          path = new URL(typeof input === 'string' ? input : input.url, location.href).pathname;
        } catch {
          /* a Request built from something exotic — not a pull, then */
        }
        if (!PULL_PATH.test(path)) return real(input, init);
        const id = (seq += 1);
        const started = Date.now();
        window.__brutexPull?.({ kind: 'start', id, path, started });
        try {
          const r = await real(input, init);
          window.__brutexPull?.({
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
          window.__brutexPull?.({
            kind: 'end',
            id,
            path,
            started,
            ms: Date.now() - started,
            ok: false,
            status: 0,
            why: String(e?.message ?? e)
          });
          throw e;
        }
      };
    }
    return () => {
      window.__brutexPull = null;
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

  function clock(ms) {
    const s = Math.max(0, Math.floor(ms / 1000));
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const r = s % 60;
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
        <a class="tab" href={t.href} aria-current={current(t.href) ? 'page' : undefined}>{t.label}</a>
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
        : `Round trip to /feeds.json. Checked ${new Date(api.at).toLocaleTimeString()}. Click to re-check.`}
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

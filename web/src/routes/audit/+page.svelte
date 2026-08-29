<script>
  /**
   * AUDIT · LOGGING · MONITORING — the operations console.
   *
   * ═══════════════════════════════════════════════════════════════════════
   * WHAT CHANGED, AND WHY THIS FILE WAS REWRITTEN
   * ═══════════════════════════════════════════════════════════════════════
   *
   * This page used to fetch `/audit` — the Rust-rendered HTML page — and read
   * its table back out with `DOMParser`, because there was no JSON route. Two
   * things were wrong with that and neither was fixable from here:
   *
   *  1. It coupled a browser page to `crates/api/src/render.rs::audit_row`'s
   *     markup, with no compiler between them.
   *  2. `/audit` was in the dev proxy's route list, so the same URL served two
   *     different applications: a nav click rendered THIS page (SvelteKit
   *     routes in-browser and never consults the proxy) and a reload rendered
   *     the Rust page. The console was unreachable by bookmark, by refresh and
   *     by typing the address.
   *
   * `GET /audit.json` now exists — `crates/api/src/audit_json.rs` — and
   * `/audit` is out of the proxy list. This page is one fetch, and the URL is
   * the page whichever way you arrive at it.
   *
   * ═══════════════════════════════════════════════════════════════════════
   * "IS SOMETHING HAPPENING RIGHT NOW" — MEASURED, NEVER TIMED
   * ═══════════════════════════════════════════════════════════════════════
   *
   * There is no record of a run IN FLIGHT anywhere in this system. A pull is
   * one synchronous POST and its journal record is appended when it ENDS, so a
   * nine-minute backfill is nine minutes in which every surface says nothing
   * is happening. That is the single most-missed thing during an unattended
   * twelve-hour run and it is what the top strip exists for.
   *
   * The strip does not run a timer and call it progress. It subtracts two
   * successive answers and states what moved:
   *
   *   · `store.generation`  — the manifest's commit counter. It advances only
   *                           when bars are filed.
   *   · `store.bars`        — the census total.
   *   · `store.committed_at`— the manifest's mtime, in the SERVER's clock,
   *                           beside the server's `at`. Both numbers come from
   *                           the same machine, so the browser subtracts two
   *                           server numbers and never its own clock.
   *
   * Every rate on screen is (Δ measured) ÷ (Δ seconds measured), with the
   * window it was measured over printed next to it. Nothing here extrapolates
   * an ETA from a rate that has been stable for twelve seconds.
   *
   * ═══════════════════════════════════════════════════════════════════════
   * THE FAILURE REASONS THAT ARE NOT ON DISK — SAID, NOT PAPERED OVER
   * ═══════════════════════════════════════════════════════════════════════
   *
   * `crates/api/src/audit.rs` stores a failure COUNT and exactly ONE failure
   * REASON per run (`Record::of_run` keeps `failures.first()`), and the note
   * field is 68 bytes. So a run that failed 409 members carries one name — the
   * alphabetically first — and the other 408 reasons were never written. No
   * page can recover them and this one does not pretend to: it counts what is
   * missing and says so in the same panel as what is present. `CLAUDE.md` §3
   * rule 6, honest limits.
   *
   * What IS on disk is grouped, counted, ranked and searchable, which is the
   * part that was asked for and can be delivered.
   *
   * ═══════════════════════════════════════════════════════════════════════
   * COST
   * ═══════════════════════════════════════════════════════════════════════
   *
   * Search is one Map probe per keystroke against a prefix index built once
   * per payload — the pattern `web/src/lib/index.svelte.js` sets and
   * `web/typeahead.js` sets before it. The run log is virtualised at a fixed
   * row height, so the DOM holds a screenful whether the journal has 137
   * records or 11,200. The coverage grid is bounded by the target window (80
   * cells today) and not by the store.
   */
  import { untrack } from 'svelte';
  import { feeds } from '$lib/feeds.svelte.js';
  // THE ZONE IS NAMED, BECAUSE THE MACHINE'S ZONE IS NOT THE PRODUCT'S.
  //
  // A bare `toLocaleTimeString()` printed `15:29:04` in whatever zone the host
  // happens to sit in, and named neither the zone nor the day -- so a stamp
  // from yesterday's session was indistinguishable from one taken a minute
  // ago, which is the single fact these lines exist to report. `/db` diagnosed
  // this and fixed itself; `stampLabel` is what it fixed itself with, and this
  // is the third and last spelling of the rule joining the other two.
  import { stampLabel } from '$lib/dates.js';
  import { whole, oneDp } from '$lib/money.js';
  // A REQUEST THAT CANNOT END IS A SPINNER THAT LIES. `ask` is `fetch` with a
  // ceiling; see `$lib/ask.js` for why the wrapper exists rather than a signal
  // threaded through every call site.
  import { ask } from '$lib/ask.js';

  /* ══════════════════════════════════════════════════════════════════════
     CONSTANTS — each one traceable to a file in this repository
     ══════════════════════════════════════════════════════════════════════ */

  /** `docs/07-plan.md` R-2: the backfill window opens here. */
  const TARGET_FROM = '2020-01';
  /** `crates/api/src/audit.rs`: the note field is 68 bytes. */
  const NOTE_BYTES = 68;
  /** `crates/api/src/audit.rs`: `MAX_PAGE_RECORDS`. */
  const PAGE_ROWS = 200;
  /**
   * How many pages of journal this console will hold at once.
   *
   * Ten pages is 2,000 runs — more than a person reads, and a ceiling that
   * exists because "load older" with no bound is a browser that eventually
   * holds the whole file. When it is reached the page says so rather than
   * quietly stopping.
   */
  const MAX_PAGES = 10;
  /** Poll period while the store is moving, and while it is not. */
  const HOT_MS = 5000;
  const COLD_MS = 20000;
  /** A commit this recent counts as "the store was written just now". */
  const FRESH_SECS = 45;
  /** Rows kept for the growth window. 40 × 5 s ≈ the last three minutes. */
  const SAMPLES = 40;
  /** Fixed row height for the virtual run log. Matches `--row-h`. */
  const ROW_H = 30;
  /** Rows drawn above and below the viewport, so a fast scroll has no gap. */
  const OVERSCAN = 6;
  /** `web/src/lib/index.svelte.js`: one Map probe up to this prefix length. */
  const MAX_PREFIX = 4;
  /**
   * How many causes are shown before the list is folded.
   *
   * The tail of this list is long and thin — 28 distinct causes today, of
   * which 20 are one-run refusals from a form typed wrongly once. Showing all
   * of them pushed the run log and the timeline below three screens of noise,
   * so the largest are shown and the rest are one click away. Nothing is
   * dropped and the count of what is folded is printed.
   */
  const TOP_CAUSES = 8;

  const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

  /* Outcome tone as three predicates rather than a computed class name: a
     class built from an expression cannot be checked, and a colour that
     silently stops applying on a monitoring page is the worst regression
     there is. `crates/api/src/audit.rs::Outcome` has five labels. */
  const isOk = (/** @type {string} */ o) => o === 'STORED';
  const isMid = (/** @type {string} */ o) => o === 'STORED NOTHING';
  const isBad = (/** @type {string} */ o) => !isOk(o) && !isMid(o);

  /* ══════════════════════════════════════════════════════════════════════
     FORMATTING — Indian grouping, IST, and no invented precision
     ══════════════════════════════════════════════════════════════════════ */
  // ONE SPELLING OF THE GROUPING, NOT FIVE. `en-IN` was constructed
  // independently here, on `/db`, twice on this page and on `/ingest` — and the
  // spellings had DIVERGED: `/db`'s rendered an unknown as the literal "NaN".
  const n0 = whole;
  const n1 = oneDp;

  /* The server sends epoch seconds. Formatting them in Asia/Kolkata is exact
     whatever this browser's zone is, which is why the zone is named here
     rather than left to the host. */
  const IST_TIME = new Intl.DateTimeFormat('en-GB', {
    timeZone: 'Asia/Kolkata',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  });
  const IST_DATE = new Intl.DateTimeFormat('en-GB', {
    timeZone: 'Asia/Kolkata',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit'
  });
  const istTime = (/** @type {number} */ secs) =>
    Number.isFinite(secs) ? IST_TIME.format(new Date(secs * 1000)) : '—';
  const istStamp = (/** @type {number} */ secs) =>
    Number.isFinite(secs)
      ? `${IST_DATE.format(new Date(secs * 1000)).split('/').reverse().join('-')} ${istTime(secs)}`
      : '—';

  /** A duration in whole units, largest two. Never "0.0 s" for four minutes. */
  function dur(/** @type {number} */ secs) {
    if (!Number.isFinite(secs) || secs < 0) return '—';
    const s = Math.floor(secs % 60);
    const m = Math.floor((secs / 60) % 60);
    const h = Math.floor(secs / 3600);
    if (h > 0) return `${h}h ${m}m`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
  }
  const micros = (/** @type {number} */ us) => dur(us / 1_000_000);

  /* ══════════════════════════════════════════════════════════════════════
     MOTION — asked once, honoured everywhere
     ══════════════════════════════════════════════════════════════════════ */
  let still = $state(false);
  $effect(() => {
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    still = mq.matches;
    const on = (/** @type {MediaQueryListEvent} */ e) => (still = e.matches);
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  });

  /**
   * A number that counts to its value — and is NEVER wrong while it does.
   *
   * The previous version of this page animated every headline figure through
   * a class whose only assignment was inside a `requestAnimationFrame`
   * callback. rAF does not fire in a tab that is not painting, so five
   * headline numbers rendered `0` on a 139-million-bar store while their own
   * subtitles printed the right figure — a monitoring dashboard left in a
   * background tab, which is the normal way one is used.
   *
   * So: `v` is assigned the target IMMEDIATELY, always. The tween moves a
   * SEPARATE display value, and the getter falls back to the truth whenever a
   * frame has not run. A frame that never comes costs the animation, not the
   * number.
   */
  class Tally {
    /** The truth. Assigned synchronously, every time. */
    #to = $state(0);
    /** What the tween is showing, or `null` when no tween is in flight. */
    #shown = $state(/** @type {number | null} */ (null));
    #from = 0;
    #t0 = 0;
    #raf = 0;
    #started = false;

    get v() {
      return this.#shown ?? this.#to;
    }

    /** @param {number} target */
    set(target) {
      // A FIGURE NOBODY SENT IS NOT ZERO, AND THIS LINE USED TO SAY IT WAS.
      //
      // Together with the `?? 0` at every call site, a `/audit.json` that
      // omitted `store.bars` -- an older payload, a partial answer, a field
      // renamed server-side -- rendered "0 bars stored" across a store holding
      // millions. `n0` already formats a non-finite number as an em dash, and
      // this coercion is what stopped it ever seeing one. The page had the
      // right formatter and could not reach it.
      //
      // `NaN` is carried rather than animated: a count-up towards an unknown
      // is a transition through numbers the store never held, which is the
      // same objection the first-value branch below already makes.
      if (!Number.isFinite(target)) {
        this.#to = Number.NaN;
        this.#shown = null;
        this.#started = true;
        cancelAnimationFrame(this.#raf);
        return;
      }
      if (target === this.#to) return;
      const previous = this.v;
      this.#to = target;
      // The first value a page ever shows is not an animation from zero — that
      // is a made-up transition through numbers the store never held.
      if (still || !this.#started || typeof document === 'undefined' || document.hidden) {
        this.#started = true;
        this.#shown = null;
        cancelAnimationFrame(this.#raf);
        return;
      }
      this.#from = previous;
      this.#shown = previous;
      this.#t0 = performance.now();
      cancelAnimationFrame(this.#raf);
      const step = (/** @type {number} */ now) => {
        const p = Math.min(1, (now - this.#t0) / 480);
        if (p >= 1) {
          this.#shown = null;
          return;
        }
        this.#shown = this.#from + (this.#to - this.#from) * (1 - (1 - p) ** 3);
        this.#raf = requestAnimationFrame(step);
      };
      this.#raf = requestAnimationFrame(step);
    }
  }

  /* ══════════════════════════════════════════════════════════════════════
     THE ONE FETCH
     ══════════════════════════════════════════════════════════════════════ */

  const load = $state({
    /** @type {'idle'|'first'|'ok'|'refreshing'|'error'} */
    state: 'idle',
    /** @type {string | null} */
    error: null,
    /** Round-trip in ms, measured. */
    ms: 0,
    /** When this browser last got an answer, for the "stale" line. */
    at: 0
  });

  /** @type {any} */
  let payload = $state(null);
  /** Older pages, merged in when the operator asks for them. */
  /** @type {any[]} */
  let older = $state([]);
  let pagesHeld = $state(1);
  let loadingOlder = $state(false);

  /** Round trips, newest last, for the probe sparkline. */
  /** @type {number[]} */
  let rtt = $state([]);

  /**
   * Growth samples. One per successful poll, bounded.
   * @type {{at:number, bars:number, im:number, gen:number, months:Map<string,number>}[]}
   */
  let samples = $state([]);

  /** The store as it was when this page opened — the "since you arrived" base. */
  /** @type {{at:number, bars:number, im:number, gen:number} | null} */
  let base = $state(null);

  let live = $state(true);

  async function read(page = 0) {
    const feed = feeds.active;
    if (!feed) return null;
    const url = `/audit.json?feed=${encodeURIComponent(feed)}${page ? `&page=${page}` : ''}`;
    const t0 = performance.now();
    const r = await ask(url, { cache: 'no-store' });
    const ms = Math.round(performance.now() - t0);
    if (!r.ok) {
      // THE REAL ERROR, NAMED. A 404 here means the process serving this port
      // was started before the route existed — which is a stale binary, not a
      // missing feature, and the two need different actions.
      let extra = '';
      if (r.status === 404) {
        extra =
          ' — the route exists in this source tree (crates/api/src/audit_json.rs). A 404 means the ' +
          'running process was started before it was built. Restart the API binary.';
      } else {
        const said = (await r.text().catch(() => '')).trim();
        extra = said
          ? ` — ${said.slice(0, 400)}`
          : ' — the response had no body. Through the dev proxy a 500 with no body is what an API ' +
            'that is not listening looks like: check that the port in web/vite.config.js is the one ' +
            'the binary was started on.';
      }
      throw new Error(`GET ${url} answered HTTP ${r.status}${extra}`);
    }
    const ct = r.headers.get('content-type') ?? '';
    if (!ct.includes('json')) {
      throw new Error(`GET ${url} answered ${ct || 'no content-type'}, which is not JSON`);
    }
    return { body: await r.json(), ms };
  }

  async function refresh() {
    if (!feeds.active) {
      /* NO FEED CLEARS, RATHER THAN FREEZING WHAT WAS THERE. This was a bare
         `return`, so on clearing the feed `payload` and `samples` both kept
         the previous feed's values while the poll went on ticking and the
         button went on reading "Live · 2s".

         Two lies came out of that. `{#if !feeds.active}` renders a "No feed is
         selected" card, and `{#if payload}` renders independently below it —
         so the page asserted no feed and drew the previous feed's whole audit
         underneath, down to "Every figure here is <feed>'s". And `pulse` is
         computed from the last two `samples`: with those frozen mid-growth it
         reported RUNNING — "the store gained N bars in the last <frozen
         duration> — measured, not timed" — for a feed nobody had selected.

         Clearing both is what makes the no-feed card the whole truth. */
      payload = null;
      samples = [];
      load.state = 'idle';
      return;
    }
    load.state = payload ? 'refreshing' : 'first';
    try {
      const got = await read(0);
      if (!got) return;
      const body = got.body;
      payload = body;
      load.ms = got.ms;
      load.at = Date.now();
      load.state = 'ok';
      load.error = null;
      rtt = [...rtt, got.ms].slice(-32);

      const months = new Map(body.store.months.map((/** @type {any} */ m) => [m.month, m.bars]));
      const s = {
        at: body.at,
        bars: body.store.bars,
        im: body.store.instrument_months,
        gen: body.store.generation ?? 0,
        months
      };
      // The server's own clock decides the ordering. Two answers with the same
      // `at` are one sample: a rate over a zero-second window is a division by
      // zero dressed up as a measurement.
      const last = samples.at(-1);
      if (!last || s.at > last.at) samples = [...samples, s].slice(-SAMPLES);
      base ??= { at: s.at, bars: s.bars, im: s.im, gen: s.gen };
    } catch (why) {
      load.state = 'error';
      load.error = String(why instanceof Error ? why.message : why);
    }
  }

  async function loadOlder() {
    if (loadingOlder || pagesHeld >= Math.min(MAX_PAGES, payload?.journal?.pages ?? 1)) return;
    loadingOlder = true;
    try {
      const got = await read(pagesHeld);
      if (got) {
        older = [...older, ...got.body.runs];
        pagesHeld += 1;
      }
    } catch (why) {
      load.error = String(why instanceof Error ? why.message : why);
      load.state = 'error';
    } finally {
      loadingOlder = false;
    }
  }

  /**
   * THE FEED CHANGED, SO EVERYTHING MEASURED SO FAR BELONGS TO ANOTHER FEED.
   *
   * `untrack` is load-bearing and its absence was a live bug: this effect
   * writes `payload`, and `refresh()` READS `payload` on its first synchronous
   * line. That read made the effect depend on the state it had just written,
   * so every answer re-ran the effect, which cleared the answer and fetched
   * again — a request storm, ~30 identical GETs before it was seen in the
   * network log. The dependency here is the FEED and nothing else.
   */
  $effect(() => {
    const feed = feeds.active;
    if (!feed) return;
    untrack(() => {
      payload = null;
      older = [];
      pagesHeld = 1;
      samples = [];
      base = null;
      refresh();
    });
  });

  /* ══════════════════════════════════════════════════════════════════════
     PRIORITY 1 — IS SOMETHING HAPPENING RIGHT NOW
     ══════════════════════════════════════════════════════════════════════ */

  const pulse = $derived.by(() => {
    const store = payload?.store;
    const now = payload?.at ?? 0;
    const sinceCommit = store?.committed_at == null ? null : now - store.committed_at;

    // Growth between the oldest sample still held and the newest. Both numbers
    // are the server's, so the window is the server's too.
    const first = samples[0];
    const last = samples.at(-1);
    const span = first && last ? last.at - first.at : 0;
    const dBars = first && last ? last.bars - first.bars : 0;
    const dIm = first && last ? last.im - first.im : 0;
    const dGen = first && last ? last.gen - first.gen : 0;
    const barsPerMin = span > 0 ? (dBars / span) * 60 : null;

    // Did anything move between the last TWO answers? That is the strongest
    // evidence available and it is a comparison, not a guess.
    const prev = samples.at(-2);
    const stepped = prev && last ? last.gen - prev.gen : 0;
    const steppedBars = prev && last ? last.bars - prev.bars : 0;

    /** @type {'moving'|'fresh'|'still'|'unknown'} */
    let state = 'unknown';
    if (!store || store.state !== 'held') state = 'unknown';
    else if (stepped > 0 || steppedBars > 0 || dGen > 0 || dBars > 0) state = 'moving';
    else if (sinceCommit != null && sinceCommit <= FRESH_SECS) state = 'fresh';
    else state = 'still';

    // WHICH MONTH IS BEING FILLED. Not a label the server sent — there is no
    // such label — but the month whose bar count grew most between the two
    // most recent answers. Named as a measurement, and absent when nothing
    // grew.
    /** @type {{month:string, bars:number} | null} */
    let filling = null;
    if (prev && last) {
      for (const [month, bars] of last.months) {
        const was = prev.months.get(month) ?? 0;
        const grew = bars - was;
        if (grew > 0 && (!filling || grew > filling.bars)) filling = { month, bars: grew };
      }
    }

    return {
      state,
      moving: state === 'moving',
      sinceCommit,
      span,
      dBars,
      dIm,
      dGen,
      barsPerMin,
      stepped,
      filling,
      sinceOpen: base && last ? { bars: last.bars - base.bars, im: last.im - base.im, gen: last.gen - base.gen, secs: last.at - base.at } : null
    };
  });

  /* THE POLL. Faster while the store is moving, slower while it is not, and
     stopped while the tab is hidden — a monitoring page that keeps a
     five-second request loop running in a background tab for eight hours is
     spending on nobody's behalf. Declared after `pulse` because it reads it. */
  /**
   * A BOOLEAN, NOT THE WHOLE PULSE.
   *
   * The poll below must depend on "is it hot", which flips rarely — not on
   * `pulse`, which is rebuilt on every answer. An interval torn down and
   * recreated on every answer is an interval that can be reset just before it
   * would have fired, forever.
   */
  const hot = $derived(pulse.moving);

  $effect(() => {
    if (!live) return;
    const period = hot ? HOT_MS : COLD_MS;
    const tick = () => {
      if (!document.hidden) refresh();
    };
    const id = window.setInterval(tick, period);
    // Coming back to the tab re-reads at once rather than waiting out the
    // period, so a page brought forward is never showing a stale answer while
    // it counts down.
    const wake = () => {
      if (!document.hidden) refresh();
    };
    document.addEventListener('visibilitychange', wake);
    return () => {
      clearInterval(id);
      document.removeEventListener('visibilitychange', wake);
    };
  });

  /** The measured-rate sparkline: bars per minute between successive answers. */
  const ratePath = $derived.by(() => {
    if (samples.length < 3) return '';
    /** @type {number[]} */
    const rates = [];
    for (let i = 1; i < samples.length; i += 1) {
      const a = samples[i - 1];
      const b = samples[i];
      const dt = b.at - a.at;
      rates.push(dt > 0 ? ((b.bars - a.bars) / dt) * 60 : 0);
    }
    const max = Math.max(...rates, 1);
    const w = 100 / Math.max(1, rates.length - 1);
    return rates
      .map((v, i) => `${i === 0 ? 'M' : 'L'}${(i * w).toFixed(2)},${(22 - (v / max) * 20).toFixed(2)}`)
      .join(' ');
  });

  const probePath = $derived.by(() => {
    if (rtt.length < 2) return '';
    const max = Math.max(...rtt, 1);
    const w = 100 / (rtt.length - 1);
    return rtt
      .map((v, i) => `${i === 0 ? 'M' : 'L'}${(i * w).toFixed(2)},${(20 - (v / max) * 18).toFixed(2)}`)
      .join(' ');
  });

  /* ══════════════════════════════════════════════════════════════════════
     PRIORITY 3 — COVERAGE, DRAWN AGAINST A DENOMINATOR THAT IS CITED
     ══════════════════════════════════════════════════════════════════════ */

  /** `YYYY-MM` → ordinal, so a month is arithmetic and never a string compare. */
  const ord = (/** @type {string} */ m) => {
    const [y, mo] = m.split('-').map(Number);
    return Number.isFinite(y) && Number.isFinite(mo) ? y * 12 + (mo - 1) : NaN;
  };
  const unord = (/** @type {number} */ o) =>
    `${Math.floor(o / 12)}-${String((o % 12) + 1).padStart(2, '0')}`;

  const coverage = $derived.by(() => {
    const store = payload?.store;
    const today = payload?.today;
    if (!store || !today) return null;
    const from = ord(TARGET_FROM);
    const to = ord(today.slice(0, 7));
    if (!Number.isFinite(from) || !Number.isFinite(to) || to < from) return null;

    /** @type {Map<string, {im:number, bars:number}>} */
    const held = new Map();
    for (const m of store.months) held.set(m.month, { im: m.instrument_months, bars: m.bars });

    /** @type {{month:string, im:number, bars:number, inTarget:boolean}[]} */
    const cells = [];
    let present = 0;
    let barsInTarget = 0;
    for (let o = from; o <= to; o += 1) {
      const month = unord(o);
      const h = held.get(month);
      if (h) {
        present += 1;
        barsInTarget += h.bars;
      }
      cells.push({ month, im: h?.im ?? 0, bars: h?.bars ?? 0, inTarget: true });
    }

    // Months the store holds that the target window does not ask for. Shown,
    // because a month nobody asked for is still data on disk and hiding it
    // would make the totals on this page disagree with the store's own.
    const outside = store.months.filter((/** @type {any} */ m) => {
      const o = ord(m.month);
      return o < from || o > to;
    });

    // Gaps as RANGES, not as 40 red squares nobody counts.
    /** @type {{from:string, to:string, months:number}[]} */
    const gaps = [];
    let run = -1;
    for (let o = from; o <= to + 1; o += 1) {
      const missing = o <= to && !held.has(unord(o));
      if (missing && run < 0) run = o;
      if (!missing && run >= 0) {
        gaps.push({ from: unord(run), to: unord(o - 1), months: o - run });
        run = -1;
      }
    }
    gaps.sort((a, b) => b.months - a.months);

    const maxIm = Math.max(1, ...cells.map((c) => c.im));
    const years = [...new Set(cells.map((c) => Number(c.month.slice(0, 4))))];
    // A MAP AND NOT A `find` PER CELL. The grid draws 12 cells per year row
    // and a linear search inside that loop is O(months squared) for no reason.
    const byMonth = new Map(cells.map((c) => [c.month, c]));
    return {
      cells,
      byMonth,
      years,
      maxIm,
      from: TARGET_FROM,
      to: unord(to),
      wanted: to - from + 1,
      present,
      gaps,
      outside,
      barsInTarget
    };
  });

  /** The extremes, which is what "visualising extremes" means here. */
  const extremes = $derived.by(() => {
    const months = payload?.store?.months ?? [];
    if (months.length === 0) return null;
    const byIm = [...months].sort((a, b) => a.instrument_months - b.instrument_months);
    const byBars = [...months].sort((a, b) => a.bars - b.bars);
    return {
      thinnest: byIm[0],
      fattest: byIm[byIm.length - 1],
      fewestBars: byBars[0],
      mostBars: byBars[byBars.length - 1]
    };
  });

  /* ══════════════════════════════════════════════════════════════════════
     PRIORITIES 2 AND 4 — RUNS, CAUSES, AND THE REASONS THAT ARE NOT ON DISK
     ══════════════════════════════════════════════════════════════════════ */

  const runs = $derived([...(payload?.runs ?? []), ...older]);

  /**
   * Fold a note into a CAUSE.
   *
   * Two runs that failed for the same reason on different instruments, in
   * different months, under different paths must land in one group or the
   * grouping is a list. So: a leading instrument name is replaced by a token,
   * digit runs by `#`, and absolute paths by `<path>`. Everything else is the
   * reason in the writer's own words.
   *
   * A note the server had to cut at 68 bytes is folded on what survived, and
   * the group carries that fact — two long reasons that share a prefix are
   * INDISTINGUISHABLE here and the panel says so rather than claiming they are
   * one fault.
   */
  function fold(/** @type {string} */ note) {
    if (!note) return '(no reason recorded)';
    let text = note;
    // `SYMBOL — reason` and `SYMBOL: reason`: the symbol is the SUBJECT, not
    // the cause, and both separators are in the journal. Stripping it first
    // also stops the digit rule below from turning `360ONE` into `#ONE`, which
    // is what the first version of this function did on real records.
    const sep = ['—', ':'].map((s) => text.indexOf(` ${s} `)).filter((i) => i > 0);
    const dash = Math.min(...(sep.length ? sep : [-1]));
    if (dash > 0 && dash <= 24 && /^[A-Z0-9&.\-*]+$/.test(text.slice(0, dash))) {
      text = `<instrument>${text.slice(dash)}`;
    }
    return text
      .replace(/\/[^\s]+/g, '<path>')
      .replace(/\d{4}-\d{2}(-\d{2})?/g, '<month>')
      .replace(/\d+/g, '#')
      .trim();
  }

  /* WHETHER THIS SERVER LABELS ITS RECORDS AT ALL.
     `kind` arrived with `audit::Kind::MemberFailure`. A binary older than it
     sends no such key, and on THAT journal there really is one record per run
     and one reason per run — so the arithmetic below is right for the old
     payload and wrong for this one. Testing for the KEY rather than for the
     word keeps both correct instead of applying the new rule to an old file. */
  const labelled = $derived(runs.some((/** @type {any} */ r) => r.kind !== undefined));

  /* RUN RECORDS ONLY. `recorded_with_failures` appends one `kind:"run"` and
     then one `kind:"member"` per failure, so counting records counted 1 + N
     records as 1 + N runs — inflating "N run(s) read", all three tab badges
     and the loud tally by one per failed member. The member rows stay in
     `runs` and are still drawn in the log below; they are just not runs. */
  const runOnly = $derived(
    labelled ? runs.filter((/** @type {any} */ r) => r.kind !== 'member') : runs
  );

  const faults = $derived.by(() => {
    /** @type {Map<string, {cause:string, runs:any[], members:number, first:number, last:number, cut:boolean, examples:Set<string>}>} */
    const by = new Map();
    let membersFailed = 0;
    let reasonsKept = 0;
    for (const r of runs) {
      if (r.fault) continue;
      const loud = r.loud;
      if (!loud && r.failures === 0) continue;
      /* ONE FAILURE COUNTED ONCE. The run record carries `failures: N` and each
         member record carries `failures: 1`, and `Outcome::Failed.is_loud()` is
         true so the guard above never excluded them — so a run that refused 409
         members reported 818. A labelled journal is counted on its MEMBER rows,
         because those are the rows that carry one reason each; an unlabelled
         one has no such rows, so its run records are the only count there is. */
      const member = r.kind === 'member';
      const weight = labelled ? (member ? 1 : 0) : (r.failures ?? 0);
      membersFailed += weight;
      /* AND THE REASON BESIDE IT. `reasonsLost` was reporting 408 reasons
         missing from a journal that held all 409 — a red alert block, with
         `role="alert"`, about a loss that had not happened. `Kind::MemberFailure`
         was added precisely so every reason reaches disk. */
      if (r.note) reasonsKept += weight;
      /* THE TWO WRITERS SPELL ONE CAUSE TWO WAYS. `Record::of_run` writes
         "{instrument} — {why}" into `note`; `Record::member_failure` writes the
         bare `why` and puts the instrument in `source`. `fold` strips a leading
         "SYMBOL — ", so one fault folded to two keys and split across two rows,
         inflating the group count as well. Re-joining them folds both spellings
         to one string. */
      const cause = fold(member && r.source ? `${r.source} — ${r.note ?? ''}` : (r.note ?? ''));
      let g = by.get(cause);
      if (!g) {
        g = { cause, runs: [], members: 0, first: r.at, last: r.at, cut: false, examples: new Set() };
        by.set(cause, g);
      }
      g.runs.push(r);
      g.members += weight;
      g.first = Math.min(g.first, r.at);
      g.last = Math.max(g.last, r.at);
      if (r.note_bytes > NOTE_BYTES) g.cut = true;
      const dash = (r.note ?? '').indexOf(' — ');
      if (dash > 0 && dash <= 24) g.examples.add(r.note.slice(0, dash));
      /* A MEMBER ROW NAMES ITS INSTRUMENT IN `source`, NOT IN `note`, so the
         dash test above finds nothing on exactly the records that carry the
         per-instrument fact and the examples list stayed empty. */
      else if (member && r.source) g.examples.add(r.source);
    }
    const groups = [...by.values()].sort(
      (a, b) => b.members - a.members || b.runs.length - a.runs.length
    );
    return {
      groups,
      membersFailed,
      // WHAT IS NOT HERE. On a labelled journal this is zero, and that is the
      // truth: every member failure writes its own reason.
      reasonsLost: Math.max(0, membersFailed - reasonsKept),
      reasonsKept
    };
  });

  /* ── the search index: one Map probe per keystroke ───────────────────── */

  /** @type {Map<string, Set<number>>} */
  let byPrefix = new Map();
  /** @type {Map<number, any>} */
  let byOrdinal = new Map();

  $effect(() => {
    const next = new Map();
    const ords = new Map();
    for (const r of runs) {
      ords.set(r.ordinal, r);
      const text = [
        r.outcome ?? '',
        r.scope ?? '',
        r.source ?? '',
        r.note ?? '',
        r.window ?? '',
        r.fault ?? ''
      ]
        .join(' ')
        .toUpperCase();
      // Tokens, not the whole string: a prefix bucket over words is what makes
      // "FAIL" one probe instead of a substring scan of every note.
      for (const token of text.split(/[^A-Z0-9&.]+/)) {
        if (!token) continue;
        for (let n = 1; n <= Math.min(MAX_PREFIX, token.length); n += 1) {
          const key = token.slice(0, n);
          let bucket = next.get(key);
          if (!bucket) next.set(key, (bucket = new Set()));
          bucket.add(r.ordinal);
        }
      }
    }
    byPrefix = next;
    byOrdinal = ords;
  });

  let allCauses = $state(false);
  let query = $state('');
  /** @type {'all'|'loud'|'STORED'|'STORED NOTHING'|'REFUSED'|'NOT STARTED'|'FAILED'} */
  let outcomeFilter = $state('all');

  const found = $derived.by(() => {
    const q = query.trim().toUpperCase();
    let rows = runs;
    if (q) {
      // ONE PROBE up to four characters; beyond that, one filter over a bucket
      // that is already small. Never a scan of the journal.
      const bucket = byPrefix.get(q.slice(0, MAX_PREFIX));
      if (!bucket) rows = [];
      else {
        const hits = [...bucket].map((o) => byOrdinal.get(o)).filter(Boolean);
        rows =
          q.length <= MAX_PREFIX
            ? hits
            : hits.filter((r) =>
                [r.outcome, r.scope, r.source, r.note, r.window, r.fault]
                  .join(' ')
                  .toUpperCase()
                  .includes(q)
              );
      }
    }
    if (outcomeFilter === 'loud') rows = rows.filter((r) => r.fault || r.loud);
    else if (outcomeFilter !== 'all') rows = rows.filter((r) => r.outcome === outcomeFilter);
    return [...rows].sort((a, b) => b.ordinal - a.ordinal);
  });

  const counts = $derived.by(() => {
    /** @type {Record<string, number>} */
    // RUN RECORDS ONLY. `recorded_with_failures` appends one `kind:"run"` and
    // then one `kind:"member"` per failure, so this counted 1 + N records as
    // 1 + N runs -- a single run that refused 409 members reported 410 runs
    // read, in the headline, in all three tab badges and in the loud tally.
    const c = { all: runOnly.length, loud: 0 };
    for (const r of runOnly) {
      const key = r.fault ? 'DAMAGED' : r.outcome;
      c[key] = (c[key] ?? 0) + 1;
      if (r.fault || r.loud) c.loud += 1;
    }
    return c;
  });

  /* ── priority 4: outcomes over time ──────────────────────────────────── */

  const timeline = $derived.by(() => {
    // RUN RECORDS ONLY, for the reason `runOnly` gives: a member row is not a
    // run, and counting it as one made every column of this chart taller than
    // the truth by one per failed member.
    const rows = runOnly.filter((r) => !r.fault);
    if (rows.length === 0) return null;
    const from = Math.min(...rows.map((r) => r.at));
    const to = Math.max(...rows.map((r) => r.at));
    // The bucket is chosen from the span, and the choice is printed. An hour
    // bucket over a three-week journal is 500 columns nobody can read.
    const span = Math.max(1, to - from);
    const target = 48;
    const raw = span / target;
    const steps = [900, 1800, 3600, 7200, 21600, 43200, 86400, 604800];
    const step = steps.find((s) => s >= raw) ?? steps[steps.length - 1];
    // BUCKETS ALIGNED TO IST, NOT TO UTC. Epoch seconds floor to UTC
    // boundaries, and IST is UTC+05:30, so an "hourly" bucket labelled from a
    // raw floor starts at :30 past every hour — arithmetically right and
    // unreadable beside a column of IST stamps. The offset is a constant:
    // India has no daylight saving.
    const IST = 5.5 * 3600;
    const floorIst = (/** @type {number} */ t) => Math.floor((t + IST) / step) * step - IST;
    const start = floorIst(from);
    /** @type {Map<number, {at:number, ok:number, mid:number, bad:number, bars:number, failures:number}>} */
    const by = new Map();
    for (const r of rows) {
      const k = floorIst(r.at);
      let b = by.get(k);
      if (!b) by.set(k, (b = { at: k, ok: 0, mid: 0, bad: 0, bars: 0, failures: 0 }));
      if (isOk(r.outcome)) b.ok += 1;
      else if (isMid(r.outcome)) b.mid += 1;
      else b.bad += 1;
      b.bars += r.bars_stored ?? 0;
      b.failures += r.failures ?? 0;
    }
    /** @type {{at:number, ok:number, mid:number, bad:number, bars:number, failures:number}[]} */
    const cols = [];
    for (let k = start; k <= to; k += step) {
      cols.push(by.get(k) ?? { at: k, ok: 0, mid: 0, bad: 0, bars: 0, failures: 0 });
    }
    const peak = Math.max(1, ...cols.map((c) => c.ok + c.mid + c.bad));
    const label =
      step >= 86400 ? `${step / 86400} day` : step >= 3600 ? `${step / 3600} hour` : `${step / 60} min`;
    return { cols, peak, step, label, from, to };
  });

  /** @type {number | null} */
  let hotCol = $state(null);

  /* ── the run log's virtual window ────────────────────────────────────── */
  /** @type {HTMLElement | undefined} */
  let listEl = $state();
  let scrollTop = $state(0);
  let listH = $state(360);

  const window_ = $derived.by(() => {
    const total = found.length;
    const first = Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN);
    const count = Math.min(total - first, Math.ceil(listH / ROW_H) + OVERSCAN * 2);
    return { first, rows: found.slice(first, first + Math.max(0, count)), total };
  });

  $effect(() => {
    const el = listEl;
    if (!el) return;
    const ro = new ResizeObserver(() => (listH = el.clientHeight));
    ro.observe(el);
    listH = el.clientHeight;
    return () => ro.disconnect();
  });

  /** @type {any} */
  let picked = $state(null);

  /* ── the counters ────────────────────────────────────────────────────── */
  const tBars = new Tally();
  const tIm = new Tally();
  const tRuns = new Tally();
  const tLoud = new Tally();
  const tMonths = new Tally();
  const tGen = new Tally();

  $effect(() => {
    // `?? NaN` AND NOT `?? 0`. The nullish default is kept because the field
    // may genuinely be absent, but what it defaults TO is now the unknown that
    // `n0` renders as an em dash, rather than a zero indistinguishable from a
    // store that really holds nothing. Zero means zero here, the same way
    // CLAUDE.md section 7 means it for the open-interest sentinel.
    tBars.set(payload?.store?.bars ?? Number.NaN);
    tIm.set(payload?.store?.instrument_months ?? Number.NaN);
    tRuns.set(payload?.journal?.records ?? Number.NaN);
    tLoud.set(counts.loud ?? Number.NaN);
    tMonths.set(coverage?.present ?? Number.NaN);
    tGen.set(payload?.store?.generation ?? Number.NaN);
  });

  const stale = $derived(load.state === 'error' && payload !== null);
</script>

<!-- THE OTHER HALF OF THE `/audit` COLLISION — see `crates/api/src/render.rs`,
     which now titles its own page "server-rendered". Two applications answer
     this one path: a CLICK renders this file, because SvelteKit routes in the
     browser and never asks the server, while a reload, a bookmark or a typed
     address reaches the Rust page instead. They have different nav, and this one
     alone has the feed picker and the theme toggle.
     Both titles were "brutex · audit", so a tab, a bookmark and a history entry
     could not tell them apart — and neither could the reader. Naming each is the
     operator's decision of 21 Aug 2026: distinguish them rather than delete one,
     so the script-free surface `app.html` advertises stays reachable. -->
<svelte:head><title>brutex · audit · console</title></svelte:head>

<div class="pane">
  <div class="pane-head">
    <span class="pane-title">Audit · logging · monitoring</span>
    <span class="head-sub">
      every pull this store root has seen, and what the store is doing right now
    </span>
    <span class="spacer"></span>

    {#if load.state === 'error'}
      <span class="status bad" role="status">
        <span class="dot down"></span><span class="txt">/audit.json failed</span>
      </span>
    {:else if payload}
      <span class="status" title="Round trip to /audit.json, measured. Last answer {stampLabel(load.at)}.">
        <span class="dot up" class:live={load.state === 'refreshing'}></span>
        <span class="txt">{n0(payload.journal.records)} runs</span>
        <span class="ms">{load.ms} ms</span>
      </span>
    {:else}
      <span class="status"><span class="dot warn live"></span><span class="txt">Reading /audit.json…</span></span>
    {/if}

    <button
      class="btn"
      type="button"
      aria-pressed={live}
      onclick={() => (live = !live)}
      title="Re-read /audit.json every {hot ? HOT_MS / 1000 : COLD_MS / 1000} s — faster while the store is moving, and stopped while this tab is hidden"
    >
      <span class="dot" class:acc={live} class:live></span>
      {live ? `Live · ${hot ? HOT_MS / 1000 : COLD_MS / 1000}s` : 'Paused'}
    </button>
    <button class="btn" type="button" onclick={refresh}>Refresh</button>
  </div>

  <div class="scroll">
    {#if !feeds.active}
      <!-- ══ NO FEED. Say which of the two things is true. ══ -->
      <section class="card bad">
        <div class="card-head"><h2>No feed is selected</h2></div>
        <p class="why" role="alert">
          {feeds.error
            ? `/feeds.json — ${feeds.error}`
            : 'The feed picker in the top bar has not settled yet. Every figure on this page belongs to one feed, and this console never guesses which.'}
        </p>
      </section>
    {:else if load.state === 'error' && !payload}
      <!-- ══ THE FETCH FAILED AND THERE IS NOTHING TO SHOW. The real error. ══ -->
      <section class="card bad">
        <div class="card-head"><h2>The audit journal could not be read</h2></div>
        <p class="why" role="alert">{load.error}</p>
        <p class="fine">
          Nothing below is drawn, because there is nothing to draw it from. This page shows no
          figures it did not receive.
        </p>
      </section>
    {/if}

    {#if payload}
      <!-- ════════════════════════════════════════════════════════════════
           1 · IS SOMETHING HAPPENING RIGHT NOW
           ════════════════════════════════════════════════════════════════ -->
      <section class="card pulse" class:moving={pulse.state === 'moving'} aria-label="Live state">
        <div class="card-head">
          <h2>Right now</h2>
          <span class="spacer"></span>
          <span class="fine">
            {payload.store.feed} · server clock {istTime(payload.at)} IST
          </span>
        </div>

        <div class="verdict">
          {#if pulse.state === 'moving'}
            <span class="big up">
              <span class="beacon" aria-hidden="true"></span>RUNNING
            </span>
            <span class="sep">·</span>
            <span>
              the manifest committed <b>{n0(pulse.dGen)}</b> time{pulse.dGen === 1 ? '' : 's'} and the
              store gained <b>{n0(pulse.dBars)}</b> bars in the last <b>{dur(pulse.span)}</b> — measured,
              not timed
            </span>
          {:else if pulse.state === 'fresh'}
            <span class="big warn">JUST WROTE</span>
            <span class="sep">·</span>
            <span>
              nothing has moved between the last two answers, and the manifest was written
              <b>{dur(pulse.sinceCommit ?? 0)}</b> ago. Either a run is between commits or it has just
              finished.
            </span>
          {:else if pulse.state === 'still'}
            <span class="big">IDLE</span>
            <span class="sep">·</span>
            <span>
              the store has not been written for <b>{dur(pulse.sinceCommit ?? 0)}</b>. Nothing is
              filing bars.
            </span>
          {:else}
            <span class="big down">UNKNOWN</span>
            <span class="sep">·</span>
            <span>{payload.store.note}</span>
          {/if}
        </div>

        <div class="stats inner">
          <div class="stat">
            <span class="k">Last commit</span>
            <span class="v" class:up={pulse.state === 'moving'}>
              {pulse.sinceCommit == null ? '—' : dur(pulse.sinceCommit)}
              <em>ago</em>
            </span>
            <span class="n">
              {payload.store.committed_at == null
                ? 'the manifest file has no readable mtime'
                : `${istStamp(payload.store.committed_at)} IST · manifest mtime`}
            </span>
          </div>

          <div class="stat">
            <span class="k">Commits (generation)</span>
            <span class="v">{n0(tGen.v)}</span>
            <span class="n">
              {#if pulse.sinceOpen && pulse.sinceOpen.gen > 0}
                +{n0(pulse.sinceOpen.gen)} since you opened this page
              {:else}
                unchanged since you opened this page
              {/if}
            </span>
          </div>

          <div class="stat">
            <span class="k">Measured rate</span>
            <span class="v" class:up={(pulse.barsPerMin ?? 0) > 0}>
              {pulse.barsPerMin == null ? '—' : n0(pulse.barsPerMin)}
              <em>bars/min</em>
            </span>
            <span class="n">
              {#if ratePath}
                <svg class="spark" viewBox="0 0 100 22" preserveAspectRatio="none" aria-hidden="true">
                  <path d={ratePath} />
                </svg>
              {/if}
              {pulse.span > 0 ? `over the last ${dur(pulse.span)}` : 'needs two answers to measure'}
            </span>
          </div>

          <div class="stat">
            <span class="k">Being filled</span>
            <span class="v">{pulse.filling ? pulse.filling.month : '—'}</span>
            <span class="n">
              {pulse.filling
                ? `+${n0(pulse.filling.bars)} bars since the previous answer`
                : 'no month grew between the last two answers'}
            </span>
          </div>

          <div class="stat">
            <span class="k">Bars in this store</span>
            <span class="v">{n0(tBars.v)}</span>
            <span class="n">
              {#if pulse.sinceOpen && pulse.sinceOpen.bars > 0}
                +{n0(pulse.sinceOpen.bars)} in the {dur(pulse.sinceOpen.secs)} you have been watching
              {:else}
                {n0(payload.store.instrument_months)} instrument-months
              {/if}
            </span>
          </div>

          <div class="stat">
            <span class="k">API · /audit.json</span>
            <span class="v" class:down={load.state === 'error'}>{load.ms} <em>ms</em></span>
            <span class="n">
              {#if probePath}
                <svg class="spark" viewBox="0 0 100 20" preserveAspectRatio="none" aria-hidden="true">
                  <path d={probePath} />
                </svg>
              {/if}
              last {rtt.length} read{rtt.length === 1 ? '' : 's'}
            </span>
          </div>
        </div>

        {#if stale}
          <p class="why" role="alert">
            The figures above are the last good answer, from {stampLabel(load.at)}.
            The poll since then failed: {load.error}
          </p>
        {/if}

        <p class="fine">
          <b>There is no record of a run in flight anywhere in this system.</b> A pull is one
          synchronous POST and its journal record is appended when it ENDS, so a nine-minute
          backfill writes nothing to the journal for nine minutes. Everything in this panel is
          therefore derived from the store itself — the manifest's commit counter, its byte
          timestamp and its totals, subtracted between two answers. No timer stands in for progress
          and no percentage is shown that nobody could check.
        </p>
      </section>

      <!-- ════════════════════════════════════════════════════════════════
           3 · COVERAGE — the gap at a glance
           ════════════════════════════════════════════════════════════════ -->
      <section class="card" aria-label="Coverage">
        <div class="card-head">
          <h2>Coverage</h2>
          <span class="spacer"></span>
          {#if coverage}
            <span class="fine">
              target {coverage.from} → {coverage.to} · {n0(coverage.wanted)} months ·
              <code>docs/07-plan.md</code> R-2
            </span>
          {/if}
        </div>

        {#if payload.clock}
          <p class="why" role="alert">
            The server's clock could not name today — {payload.clock}. The target window ends at
            "today", so no grid is drawn. The store's own months are still listed below.
          </p>
        {:else if coverage}
          <div class="verdict">
            <span class="big" class:up={coverage.present === coverage.wanted} class:warn={coverage.present > 0 && coverage.present < coverage.wanted}>
              {n0(tMonths.v)}
            </span>
            <span class="sep">of {n0(coverage.wanted)} months hold anything at all</span>
            <span class="spacer"></span>
            <span class="fine">{n0(coverage.barsInTarget)} bars inside the window</span>
          </div>

          <div class="bar" aria-hidden="true">
            <i style="width:{(coverage.present / coverage.wanted) * 100}%"></i>
          </div>

          <div class="calendar" role="table" aria-label="Months held, by year">
            <div class="cal-head" role="row">
              <span class="yr" role="columnheader"></span>
              {#each MONTHS as m}<span class="mh" role="columnheader">{m}</span>{/each}
            </div>
            {#each coverage.years as year}
              <div class="cal-row" role="row">
                <span class="yr" role="rowheader">{year}</span>
                {#each MONTHS as _, i}
                  {@const month = `${year}-${String(i + 1).padStart(2, '0')}`}
                  {@const cell = coverage.byMonth.get(month)}
                  {#if !cell}
                    <span class="cell outside" role="cell" title="{month} — outside the target window"></span>
                  {:else if cell.im === 0}
                    <span class="cell gap" role="cell" title="{month} — nothing held">·</span>
                  {:else}
                    <span
                      class="cell held"
                      role="cell"
                      class:dense={cell.im / coverage.maxIm > 0.5}
                      style="--fill:{Math.max(0.18, cell.im / coverage.maxIm)}"
                      class:filling={pulse.filling?.month === month}
                      title="{month} — {n0(cell.im)} instrument-months, {n0(cell.bars)} bars"
                    >{n0(cell.im)}</span>
                  {/if}
                {/each}
              </div>
            {/each}
          </div>

          <div class="legend">
            <span><i class="sw gap"></i> nothing held</span>
            <span><i class="sw held q1"></i> few instruments</span>
            <span><i class="sw held"></i> many</span>
            <span><i class="sw outside"></i> outside the target window</span>
          </div>

          {#if coverage.gaps.length > 0}
            <h3>What is missing</h3>
            <ul class="gaps">
              {#each coverage.gaps.slice(0, 8) as g}
                <li>
                  <b>{g.from}{g.months > 1 ? ` … ${g.to}` : ''}</b>
                  <span class="tag bad">{n0(g.months)} month{g.months === 1 ? '' : 's'}</span>
                  <span class="fine">nothing held for {payload.store.feed}</span>
                </li>
              {/each}
            </ul>
            {#if coverage.gaps.length > 8}
              <p class="fine">{coverage.gaps.length - 8} shorter gap(s) not listed.</p>
            {/if}
          {:else}
            <p class="fine">Every month in the target window holds something.</p>
          {/if}

          {#if coverage.outside.length > 0}
            <p class="fine">
              {n0(coverage.outside.length)} month(s) held outside the target window
              ({coverage.outside.map((/** @type {any} */ m) => m.month).join(', ')}) — counted in the
              totals, drawn in no cell above.
            </p>
          {/if}

          {#if extremes}
            <h3>Extremes</h3>
            <div class="stats inner">
              <div class="stat">
                <span class="k">Thinnest month held</span>
                <span class="v down">{extremes.thinnest.month}</span>
                <span class="n">{n0(extremes.thinnest.instrument_months)} instruments · {n0(extremes.thinnest.bars)} bars</span>
              </div>
              <div class="stat">
                <span class="k">Fattest month held</span>
                <span class="v up">{extremes.fattest.month}</span>
                <span class="n">{n0(extremes.fattest.instrument_months)} instruments · {n0(extremes.fattest.bars)} bars</span>
              </div>
              <div class="stat">
                <span class="k">Fewest bars</span>
                <span class="v">{extremes.fewestBars.month}</span>
                <span class="n">{n0(extremes.fewestBars.bars)} bars</span>
              </div>
              <div class="stat">
                <span class="k">Most bars</span>
                <span class="v">{extremes.mostBars.month}</span>
                <span class="n">{n0(extremes.mostBars.bars)} bars</span>
              </div>
            </div>
          {/if}
        {:else}
          <p class="empty">
            The store answered, and it holds no month this build can name. Manifest:
            <code>{payload.store.manifest}</code>
          </p>
        {/if}
      </section>

      <!-- ════════════════════════════════════════════════════════════════
           2 · WHY DID IT FAIL — grouped, counted, and honest about the rest
           ════════════════════════════════════════════════════════════════ -->
      <section class="card" class:bad={faults.groups.length > 0} aria-label="Failures by cause">
        <div class="card-head">
          <h2>Failures, by cause</h2>
          <span class="spacer"></span>
          <span class="fine">{n0(faults.groups.length)} distinct cause(s) across {n0(runs.length)} run(s) read</span>
        </div>

        {#if faults.reasonsLost > 0}
          <p class="why" role="alert">
            <b>{n0(faults.membersFailed)} member-level failures are recorded, and
            {n0(faults.reasonsKept)} reason(s) survive on disk.</b>
            {n0(faults.reasonsLost)} reasons were never written. This is not a rendering gap and no
            page can recover them: <code>crates/api/src/audit.rs</code> stores the failure COUNT and
            keeps <code>failures.first()</code> — one name, the alphabetically first — in a
            {NOTE_BYTES}-byte field. The rest reach the POST's HTML reply
            (<code>take(5)</code>) and are dropped when the run returns.
          </p>
        {/if}

        {#if faults.groups.length === 0}
          <p class="empty">
            No run in the {n0(runs.length)} read is loud and none reports a failed member. That is
            the journal's own account, not an absence of checking.
          </p>
        {:else}
          <ul class="causes">
            {#each allCauses ? faults.groups : faults.groups.slice(0, TOP_CAUSES) as g (g.cause)}
              <li>
                <div class="cause-head">
                  <span class="tag bad">{n0(g.members)} member{g.members === 1 ? '' : 's'}</span>
                  <span class="tag">{n0(g.runs.length)} run{g.runs.length === 1 ? '' : 's'}</span>
                  <span class="cause">{g.cause}</span>
                </div>
                <div class="cause-foot">
                  <span class="fine">
                    first {istStamp(g.first)} · last {istStamp(g.last)}
                  </span>
                  {#if g.examples.size > 0}
                    <span class="fine">
                      named: {[...g.examples].slice(0, 6).join(', ')}{g.examples.size > 6
                        ? ` and ${g.examples.size - 6} more`
                        : ''}
                    </span>
                  {/if}
                  {#if g.cut}
                    <span class="tag warn" title="The server cut this note at {NOTE_BYTES} bytes. Two long reasons sharing a prefix are indistinguishable here.">
                      truncated at source
                    </span>
                  {/if}
                  <button class="link" type="button" onclick={() => { query = ''; outcomeFilter = 'loud'; picked = g.runs[0]; }}>
                    show the runs
                  </button>
                </div>
              </li>
            {/each}
          </ul>
          {#if faults.groups.length > TOP_CAUSES}
            <button class="btn" type="button" onclick={() => (allCauses = !allCauses)}>
              {allCauses
                ? `Show the ${TOP_CAUSES} largest only`
                : `Show all ${n0(faults.groups.length)} causes`}
            </button>
            <p class="fine">
              Ranked by members failed, then by how many runs hit them. The tail is mostly
              single-run refusals — a form filled in wrongly once — and they are all still here.
            </p>
          {/if}
        {/if}
      </section>

      <!-- ════════════════════════════════════════════════════════════════
           4 · OVER TIME
           ════════════════════════════════════════════════════════════════ -->
      <section class="card" aria-label="Runs over time">
        <div class="card-head">
          <h2>Over time</h2>
          <span class="spacer"></span>
          {#if timeline}
            <span class="fine">
              one column per {timeline.label} · {istStamp(timeline.from)} → {istStamp(timeline.to)}
            </span>
          {/if}
        </div>

        {#if !timeline}
          <p class="empty">No decodable run in the journal yet, so there is nothing to plot.</p>
        {:else}
          <div class="cols" role="img" aria-label="Runs per {timeline.label}, by outcome">
            {#each timeline.cols as c, i}
              {@const total = c.ok + c.mid + c.bad}
              <button
                class="col"
                class:hot={hotCol === i}
                type="button"
                onmouseenter={() => (hotCol = i)}
                onfocus={() => (hotCol = i)}
                onmouseleave={() => (hotCol = null)}
                onblur={() => (hotCol = null)}
                title="{istStamp(c.at)} — {total} run(s), {c.bad} loud, {n0(c.bars)} bars"
              >
                <span class="stack" style="height:{(total / timeline.peak) * 100}%">
                  {#if c.bad > 0}<i class="seg bad" style="flex:{c.bad}"></i>{/if}
                  {#if c.mid > 0}<i class="seg mid" style="flex:{c.mid}"></i>{/if}
                  {#if c.ok > 0}<i class="seg ok" style="flex:{c.ok}"></i>{/if}
                </span>
                {#if total === 0}<span class="nil"></span>{/if}
              </button>
            {/each}
          </div>
          <div class="axis">
            <span>{istStamp(timeline.from)}</span>
            <span class="spacer"></span>
            {#if hotCol != null && timeline.cols[hotCol]}
              {@const c = timeline.cols[hotCol]}
              <span class="readout">
                {istStamp(c.at)} · {c.ok} stored · {c.mid} empty · {c.bad} loud ·
                {n0(c.bars)} bars · {n0(c.failures)} member failures
              </span>
            {:else}
              <span class="fine">hover a column</span>
            {/if}
            <span class="spacer"></span>
            <span>{istStamp(timeline.to)}</span>
          </div>
          <div class="legend">
            <span><i class="sw ok"></i> stored</span>
            <span><i class="sw mid"></i> stored nothing</span>
            <span><i class="sw bad"></i> refused / not started / failed</span>
          </div>
        {/if}
      </section>

      <!-- ════════════════════════════════════════════════════════════════
           THE RUN LOG — searchable, filterable, virtualised
           ════════════════════════════════════════════════════════════════ -->
      <section class="card log" aria-label="Run log">
        <div class="card-head">
          <h2>Run log</h2>
          <span class="spacer"></span>
          <span class="fine">
            {n0(found.length)} shown of {n0(runs.length)} read · journal holds {n0(payload.journal.records)}
          </span>
        </div>

        <div class="controls">
          <input
            class="search"
            type="search"
            placeholder="Search outcome, target, window or reason — one Map probe per keystroke"
            bind:value={query}
            aria-label="Search runs"
          />
          <div class="tabs" role="group" aria-label="Filter by outcome">
            {#each [['all', 'All'], ['loud', 'Needs attention'], ['STORED', 'Stored'], ['STORED NOTHING', 'Stored nothing'], ['REFUSED', 'Refused'], ['NOT STARTED', 'Not started'], ['FAILED', 'Failed']] as [key, label]}
              <button
                class="utab"
                type="button"
                aria-pressed={outcomeFilter === key}
                onclick={() => (outcomeFilter = /** @type {any} */ (key))}
              >
                {label}<span class="n">{n0(counts[key] ?? 0)}</span>
              </button>
            {/each}
          </div>
        </div>

        <div class="lhead">
          <span>#</span>
          <span>When (IST)</span>
          <span>Outcome</span>
          <span>Asked for</span>
          <span>Window</span>
          <span class="r">Members</span>
          <span class="r">Rows read</span>
          <!-- SAME CORRECTION AS THE DETAIL PANEL BELOW, and it has to be made
               in both or the column and the cell it expands into disagree about
               what one number means. `bars_stored` is what the rung OFFERED;
               the written figure is `bars_committed` and this journal has no
               such field. -->
          <span
            class="r"
            title="What the rung that was pulled OFFERED. A re-pull offers every bar and writes none, because the file already holds them — measured here as bars_stored 16,43,341 against bars_committed 0. The written count is on the pull.run finished line in /logs."
            >Bars offered</span
          >
          <span class="r">Failed</span>
          <span class="r">Took</span>
        </div>

        {#if found.length === 0}
          <p class="empty">
            {#if runs.length === 0}
              The journal at <code>{payload.journal.path}</code> holds
              {n0(payload.journal.records)} record(s) and none was returned.
              {payload.runs_error ?? 'No run has been recorded against this store root yet.'}
            {:else}
              No run matches {query ? `“${query}”` : 'this filter'}. {n0(runs.length)} were searched.
            {/if}
          </p>
        {:else}
          <div class="vlist" bind:this={listEl} onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}>
            <div class="spacerV" style="height:{window_.total * ROW_H}px">
              <div class="shift" style="transform:translateY({window_.first * ROW_H}px)">
                {#each window_.rows as r (r.ordinal)}
                  <div
                    class="lrow"
                    class:bad={r.fault || isBad(r.outcome)}
                    class:mid={!r.fault && isMid(r.outcome)}
                    class:sel={picked?.ordinal === r.ordinal}
                    role="button"
                    tabindex="0"
                    onclick={() => (picked = picked?.ordinal === r.ordinal ? null : r)}
                    onkeydown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        picked = picked?.ordinal === r.ordinal ? null : r;
                      }
                    }}
                  >
                    {#if r.fault}
                      <span class="ord">{r.ordinal}</span>
                      <span class="fault" style="grid-column: 2 / -1">DAMAGED RECORD — {r.fault}</span>
                    {:else}
                      <span class="ord">{r.ordinal}</span>
                      <span>{istStamp(r.at)}</span>
                      <span class="oc" class:ok={isOk(r.outcome)} class:mid={isMid(r.outcome)} class:bad={isBad(r.outcome)}>{r.outcome}</span>
                      <span class="src" title={r.source}>{r.source || '—'}</span>
                      <span class="mono">{r.window}</span>
                      <span class="r">{n0(r.members)}</span>
                      <span class="r">{n0(r.rows_read)}</span>
                      <span class="r">{n0(r.bars_stored)}</span>
                      <span class="r" class:down={r.failures > 0}>{r.failures ? n0(r.failures) : '—'}</span>
                      <span class="r">{micros(r.took_micros)}</span>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          </div>
        {/if}

        <div class="logfoot">
          {#if pagesHeld < Math.min(MAX_PAGES, payload.journal.pages)}
            <button class="btn" type="button" onclick={loadOlder} disabled={loadingOlder}>
              {loadingOlder ? 'Reading…' : `Load the previous ${PAGE_ROWS}`}
            </button>
            <span class="fine">
              holding {n0(pagesHeld)} page(s) of {n0(payload.journal.pages)} · newest first
            </span>
          {:else if payload.journal.pages > MAX_PAGES}
            <span class="fine">
              This console holds at most {MAX_PAGES} pages ({n0(MAX_PAGES * PAGE_ROWS)} runs). The
              journal has {n0(payload.journal.pages)} — the rest are on disk at
              <code>{payload.journal.path}</code> and are not in this browser.
            </span>
          {:else}
            <span class="fine">Every record in the journal is loaded.</span>
          {/if}
        </div>
      </section>

      {#if picked && !picked.fault}
        <!-- ══ ONE RUN, IN FULL ══ -->
        <section class="card" aria-label="Selected run">
          <div class="card-head">
            <h2>Run #{picked.ordinal}</h2>
            <span class="spacer"></span>
            <button class="link" type="button" onclick={() => (picked = null)}>close</button>
          </div>
          <div class="stats inner">
            <div class="stat"><span class="k">Outcome</span><span class="v" class:up={isOk(picked.outcome)} class:down={isBad(picked.outcome)}>{picked.outcome}</span><span class="n">{picked.scope}</span></div>
            <div class="stat"><span class="k">Started</span><span class="v">{istTime(picked.at)}</span><span class="n">{istStamp(picked.at)} IST</span></div>
            <div class="stat"><span class="k">Took</span><span class="v">{micros(picked.took_micros)}</span><span class="n">so it ended near {istTime(picked.at + picked.took_micros / 1_000_000)}</span></div>
            <div class="stat"><span class="k">Members</span><span class="v">{n0(picked.members)}</span><span class="n">{n0(picked.failures)} failed</span></div>
            <div class="stat"><span class="k">Rows read</span><span class="v">{n0(picked.rows_read)}</span><span class="n">{n0(picked.rows_folded)} folded into an open bar</span></div>
            <!-- "OFFERED", NOT "STORED", AND THE DIFFERENCE IS A RUN THAT WROTE
                 NOTHING READING AS A RUN THAT WROTE EVERYTHING.

                 `bars_stored` counts what the pulled rung OFFERED. What reached
                 the file is `bars_committed`, and they diverge on every re-pull:
                 a second run over a window offers every bar and writes none,
                 because the file already holds them byte for byte — which §3
                 rule 5 requires of it.

                 MEASURED on this store, from the telemetry line D-0259 added:
                 `bars_stored: 1643341, bars_committed: 0`. Sixteen lakh bars
                 offered, ZERO written — and this cell called it "Bars stored"
                 and printed 16,43,341. A large success figure over an empty
                 write is the failure wearing a success's clothes that §4 bans.

                 THE FIGURE CANNOT BE CORRECTED HERE, ONLY THE CLAIM. The number
                 this row has is the offered one: `/audit.json` sends no
                 `bars_committed`, and it cannot without `audit::Record` growing
                 a field — which §4 makes a new file version at its own stride,
                 not an addition. So the label stops overstating and the caption
                 says where the written figure does live. -->
            <div class="stat"
              ><span class="k" title="What the rung that was pulled OFFERED. Not what reached the file — a re-pull offers every bar and writes none, because the file already holds them. The written figure is bars_committed, on the pull.run finished line in /logs; the journal this page reads carries no such field."
                >Bars offered</span
              ><span class="v">{n0(picked.bars_stored)}</span><span class="n"
                >{n0(picked.counted)} slices counted after · written count is in <a
                  class="link"
                  href="/logs?target=pull.run"
                  data-sveltekit-reload>the log</a
                ></span
              ></div
            >
          </div>
          <h3>What it says</h3>
          <p class="note">{picked.note || '(no note)'}</p>
          {#if picked.note_bytes > NOTE_BYTES}
            <p class="why">
              The server had {n0(picked.note_bytes)} bytes to say and the record holds {NOTE_BYTES}.
              The remaining {n0(picked.note_bytes - NOTE_BYTES)} bytes were never written to disk and
              cannot be recovered from here.
            </p>
          {/if}
          <h3>Rows dropped</h3>
          {#if picked.drops.every((/** @type {any} */ d) => d.rows === 0)}
            <p class="fine">None. Every row read was stored, folded, or is accounted for above.</p>
          {:else}
            <ul class="drops">
              {#each picked.drops as d}
                <li class:zero={d.rows === 0}><b>{n0(d.rows)}</b> <span>{d.reason}</span></li>
              {/each}
            </ul>
          {/if}
          <p class="fine">
            Asked for <code>{picked.source}</code>{picked.source_bytes > picked.source.length
              ? ` (cut from ${n0(picked.source_bytes)} bytes)`
              : ''} · window {picked.window}
          </p>
        </section>
      {/if}

      <!-- ══ WHERE EVERY NUMBER CAME FROM ══ -->
      <section class="card prov">
        <div class="card-head"><h2>Provenance</h2></div>
        <ul class="prov-list">
          <li>
            <code>GET /audit.json?feed={payload.store.feed}</code> — {load.ms} ms,
            {n0(payload.journal.records)} record(s) in the journal, page {payload.journal.page + 1} of
            {n0(payload.journal.pages)} at {PAGE_ROWS} per page.
          </li>
          <li>
            Journal: <code>{payload.journal.path}</code> — {n0(payload.journal.bytes)} bytes, one
            256-byte record per pull, appended and fsync-ed before the answer is rendered.
            {#if payload.journal.trouble}<b class="down"> {payload.journal.trouble}</b>{/if}
          </li>
          <li>
            Census: <code>{payload.store.manifest}</code> — {payload.store.state}, generation
            {n0(payload.store.generation ?? 0)}, {n0(payload.store.commits ?? 0)} committed entries.
            {#if payload.store.degraded}<b class="down"> {payload.store.degraded}</b>{/if}
          </li>
          <li>
            Read fresh per request, not from the process's startup snapshot — which is why these
            totals move during a backfill while <code>/store</code> (the server-rendered page) does
            not.
          </li>
          <li>
            Every figure here is <b>{payload.store.feed}</b>'s. This console never puts one feed's
            numbers beside another's.
          </li>
        </ul>
      </section>
    {:else if load.state !== 'error'}
      <!-- ══ SKELETON. A shape claim, not decoration. ══ -->
      <section class="card">
        <div class="card-head"><h2>Reading the journal</h2></div>
        <div class="stats inner">
          {#each Array.from({ length: 6 }) as _}
            <div class="stat">
              <span class="skel line" style="width:60%"></span>
              <span class="skel line" style="width:40%;height:18px"></span>
              <span class="skel line" style="width:80%"></span>
            </div>
          {/each}
        </div>
        <p class="fine">GET /audit.json?feed={feeds.active}</p>
      </section>
    {/if}
  </div>
</div>

<style>
  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s5);
    min-height: 0;
  }
  .head-sub {
    font-size: var(--fs-xs);
    color: var(--faint);
  }
  .card {
    background: var(--bg-2);
    border: 1px solid var(--line);
    border-radius: var(--r);
    padding: var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s4);
    min-width: 0;
  }
  .card.bad {
    border-color: color-mix(in srgb, var(--down) 40%, var(--line));
  }
  .card-head {
    display: flex;
    align-items: baseline;
    gap: var(--s4);
    flex-wrap: wrap;
  }
  .card-head h2 {
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }
  .card h3 {
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    margin-top: var(--s2);
  }
  .fine {
    font-size: var(--fs-xs);
    color: var(--faint);
    line-height: var(--lh-base);
  }
  .why {
    font-size: var(--fs-sm);
    color: var(--down);
    background: var(--down-soft);
    border-left: 2px solid var(--down);
    padding: var(--s4) var(--s5);
    border-radius: 0 var(--r1) var(--r1) 0;
    line-height: var(--lh-base);
  }
  .spacer {
    flex: 1;
  }
  code {
    font-family: var(--mono);
    font-size: 0.92em;
    color: var(--ink-2);
    word-break: break-all;
  }

  /* ---- the live strip ------------------------------------------------- */
  .pulse.moving {
    border-color: color-mix(in srgb, var(--up) 45%, var(--line));
  }
  .verdict {
    display: flex;
    align-items: baseline;
    gap: var(--s4);
    flex-wrap: wrap;
    font-size: var(--fs-sm);
    color: var(--dim);
    line-height: var(--lh-base);
  }
  .verdict .big {
    font-family: var(--mono);
    font-size: var(--fs-xl);
    font-weight: var(--w-heavy);
    letter-spacing: -0.5px;
    color: var(--ink);
    display: inline-flex;
    align-items: center;
    gap: var(--s3);
  }
  .verdict .sep {
    color: var(--faint);
  }
  .verdict b {
    color: var(--ink);
    font-variant-numeric: tabular-nums;
  }
  .beacon {
    width: 9px;
    height: 9px;
    border-radius: var(--r-full);
    background: var(--up);
    box-shadow: 0 0 0 0 var(--up-soft);
  }
  .stats.inner {
    border-bottom: 0;
    border: 1px solid var(--line);
    border-radius: var(--r2);
    overflow: hidden;
  }
  .stat .v em {
    font-style: normal;
    font-size: var(--fs-xs);
    color: var(--faint);
    font-weight: var(--w-mid);
  }
  .spark {
    width: 46px;
    height: 12px;
    vertical-align: middle;
    margin-right: var(--s2);
  }
  .spark path {
    fill: none;
    stroke: var(--acc);
    stroke-width: 1.5;
    vector-effect: non-scaling-stroke;
  }

  /* ---- coverage ------------------------------------------------------- */
  .bar {
    height: 6px;
    background: var(--well);
    border-radius: var(--r-full);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--acc);
    border-radius: var(--r-full);
  }
  .calendar {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-x: auto;
  }
  .cal-head,
  .cal-row {
    display: grid;
    grid-template-columns: 42px repeat(12, minmax(38px, 1fr));
    gap: 2px;
    align-items: stretch;
  }
  .yr {
    font-family: var(--mono);
    font-size: var(--fs-mini);
    color: var(--faint);
    display: flex;
    align-items: center;
  }
  .mh {
    font-size: var(--fs-micro);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    text-align: center;
  }
  .cell {
    height: 24px;
    border-radius: var(--r1);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--mono);
    font-size: var(--fs-micro);
    font-variant-numeric: tabular-nums;
  }
  .cell.gap {
    background: var(--down-soft);
    color: var(--down);
    border: 1px dashed color-mix(in srgb, var(--down) 45%, transparent);
  }
  .cell.held {
    /* THE INK IS CHOSEN, NOT ASSUMED. A pale cell with white digits on it is a
       number nobody can read, which is what a fixed `--on-acc` gave for every
       month below half the busiest one. */
    /* THE CEILING IS A THEME TOKEN, NOT 72% HARDCODED. Light and dark ramp in
       opposite directions, so the value that keeps one ink legible differs per
       theme — 72% in light, 50% in dark. The whole derivation, with the
       measurements, is on `--heat-ceiling` in `theme.css`. */
    background: color-mix(
      in srgb,
      var(--acc) calc(var(--fill) * var(--heat-ceiling)),
      var(--well)
    );
    color: var(--ink);
    border: 1px solid transparent;
  }
  /* THE WHITE-INK BRANCH IS GONE, BECAUSE IT COULD NEVER PASS AT ANY FILL.
     The comment above got the principle right and the threshold wrong. The
     background tops out at `--fill * 72%` of `--acc` over `--well`, so the
     BUSIEST month in the grid -- `--fill: 1` -- still only reaches 73.6% of
     the accent. Measured on the running page, that cell renders `#4b9eb1`,
     and `--on-acc` white on it is 3.08:1 against the 4.5 these digits need.
     Not "pale cells are the problem": every cell was, the densest included,
     because the ceiling is 72% and no cell is ever darker than that.
     `--ink` on that same densest cell measures 5.82:1, and on the palest it
     is better still, so one ink covers the whole ramp. If a dark ink ever
     stops working it will be because the ceiling was raised, and that is the
     thing to check first. */
  .cell.held.filling {
    outline: 2px solid var(--up);
    outline-offset: -2px;
  }
  .cell.outside {
    background: var(--well);
    border: 1px solid var(--line-soft);
  }
  .legend {
    display: flex;
    gap: var(--s5);
    flex-wrap: wrap;
    font-size: var(--fs-mini);
    color: var(--faint);
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: var(--s2);
  }
  .sw {
    width: 11px;
    height: 11px;
    border-radius: 3px;
    display: inline-block;
  }
  .sw.gap {
    background: var(--down-soft);
    border: 1px dashed var(--down);
  }
  .sw.held {
    background: var(--acc);
  }
  .sw.held.q1 {
    background: color-mix(in srgb, var(--acc) 25%, var(--well));
  }
  .sw.outside {
    background: var(--well);
    border: 1px solid var(--line);
  }
  .sw.ok {
    background: var(--up);
  }
  .sw.mid {
    background: var(--warn);
  }
  .sw.bad {
    background: var(--down);
  }
  .gaps {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--s2);
  }
  .gaps li {
    display: flex;
    align-items: center;
    gap: var(--s4);
    font-family: var(--mono);
    font-size: var(--fs-sm);
    padding: var(--s2) var(--s4);
    background: var(--panel-2);
    border-radius: var(--r1);
  }

  /* ---- causes --------------------------------------------------------- */
  .causes {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }
  .causes li {
    border: 1px solid var(--line);
    border-radius: var(--r2);
    padding: var(--s4) var(--s5);
    background: var(--panel);
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }
  .cause-head {
    display: flex;
    align-items: baseline;
    gap: var(--s4);
    flex-wrap: wrap;
  }
  .cause {
    font-family: var(--mono);
    font-size: var(--fs-sm);
    color: var(--ink);
    word-break: break-word;
  }
  .cause-foot {
    display: flex;
    align-items: center;
    gap: var(--s5);
    flex-wrap: wrap;
  }

  /* ---- over time ------------------------------------------------------ */
  .cols {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    height: 120px;
    padding-top: var(--s3);
    overflow-x: auto;
  }
  .col {
    flex: 1 0 6px;
    min-width: 6px;
    height: 100%;
    background: none;
    border: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    cursor: pointer;
  }
  .col .stack {
    display: flex;
    flex-direction: column;
    width: 100%;
    min-height: 2px;
    border-radius: 2px 2px 0 0;
    overflow: hidden;
  }
  .col .seg {
    display: block;
    width: 100%;
  }
  .col .seg.ok {
    background: var(--up);
  }
  .col .seg.mid {
    background: var(--warn);
  }
  .col .seg.bad {
    background: var(--down);
  }
  .col .nil {
    height: 2px;
    background: var(--line);
  }
  .col.hot .stack {
    filter: brightness(1.25);
  }
  .axis {
    display: flex;
    gap: var(--s4);
    font-size: var(--fs-mini);
    color: var(--faint);
    font-family: var(--mono);
    align-items: center;
  }
  .readout {
    color: var(--ink-2);
  }

  /* ---- the log -------------------------------------------------------- */
  .log {
    min-height: 420px;
  }
  .controls {
    display: flex;
    gap: var(--s4);
    flex-wrap: wrap;
    align-items: center;
  }
  .controls .search {
    flex: 1 1 280px;
    width: auto;
  }
  .lhead,
  .lrow {
    display: grid;
    grid-template-columns: 52px 148px 118px minmax(0, 1.4fr) 152px 74px 96px 104px 72px 72px;
    gap: var(--s4);
    align-items: center;
    padding: 0 var(--s4);
  }
  .lhead {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    border-bottom: 1px solid var(--line);
    padding-bottom: var(--s2);
  }
  .lhead .r,
  .lrow .r {
    text-align: right;
  }
  .vlist {
    flex: 1;
    min-height: 300px;
    max-height: 46vh;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .spacerV {
    position: relative;
  }
  .shift {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
  }
  .lrow {
    height: 30px;
    font-size: var(--fs-sm);
    font-variant-numeric: tabular-nums;
    border-bottom: 1px solid var(--line-soft);
    cursor: pointer;
    white-space: nowrap;
  }
  .lrow:hover {
    background: var(--panel-2);
  }
  .lrow.sel {
    background: var(--acc-soft);
    box-shadow: inset 2px 0 0 var(--acc);
  }
  .lrow.bad {
    background: color-mix(in srgb, var(--down) 5%, transparent);
  }
  /* NO CELL WIDENS THE GRID. A timestamp is 118px at 11px type and the narrow
     layout's column is 92px, which is how this page put 50px of horizontal
     scroll on a 375px screen. Clipping is the floor; the columns above are
     sized so it is not reached. */
  .lrow > span,
  .lhead > span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .lrow .ord {
    color: var(--faint);
    font-family: var(--mono);
  }
  .lrow .src,
  .lrow .mono {
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--mono);
    font-size: var(--fs-xs);
    color: var(--dim);
  }
  .lrow .oc {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: 0.06em;
  }
  .lrow .oc.ok {
    color: var(--up);
  }
  .lrow .oc.mid {
    color: var(--warn);
  }
  .lrow .oc.bad {
    color: var(--down);
  }
  .lrow .fault {
    color: var(--down);
    font-size: var(--fs-xs);
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .logfoot {
    display: flex;
    align-items: center;
    gap: var(--s4);
    flex-wrap: wrap;
  }
  .note {
    font-family: var(--mono);
    font-size: var(--fs-sm);
    background: var(--panel-2);
    padding: var(--s4) var(--s5);
    border-radius: var(--r1);
    word-break: break-word;
  }
  .drops {
    list-style: none;
    display: flex;
    gap: var(--s5);
    flex-wrap: wrap;
  }
  .drops li {
    font-size: var(--fs-xs);
    display: flex;
    gap: var(--s2);
    align-items: baseline;
  }
  .drops li b {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
  }
  .drops li.zero {
    color: var(--faint);
  }
  .prov-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    font-size: var(--fs-xs);
    color: var(--faint);
    line-height: var(--lh-base);
  }

  @media (max-width: 1180px) {
    .lhead,
    .lrow {
      grid-template-columns: 44px 132px 104px minmax(0, 1fr) 130px 64px 84px 92px 60px 60px;
      font-size: var(--fs-xs);
    }
  }
  @media (max-width: 760px) {
    .scroll {
      padding: var(--s4);
    }
    /* NARROW IS A LAYOUT, NOT A SCROLLBAR. Measured at 375: the outcome tabs
       did not wrap and a nine-digit Indian-grouped figure did not fit a 92px
       stat, so this page pushed 50px of horizontal scroll onto a shell that
       already had its own. Both are fixed here; the shell's own minimum width
       is in theme.css and is not this page's to change. */
    .stats {
      grid-template-columns: repeat(auto-fit, minmax(128px, 1fr));
    }
    .stat .v {
      font-size: 15px;
      letter-spacing: -0.3px;
      overflow-wrap: anywhere;
    }
    .controls :global(.tabs) {
      flex-wrap: wrap;
      flex: 1 1 100%;
      min-width: 0;
    }
    .verdict .big {
      font-size: var(--fs-lg);
    }
    .lhead,
    .lrow {
      grid-template-columns: 34px 124px 78px minmax(0, 1fr) 62px;
      gap: var(--s3);
    }
    .lhead span:nth-child(n + 6),
    .lrow span:nth-child(n + 6) {
      display: none;
    }
    .cal-head,
    .cal-row {
      grid-template-columns: 34px repeat(12, minmax(26px, 1fr));
    }
    .cell {
      font-size: 8px;
    }
  }

  /* MOTION — every animation on this page is inside this guard. */
  @media (prefers-reduced-motion: no-preference) {
    .card {
      animation: rise var(--d-enter) var(--ease-out) both;
    }
    .beacon {
      animation: beat 1.6s var(--ease-in-out) infinite;
    }
    .cell.held {
      transition: background var(--d-state) var(--ease-out);
    }
    .cell.held.filling {
      animation: grew var(--d-flash) var(--ease-out);
    }
    .bar i {
      transition: width var(--d-panel) var(--ease-out);
    }
    .col .stack {
      transition: filter var(--d-hover) var(--ease-out);
    }
    .lrow {
      transition: background var(--d-hover) var(--ease-out);
    }
    @keyframes rise {
      from {
        opacity: 0;
        transform: translateY(6px);
      }
      to {
        opacity: 1;
        transform: none;
      }
    }
    @keyframes beat {
      0% {
        box-shadow: 0 0 0 0 var(--up-soft);
      }
      70% {
        box-shadow: 0 0 0 7px transparent;
      }
      100% {
        box-shadow: 0 0 0 0 transparent;
      }
    }
    @keyframes grew {
      0% {
        background: var(--up);
      }
      100% {
        background: color-mix(in srgb, var(--acc) calc(var(--fill) * 72%), var(--well));
      }
    }
  }
</style>

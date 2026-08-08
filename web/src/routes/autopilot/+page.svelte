<script>
  /**
   * THE AUTOPILOT — what it is doing right now, and the proof it is doing it.
   *
   * The operator's requirement is one click and no manual step. The failure
   * mode of that requirement is not a crash; it is a process that is up,
   * answering, and quietly doing nothing, which from the outside is
   * indistinguishable from a process that is working. This page exists to make
   * those two states distinguishable at a glance. It is the difference between
   * "automated" and "hung".
   *
   * ---------------------------------------------------------------------------
   * TWO SOURCES, AND THEY ARE NEVER MERGED
   * ---------------------------------------------------------------------------
   *
   * 1. `/autopilot.json` — what the autopilot SAYS it is doing. Live, small,
   *    polled every 2s. It is the only thing that can answer "what is in
   *    flight", "why did it stop" and "what failed".
   *
   * 2. `/store.json` — what the store ACTUALLY HOLDS. The census, measured off
   *    disk, 400 KB on a real store, polled every 30s. It owes the autopilot
   *    nothing and would report the same numbers if the autopilot were deleted.
   *
   * Coverage is drawn from (2), never from (1). A progress bar fed by the
   * process reporting its own progress is a progress bar that reads 100% when
   * the process is lying to itself. Every figure on this page is labelled with
   * which of the two it came from, because "reported" and "measured" are not
   * the same claim and CLAUDE.md §3 rule 6 says so.
   *
   * ---------------------------------------------------------------------------
   * WHEN THE ENDPOINT IS NOT THERE
   * ---------------------------------------------------------------------------
   *
   * `/autopilot.json` is a route the served binary either has or does not have.
   * When it does not, this page says exactly that, names what is therefore
   * unknown, and STILL renders the measured coverage — because the census is
   * true either way. What it must never do is render a calm empty panel: a
   * missing autopilot and an idle autopilot look identical in a blank, and
   * CLAUDE.md §4 bans a fallback that hides a failure.
   *
   * The same applies to a 200 with the wrong shape. The reader below validates
   * the payload field by field and reports the FIRST field that was not what
   * the contract says, by name. A page that silently renders `undefined` as an
   * em-dash is a page that turns a broken server into a design quirk.
   *
   * ---------------------------------------------------------------------------
   * THE CONTRACT — docs/05-decisions.md D-0057
   * ---------------------------------------------------------------------------
   *
   *   GET  /autopilot.json    the state, the current cell, the target, failures
   *   POST /autopilot/pause   stop after the cell in flight; never mid-write
   *   POST /autopilot/resume  continue from the census, not from a variable
   *
   * Pause is "finish this cell and stop", not "abort". An aborted write is a
   * half-month the append-only store can never correct, and the whole reason
   * the backfill runs oldest-first is that the store cannot prepend.
   */
  import { feeds } from '$lib/feeds.svelte.js';

  /** The live state. Small payload, so a short period is cheap. */
  const TICK_MS = 2000;
  /** The census. ~400 KB on a real store — polled slowly, on purpose. */
  const CENSUS_MS = 30000;

  /* ======================================================================
     THE CONTRACT READER
     ----------------------------------------------------------------------
     Strict by construction. Every refusal below names the field, because
     "the autopilot looks wrong" is not a bug report and "the payload has no
     `target.from`" is.
     ====================================================================== */

  /** The states the contract defines. Anything else is a contract violation. */
  const STATES = ['starting', 'running', 'waiting', 'paused', 'halted', 'complete'];

  /** States in which a REASON is mandatory. A stop with no why is banned. */
  const MUST_EXPLAIN = ['paused', 'halted'];

  function str(v) {
    return typeof v === 'string' && v.length > 0 ? v : null;
  }
  function num(v) {
    return typeof v === 'number' && Number.isFinite(v) ? v : null;
  }

  /**
   * Validate one payload. Returns `{ ok: true, value }` or `{ ok: false, why }`.
   *
   * Deliberately not a schema library: the shape is nine fields and a list, and
   * a dependency that turns a named refusal into `/target/from: required` is a
   * worse message for the person who has to fix it.
   */
  function readState(raw) {
    if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
      return { ok: false, why: 'the body is not a JSON object' };
    }
    const state = str(raw.state);
    if (!state) return { ok: false, why: 'no `state` field' };
    if (!STATES.includes(state)) {
      return { ok: false, why: `\`state\` is "${state}", which is not one of ${STATES.join(', ')}` };
    }
    const why = str(raw.why);
    if (MUST_EXPLAIN.includes(state) && !why) {
      return {
        ok: false,
        why: `\`state\` is "${state}" and \`why\` is empty — a stop that does not name its reason is the failure CLAUDE.md §4 bans`
      };
    }

    const t = raw.target;
    if (t === null || typeof t !== 'object') return { ok: false, why: 'no `target` object' };
    const target = {
      from: str(t.from),
      to: str(t.to),
      instruments: num(t.instruments),
      timeframe: str(t.timeframe),
      feed: str(t.feed)
    };
    for (const k of ['from', 'to', 'instruments']) {
      if (target[k] === null) return { ok: false, why: `no \`target.${k}\`` };
    }

    // `now` is null when nothing is in flight, and that is a legal answer —
    // but only for a state that is not running. A "running" autopilot with
    // nothing in flight is exactly the hang this page exists to expose.
    let now = null;
    if (raw.now !== null && raw.now !== undefined) {
      const n = raw.now;
      if (typeof n !== 'object') return { ok: false, why: '`now` is neither null nor an object' };
      // `elapsed_ms` IS A DURATION MEASURED BY THE SERVER, not a timestamp.
      // A start instant would have to be compared against the browser's clock,
      // and two clocks that disagree by a minute render a cell that has been
      // running for "-00:47". A duration the server measured has no such term.
      now = {
        instrument: str(n.instrument),
        month: str(n.month),
        timeframe: str(n.timeframe),
        feed: str(n.feed),
        elapsed_ms: num(n.elapsed_ms),
        index: num(n.index),
        of: num(n.of)
      };
      for (const k of ['instrument', 'month']) {
        if (now[k] === null) return { ok: false, why: `no \`now.${k}\`` };
      }
    }
    if (state === 'running' && now === null) {
      return {
        ok: false,
        why: '`state` is "running" and `now` is null — nothing is in flight, so it is not running'
      };
    }

    // `failures` IS MANDATORY, and defaulting it to `[]` was the one fallback
    // that had crept into this reader. A payload with no `failures` key would
    // have rendered "Nothing has failed" — the exact sentence CLAUDE.md §4
    // bans, and one the panel below already claims cannot happen. A missing
    // list and an empty list are different facts; only the second is good news.
    if (!Array.isArray(raw.failures)) {
      return {
        ok: false,
        why: 'no `failures` array — an absent failure list rendered as "none" is a fallback that hides a failure'
      };
    }
    const failures = raw.failures;
    return {
      ok: true,
      value: {
        state,
        why,
        cursor: str(raw.cursor),
        target,
        now,
        waiting_ms: num(raw.waiting_ms),
        absorbed_ms: num(raw.absorbed_ms),
        journal: str(raw.journal),
        failures: failures.map((f) => ({
          instrument: str(f?.instrument) ?? '(unnamed)',
          month: str(f?.month) ?? '(no month)',
          why: str(f?.why) ?? '(the payload carried no reason — that is itself the bug)',
          at: str(f?.at)
        }))
      }
    };
  }

  /* ======================================================================
     POLL 1 — the live state
     ====================================================================== */

  // `link` is how this page reaches the server, and it is four states rather
  // than a boolean because they need four different sentences:
  //   probing  — first read has not landed
  //   ok       — a valid payload
  //   absent   — 404. The binary has no autopilot.
  //   broken   — it answered, and the answer was not the contract.
  let link = $state({ kind: 'probing', why: null, at: 0, ms: 0 });
  let ap = $state(null);
  let now = $state(Date.now());

  async function tick() {
    const t0 = performance.now();
    try {
      const r = await fetch('/autopilot.json', { cache: 'no-store' });
      const ms = Math.round(performance.now() - t0);
      if (r.status === 404) {
        link = {
          kind: 'absent',
          why: 'GET /autopilot.json answered 404 — this binary has no autopilot route',
          at: Date.now(),
          ms
        };
        ap = null;
        return;
      }
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      // THE CONTENT-TYPE CHECK IS NOT PEDANTRY. In development the Vite proxy
      // forwards a fixed list of routes; a route missing from that list is
      // answered by the dev server's own HTML fallback with a 200. `r.ok`
      // cannot see it, and a green light wired to an HTML page is worse than
      // a red one.
      const ct = r.headers.get('content-type') ?? '';
      if (!ct.includes('json')) {
        throw new Error(
          `answered ${ct || 'no content-type'}, not JSON — the API is not behind this route (in development, add /autopilot.json to the proxy list in web/vite.config.js)`
        );
      }
      const parsed = readState(await r.json());
      if (!parsed.ok) {
        link = {
          kind: 'broken',
          why: `the autopilot answered, and the payload does not match the contract: ${parsed.why}`,
          at: Date.now(),
          ms
        };
        return;
      }
      ap = parsed.value;
      link = { kind: 'ok', why: null, at: Date.now(), ms };
    } catch (e) {
      link = {
        kind: 'broken',
        why: String(e?.message ?? e),
        at: Date.now(),
        ms: Math.round(performance.now() - t0)
      };
    }
  }

  $effect(() => {
    tick();
    const id = setInterval(tick, TICK_MS);
    // A hidden tab is a tab nobody is reading. Polling it is a request per two
    // seconds spent on a screen that is not on screen.
    const wake = () => document.visibilityState === 'visible' && tick();
    document.addEventListener('visibilitychange', wake);
    return () => {
      clearInterval(id);
      document.removeEventListener('visibilitychange', wake);
    };
  });

  // The elapsed clock ticks only while something is genuinely in flight.
  $effect(() => {
    if (!ap?.now) return;
    now = Date.now();
    const id = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(id);
  });

  /* ======================================================================
     POLL 2 — the census, which is the truth
     ----------------------------------------------------------------------
     One fetch, then ONE PASS folding cells into per-month totals. The fold is
     O(cells returned) and happens on arrival, not on render: a derived value
     recomputed per keystroke over 60,000 cells is a page that stutters for a
     number that changed thirty seconds ago.
     ====================================================================== */

  let census = $state({ kind: 'probing', why: null, at: 0, months: [], cells: 0, rows: 0 });

  async function readCensus(feed) {
    if (!feed) return;
    try {
      const r = await fetch(`/store.json?feed=${encodeURIComponent(feed)}`, { cache: 'no-store' });
      if (!r.ok) throw new Error(`HTTP ${r.status}`);
      const ct = r.headers.get('content-type') ?? '';
      if (!ct.includes('json')) throw new Error(`answered ${ct || 'no content-type'}, not JSON`);
      const rowsIn = await r.json();
      if (!Array.isArray(rowsIn)) throw new Error('the body is not a JSON array');

      const byMonth = new Map();
      let cells = 0;
      let rows = 0;
      for (const c of rowsIn) {
        const m = typeof c?.month === 'string' ? c.month : null;
        if (!m) continue;
        const n = typeof c.rows === 'number' ? c.rows : 0;
        let acc = byMonth.get(m);
        if (!acc) byMonth.set(m, (acc = { month: m, cells: 0, rows: 0 }));
        acc.cells += 1;
        acc.rows += n;
        cells += 1;
        rows += n;
      }
      census = {
        kind: 'ok',
        why: null,
        at: Date.now(),
        months: [...byMonth.values()].sort((a, b) => (a.month < b.month ? -1 : 1)),
        cells,
        rows
      };
    } catch (e) {
      census = {
        kind: 'broken',
        why: String(e?.message ?? e),
        at: Date.now(),
        months: [],
        cells: 0,
        rows: 0
      };
    }
  }

  // The feed is part of the question: the brokers do not hold the same
  // instruments, so a census is per feed and re-reads when the selection moves.
  const feed = $derived(ap?.target?.feed ?? feeds.active);
  $effect(() => {
    const f = feed;
    if (!f) return;
    readCensus(f);
    const id = setInterval(() => readCensus(f), CENSUS_MS);
    return () => clearInterval(id);
  });

  /* ======================================================================
     THE MONTH LADDER — oldest at the top, because that is the order it works
     ----------------------------------------------------------------------
     The backfill runs oldest -> newest and the ladder is drawn the same way,
     so "where is it" is a position on the page rather than an inference. The
     store is append-only with monotonic timestamps and one file per month: a
     later month written first permanently blocks the earlier days inside that
     same month-file. The ordering is not a preference.
     ====================================================================== */

  function monthsBetween(from, to) {
    // Bounded by construction: a target that is somehow reversed or absurd
    // yields an empty ladder rather than an infinite loop in a render path.
    const out = [];
    const [fy, fm] = from.split('-').map(Number);
    const [ty, tm] = to.split('-').map(Number);
    if (!Number.isFinite(fy) || !Number.isFinite(tm)) return out;
    let y = fy;
    let m = fm;
    for (let guard = 0; guard < 1200; guard += 1) {
      if (y > ty || (y === ty && m > tm)) break;
      out.push(`${y}-${String(m).padStart(2, '0')}`);
      m += 1;
      if (m > 12) {
        m = 1;
        y += 1;
      }
    }
    return out;
  }

  const held = $derived(new Map(census.months.map((m) => [m.month, m])));

  /**
   * The ladder. When the target is known it runs the whole span, so a month
   * with nothing in it is a VISIBLE GAP rather than a row that was never
   * drawn. When it is not known, only the observed months appear — and the
   * page says so, rather than inventing a floor.
   */
  const ladder = $derived(
    ap?.target
      ? monthsBetween(ap.target.from, ap.target.to).map((m) => ({
          month: m,
          ...(held.get(m) ?? { cells: 0, rows: 0 })
        }))
      : census.months
  );

  const per = $derived(ap?.target?.instruments ?? null);
  const monthsDone = $derived(per ? ladder.filter((m) => m.cells >= per).length : null);
  const monthsPartial = $derived(per ? ladder.filter((m) => m.cells > 0 && m.cells < per).length : null);
  const cellsTarget = $derived(per ? per * ladder.length : null);

  /* ======================================================================
     THE CONTROL
     ====================================================================== */

  let control = $state({ busy: false, note: null, bad: false });

  async function send(path, label) {
    control = { busy: true, note: null, bad: false };
    try {
      const r = await fetch(path, { method: 'POST' });
      if (r.status === 404) {
        throw new Error(`POST ${path} answered 404 — this binary has no ${label} control`);
      }
      if (!r.ok) throw new Error(`POST ${path} answered HTTP ${r.status}`);
      control = { busy: false, note: `${label} accepted`, bad: false };
      await tick();
    } catch (e) {
      // A control that failed and said nothing is a button that lies. The
      // reason goes on the page beside the button that produced it.
      control = { busy: false, note: String(e?.message ?? e), bad: true };
    }
  }

  /* ======================================================================
     FORMATTING
     ====================================================================== */

  const nf = new Intl.NumberFormat('en-IN');
  const n = (v) => (typeof v === 'number' && Number.isFinite(v) ? nf.format(v) : '—');

  function clock(ms) {
    if (typeof ms !== 'number' || !Number.isFinite(ms) || ms < 0) return '—';
    const s = Math.floor(ms / 1000);
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const r = s % 60;
    const pad = (x) => String(x).padStart(2, '0');
    return h ? `${h}:${pad(m)}:${pad(r)}` : `${pad(m)}:${pad(r)}`;
  }

  const TONE = {
    starting: 'acc',
    running: 'up',
    waiting: 'warn',
    paused: 'warn',
    halted: 'down',
    complete: 'up'
  };

  const SAYS = {
    starting: 'Opening the store and reading the census',
    running: 'Fetching',
    waiting: 'Waiting on the rate governor — absorbing throttle rather than failing',
    paused: 'Paused by the operator',
    halted: 'Halted',
    complete: 'The target span is complete'
  };

  /**
   * How long the current cell has been running.
   *
   * The server's own measurement, plus the local drift since that measurement
   * arrived. Only the drift is the browser's guess, and it is bounded by the
   * poll period — so the number ticks smoothly between polls without ever
   * being derived from a comparison of two machines' clocks.
   */
  const elapsed = $derived(
    ap?.now && ap.now.elapsed_ms !== null ? ap.now.elapsed_ms + Math.max(0, now - link.at) : null
  );
</script>

<div class="pane">
  <div class="pane-head">
    <span class="pane-title">Autopilot</span>
    {#if ap}
      <span class="tag {TONE[ap.state] ?? ''}">{ap.state}</span>
      {#if ap.target.feed}<span class="tag acc">{ap.target.feed}</span>{/if}
      {#if ap.target.timeframe}<span class="tag">{ap.target.timeframe}</span>{/if}
    {:else if link.kind === 'absent'}
      <span class="tag down">not in this binary</span>
    {:else if link.kind === 'broken'}
      <span class="tag down">contract broken</span>
    {:else}
      <span class="tag">probing</span>
    {/if}

    <span class="spacer"></span>

    <span class="status" title="Round trip to /autopilot.json, measured. Last read {link.at ? new Date(link.at).toLocaleTimeString() : 'never'}.">
      <span
        class="dot"
        class:up={link.kind === 'ok'}
        class:down={link.kind === 'absent' || link.kind === 'broken'}
        class:warn={link.kind === 'probing'}
        class:live={ap?.state === 'running'}
      ></span>
      <span class="txt">/autopilot.json</span>
      <span class="ms">{link.ms} ms</span>
    </span>
  </div>

  <!-- ================================================================
       THE LOUD BANNER. It is the first thing on the page when the thing
       this page is about is not there.
       ================================================================ -->
  {#if link.kind === 'absent' || link.kind === 'broken'}
    <div class="alarm" role="alert">
      <div class="alarm-h">
        <span class="tag down">{link.kind === 'absent' ? 'no autopilot' : 'contract violated'}</span>
        <strong>
          {link.kind === 'absent'
            ? 'This process is serving pages, but nothing is driving the pull.'
            : 'The autopilot answered, and the answer cannot be trusted.'}
        </strong>
      </div>
      <p class="alarm-why">{link.why}</p>
      <p class="alarm-what">
        <span class="lbl">What is therefore unknown</span>
        What is in flight, what has been attempted, what failed and why, and whether anything is
        advancing at all. The coverage below is still real — it is read from the store's own census
        and it would report the same numbers if this route never existed — but a number that is not
        moving cannot be told apart from a number that is finished.
      </p>
      <p class="alarm-what">
        <span class="lbl">What still has to be done by hand</span>
        Until <code>GET /autopilot.json</code> exists, the backfill advances only for as long as
        somebody drives it. Oldest month first, always:
        <b>a later month written first permanently blocks the earlier days inside that same
        month-file.</b>
      </p>
    </div>
  {/if}

  <!-- ================================================================
       THE FIVE NUMBERS. Each says where it came from.
       ================================================================ -->
  <div class="stats">
    <div class="stat">
      <span class="k">State</span>
      <span class="v {ap ? (TONE[ap.state] ?? '') : 'down'}">{ap ? ap.state : 'unknown'}</span>
      <span class="n">{ap ? (SAYS[ap.state] ?? '') : 'reported by /autopilot.json'}</span>
    </div>

    <div class="stat">
      <span class="k">In flight</span>
      <span class="v">{ap?.now ? clock(elapsed) : '—'}</span>
      <span class="n">
        {#if ap?.now}
          {ap.now.instrument} · {ap.now.month}
        {:else if ap}
          nothing in flight
        {:else}
          unknown — the route did not answer
        {/if}
      </span>
    </div>

    <div class="stat">
      <span class="k">Months complete</span>
      <span class="v">{monthsDone === null ? '—' : n(monthsDone)}</span>
      <!-- COMPLETE MEANS EVERY INSTRUMENT, and the label says so. A month
           holding 481 of 774 is genuinely not done, and rounding that up to
           "complete" would be the silent skip CLAUDE.md §4 bans — but "0"
           beside "10 partial" reads as broken unless the denominator is
           named. Audit's grid uses the same denominator and paints those
           months PARTIAL for the same reason. -->
      <span class="n">
        {#if monthsDone === null}
          needs target.instruments from /autopilot.json
        {:else}
          of {n(ladder.length)}{monthsPartial ? ` · ${n(monthsPartial)} partial` : ''} · complete =
          all {n(per)} instruments held
        {/if}
      </span>
    </div>

    <div class="stat">
      <span class="k">Instrument-months held</span>
      <span class="v">{n(census.cells)}</span>
      <span class="n">
        {cellsTarget ? `of ${n(cellsTarget)} · ` : ''}measured from /store.json
      </span>
    </div>

    <div class="stat">
      <span class="k">Bars stored</span>
      <span class="v">{n(census.rows)}</span>
      <span class="n">measured from /store.json{feed ? ` · ${feed}` : ''}</span>
    </div>

    <div class="stat">
      <span class="k">Failures</span>
      <span class="v" class:down={(ap?.failures?.length ?? 0) > 0}>
        {ap ? n(ap.failures.length) : '—'}
      </span>
      <span class="n">{ap ? 'named below, never counted only' : 'unknown'}</span>
    </div>
  </div>

  <div class="body">
    <!-- ============================================ RIGHT NOW ========= -->
    <section class="card">
      <header class="card-h">
        <span class="pane-title">Right now</span>
        <span class="spacer"></span>
        {#if ap?.state === 'running'}
          <span class="status busy" role="status" aria-live="polite">
            <span class="dot acc live"></span><span class="txt">fetching</span>
          </span>
        {/if}
      </header>

      {#if !ap}
        <div class="empty">
          Nothing can be said about the current cell: <code>/autopilot.json</code> did not answer
          with a payload this page can read. The reason is in the banner above.
        </div>
      {:else if ap.now}
        <dl class="kv">
          <dt>Instrument</dt>
          <dd class="mono">{ap.now.instrument}</dd>
          <dt>Month</dt>
          <dd class="mono">{ap.now.month}</dd>
          {#if ap.now.timeframe}
            <dt>Timeframe</dt>
            <dd class="mono">{ap.now.timeframe}</dd>
          {/if}
          {#if ap.now.feed}
            <dt>Feed</dt>
            <dd class="mono">{ap.now.feed}</dd>
          {/if}
          <dt>Elapsed</dt>
          <dd class="mono">{clock(elapsed)}</dd>
          {#if ap.now.index !== null && ap.now.of !== null}
            <dt>Position</dt>
            <dd class="mono">{n(ap.now.index)} of {n(ap.now.of)} in this month</dd>
          {/if}
        </dl>
        {#if ap.now.index !== null && ap.now.of}
          <div class="track" aria-hidden="true">
            <i style="width:{Math.max(0, Math.min(100, (ap.now.index / ap.now.of) * 100))}%"></i>
          </div>
        {/if}
      {:else}
        <div class="empty">
          <b>Nothing is in flight.</b>
          {#if ap.why}
            {ap.why}
          {:else if ap.state === 'complete'}
            The target span is complete — a re-run would fetch nothing, which is what makes
            restarting safe.
          {:else}
            The autopilot reported state <code>{ap.state}</code> and no current cell.
          {/if}
        </div>
      {/if}

      {#if ap?.waiting_ms}
        <p class="note warn">
          <span class="tag warn">throttled</span>
          Waiting <b>{clock(ap.waiting_ms)}</b> on the rate governor. This is the governor doing its
          job: it <b>waits</b> rather than refusing, so a throttled window costs time and never costs
          a month.
        </p>
      {/if}
      {#if ap?.absorbed_ms}
        <p class="note">
          <span class="lbl">Throttle absorbed since start</span>
          <b class="mono">{clock(ap.absorbed_ms)}</b> — time spent waiting rather than failing.
        </p>
      {/if}

      <!-- ---------------------------------------- THE CONTROL --------- -->
      <div class="ctl">
        {#if ap?.state === 'paused'}
          <button class="btn primary" type="button" disabled={control.busy} onclick={() => send('/autopilot/resume', 'resume')}>
            Resume
          </button>
          <span class="ctl-why">
            Resume continues from the store's own census, not from a progress variable — so it picks
            up exactly where the store stops, whatever happened in between.
          </span>
        {:else if ap && ap.state !== 'halted' && ap.state !== 'complete'}
          <button class="btn" type="button" disabled={control.busy} onclick={() => send('/autopilot/pause', 'pause')}>
            Pause
          </button>
          <span class="ctl-why">
            Pause finishes the cell in flight and then stops. It never aborts mid-write: a half
            month is a hole the append-only store cannot go back and fill.
          </span>
        {:else if ap?.state === 'halted'}
          <button class="btn primary" type="button" disabled={control.busy} onclick={() => send('/autopilot/resume', 'resume')}>
            Retry
          </button>
          <span class="ctl-why">
            It halted rather than skipping. Fix the reason above, then retry — nothing was silently
            passed over while it was down.
          </span>
        {:else}
          <span class="ctl-why">
            No control is offered: the autopilot has not reported a state this page can act on.
          </span>
        {/if}
        {#if control.note}
          <span class="ctl-note" class:bad={control.bad} role="status">{control.note}</span>
        {/if}
      </div>
    </section>

    <!-- ============================================ COVERAGE ========== -->
    <section class="card">
      <header class="card-h">
        <span class="pane-title">Coverage — oldest first</span>
        <span class="spacer"></span>
        {#if ap?.target}
          <span class="tag">{ap.target.from} → {ap.target.to}</span>
        {/if}
        <span class="tag {census.kind === 'ok' ? 'up' : 'down'}">
          {census.kind === 'ok' ? 'measured' : census.kind}
        </span>
      </header>

      <p class="note">
        Read from <code>/store.json</code>, not from the autopilot. The ladder runs oldest at the
        top because that is the order the backfill must run: the store is append-only with monotonic
        timestamps and one file per month, so a later month written first permanently blocks the
        earlier days in that same file.
        {#if !ap?.target}
          <b>The target span is not known here</b> — only the months the store already holds are
          listed, because <code>/autopilot.json</code> is what states the target and it did not
          answer. A month missing from this list may be a gap or may be outside the target; this
          page will not guess which.
        {/if}
        <br />
        <span class="lbl">Why this is not the Audit grid</span>
        <a class="link" href="/audit">Audit</a> draws the same census as a year-by-month grid against
        the target <code>docs/07-plan.md</code> R-2 states, and that is the picture to read for
        <em>how much of the backfill exists</em>. This ladder answers a different question — <em>what
        is the autopilot working on, and what is next</em> — so it is linear, oldest first, and it
        marks the cursor. The target here is the one the autopilot reports rather than one re-derived
        in the browser: two derivations of one window are two things that can drift apart, and the
        one that decides what to fetch is the one worth showing.
      </p>

      {#if census.kind === 'broken'}
        <div class="empty down">
          <b>The census could not be read.</b> {census.why}
          <br />Nothing below is current.
        </div>
      {:else if ladder.length === 0}
        <div class="empty">
          The store holds nothing for {feed ?? 'this feed'} yet. That is the state before the first
          month lands, not an error.
        </div>
      {:else}
        <div class="grid">
          <table>
            <thead>
              <tr>
                <th>Month</th>
                <th class="num">Instrument-months</th>
                <th class="num">Bars</th>
                <th>Coverage</th>
              </tr>
            </thead>
            <tbody>
              {#each ladder as m (m.month)}
                {@const pct = per ? Math.min(100, (m.cells / per) * 100) : null}
                {@const done = per !== null && m.cells >= per}
                <tr class:cursor={ap?.cursor === m.month}>
                  <td class="mono">
                    {m.month}
                    {#if ap?.cursor === m.month}<span class="tag acc">working</span>{/if}
                  </td>
                  <td class="num mono">{n(m.cells)}{per ? ` / ${n(per)}` : ''}</td>
                  <td class="num mono">{n(m.rows)}</td>
                  <td>
                    {#if pct === null}
                      <span class="faint">no target to compare against</span>
                    {:else}
                      <span class="bar" title="{Math.round(pct)}%">
                        <i class:done style="width:{pct}%"></i>
                      </span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

    <!-- ============================================ FAILURES ========== -->
    <section class="card">
      <header class="card-h">
        <span class="pane-title">What failed, and why</span>
        <span class="spacer"></span>
        {#if ap}
          <span class="tag" class:down={ap.failures.length > 0} class:up={ap.failures.length === 0}>
            {ap.failures.length === 0 ? 'none' : `${ap.failures.length}`}
          </span>
        {/if}
      </header>

      {#if !ap}
        <div class="empty">
          Unknown. The failure list lives in <code>/autopilot.json</code>, which did not answer.
          The durable record is the run journal at
          <code>~/.brutex/store/audit/pull.journal</code>, rendered at <a class="link" href="/audit">Audit</a>.
        </div>
      {:else if ap.failures.length === 0}
        <div class="empty">
          Nothing has failed. This is a real zero from the autopilot, not an empty list because the
          field was missing — a payload with no <code>failures</code> key is refused by the reader
          above rather than shown as none.
        </div>
      {:else}
        <div class="grid">
          <table>
            <thead>
              <tr>
                <th>Instrument</th>
                <th>Month</th>
                <th>Why</th>
                <th>When</th>
              </tr>
            </thead>
            <tbody>
              {#each ap.failures as f, i (`${f.instrument}-${f.month}-${i}`)}
                <tr>
                  <td class="mono">{f.instrument}</td>
                  <td class="mono">{f.month}</td>
                  <td class="why">{f.why}</td>
                  <td class="mono faint">{f.at ?? '—'}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      <p class="note">
        <span class="lbl">The durable record</span>
        This list is what the running process holds. The record that survives a restart is the run
        journal — <code>{ap?.journal ?? '~/.brutex/store/audit/pull.journal'}</code> — rendered at
        <a class="link" href="/audit">Audit</a>. A failure that appears here and not there is a
        failure that was never written down, and that is a defect in the journal, not in this page.
      </p>
    </section>
  </div>
</div>

<style>
  /* Local shapes only. Every colour, space, radius and weight below resolves
     through a token in $lib/theme.css — no literal is introduced here, so a
     theme edit reaches this page for free. */
  .body {
    flex: 1;
    overflow: auto;
    padding: var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s5);
  }

  .card {
    border: 1px solid var(--line);
    border-radius: var(--r3);
    background: var(--panel);
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .card-h {
    display: flex;
    align-items: center;
    gap: var(--s3);
    padding: var(--s4) var(--s5);
    border-bottom: 1px solid var(--line-soft);
  }
  /* ---- the banner. It is loud because the thing it reports is loud. ---- */
  .alarm {
    margin: var(--s5) var(--s5) 0;
    padding: var(--s5);
    border: 1px solid var(--down);
    border-left-width: 3px;
    border-radius: var(--r3);
    background: var(--down-soft);
  }
  .alarm-h {
    display: flex;
    align-items: center;
    gap: var(--s3);
    font-size: var(--fs-md);
    color: var(--ink);
  }
  .alarm-why {
    margin: var(--s4) 0 0;
    font-family: var(--mono);
    font-size: var(--fs-sm);
    color: var(--down);
    word-break: break-word;
  }
  .alarm-what {
    margin: var(--s4) 0 0;
    font-size: var(--fs-sm);
    line-height: var(--lh-base);
    color: var(--ink-2);
  }

  .lbl {
    display: block;
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }

  /* ---- key/value ---- */
  .kv {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--s2) var(--s5);
    margin: 0;
    padding: var(--s5);
  }
  .kv dt {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    align-self: center;
  }
  .kv dd {
    margin: 0;
    font-size: var(--fs-base);
    color: var(--ink);
    font-variant-numeric: tabular-nums;
  }

  /* ---- progress within the current month ---- */
  .track {
    height: 4px;
    margin: 0 var(--s5) var(--s5);
    border-radius: var(--r-full);
    background: var(--well);
    overflow: hidden;
  }
  .track i {
    display: block;
    height: 100%;
    background: var(--acc);
  }

  /* ---- the per-month coverage bar ---- */
  .bar {
    display: block;
    width: 100%;
    min-width: 80px;
    height: 6px;
    border-radius: var(--r-full);
    background: var(--well);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--acc);
  }
  .bar i.done {
    background: var(--up);
  }
  tr.cursor td {
    background: var(--acc-soft);
  }
  td.why {
    white-space: normal;
    max-width: 40ch;
    color: var(--down);
  }

  .note {
    margin: 0;
    padding: var(--s4) var(--s5);
    font-size: var(--fs-sm);
    line-height: var(--lh-base);
    color: var(--dim);
    border-top: 1px solid var(--line-soft);
  }
  .note.warn {
    color: var(--warn);
  }
  .note b {
    color: var(--ink);
  }

  /* ---- the control ---- */
  .ctl {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--s4);
    padding: var(--s5);
    border-top: 1px solid var(--line-soft);
  }
  .ctl-why {
    flex: 1;
    min-width: 16rem;
    font-size: var(--fs-sm);
    line-height: var(--lh-base);
    color: var(--faint);
  }
  .ctl-note {
    font-family: var(--mono);
    font-size: var(--fs-sm);
    color: var(--up);
  }
  .ctl-note.bad {
    color: var(--down);
  }

  code {
    font-family: var(--mono);
    font-size: 0.92em;
    padding: 0 3px;
    border-radius: var(--r1);
    background: var(--well);
    color: var(--ink-2);
  }

  .empty.down {
    color: var(--down);
  }
  .stat .v.up {
    color: var(--up);
  }
  .stat .v.down {
    color: var(--down);
  }
  .stat .v.warn {
    color: var(--warn);
  }
  .stat .v.acc {
    color: var(--acc);
  }
</style>

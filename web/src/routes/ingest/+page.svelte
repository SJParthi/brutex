<script>
  /**
   * INGEST — ask for a window, watch it happen, read every outcome.
   *
   * THE THREE THINGS THIS PAGE EXISTS TO FIX
   *
   * 1. A pull takes up to 32 minutes and used to show NOTHING until it
   *    finished. A working pull and a hung one looked identical, so the only
   *    way to tell them apart was to wait half an hour and find out.
   *
   * 2. A run that refused 407 of 785 instruments showed ONE reason. The
   *    receipt renders `done.failures.iter().take(5)` — five, and the run's
   *    own `Members failed` count is beside it, so the page can SEE that it
   *    was truncated and say so instead of implying the list is complete.
   *
   * 3. The bar length was not askable. `api::ingest::parse_spot` has read a
   *    `granularity` field all along; the server's own form never emitted one,
   *    so every request meant `1min` by omission and nobody could see that
   *    from the page.
   *
   * WHAT IS MEASURED AND WHAT IS ESTIMATED, AND THE PAGE NEVER BLURS THEM
   *
   * There is no pull-status endpoint. `crates/api/src/server.rs`'s router is
   * `/`, `/instruments`, `/instruments.json`, `/feeds.json`, `/bars.json`,
   * `/store.json`, `/typeahead.js`, `/pull`, `/pull/spot`, `/pull/fno`,
   * `/audit`, `/store`, `/health` — and nothing there answers "how far along
   * is the run". So this page does NOT invent a progress bar out of a timer.
   *
   * It polls `/store.json`, which the server rebuilds per request — its own
   * comment says "FRESH, NOT THE STARTUP SNAPSHOT" — and reports the growth it
   * measures. Instrument-months that gained rows are a FACT. The share of the
   * request they represent is an ESTIMATE, because the denominator is
   * arithmetic on what was asked for rather than a number the server states,
   * and the page labels it as one. A remaining time appears only once a rate
   * has actually been observed, and it is labelled an extrapolation.
   *
   * `CLAUDE.md` §3 rule 6: never claim a measurement not taken, and label
   * extrapolations as extrapolations.
   */
  import { feeds } from '$lib/feeds.svelte.js';
  import { catalogue } from '$lib/index.svelte.js';
  import { onMount } from 'svelte';

  // ---------------------------------------------------------------- constants

  /**
   * The bar-length ladder, spelled with the wire values
   * `pull::vendor::Granularity::dir()` emits — which is also the directory the
   * store writes under and the value `api::ingest::parse_granularity` matches
   * case-insensitively. One spelling for the control, the request and the path.
   *
   * `stored` is `Granularity::store_timeframe().is_some()`: only the minute and
   * the day rung have a `store::path::Timeframe`, so every other rung is
   * refused at the WRITE boundary rather than here. The control offers them and
   * warns, instead of hiding a rung the parser accepts — a control that silently
   * omits a legal value is the same lie as one that silently adds one.
   */
  const RUNGS = [
    { dir: 'tick', label: 'Tick', note: 'every print — no fixed interval', stored: false },
    { dir: '1s', label: '1 second', note: 'one-second grid', stored: false },
    { dir: '5s', label: '5 seconds', note: 'five-second grid', stored: false },
    { dir: '1min', label: '1 minute', note: 'what the engine sweeps', stored: true },
    { dir: '3min', label: '3 minutes', note: 'three-minute grid', stored: false },
    { dir: '5min', label: '5 minutes', note: 'five-minute grid', stored: false },
    { dir: '15min', label: '15 minutes', note: 'fifteen-minute grid', stored: false },
    { dir: '30min', label: '30 minutes', note: 'thirty-minute grid', stored: false },
    { dir: '1hr', label: '1 hour', note: 'hourly grid', stored: false },
    { dir: '1day', label: '1 day', note: 'one bar per session', stored: true },
    { dir: '1week', label: '1 week', note: 'one bar per trading week', stored: false }
  ];

  /** The three targets `api::ingest::SpotTarget` spells, with its own labels. */
  const TARGETS = [
    { slug: 'swept', label: 'Swept indices', note: 'NSE-NIFTY and NSE-BANKNIFTY — the only two swept' },
    { slug: 'indices', label: 'Reference indices', note: 'stored and stamped onto trades; never swept' },
    { slug: 'equities', label: 'NIFTY Total Market', note: 'stored, never swept' }
  ];

  /** `api::ingest::MAX_WINDOW_DAYS`. Refused by the parser before anything runs. */
  const MAX_WINDOW_DAYS = 3653;

  /** The earliest year the server's own picker offers, read from `GET /pull`. */
  const FLOOR_YEAR = 2015;

  const MONTHS = [
    'January', 'February', 'March', 'April', 'May', 'June',
    'July', 'August', 'September', 'October', 'November', 'December'
  ];
  const MONTHS_SHORT = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  /** Monday first — the order `crates/api/src/calendar.rs` uses, and Dhan's. */
  const WEEKDAYS = ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'];

  /** One virtual row, in pixels. Must equal `--row-h` or the spacer lies. */
  const ROW_H = 30;
  /** Rows kept above and below the viewport so a fast scroll does not tear. */
  const OVERSCAN = 8;
  /** Prefix depth of the outcome search index — the same bound `$lib/index` uses. */
  const MAX_PREFIX = 4;

  // ------------------------------------------------------------- date helpers
  //
  // EVERY DATE IS AN ISO STRING AND EVERY SUM IS IN UTC. A local-time `Date`
  // shifts under DST and this operator's own machine renders `dd/mm/yyyy`,
  // which is the exact ambiguity `crates/api/src/calendar.rs` was written to
  // remove: `01/07/2025` is 1 July here and 7 January in half the world.

  const DAY_MS = 86_400_000;
  const IST_OFFSET_MS = 19_800_000; // +05:30, and India has no daylight saving.

  /** The IST date of a moment, as `YYYY-MM-DD`. */
  function istDay(ms = Date.now()) {
    return new Date(ms + IST_OFFSET_MS).toISOString().slice(0, 10);
  }
  function pad(n, w = 2) {
    return String(n).padStart(w, '0');
  }
  function iso(y, m, d) {
    return `${pad(y, 4)}-${pad(m)}-${pad(d)}`;
  }
  function parts(isoDay) {
    const [y, m, d] = isoDay.split('-').map(Number);
    return { y, m, d };
  }
  /** Days since the epoch, so two dates can be compared and subtracted. */
  function dayNum(isoDay) {
    const { y, m, d } = parts(isoDay);
    return Math.round(Date.UTC(y, m - 1, d) / DAY_MS);
  }
  function addDays(isoDay, n) {
    return new Date((dayNum(isoDay) + n) * DAY_MS).toISOString().slice(0, 10);
  }
  function daysInMonth(y, m) {
    return new Date(Date.UTC(y, m, 0)).getUTCDate();
  }
  /** Which column the 1st falls in, 0..6, Monday first. */
  function firstColumn(y, m) {
    return (new Date(Date.UTC(y, m - 1, 1)).getUTCDay() + 6) % 7;
  }
  function isValidIso(s) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(s)) return false;
    const { y, m, d } = parts(s);
    return m >= 1 && m <= 12 && d >= 1 && d <= daysInMonth(y, m);
  }
  /** Every `YYYY-MM` the window touches, inclusive at both ends. */
  function monthsBetween(a, b) {
    if (!isValidIso(a) || !isValidIso(b)) return [];
    const from = parts(a);
    const to = parts(b);
    const out = [];
    let y = from.y;
    let m = from.m;
    // Bounded by the window cap: 3,653 days is at most 122 months.
    while (y < to.y || (y === to.y && m <= to.m)) {
      out.push(`${pad(y, 4)}-${pad(m)}`);
      m += 1;
      if (m > 12) {
        m = 1;
        y += 1;
      }
    }
    return out;
  }
  function clockOf(ms) {
    const s = Math.max(0, Math.round(ms / 1000));
    const h = Math.floor(s / 3600);
    const rest = s % 3600;
    const body = `${pad(Math.floor(rest / 60))}:${pad(rest % 60)}`;
    return h > 0 ? `${h}:${body}` : body;
  }
  function n(x) {
    return Number(x ?? 0).toLocaleString();
  }

  // ------------------------------------------------------------------- feed

  const active = $derived(feeds.all.find((f) => f.wire === feeds.active));
  const isBroker = $derived(active?.transport === 'broker');
  /**
   * ARCHIVE FEEDS STAY OUT OF SIGHT UNTIL THEIR DATA IS BOUGHT, and the gate is
   * the server's own `ready` flag rather than a list of vendor names here. The
   * day TrueData has a store prefix its row flips and the folder form appears
   * with no edit to this file — the same reason the feed picker is built from
   * `/feeds.json`.
   */
  const archiveHidden = $derived(active?.transport === 'archive' && active?.ready === false);

  // ---------------------------------------------------------------- the form

  let target = $state('swept');
  let rung = $state('1min');
  let from = $state('');
  let to = $state('');
  let folder = $state('');
  let showProblems = $state(false);

  let nowMs = $state(Date.now());
  const istToday = $derived(istDay(nowMs));
  /**
   * A broker is never asked for a day that has not finished — a running session
   * yields a partial day the append-only store cannot correct later. The server
   * enforces it; this only stops you asking.
   */
  const brokerMax = $derived(addDays(istToday, -1));
  const maxDay = $derived(isBroker ? brokerMax : istToday);
  const minDay = $derived(iso(FLOOR_YEAR, 1, 1));

  const windowDays = $derived(
    isValidIso(from) && isValidIso(to) ? dayNum(to) - dayNum(from) + 1 : 0
  );
  /**
   * A BACKWARDS WINDOW HAS NO SPAN, and the counters say so instead of printing
   * `-3 days` and `2 instrument-months` as if the request meant something. The
   * refusal below names the fault; the tiles must not quietly contradict it.
   */
  const windowOk = $derived(windowDays > 0);
  const windowMonths = $derived(windowOk ? monthsBetween(from, to) : []);

  /** The exact bytes that will go on the wire. Shown, not described. */
  const wireBody = $derived.by(() => {
    const b = new URLSearchParams();
    b.set('target', target);
    b.set('vendor', feeds.active ?? '');
    b.set('from', from);
    b.set('to', to);
    b.set('granularity', rung);
    if (!isBroker) b.set('folder', folder);
    return b.toString();
  });

  // -------------------------------------------------------------- validation
  //
  // EACH ONE NAMES THE FIELD, THE VALUE AND THE RULE. "Invalid date" is a
  // refusal that tells the operator nothing; every message below carries the
  // number that failed and the number it failed against.

  const problems = $derived.by(() => {
    const out = [];
    if (!from) {
      out.push({ field: 'from', why: 'From has no date. Open its calendar and pick the first day of the window.' });
    } else if (!isValidIso(from)) {
      out.push({ field: 'from', why: `From reads "${from}", which is not a real day. Dates here are YYYY-MM-DD.` });
    }
    if (!to) {
      out.push({ field: 'to', why: 'To has no date. Open its calendar and pick the last day of the window.' });
    } else if (!isValidIso(to)) {
      out.push({ field: 'to', why: `To reads "${to}", which is not a real day. Dates here are YYYY-MM-DD.` });
    }
    if (isValidIso(from) && isValidIso(to) && dayNum(to) < dayNum(from)) {
      out.push({
        field: 'to',
        why: `To is ${to}, which is ${n(dayNum(from) - dayNum(to))} day(s) BEFORE From (${from}). A window runs forward — the two are the wrong way round.`
      });
    }
    for (const [field, value] of [['from', from], ['to', to]]) {
      if (!isValidIso(value)) continue;
      if (dayNum(value) > dayNum(maxDay)) {
        out.push({
          field,
          why: isBroker
            ? `${field === 'from' ? 'From' : 'To'} is ${value}. Today in IST is ${istToday}, and the newest day a broker is asked for is ${brokerMax} — a session still running yields a partial day this append-only store cannot correct later.`
            : `${field === 'from' ? 'From' : 'To'} is ${value}, which is in the future. Today in IST is ${istToday}.`
        });
      }
      if (dayNum(value) < dayNum(minDay)) {
        out.push({
          field,
          why: `${field === 'from' ? 'From' : 'To'} is ${value}, earlier than ${minDay} — the floor the server's own picker offers.`
        });
      }
    }
    if (windowDays > MAX_WINDOW_DAYS) {
      out.push({
        field: 'to',
        why: `That window is ${n(windowDays)} days. api::ingest::MAX_WINDOW_DAYS caps one request at ${n(MAX_WINDOW_DAYS)}, so the parser refuses it before any vendor is contacted.`
      });
    }
    if (!isBroker && !folder.trim()) {
      out.push({ field: 'folder', why: 'Folder is empty. An archive run reads CSV files from a directory you name.' });
    }
    return out;
  });

  const problemFor = $derived.by(() => {
    const m = new Map();
    for (const p of problems) if (!m.has(p.field)) m.set(p.field, p.why);
    return m;
  });

  /**
   * Not refusals — cautions. The SERVER has the last word on every one of
   * these, and the request is still sent. They are here because an operator
   * who is about to spend thirty-two minutes should see the known blocker
   * before the clock starts, not on the receipt.
   */
  const cautions = $derived.by(() => {
    const out = [];
    const r = RUNGS.find((x) => x.dir === rung);
    if (r && !r.stored) {
      out.push(
        `The store has no directory for ${r.label.toLowerCase()}. pull::vendor::Granularity::store_timeframe answers for 1 minute and 1 day only, so this rung is refused at the write boundary — expect a refusal rather than bars.`
      );
    }
    if (isBroker && target !== 'swept') {
      out.push(
        `The broker path in this build addresses ONE instrument. pull::vendor::HttpSpec carries no request-parameter map, so a target naming a set has been refused rather than fetching one series and filing it under a name you did not ask for.`
      );
    }
    if (rung !== '1min') {
      out.push(
        `Live progress below is measured from /store.json, which stamps every row "1m" regardless of rung. Growth shown while this run is going may belong to another rung — it cannot be separated from the wire.`
      );
    }
    return out;
  });

  // ------------------------------------------------------------ the universe

  /** Which instruments the chosen target names, out of the catalogue. */
  const members = $derived.by(() => {
    const rows = catalogue.rows ?? [];
    if (target === 'swept') {
      // `CLAUDE.md` §1 names these two and no others.
      return rows.filter((r) => r.key === 'NSE-NIFTY' || r.key === 'NSE-BANKNIFTY');
    }
    const want = target === 'indices' ? 'index' : 'ntm';
    return rows.filter((r) => String(r.universe ?? '').split('+').includes(want));
  });

  /** The census spells an instrument `EXCHANGE-SEGMENT-SYMBOL`. */
  function censusKey(row) {
    return `${row.exchange}-${row.segment}-${row.symbol}`;
  }

  /**
   * Instrument-months the request would fill if every name answered for every
   * month. An ESTIMATE, and the only denominator available: the server states
   * no total.
   */
  const expectedUnits = $derived(members.length * windowMonths.length);

  // -------------------------------------------------------------- the run
  //
  // `phase` is the whole state machine: idle → running → done. Nothing else
  // decides what the right-hand column shows.

  let phase = $state('idle');
  let startedAt = $state(0);
  let finishedAt = $state(0);
  let receipt = $state(null);
  let netError = $state(null);
  let aborted = $state(false);
  let controller = null;

  /** Where the store stood the moment the request left, and where it stands now. */
  let baseline = $state(null);
  let live = $state(null);
  let pollError = $state(null);
  let lastGrowthAt = $state(0);
  /** {t, units, rows} samples, newest last. Bounded — only the tail is kept. */
  let samples = $state([]);

  const elapsedMs = $derived(
    phase === 'running' ? nowMs - startedAt : finishedAt > startedAt ? finishedAt - startedAt : 0
  );

  const unitsDone = $derived(
    baseline && live ? Math.max(0, live.units - baseline.units) : 0
  );
  const rowsGained = $derived(
    baseline && live ? Math.max(0, live.rows - baseline.rows) : 0
  );
  const unitsLeft = $derived(Math.max(0, expectedUnits - (baseline?.units ?? 0) - unitsDone));
  const share = $derived(
    expectedUnits > 0 ? Math.min(1, (baseline ? baseline.units + unitsDone : 0) / expectedUnits) : 0
  );

  /**
   * Instrument-months per second, from the OBSERVED samples only. `null` until
   * two samples exist that actually differ — a rate computed from one point is
   * a number with no measurement behind it.
   */
  const rate = $derived.by(() => {
    if (samples.length < 2) return null;
    const first = samples[0];
    const last = samples[samples.length - 1];
    const secs = (last.t - first.t) / 1000;
    const gained = last.units - first.units;
    if (secs < 5 || gained <= 0) return null;
    return gained / secs;
  });

  const etaSecs = $derived(rate && unitsLeft > 0 ? Math.round(unitsLeft / rate) : null);

  /**
   * THE GOVERNOR IS NOT VISIBLE FROM HERE, and this is the closest honest thing
   * to seeing it. `crates/pull/src/rate.rs` runs an AIMD governor inside the
   * synchronous POST; nothing reports its state. What CAN be observed is that
   * the store has stopped growing while the request is still open — which is
   * what a waiting governor looks like from outside, and also what a slow
   * vendor looks like. It is named as an inference, not as a reading.
   */
  const stalledSecs = $derived(
    phase === 'running' && lastGrowthAt > 0 ? Math.round((nowMs - lastGrowthAt) / 1000) : 0
  );
  const stalled = $derived(phase === 'running' && stalledSecs >= 20);
  /**
   * WHETHER ANYTHING HAS ACTUALLY LANDED. Without this the panel read "Bars are
   * landing" three seconds into a run that had written nothing — a claim with
   * no measurement under it, which is the exact failure the rest of this page
   * is built to avoid.
   */
  const everGrew = $derived(rowsGained > 0 || unitsDone > 0);

  // ------------------------------------------------------------ store census

  /**
   * One reading of the store, reduced to two numbers and a map before the array
   * is dropped. The array is ~266 KB at 3,353 rows today and grows toward the
   * ~93,776 the store is heading for, so nothing holds on to it.
   */
  async function snapshot(signal) {
    const feed = feeds.active ?? '';
    const r = await fetch(`/store.json?feed=${encodeURIComponent(feed)}`, { signal });
    if (!r.ok) throw new Error(`/store.json answered HTTP ${r.status}`);
    const rows = await r.json();
    const want = new Set(windowMonths);
    const byInstrument = new Map();
    let unitsIn = 0;
    let rowsIn = 0;
    for (const row of rows) {
      if (!want.has(row.month)) continue;
      unitsIn += 1;
      rowsIn += row.rows;
      byInstrument.set(row.instrument, (byInstrument.get(row.instrument) ?? 0) + row.rows);
    }
    return { at: Date.now(), units: unitsIn, rows: rowsIn, byInstrument, held: rows.length };
  }

  /**
   * The watcher. A self-scheduling loop rather than an interval, so a slow
   * response cannot stack requests on top of each other, and a generation
   * counter so a second run does not race the first one's tail.
   */
  let watchGen = 0;
  async function watch(gen) {
    while (gen === watchGen && phase === 'running') {
      try {
        const shot = await snapshot();
        if (gen !== watchGen) return;
        pollError = null;
        if (!live || shot.units > live.units || shot.rows > live.rows) lastGrowthAt = shot.at;
        live = shot;
        samples = [...samples, { t: shot.at, units: shot.units, rows: shot.rows }].slice(-12);
      } catch (why) {
        if (gen !== watchGen) return;
        // NAMED, NOT SWALLOWED. A watcher that quietly stops is a progress
        // display that silently freezes, which is the failure this page exists
        // to remove.
        pollError = String(why);
      }
      await new Promise((done) => setTimeout(done, 5000));
    }
  }

  // ------------------------------------------------------------- the receipt

  /**
   * The server's own answer, read rather than reconstructed.
   *
   * `DOMParser` on `text/html` builds an INERT document: no browsing context,
   * so no script runs and no resource is fetched. Every value below comes out
   * as `textContent`, so a symbol carrying `&` — M&M, M&MFIN are real
   * instruments — is a string and never markup. Nothing on this page is ever
   * assigned through `innerHTML`.
   */
  function readReceipt(html, ok, status) {
    const doc = new DOMParser().parseFromString(html, 'text/html');
    const facts = [];
    for (const tr of doc.querySelectorAll('table.kv tr')) {
      const k = tr.querySelector('th')?.textContent?.trim() ?? '';
      const v = tr.querySelector('td')?.textContent?.trim() ?? '';
      if (k) facts.push({ k, v });
    }
    const badge = doc.querySelector('.badge');
    const halt = doc.querySelector('.halt');
    const scope = halt?.querySelector('b')?.textContent?.trim() ?? '';
    let reason = halt?.textContent?.trim() ?? '';
    if (scope && reason.startsWith(scope)) reason = reason.slice(scope.length).trim();
    return {
      ok,
      status,
      verdict: badge?.textContent?.trim() || (ok ? 'OK' : `HTTP ${status}`),
      good: badge ? badge.classList.contains('good') : ok,
      scope,
      reason,
      facts,
      raw: html
    };
  }

  const factValue = $derived.by(() => {
    const m = new Map();
    for (const f of receipt?.facts ?? []) if (!m.has(f.k)) m.set(f.k, f.v);
    return m;
  });

  /** Every `Failed` row the receipt carried — the server sends at most five. */
  const namedFailures = $derived((receipt?.facts ?? []).filter((f) => f.k === 'Failed'));
  const failedCount = $derived.by(() => {
    const raw = factValue.get('Members failed');
    const parsed = Number(String(raw ?? '').replace(/[^0-9]/g, ''));
    return Number.isFinite(parsed) ? parsed : 0;
  });
  /**
   * THE TRUNCATION, STATED. `landed_answer` writes
   * `for f in done.failures.iter().take(5)`, so a run with 407 failures puts
   * five on the wire and the count beside them. The page can see the gap and
   * says so rather than presenting five as the whole story.
   */
  const hiddenFailures = $derived(Math.max(0, failedCount - namedFailures.length));

  // -------------------------------------------------------------- outcomes
  //
  // ONE ROW PER INSTRUMENT THE REQUEST NAMED, whatever happened to it. The
  // reasons come from the server's own words where it gave them and from the
  // measured census where it did not; nothing here is a guess dressed as a
  // verdict.

  let outcomes = $state([]);
  let outcomeIndex = new Map();

  function classifyBuild() {
    if (!baseline) {
      outcomes = [];
      outcomeIndex = new Map();
      return;
    }
    const after = live ?? baseline;
    const failedBySymbol = new Map();
    for (const f of namedFailures) {
      // `landed_answer` writes `{instrument} — {why}`.
      const split = f.v.split(' — ');
      const who = (split.shift() ?? '').trim();
      const why = split.join(' — ').trim() || f.v;
      if (who) failedBySymbol.set(who.toUpperCase(), why);
    }
    /**
     * A WHOLESALE REFUSAL AND A RUN THAT FAILED PART-WAY ARE NOT THE SAME
     * THING, and stamping the run's verdict onto every member conflated them:
     * a run that read 785 members and failed 407 came back with 745 rows all
     * labelled "Refused", quoting a sentence that was never about them.
     *
     * `Members read` is the discriminator. A refusal that never reached the
     * members — the parser's, or the broker path's before the socket — emits no
     * such fact, and only then does the run's own reason belong on every row.
     */
    const refused = factValue.get('Refused') ?? null;
    const reachedMembers = factValue.has('Members read');
    const runReason = reachedMembers
      ? refused
      : (refused ?? (receipt && !receipt.good ? receipt.reason : null));

    const rows = [];
    const seen = new Set();
    for (const m of members) {
      const key = censusKey(m);
      seen.add(key);
      const before = baseline.byInstrument.get(key) ?? 0;
      const now = after.byInstrument.get(key) ?? 0;
      const gained = now - before;
      const named =
        failedBySymbol.get(m.symbol.toUpperCase()) ??
        failedBySymbol.get(m.key.toUpperCase()) ??
        failedBySymbol.get(key.toUpperCase());
      let group;
      let reason;
      let tone;
      if (named) {
        group = 'Failed';
        reason = named;
        tone = 'down';
      } else if (gained > 0) {
        group = 'Stored';
        reason = `${n(gained)} bar(s) landed across ${windowMonths.length} month(s) of the window`;
        tone = 'up';
      } else if (runReason) {
        group = 'Refused';
        reason = runReason;
        tone = 'down';
      } else if (before > 0) {
        group = 'Already held';
        reason = `The store already carried ${n(before)} bar(s) for this window and gained none — nothing new was written and the run named no failure for it`;
        tone = 'info';
      } else {
        group = 'No bars landed';
        reason =
          hiddenFailures > 0
            ? `The store gained nothing for this instrument and no reason was sent for it. The run recorded ${n(failedCount)} failed member(s) and put only ${n(namedFailures.length)} reason(s) on the wire, so this may be one of the ${n(hiddenFailures)} whose reason the server truncated.`
            : 'The run reported no failure for this instrument and the store gained nothing for it. It was named by the target and is not accounted for.';
        tone = 'warn';
      }
      rows.push({ key, symbol: m.symbol, kind: m.kind, gained, before, group, reason, tone });
    }

    // Anything that GREW but was not in the target set. Surprising, and worth
    // seeing: it means something wrote outside the request.
    for (const [key, now] of after.byInstrument) {
      if (seen.has(key)) continue;
      const before = baseline.byInstrument.get(key) ?? 0;
      if (now - before <= 0) continue;
      rows.push({
        key,
        symbol: key.split('-').slice(2).join('-') || key,
        kind: key.split('-')[1] ?? '',
        gained: now - before,
        before,
        group: 'Outside the target',
        reason: `${n(now - before)} bar(s) landed for an instrument this request did not name. Another run may be writing at the same time.`,
        tone: 'info'
      });
    }

    outcomes = rows;
    // O(1) PER KEYSTROKE, the same prefix index `$lib/index.svelte.js` uses.
    // Built once per run — O(n) and paid a single time — so the search box is a
    // Map probe rather than a scan of 750.
    const next = new Map();
    for (const row of rows) {
      const s = row.symbol.toUpperCase();
      for (let i = 1; i <= Math.min(MAX_PREFIX, s.length); i += 1) {
        const k = s.slice(0, i);
        let bucket = next.get(k);
        if (!bucket) next.set(k, (bucket = []));
        bucket.push(row);
      }
    }
    outcomeIndex = next;
  }

  /** Reasons, grouped and counted, most common first. */
  const groups = $derived.by(() => {
    const m = new Map();
    for (const row of outcomes) {
      let g = m.get(row.group);
      if (!g) m.set(row.group, (g = { group: row.group, tone: row.tone, count: 0, reasons: new Map() }));
      g.count += 1;
      g.reasons.set(row.reason, (g.reasons.get(row.reason) ?? 0) + 1);
    }
    return [...m.values()].sort((a, b) => b.count - a.count);
  });

  let q = $state('');
  let pickedGroup = $state(null);
  let sortKey = $state('group');
  let sortDir = $state(1);

  const filtered = $derived.by(() => {
    const typed = q.trim().toUpperCase();
    // ONE PROBE for 1..4 characters; a filter over one bucket beyond that. The
    // group narrowing is applied to whichever set is already smaller, so the
    // cost is bounded by the bucket and never by the universe.
    let base;
    if (!typed) base = outcomes;
    else if (typed.length <= MAX_PREFIX) base = outcomeIndex.get(typed) ?? [];
    else base = (outcomeIndex.get(typed.slice(0, MAX_PREFIX)) ?? []).filter((r) => r.symbol.toUpperCase().startsWith(typed));
    const narrowed = pickedGroup ? base.filter((r) => r.group === pickedGroup) : base;
    const dir = sortDir;
    const sorted = [...narrowed];
    sorted.sort((a, b) => {
      if (sortKey === 'gained') return (a.gained - b.gained) * dir;
      if (sortKey === 'symbol') return a.symbol.localeCompare(b.symbol) * dir;
      return (a.group.localeCompare(b.group) || a.symbol.localeCompare(b.symbol)) * dir;
    });
    return sorted;
  });

  function sortBy(key) {
    if (sortKey === key) sortDir = -sortDir;
    else {
      sortKey = key;
      sortDir = key === 'gained' ? -1 : 1;
    }
  }

  // --------------------------------------------------------- virtualisation
  //
  // ONLY THE VISIBLE ROWS ARE IN THE DOM. 750 today; the store is heading for
  // ~93,776 census rows and a target list that grows with it. A spacer carries
  // the full height so the scrollbar is honest.

  let vEl = $state(null);
  let vTop = $state(0);
  let vH = $state(420);
  const vFirst = $derived(Math.max(0, Math.floor(vTop / ROW_H) - OVERSCAN));
  const vCount = $derived(Math.min(filtered.length - vFirst, Math.ceil(vH / ROW_H) + OVERSCAN * 2));
  const vSlice = $derived(filtered.slice(vFirst, vFirst + Math.max(0, vCount)));

  function onScroll() {
    if (vEl) vTop = vEl.scrollTop;
  }

  $effect(() => {
    if (!vEl) return;
    const ro = new ResizeObserver(() => {
      vH = vEl.clientHeight;
    });
    ro.observe(vEl);
    vH = vEl.clientHeight;
    return () => ro.disconnect();
  });

  // Re-filtering must not leave the viewport scrolled past the end.
  $effect(() => {
    const max = Math.max(0, filtered.length * ROW_H - vH);
    if (vTop > max && vEl) {
      vEl.scrollTop = max;
      vTop = max;
    }
  });

  // ------------------------------------------------------------ the calendar
  //
  // ONE PANEL AT A TIME, and the drill-down `crates/api/src/calendar.rs`
  // settled on: years, then months, then that month's grid. `cal.field` is the
  // latch — a single value, so two panels cannot coexist to overlap each other.

  let cal = $state({ field: null, view: 'day', y: 2026, m: 8, cursor: '', up: false });
  let calEl = $state(null);

  /** Tall enough for the year pane, which is the tallest of the three. */
  const CAL_H = 400;

  function openCal(field) {
    if (cal.field === field) {
      cal = { ...cal, field: null };
      return;
    }
    const current = field === 'from' ? from : to;
    const seed = isValidIso(current) ? current : maxDay;
    const p = parts(seed);
    // OPENS UPWARD WHEN THERE IS NO ROOM BELOW. A picker that drops off the
    // bottom of a scroll container is a picker the operator has to hunt for,
    // and this form sits in one.
    const box = document.getElementById(`cal-${field}`)?.getBoundingClientRect();
    const up = box ? box.bottom + CAL_H > window.innerHeight && box.top > CAL_H : false;
    cal = { field, view: 'day', y: p.y, m: p.m, cursor: seed, up };
    // FOCUS HAS TO ENTER THE PANEL, ONCE. The keydown handler is on the panel,
    // so with focus left on the trigger every arrow key went to the page
    // instead: the picker opened and then ignored the keyboard entirely.
    enterOnOpen = true;
  }
  function shutCal(refocus = true) {
    const field = cal.field;
    cal = { ...cal, field: null };
    if (refocus && field) document.getElementById(`cal-${field}`)?.focus();
  }
  function commit(day) {
    if (!inRange(day)) return;
    if (cal.field === 'from') from = day;
    else if (cal.field === 'to') to = day;
    showProblems = true;
    shutCal();
  }
  function inRange(day) {
    return dayNum(day) >= dayNum(minDay) && dayNum(day) <= dayNum(maxDay);
  }
  /**
   * THE CURSOR NEVER LANDS ON A DAY THAT CANNOT BE PICKED.
   *
   * Not tidiness — a correctness fix. A day outside the range renders as a
   * `disabled` button, and a disabled button cannot take focus: `PageDown` from
   * 23 July put the cursor on 23 August, focus fell through to `<body>`, and
   * from there the panel's keydown handler never fired again. Every subsequent
   * key, Enter included, went nowhere. Clamping keeps the cursor on a real
   * target, so focus always has somewhere to be.
   */
  function clampDay(day) {
    if (dayNum(day) < dayNum(minDay)) return minDay;
    if (dayNum(day) > dayNum(maxDay)) return maxDay;
    return day;
  }
  /** Moves the cursor and pulls the visible month along with it. */
  function placeCursor(day) {
    const at = clampDay(day);
    const p = parts(at);
    cal = { ...cal, cursor: at, y: p.y, m: p.m };
  }
  function stepMonth(delta) {
    let y = cal.y;
    let m = cal.m + delta;
    while (m > 12) {
      m -= 12;
      y += 1;
    }
    while (m < 1) {
      m += 12;
      y -= 1;
    }
    const d = Math.min(parts(cal.cursor || iso(y, m, 1)).d, daysInMonth(y, m));
    placeCursor(iso(y, m, d));
  }
  function moveCursor(days) {
    placeCursor(addDays(cal.cursor || iso(cal.y, cal.m, 1), days));
  }
  /** Month pane → day pane, keeping the day-of-month where it fits. */
  function pickMonth(month) {
    const wanted = parts(cal.cursor || iso(cal.y, month, 1)).d;
    cal = { ...cal, m: month, view: 'day' };
    placeCursor(iso(cal.y, month, Math.min(wanted, daysInMonth(cal.y, month))));
    enterOnOpen = true;
  }
  function onCalKey(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      shutCal();
      return;
    }
    if (cal.view !== 'day') return;
    const map = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7 };
    if (e.key in map) {
      e.preventDefault();
      moveCursor(map[e.key]);
    } else if (e.key === 'PageUp') {
      e.preventDefault();
      stepMonth(-1);
    } else if (e.key === 'PageDown') {
      e.preventDefault();
      stepMonth(1);
    } else if (e.key === 'Home') {
      e.preventDefault();
      placeCursor(iso(cal.y, cal.m, 1));
    } else if (e.key === 'End') {
      e.preventDefault();
      placeCursor(iso(cal.y, cal.m, daysInMonth(cal.y, cal.m)));
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      commit(cal.cursor);
    }
  }
  /**
   * Roving focus: the cursor day is the only day button in the tab order.
   *
   * It moves focus on OPEN — once, which is what makes the arrow keys reachable
   * at all — and after that only while focus is already on a day. Without that
   * second condition the nav arrows and the header would lose focus to the grid
   * the instant they were pressed.
   */
  let enterOnOpen = false;
  $effect(() => {
    if (!cal.field || cal.view !== 'day' || !calEl) return;
    const el = calEl.querySelector(`[data-day="${cal.cursor}"]`);
    if (!el || document.activeElement === el) return;
    const now = document.activeElement;
    const onADay = now?.classList?.contains('day') ?? false;
    // ARROWING ACROSS A MONTH BOUNDARY DESTROYS THE FOCUSED BUTTON. The grid is
    // re-rendered for the new month, the node focus was on is gone, and the
    // browser drops focus to `body` — after which the panel's keydown handler
    // never fires again and the picker is dead to the keyboard from the second
    // Up arrow onward. An orphaned focus is therefore reclaimed, while a nav
    // button or the header title — which survive the re-render — keep theirs.
    const orphaned = !now || now === document.body || now === calEl;
    if (enterOnOpen || onADay || orphaned) {
      enterOnOpen = false;
      el.focus();
    }
  });

  const calDays = $derived.by(() => {
    const lead = firstColumn(cal.y, cal.m);
    const count = daysInMonth(cal.y, cal.m);
    const cells = [];
    for (let i = 0; i < lead; i += 1) cells.push(null);
    for (let d = 1; d <= count; d += 1) cells.push(iso(cal.y, cal.m, d));
    return cells;
  });
  const calYears = $derived.by(() => {
    const top = parts(istToday).y;
    const out = [];
    for (let y = FLOOR_YEAR; y <= top; y += 1) out.push(y);
    return out;
  });

  // ------------------------------------------------------------------ submit

  async function start(e) {
    e.preventDefault();
    showProblems = true;
    if (problems.length > 0 || phase === 'running') return;

    receipt = null;
    netError = null;
    pollError = null;
    aborted = false;
    outcomes = [];
    outcomeIndex = new Map();
    samples = [];
    q = '';
    pickedGroup = null;

    // THE BEFORE READING IS TAKEN FIRST AND IS NOT OPTIONAL. Every outcome
    // below is a difference against it; without one there is nothing to
    // subtract and the page would have to guess.
    try {
      baseline = await snapshot();
      live = baseline;
    } catch (why) {
      baseline = null;
      live = null;
      netError = `The store could not be read before starting, so nothing this run does could be measured against it: ${why}`;
      return;
    }

    startedAt = Date.now();
    lastGrowthAt = startedAt;
    finishedAt = 0;
    phase = 'running';
    watchGen += 1;
    watch(watchGen);

    controller = new AbortController();
    try {
      const r = await fetch('/pull/spot', {
        method: 'POST',
        headers: { 'content-type': 'application/x-www-form-urlencoded' },
        body: wireBody,
        signal: controller.signal
      });
      receipt = readReceipt(await r.text(), r.ok, r.status);
    } catch (why) {
      if (controller?.signal.aborted) aborted = true;
      else netError = String(why);
    }
    controller = null;
    finishedAt = Date.now();
    phase = 'done';
    watchGen += 1;

    // THE AFTER READING. Taken even when the request failed — a run that
    // refused halfway still wrote whatever landed before it, and pretending
    // otherwise is the fallback that hides a failure.
    try {
      live = await snapshot();
      pollError = null;
    } catch (why) {
      pollError = String(why);
    }
    classifyBuild();
  }

  function stopWatching() {
    controller?.abort();
  }

  function reset() {
    phase = 'idle';
    receipt = null;
    netError = null;
    outcomes = [];
    outcomeIndex = new Map();
    baseline = null;
    live = null;
    samples = [];
    aborted = false;
  }

  let showRaw = $state(false);

  onMount(() => {
    const tick = setInterval(() => (nowMs = Date.now()), 500);
    const away = (e) => {
      if (cal.field && calEl && !calEl.contains(e.target) && !e.target.closest?.('.cal-open')) shutCal(false);
    };
    document.addEventListener('pointerdown', away, true);
    return () => {
      clearInterval(tick);
      document.removeEventListener('pointerdown', away, true);
      watchGen += 1;
      controller?.abort();
    };
  });
</script>

<div class="pane">
  <div class="pane-head">
    <span class="pane-title">Ingest</span>
    {#if active}
      <span class="tag acc">{active.display}</span>
      <span class="tag">{active.transport}</span>
      <span class="tag info">{RUNGS.find((r) => r.dir === rung)?.label ?? rung}</span>
    {/if}
    <span class="spacer"></span>
    {#if phase === 'running'}
      <span class="status busy" aria-live="polite">
        <span class="dot acc live"></span>
        <span class="txt">Pull running</span>
        <span class="ms">{clockOf(elapsedMs)}</span>
      </span>
    {:else if phase === 'done' && receipt}
      <span class="status" class:bad={!receipt.good}>
        <span class="dot" class:up={receipt.good} class:down={!receipt.good}></span>
        <span class="txt">{receipt.verdict}</span>
        <span class="ms">{clockOf(elapsedMs)}</span>
      </span>
    {/if}
    <a class="link" href="/audit">The record of every run</a>
  </div>

  {#if feeds.error}
    <div class="blank">
      <h2>The feed list could not be read</h2>
      <p>Nothing can be ingested until the server names a feed to ingest into.</p>
      <p class="mono down">{feeds.error}</p>
    </div>
  {:else if !active}
    <div class="blank">
      <h2>No feed is selected</h2>
      <p>
        Ingest writes into exactly one feed's store, so one has to be chosen before a window means
        anything. Pick one in the top bar.
      </p>
    </div>
  {:else if archiveHidden}
    <div class="blank">
      <h2>{active.display} has nothing to ingest yet</h2>
      <p>
        This is an archive feed: it reads CSV files that have to be bought first. The server refuses
        it for a reason of its own, quoted here rather than paraphrased.
      </p>
      <p class="warn mono">{active.why}</p>
      <p class="alt">
        <span class="lbl">What to do</span>
        Buy the archive and give the feed a store prefix, or select a broker feed in the top bar. The
        folder form appears here the day this feed reports itself ready — nothing on this page names
        a vendor.
      </p>
    </div>
  {:else}
    <div class="stats">
      <div class="stat">
        <span class="k">Names in this target</span>
        <span class="v">{catalogue.ready ? n(members.length) : '—'}</span>
        <span class="n">
          {#if catalogue.error}
            <span class="down">catalogue unavailable</span>
          {:else if !catalogue.ready}
            reading /instruments.json…
          {:else}
            of {n(catalogue.rows.length)} in {active.display}'s master
          {/if}
        </span>
      </div>
      <div class="stat">
        <span class="k">Window</span>
        <span class="v" class:down={windowDays > MAX_WINDOW_DAYS || (from && to && !windowOk)}>
          {windowOk ? n(windowDays) : '—'}
        </span>
        <span class="n">
          {#if windowOk}
            days · {windowMonths.length} month(s)
          {:else if from && to}
            the two dates are the wrong way round
          {:else}
            no window picked
          {/if}
        </span>
      </div>
      <div class="stat">
        <span class="k">Instrument-months asked</span>
        <span class="v">{expectedUnits ? n(expectedUnits) : '—'}</span>
        <span class="n">names × months — an estimate, not a server figure</span>
      </div>
      <div class="stat">
        <span class="k">Held in this window</span>
        <span class="v">{live ? n(live.units) : '—'}</span>
        <span class="n">{live ? `${n(live.rows)} bar(s) · measured from /store.json` : 'measured when a run starts'}</span>
      </div>
    </div>

    <div class="grid body">
      <div class="cols">
        <!-- ============================================ THE REQUEST ======= -->
        <section class="card">
          <header class="card-h">
            <span class="pane-title">The request</span>
            <span class="spacer"></span>
            <span class="tag" class:warn={problems.length > 0} class:up={problems.length === 0}>
              {problems.length === 0 ? 'ready' : `${problems.length} to fix`}
            </span>
          </header>

          <form onsubmit={start} class="form">
            <fieldset disabled={phase === 'running'}>
              <div class="fld">
                <span class="lbl">Target</span>
                <div class="seg">
                  {#each TARGETS as t (t.slug)}
                    <button
                      type="button"
                      class="utab"
                      aria-pressed={target === t.slug}
                      title={t.note}
                      onclick={() => (target = t.slug)}
                    >
                      {t.label}
                    </button>
                  {/each}
                </div>
                <span class="hint">{TARGETS.find((t) => t.slug === target)?.note}</span>
              </div>

              <div class="fld">
                <span class="lbl">Bar length</span>
                <div class="rungs">
                  {#each RUNGS as r (r.dir)}
                    <button
                      type="button"
                      class="rung"
                      aria-pressed={rung === r.dir}
                      class:on={rung === r.dir}
                      class:cold={!r.stored}
                      title={r.stored ? `${r.note} — the store has a directory for it` : `${r.note} — the store has NO directory for it`}
                      onclick={() => (rung = r.dir)}
                    >
                      <b>{r.label}</b>
                      <i class="mono">{r.dir}</i>
                    </button>
                  {/each}
                </div>
                <span class="hint">
                  Two rungs have a home on disk — <span class="mono">1min</span> and
                  <span class="mono">1day</span>. The others parse and are refused where the bar is
                  written, which is where that question belongs.
                </span>
              </div>

              <div class="fld two">
                <div class="field cal-open" class:bad={showProblems && problemFor.has('from')}>
                  <span class="lbl">From <i>inclusive</i></span>
                  <button
                    type="button"
                    id="cal-from"
                    class="dateb"
                    aria-haspopup="dialog"
                    aria-expanded={cal.field === 'from'}
                    onclick={() => openCal('from')}
                  >
                    <span class="mono" class:faint={!from}>{from || 'YYYY-MM-DD'}</span>
                    <span class="cal-i" aria-hidden="true"></span>
                  </button>
                  {#if cal.field === 'from'}{@render monthGrid()}{/if}
                </div>
                <div class="field cal-open" class:bad={showProblems && problemFor.has('to')}>
                  <span class="lbl">To <i>inclusive</i></span>
                  <button
                    type="button"
                    id="cal-to"
                    class="dateb"
                    aria-haspopup="dialog"
                    aria-expanded={cal.field === 'to'}
                    onclick={() => openCal('to')}
                  >
                    <span class="mono" class:faint={!to}>{to || 'YYYY-MM-DD'}</span>
                    <span class="cal-i" aria-hidden="true"></span>
                  </button>
                  {#if cal.field === 'to'}{@render monthGrid()}{/if}
                </div>
              </div>

              {#if isBroker}
                <p class="hint">
                  {active.display} is a broker. The window is split to its per-request cap by the
                  server, the rate budget is charged per request, and the newest day it will answer
                  for is <b class="mono">{brokerMax}</b> — today in IST is
                  <span class="mono">{istToday}</span>, and a session still running yields a partial
                  day this store cannot correct later.
                </p>
              {:else}
                <div class="field" class:bad={showProblems && problemFor.has('folder')}>
                  <span class="lbl">Folder</span>
                  <input class="search mono" bind:value={folder} placeholder="/path/to/vendor/csvs" />
                  <span class="hint">
                    CSV files already bought. No token, no rate budget and no "not today" — a file's
                    last day is not a question about the clock.
                  </span>
                </div>
              {/if}
            </fieldset>

            {#if showProblems && problems.length > 0}
              <ul class="probs panel-in" aria-live="polite">
                {#each problems as p (p.field + p.why)}
                  <li><span class="tag down">{p.field}</span><span class="msg">{p.why}</span></li>
                {/each}
              </ul>
            {/if}

            {#each cautions as c (c)}
              <p class="caution">
                <span class="tag warn">caution</span><span class="msg">{c}</span>
              </p>
            {/each}

            <details class="wire">
              <summary>What goes on the wire</summary>
              <p class="mono wirebody">POST /pull/spot<br />{wireBody}</p>
              <p class="hint">
                Every field the server reads is here. <span class="mono">granularity</span> is a
                field <span class="mono">api::ingest::parse_spot</span> has always parsed; the
                server's own form never emitted one, so an absent value has meant
                <span class="mono">1min</span> by omission.
              </p>
            </details>

            <div class="actions">
              <button class="btn primary" type="submit" disabled={phase === 'running'}>
                {#if phase === 'running'}
                  <span class="spin ring" aria-hidden="true"></span> Running…
                {:else}
                  Start {isBroker ? 'broker' : 'archive'} pull
                {/if}
              </button>
              {#if phase === 'done'}
                <button class="btn ghost" type="button" onclick={reset}>Clear the result</button>
              {/if}
            </div>
          </form>
        </section>

        <!-- ================================================ THE RUN ======= -->
        <section class="card">
          <header class="card-h">
            <span class="pane-title">The run</span>
            <span class="spacer"></span>
            {#if phase === 'running'}
              <span class="tag acc">in flight</span>
            {:else if phase === 'done'}
              <span class="tag" class:up={receipt?.good} class:down={receipt && !receipt.good}>
                {receipt?.verdict ?? (netError ? 'no answer' : 'finished')}
              </span>
            {:else}
              <span class="tag">idle</span>
            {/if}
          </header>

          {#if phase === 'idle'}
            <div class="empty">
              Nothing is running. Progress here is measured from
              <span class="mono">/store.json</span>, which the server rebuilds per request — so it
              is the store's own growth, not a timer pretending to be one.
            </div>
          {:else}
            <div class="run">
              <div class="runrow">
                <span class="lbl">Elapsed</span>
                <span class="mono big">{clockOf(elapsedMs)}</span>
                <span class="hint">measured from the moment the request left this browser</span>
              </div>

              <div class="runrow">
                <span class="lbl">Store growth <i>measured</i></span>
                <span class="mono big" class:up={rowsGained > 0}>{n(rowsGained)}</span>
                <span class="hint">
                  bar(s) across {n(unitsDone)} new instrument-month(s), read from
                  <span class="mono">/store.json</span> every 5 s
                </span>
              </div>

              <!-- THE BAR IS DETERMINATE ONLY WHERE THERE IS SOMETHING TO
                   DETERMINE IT. Before the store has grown there is no
                   measurement, so the bar says "unknown" by being
                   indeterminate rather than by inching along on a timer. -->
              {#if expectedUnits > 0 && unitsDone > 0}
                <div class="meter" role="progressbar" aria-valuenow={Math.round(share * 100)} aria-valuemin="0" aria-valuemax="100" aria-label="Estimated share of the request filled">
                  <i style="width:{(share * 100).toFixed(2)}%"></i>
                </div>
                <p class="hint">
                  <b class="mono">{(share * 100).toFixed(1)}%</b> of
                  <span class="mono">{n(expectedUnits)}</span> instrument-months are now held.
                  <b class="warn">This share is an estimate.</b> The denominator is names × months
                  from your own request — the server states no total, and a name the vendor has no
                  history for will never fill.
                </p>
              {:else if phase === 'running'}
                <div class="meter indet"><i></i></div>
                <p class="hint">
                  No growth measured yet, so there is no share to show. The bar is indeterminate on
                  purpose: a number here would be invented.
                </p>
              {/if}

              {#if etaSecs !== null}
                <p class="hint">
                  <span class="tag info">extrapolation</span>
                  About <b class="mono">{clockOf(etaSecs * 1000)}</b> left, from a measured
                  <span class="mono">{rate.toFixed(2)}</span> instrument-months/s over the last
                  {samples.length} readings and {n(unitsLeft)} still to fill. It is a straight-line
                  projection of the recent past and nothing more.
                </p>
              {/if}

              <!-- THE GOVERNOR, AS FAR AS IT CAN HONESTLY BE SEEN ------------- -->
              <div class="gov" class:hot={stalled}>
                <span
                  class="dot"
                  class:warn={stalled}
                  class:up={!stalled && everGrew}
                  class:acc={!stalled && !everGrew}
                  class:live={phase === 'running'}
                ></span>
                <div>
                  {#if phase !== 'running'}
                    <b>Rate governor — not observable</b>
                    <span class="hint">
                      <span class="mono">crates/pull/src/rate.rs</span> runs an AIMD governor inside
                      the synchronous POST. No route reports its state, so this page never claims to
                      read it.
                    </span>
                  {:else if stalled}
                    <b class="warn">
                      {everGrew
                        ? `Nothing has landed for ${n(stalledSecs)}s`
                        : `Nothing has landed at all, ${n(stalledSecs)}s in`}
                    </b>
                    <span class="hint">
                      That is what a governor waiting for its next permit looks like from outside —
                      and also what a slow vendor, a long window split into chunks, or a credential
                      round-trip looks like. They cannot be told apart from the browser: there is no
                      status route. The request is still open, so this is not yet a hang.
                    </span>
                  {:else if everGrew}
                    <b class="up">Bars are landing</b>
                    <span class="hint">
                      Last growth {n(stalledSecs)}s ago. The governor is inside the request and
                      cannot be read directly; a store that keeps growing is the evidence that it is
                      admitting requests.
                    </span>
                  {:else}
                    <b>Nothing has landed yet</b>
                    <span class="hint">
                      {n(stalledSecs)}s in and the store has not grown. Early in a run that is
                      normal — the credential, the identity and the first chunk all come before the
                      first bar. Nothing is claimed either way until the store moves.
                    </span>
                  {/if}
                </div>
              </div>

              {#if pollError}
                <p class="caution">
                  <span class="tag down">watcher</span>
                  <span class="msg">
                    The progress reading failed and is not being silently retried into a frozen
                    display: {pollError}
                  </span>
                </p>
              {/if}

              {#if phase === 'running'}
                <div class="actions">
                  <button class="btn danger" type="button" onclick={stopWatching}>Stop watching</button>
                  <span class="hint">
                    This aborts the browser's request only. There is no cancel route — a pull is one
                    synchronous POST, and the server keeps going until it is done.
                  </span>
                </div>
              {/if}

              {#if aborted}
                <p class="caution">
                  <span class="tag warn">abandoned</span>
                  <span class="msg">
                    You stopped watching. The server was not told, so the run is most likely still
                    going; the outcome list below is a difference against whatever had landed when
                    you stopped. <a class="link" href="/audit">The audit record</a> is where the
                    finished run will appear.
                  </span>
                </p>
              {/if}

              {#if netError}
                <p class="caution">
                  <span class="tag down">no answer</span><span class="msg">{netError}</span>
                </p>
              {/if}

              {#if receipt}
                <div class="verdict" class:bad={!receipt.good}>
                  <b>{receipt.verdict}</b>
                  <span>{receipt.reason}</span>
                </div>
                <table class="kv">
                  <tbody>
                    {#each receipt.facts as f, i (f.k + i)}
                      <tr class="row-in" style="animation-delay:{Math.min(i, 14) * 12}ms">
                        <th>{f.k}</th>
                        <td class:mono={/^[\d.,\s]+$/.test(f.v)}>{f.v}</td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
                <details class="wire" bind:open={showRaw}>
                  <summary>The server's own receipt page</summary>
                  {#if showRaw}
                    <!-- SANDBOXED, AND WITHOUT allow-scripts. The page is read
                         above as text; this frame is only for comparing what
                         the server drew against what was read out of it. -->
                    <iframe title="Server receipt, sandboxed" sandbox="" srcdoc={receipt.raw}></iframe>
                  {/if}
                </details>
              {/if}
            </div>
          {/if}
        </section>
      </div>

      <!-- ============================================== THE OUTCOMES ====== -->
      <section class="card wide">
        <header class="card-h">
          <span class="pane-title">Every instrument this request named</span>
          <span class="spacer"></span>
          {#if outcomes.length > 0}
            <span class="tag">{n(filtered.length)} shown of {n(outcomes.length)}</span>
          {/if}
        </header>

        {#if phase === 'idle' || outcomes.length === 0}
          <div class="empty">
            {#if phase === 'running'}
              The outcome list is built when the run answers — it is a difference between the store
              before and the store after, and the second reading does not exist yet.
            {:else if phase === 'done'}
              No outcome rows. The target named
              {n(members.length)} instrument(s) and nothing could be compared — most likely the
              catalogue is unavailable, which the header says.
            {:else}
              One row per instrument the target names, with what happened to it. Start a run and it
              fills in.
            {/if}
          </div>
        {:else}
          {#if hiddenFailures > 0}
            <p class="caution loud">
              <span class="tag down">truncated by the server</span>
              <span class="msg">
                This run recorded <b>{n(failedCount)}</b> failed member(s) and put
                <b>{n(namedFailures.length)}</b> reason(s) on the wire.
                <span class="mono">crates/api/src/server.rs</span> renders
                <span class="mono">done.failures.iter().take(5)</span>, so
                <b>{n(hiddenFailures)}</b> reason(s) were never sent and cannot be shown here. The
                rows below fall back to the measured store difference for those members, which says
                WHAT happened but not WHY.
              </span>
            </p>
          {/if}

          <div class="gchips">
            <button
              class="utab"
              aria-pressed={pickedGroup === null}
              onclick={() => (pickedGroup = null)}
            >
              All <span class="n">{n(outcomes.length)}</span>
            </button>
            {#each groups as g (g.group)}
              <button
                class="utab"
                aria-pressed={pickedGroup === g.group}
                onclick={() => (pickedGroup = pickedGroup === g.group ? null : g.group)}
              >
                <span class="dot" class:up={g.tone === 'up'} class:down={g.tone === 'down'} class:warn={g.tone === 'warn'} class:acc={g.tone === 'info'}></span>
                {g.group} <span class="n">{n(g.count)}</span>
              </button>
            {/each}
          </div>

          {#if pickedGroup}
            {#each groups.filter((g) => g.group === pickedGroup) as g (g.group)}
              <ul class="reasons">
                {#each [...g.reasons.entries()].sort((a, b) => b[1] - a[1]) as [why, count] (why)}
                  <li><span class="tag" class:down={g.tone === 'down'} class:warn={g.tone === 'warn'} class:up={g.tone === 'up'}>{n(count)}</span>{why}</li>
                {/each}
              </ul>
            {/each}
          {/if}

          <div class="obar">
            <input
              class="search"
              type="search"
              bind:value={q}
              placeholder="Filter by symbol — one Map probe per keystroke, never a scan"
              aria-label="Filter outcomes by symbol"
            />
          </div>

          <div class="ohead">
            <button class="sort" onclick={() => sortBy('symbol')}>
              Instrument{sortKey === 'symbol' ? (sortDir > 0 ? ' ▲' : ' ▼') : ''}
            </button>
            <button class="sort" onclick={() => sortBy('group')}>
              Outcome{sortKey === 'group' ? (sortDir > 0 ? ' ▲' : ' ▼') : ''}
            </button>
            <button class="sort num" onclick={() => sortBy('gained')}>
              Bars gained{sortKey === 'gained' ? (sortDir > 0 ? ' ▲' : ' ▼') : ''}
            </button>
            <span>Why</span>
          </div>

          <div class="vlist olist" bind:this={vEl} onscroll={onScroll}>
            {#if filtered.length === 0}
              <div class="empty">
                Nothing matches <span class="mono">{q}</span>{pickedGroup ? ` in "${pickedGroup}"` : ''}.
                {n(outcomes.length)} row(s) exist — clear the filter to see them.
              </div>
            {:else}
              <div class="vspace" style="height:{filtered.length * ROW_H}px">
                {#each vSlice as row, i (row.key)}
                  <div class="orow" style="top:{(vFirst + i) * ROW_H}px" title={row.reason}>
                    <span class="sym">{row.symbol}</span>
                    <span class="tag" class:up={row.tone === 'up'} class:down={row.tone === 'down'} class:warn={row.tone === 'warn'} class:info={row.tone === 'info'}>{row.group}</span>
                    <span class="num" class:up={row.gained > 0}>{row.gained > 0 ? `+${n(row.gained)}` : '0'}</span>
                    <span class="why">{row.reason}</span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>

          <p class="hint foot">
            "Bars gained" is the difference between two readings of
            <span class="mono">/store.json</span> across the {windowMonths.length} month(s) of this
            window — measured, not reported by the run. A reason in the run's own words appears
            wherever the server sent one.
          </p>
        {/if}
      </section>
    </div>
  {/if}
</div>

<!-- ===================================================================== -->
<!-- THE MONTH GRID. Years, then months, then the days of one month — the   -->
<!-- shape crates/api/src/calendar.rs settled on after its first build put  -->
<!-- twelve years, twelve months and thirty-one days on screen at once and  -->
<!-- two open pickers overlapped each other. One panel exists at a time     -->
<!-- because `cal.field` is a single value.                                 -->
<!-- ===================================================================== -->
{#snippet monthGrid()}
  <div
    class="cal panel-in"
    class:up={cal.up}
    role="dialog"
    aria-label="Choose a date"
    bind:this={calEl}
    onkeydown={onCalKey}
    tabindex="-1"
  >
    <div class="cal-h">
      {#if cal.view === 'day'}
        <button type="button" class="nav" aria-label="Previous month" onclick={() => stepMonth(-1)}>‹</button>
        <button type="button" class="title" onclick={() => (cal = { ...cal, view: 'month' })}>
          {MONTHS[cal.m - 1]} {cal.y}
        </button>
        <button type="button" class="nav" aria-label="Next month" onclick={() => stepMonth(1)}>›</button>
      {:else if cal.view === 'month'}
        <button type="button" class="nav" aria-label="Previous year" onclick={() => (cal = { ...cal, y: cal.y - 1 })}>‹</button>
        <button type="button" class="title" onclick={() => (cal = { ...cal, view: 'year' })}>{cal.y}</button>
        <button type="button" class="nav" aria-label="Next year" onclick={() => (cal = { ...cal, y: cal.y + 1 })}>›</button>
      {:else}
        <span class="title as-text">Choose a year</span>
      {/if}
    </div>

    {#if cal.view === 'day'}
      <div class="dow" aria-hidden="true">
        {#each WEEKDAYS as w (w)}<span>{w}</span>{/each}
      </div>
      <div class="days" role="grid">
        {#each calDays as day, i (day ?? `pad${i}`)}
          {#if day === null}
            <span class="pad"></span>
          {:else}
            <button
              type="button"
              class="day"
              data-day={day}
              disabled={!inRange(day)}
              tabindex={day === cal.cursor ? 0 : -1}
              aria-current={day === (cal.field === 'from' ? from : to) ? 'date' : undefined}
              class:picked={day === (cal.field === 'from' ? from : to)}
              class:cursor={day === cal.cursor}
              class:today={day === istToday}
              onclick={() => commit(day)}
            >
              {parts(day).d}
            </button>
          {/if}
        {/each}
      </div>
      <div class="cal-f">
        <button type="button" class="btn ghost sm" onclick={() => commit(maxDay)}>
          {isBroker ? 'Yesterday' : 'Today'} · {maxDay}
        </button>
        <span class="spacer"></span>
        <button type="button" class="btn ghost sm" onclick={() => shutCal()}>Close</button>
      </div>
    {:else if cal.view === 'month'}
      <div class="months">
        {#each MONTHS_SHORT as label, i (label)}
          <button
            type="button"
            class="mo"
            class:on={cal.m === i + 1}
            onclick={() => pickMonth(i + 1)}
          >
            {label}
          </button>
        {/each}
      </div>
    {:else}
      <div class="years">
        {#each calYears as y (y)}
          <button type="button" class="yr" class:on={cal.y === y} onclick={() => (cal = { ...cal, y, view: 'month' })}>
            {y}
          </button>
        {/each}
      </div>
    {/if}
  </div>
{/snippet}

<style>
  /* Everything below is built from the tokens in $lib/theme.css. No colour,
     radius, duration or step is spelled twice — a hex code here would be a
     second theme that the toggle does not reach. */

  .body {
    padding: var(--s6);
    display: flex;
    flex-direction: column;
    gap: var(--s6);
  }
  .cols {
    display: grid;
    grid-template-columns: minmax(0, 5fr) minmax(0, 4fr);
    gap: var(--s6);
    align-items: start;
  }
  @media (max-width: 1100px) {
    .cols {
      grid-template-columns: minmax(0, 1fr);
    }
  }

  /* NO `overflow: hidden` HERE. It squared off the header's top corners neatly
     and CLIPPED THE CALENDAR: the panel is taller than the space left under the
     date row, so the last week of the month and the whole footer were cut off
     by the card edge. The header rounds its own two corners instead, which
     costs one line and does not eat a popover. */
  .card {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r4);
    box-shadow: var(--e1);
    min-width: 0;
  }
  .card-h {
    display: flex;
    align-items: center;
    gap: var(--s4);
    padding: var(--s4) var(--s5);
    background: var(--bg-2);
    border-bottom: 1px solid var(--line);
    border-radius: var(--r4) var(--r4) 0 0;
  }
  .spacer {
    flex: 1;
  }

  /* ---- the form ---- */
  .form {
    display: flex;
    flex-direction: column;
    gap: var(--s6);
    padding: var(--s6) var(--s5);
  }
  fieldset {
    border: 0;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--s6);
    min-width: 0;
  }
  fieldset:disabled {
    opacity: 0.55;
  }
  .fld,
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    min-width: 0;
  }
  .fld.two {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--s5);
  }
  .lbl {
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
  }
  .lbl i {
    font-style: normal;
    color: var(--n7);
    letter-spacing: 0;
    text-transform: none;
  }
  .hint {
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--dim);
  }
  .hint.foot {
    padding: var(--s4) var(--s5) var(--s5);
    border-top: 1px solid var(--line-soft);
  }

  .seg {
    display: flex;
    gap: var(--s2);
    flex-wrap: wrap;
  }

  /* The bar-length ladder. Every rung the parser accepts is offered; the ones
     the store has no directory for are dimmed and struck rather than hidden,
     because a control that silently omits a legal value is the same lie as one
     that silently adds one. */
  .rungs {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(84px, 1fr));
    gap: var(--s2);
  }
  .rung {
    display: flex;
    flex-direction: column;
    gap: 1px;
    align-items: flex-start;
    padding: var(--s3) var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    cursor: pointer;
    text-align: left;
  }
  .rung b {
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
  }
  .rung i {
    font-style: normal;
    font-size: var(--fs-micro);
    color: var(--faint);
  }
  .rung:hover {
    border-color: var(--line-hard);
    background: var(--panel-2);
  }
  .rung.on {
    border-color: var(--acc);
    background: var(--acc-soft);
  }
  .rung.cold b {
    color: var(--dim);
  }
  .rung.cold i {
    text-decoration: line-through;
  }
  .rung.cold.on {
    border-color: var(--warn);
    background: var(--warn-soft);
  }

  /* ---- the date button and its panel ---- */
  .field {
    position: relative;
  }
  .dateb {
    display: flex;
    align-items: center;
    gap: var(--s4);
    width: 100%;
    padding: 7px var(--s5);
    border: 1px solid var(--line);
    border-radius: var(--r);
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    cursor: pointer;
    text-align: left;
  }
  .dateb:hover {
    border-color: var(--line-hard);
  }
  .dateb[aria-expanded='true'] {
    border-color: var(--acc);
    box-shadow: 0 0 0 3px var(--acc-soft);
  }
  .dateb .mono {
    flex: 1;
  }
  .cal-i {
    width: 13px;
    height: 12px;
    border: 1.5px solid var(--faint);
    border-radius: 2px;
    border-top-width: 4px;
    flex: none;
  }
  .field.bad .dateb,
  .field.bad .search {
    border-color: var(--down);
    background: var(--down-soft);
  }

  .cal {
    position: absolute;
    z-index: 40;
    top: calc(100% + var(--s2));
    left: 0;
    width: 268px;
    padding: var(--s4);
    background: var(--panel);
    border: 1px solid var(--line-hard);
    border-radius: var(--r4);
    box-shadow: var(--e3);
  }
  .cal.up {
    top: auto;
    bottom: calc(100% + var(--s2));
  }
  .cal-h {
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin-bottom: var(--s4);
  }
  .cal-h .nav,
  .cal-h .title {
    font: inherit;
    border: 1px solid transparent;
    border-radius: var(--r2);
    background: none;
    color: var(--ink);
    cursor: pointer;
    padding: var(--s2) var(--s4);
  }
  .cal-h .nav {
    font-size: var(--fs-md);
    line-height: 1;
    color: var(--dim);
  }
  .cal-h .title {
    flex: 1;
    font-weight: var(--w-semi);
    font-size: var(--fs-sm);
  }
  .cal-h .title.as-text {
    cursor: default;
    text-align: center;
    color: var(--faint);
    text-transform: uppercase;
    letter-spacing: var(--track-caps);
    font-size: var(--fs-mini);
  }
  .cal-h .nav:hover,
  .cal-h .title:hover {
    background: var(--panel-2);
    border-color: var(--line);
  }
  .dow,
  .days {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
  }
  .dow span {
    text-align: center;
    font-size: var(--fs-micro);
    font-weight: var(--w-bold);
    letter-spacing: 0.06em;
    color: var(--faint);
    padding-bottom: var(--s2);
  }
  .day,
  .mo,
  .yr {
    font: inherit;
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    font-size: var(--fs-sm);
    padding: var(--s3) 0;
    border: 1px solid transparent;
    border-radius: var(--r2);
    background: none;
    color: var(--ink);
    cursor: pointer;
  }
  .day:hover:not(:disabled),
  .mo:hover,
  .yr:hover {
    background: var(--panel-2);
  }
  .day:disabled {
    color: var(--n7);
    cursor: not-allowed;
  }
  .day.today {
    box-shadow: inset 0 -2px 0 var(--faint);
  }
  .day.cursor {
    border-color: var(--acc);
  }
  .day.picked {
    background: var(--acc);
    border-color: var(--acc);
    color: var(--on-acc);
    font-weight: var(--w-bold);
  }
  .pad {
    display: block;
  }
  .months,
  .years {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--s2);
  }
  .mo.on,
  .yr.on {
    background: var(--acc-soft);
    border-color: var(--acc);
  }
  .cal-f {
    display: flex;
    align-items: center;
    gap: var(--s3);
    margin-top: var(--s4);
    padding-top: var(--s4);
    border-top: 1px solid var(--line-soft);
  }
  .btn.sm {
    padding: var(--s2) var(--s4);
    font-size: var(--fs-xs);
  }

  /* ---- validation and cautions ---- */
  .probs {
    list-style: none;
    margin: 0;
    padding: var(--s4) var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    background: var(--down-soft);
    border: 1px solid var(--down);
    border-radius: var(--r3);
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
  }
  .probs li {
    display: flex;
    gap: var(--s4);
    align-items: baseline;
  }
  .caution {
    display: flex;
    gap: var(--s4);
    align-items: baseline;
    margin: 0;
    padding: var(--s4) var(--s5);
    background: var(--warn-soft);
    border-left: 2px solid var(--warn);
    border-radius: var(--r2);
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--ink-2);
  }
  .caution.loud {
    background: var(--down-soft);
    border-left-color: var(--down);
    margin: var(--s5) var(--s5) 0;
  }
  /* THE MESSAGE IS ONE FLEX ITEM, NOT SEVEN. Without this wrapper every inline
     `<b>` and `<span class="mono">` inside a `.caution` became a flex item of
     its own, and the truncation banner laid itself out as a row of words with
     ragged gaps between them instead of a sentence. */
  .probs .msg,
  .caution .msg {
    flex: 1;
    min-width: 0;
  }

  .wire {
    border: 1px solid var(--line);
    border-radius: var(--r3);
    padding: var(--s4) var(--s5);
    background: var(--well);
  }
  .wire summary {
    cursor: pointer;
    font-size: var(--fs-xs);
    font-weight: var(--w-semi);
    color: var(--dim);
  }
  .wirebody {
    margin: var(--s4) 0 var(--s3);
    font-size: var(--fs-xs);
    line-height: 1.6;
    word-break: break-all;
    color: var(--ink);
  }
  .wire iframe {
    width: 100%;
    height: 55vh;
    margin-top: var(--s4);
    border: 1px solid var(--line);
    border-radius: var(--r2);
    background: var(--n2);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: var(--s5);
    flex-wrap: wrap;
  }
  .ring {
    width: 12px;
    height: 12px;
    border: 2px solid currentColor;
    border-right-color: transparent;
    border-radius: 50%;
    display: inline-block;
  }

  /* ---- the run ---- */
  .run {
    display: flex;
    flex-direction: column;
    gap: var(--s5);
    padding: var(--s6) var(--s5);
  }
  .runrow {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .big {
    font-size: var(--fs-xl);
    font-weight: var(--w-bold);
    letter-spacing: -0.5px;
    line-height: var(--lh-tight);
  }
  .meter {
    position: relative;
    height: 6px;
    border-radius: var(--r-full);
    background: var(--well);
    overflow: hidden;
  }
  .meter i {
    display: block;
    height: 100%;
    background: var(--acc);
    border-radius: var(--r-full);
  }
  .meter.indet i {
    position: absolute;
    inset: 0;
    width: 34%;
    background: linear-gradient(90deg, transparent, var(--acc), transparent);
  }
  .gov {
    display: flex;
    gap: var(--s4);
    align-items: flex-start;
    padding: var(--s4) var(--s5);
    border: 1px dashed var(--line-hard);
    border-radius: var(--r3);
    background: var(--bg-2);
  }
  .gov.hot {
    border-color: var(--warn);
    background: var(--warn-soft);
  }
  .gov .dot {
    margin-top: 5px;
  }
  .gov b {
    display: block;
    font-size: var(--fs-sm);
  }

  .verdict {
    padding: var(--s5);
    border-radius: var(--r3);
    background: var(--up-soft);
    border-left: 3px solid var(--up);
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
  }
  .verdict.bad {
    background: var(--down-soft);
    border-left-color: var(--down);
  }
  .verdict b {
    display: block;
    font-size: var(--fs-sm);
    letter-spacing: var(--track-caps);
    margin-bottom: var(--s2);
  }
  table.kv {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-xs);
  }
  table.kv th {
    text-align: left;
    vertical-align: top;
    white-space: nowrap;
    padding: var(--s3) var(--s5) var(--s3) 0;
    color: var(--faint);
    font-weight: var(--w-semi);
    border-bottom: 1px solid var(--line-soft);
    width: 1%;
  }
  /* `white-space: normal` UNDOES A GLOBAL. theme.css sets
     `tbody td { white-space: nowrap }` for the market tables, which is right
     there and wrong here: a receipt's `Balances` line is a sentence, and
     nowrap turned it into one 900-pixel row that pushed the card's own width
     past the panel edge. */
  table.kv td {
    padding: var(--s3) 0;
    border-bottom: 1px solid var(--line-soft);
    color: var(--ink);
    line-height: var(--lh-base);
    white-space: normal;
    overflow-wrap: anywhere;
  }

  /* ---- the outcome list ---- */
  .gchips {
    display: flex;
    gap: var(--s2);
    flex-wrap: wrap;
    padding: var(--s5) var(--s5) var(--s4);
  }
  .gchips .n {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    color: var(--faint);
    margin-left: var(--s2);
  }
  .gchips .dot {
    margin-right: var(--s3);
  }
  .reasons {
    list-style: none;
    margin: 0 var(--s5);
    padding: var(--s4) var(--s5);
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    background: var(--well);
    border-radius: var(--r3);
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
  }
  .reasons li {
    display: flex;
    gap: var(--s4);
    align-items: baseline;
  }
  .obar {
    padding: var(--s4) var(--s5);
  }
  .ohead,
  .orow {
    display: grid;
    grid-template-columns: 9rem 9.5rem 6.5rem minmax(0, 1fr);
    gap: var(--s5);
    align-items: center;
    padding: 0 var(--s5);
  }
  .ohead {
    padding-bottom: var(--s3);
    border-bottom: 1px solid var(--line);
    background: var(--bg-2);
    padding-top: var(--s3);
  }
  .ohead > * {
    font-size: var(--fs-mini);
    font-weight: var(--w-bold);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--faint);
    text-align: left;
  }
  .ohead .sort {
    background: none;
    border: 0;
    padding: 0;
    font: inherit;
    color: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    cursor: pointer;
  }
  .ohead .sort:hover {
    color: var(--ink);
  }
  .ohead .sort.num {
    text-align: right;
  }
  .olist {
    height: min(46vh, 520px);
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .vspace {
    position: relative;
    width: 100%;
  }
  .orow {
    position: absolute;
    left: 0;
    right: 0;
    height: var(--row-h);
    border-bottom: 1px solid var(--line-soft);
    font-size: var(--fs-xs);
    font-variant-numeric: tabular-nums;
  }
  .orow:hover {
    background: var(--panel-2);
  }
  .orow .sym {
    font-family: var(--mono);
    font-weight: 650;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .orow .num {
    font-family: var(--mono);
    text-align: right;
  }
  .orow .why {
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .orow .tag {
    justify-self: start;
  }

  .blank .alt {
    display: block;
    margin-top: var(--s6);
    text-align: left;
    font-size: var(--fs-xs);
    line-height: var(--lh-base);
    color: var(--dim);
  }

  /* EVERY transition and animation this file adds is inside the guard, the
     same rule $lib/theme.css holds itself to. A reduced-motion operator gets
     the whole console with nothing moving. */
  @media (prefers-reduced-motion: no-preference) {
    .rung,
    .dateb,
    .day,
    .mo,
    .yr,
    .cal-h .nav,
    .cal-h .title,
    .orow {
      transition:
        background-color var(--d-hover) var(--ease-out),
        border-color var(--d-hover) var(--ease-out),
        color var(--d-hover) var(--ease-out),
        box-shadow var(--d-state) var(--ease-out);
    }
    .meter i {
      transition: width var(--d-panel) var(--ease-out);
    }
    .meter.indet i {
      animation: bx-indeterminate var(--d-sweep) var(--ease-in-out) infinite;
      transition: none;
    }
  }
</style>

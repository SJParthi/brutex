<script>
  /**
   * /db — THE SOURCE COLUMN AND ITS DRAWER (WP-55).
   *
   * Findings DB-078 (no number on /db could be audited back to its source),
   * DB-A40 (the drawer rendered nothing), DB-G56 (zip or tick store, never
   * recorded) and DB-G42 (the closing auction's print shown as an ordinary
   * trade); decisions D-3334, D-3320, D-3323 and D-3324.
   *
   * # What it shows
   *
   * Under the grid, ONE LINE PER RUN of the page's rows that share a month
   * file and an IST day (`sourceRuns`), and one line for runs that say the
   * same thing (`foldRuns`: an option chain's contracts, imported on one day
   * by one journal line): the rows it covers, the feed, the
   * price basis ("back-adjusted as of <pull date>" or "raw, as traded"), the
   * close ("official close" or "last print"), and where those bars came from:
   * a sidecar record when the month has one, else the GDFL import journal's
   * line for the day with the archive its run recorded (or, written before
   * D-3317, that none was, beside where this build's reader would find the
   * day), else the Zerodha pull run that wrote the month and its pull date.
   * Every row of the page is in exactly one line, so every row is labelled.
   *
   * On a CASH day from the closing auction's start (`/feeds.json`
   * `closing_auction.from`) the line also names each row in the closing-auction
   * session, and the one that is the closing auction's print (D-3320), from the
   * rows' own phases, the session's last print and the day's auction record —
   * never from a clock literal.
   *
   * "Details" opens the drawer on one bar: everything `/provenance.json?…&t=`
   * says about it, the bar as served, its phase and the verdict above, laid out
   * by `drawerFacts`.
   *
   * # Why the column is drawn under the grid and not inside it
   *
   * The grid's columns are positional (`nth-child` rules over `BAR_COLS`) and
   * belong to WP-52's files; WP-00 mounted this component under the grid so its
   * owner could fill it without touching them. A cell per row would repeat one
   * import day fifty times on a page of seconds; a line says it once and
   * names the rows it covers, and the drawer's row picker reaches any row.
   *
   * # Cost — bounded by the page, never by the history
   *
   * One `/provenance.json` per month file on the page (at most
   * `PROV_FILES_MAX`; a page of one series straddles one or two, and a line
   * past the cap says it was not read), answered by a path probe and the
   * month's journal rows (O(days in the month), stated by the route). On a
   * feed that files the auction's print apart (GDFL), two more per CASH day on
   * the page from the closing auction's start, for at most `CAS_DAYS_MAX`
   * share-days (the rest are counted and said): that day's bars of the
   * session (one day, at most `CAS_TAIL_LIMIT`) and `/sources/day.json` for
   * its auction record (cached per day by the server). Zerodha needs neither:
   * each row's own phase is the whole answer there.
   * The drawer adds one `/provenance.json?t=` and one `/bars/window.json?at=`
   * seek. Answers are kept by URL, at most `PROV_CACHE_MAX`, so paging back is
   * free and "Read again" asks afresh.
   *
   * Owner after WP-00: WP-55.
   */
  import { untrack } from 'svelte';
  import { ask } from '$lib/ask.js';
  import { dayLabel, barTimeLabel, secondLabel, istDay } from '$lib/dates.js';
  import {
    TF_SECS,
    fmt,
    filesOnPage,
    sourceRuns,
    foldRuns,
    rowsLabel,
    sourceOfDay,
    provenanceUrl,
    barAtUrl,
    casTailUrl,
    dayFactsUrl,
    priceBadge,
    closeBadge,
    auctionMarks,
    auctionDaysOnPage,
    reachesAuction,
    sourceNotRead,
    drawerFacts,
    shareBasisUrl,
    basisRunsOf,
    basisRunLabel,
    segmentOfKey,
    remember,
    PROV_FILES_MAX,
    CAS_DAYS_MAX
  } from '$lib/db/notes-labels.js';

  /** @type {{ db: any }} */
  let { db } = $props();

  /**
   * Every answer read, by URL: `{ state: 'loading' }`, `{ state: 'ok', body,
   * ms }` or `{ state: 'error', why, ms }`. Raw state: an answer is replaced,
   * never edited, and a deep proxy over a month's journal rows buys nothing.
   * @type {Map<string, any>}
   */
  let answers = $state.raw(new Map());

  const feed = $derived(db.activeFeed ?? null);
  const wire = $derived(typeof feed?.wire === 'string' && feed.wire !== '' ? feed.wire : null);
  const display = $derived(typeof feed?.display === 'string' ? feed.display : (wire ?? ''));
  const rows = $derived(db.view === 'bars' && Array.isArray(db.barPage) ? db.barPage : []);
  const files = $derived(filesOnPage(rows));
  /** The month files whose source is read: the first `PROV_FILES_MAX` on the page. */
  const fileRead = $derived(new Set(files.files.map((f) => f.key)));
  const runs = $derived(sourceRuns(rows));
  /** The share-days whose closing-auction session is read, and how many are not. */
  const casDays = $derived(auctionDaysOnPage(runs, feed));

  /** @param {{ instrument: string, tf: string, month: string }} f */
  function monthUrl(f) {
    return provenanceUrl({ feed: wire ?? '', instrument: f.instrument, timeframe: f.tf, month: f.month });
  }

  /**
   * The closing-auction reads one run needs, or `null`: a CASH run below the
   * day rung, on or after the day the feed's closing auction starts, on a feed
   * that files the auction's print apart. `always` reads it past the page's
   * cap — the drawer asks about one row, so one day.
   * @param {any} run
   * @param {boolean} [always]
   */
  function casUrls(run, always = false) {
    const a = feed?.closing_auction;
    if (!wire || !a || a.handling !== 'print_kept_apart' || !reachesAuction(run, feed)) return null;
    if (!always && !casDays.read.has(`${run.instrument}|${run.day}`)) return null;
    return {
      tail: casTailUrl({
        feed: wire,
        instrument: run.instrument,
        timeframe: run.tf,
        month: run.month,
        day: run.day,
        from: a.eligible_continuous_close
      }),
      day: dayFactsUrl(run.instrument, run.day)
    };
  }

  /** @param {string} url */
  async function load(url) {
    if (answers.has(url)) return;
    answers = remember(answers, url, { state: 'loading' });
    const started = performance.now();
    try {
      const res = await ask(url);
      /** @type {any} */
      let body = null;
      try {
        body = await res.json();
      } catch {
        body = null;
      }
      const ms = Math.round(performance.now() - started);
      // A MONTH WITH NO FILE IS AN ANSWER, not a failure: the route says
      // `held: false` and names the path under a 404.
      if (res.ok || body?.held === false) answers = remember(answers, url, { state: 'ok', body, ms });
      else
        answers = remember(answers, url, {
          state: 'error',
          why: typeof body?.error === 'string' ? body.error : `HTTP ${res.status} from ${url.split('?')[0]}`,
          ms
        });
    } catch (why) {
      answers = remember(answers, url, { state: 'error', why: why instanceof Error ? why.message : String(why) });
    }
  }

  /** @param {string | null | undefined} url */
  const got = (url) => (url ? answers.get(url) : undefined);
  /** @param {string | null | undefined} url */
  const bodyOf = (url) => {
    const a = got(url);
    return a?.state === 'ok' ? a.body : null;
  };

  /**
   * A ZERODHA SHARE'S ADJUSTED OR QUANTISED RUNS (R3-13 goal 4; DB-G38,
   * DB-G40): one `/sources/day.json` over the page's days of one share, at most
   * its newest 31 days (the route's page), so each row is labelled with the run
   * its day belongs to, first and last day, from the served factor. `null`
   * for another feed, a page of no share or of more than one.
   */
  const basisUrl = $derived.by(() => {
    if (wire !== 'zerodha') return null;
    const share = runs.filter((r) => segmentOfKey(r.instrument) === 'CASH');
    const keys = new Set(share.map((r) => r.instrument));
    if (keys.size !== 1) return null;
    const days = share.flatMap((r) => [istDay(r.first.ts), istDay(r.last.ts)]).filter(Boolean).sort();
    const to = days[days.length - 1] ?? '';
    const floor = to ? new Date(Date.parse(`${to}T00:00:00Z`) - 30 * 86_400_000).toISOString().slice(0, 10) : '';
    const from = days[0] && days[0] > floor ? days[0] : floor;
    return shareBasisUrl([...keys][0], from, to);
  });
  /** The runs `basisUrl` answered, oldest first; `[]` until it answers. */
  const basisRuns = $derived(basisRunsOf(bodyOf(basisUrl)));

  /** Every URL the page's run lines need. */
  const pageUrls = $derived.by(() => {
    if (!wire) return [];
    /** @type {string[]} */
    const want = files.files.map(monthUrl);
    if (basisUrl) want.push(basisUrl);
    for (const run of runs) {
      const c = casUrls(run);
      if (c) {
        want.push(c.tail);
        if (c.day) want.push(c.day);
      }
    }
    return want;
  });

  /* THE ANSWERS ARE TRACKED HERE, ON PURPOSE. A URL the page needs can leave
     the cache — "Read again" forgets it, and the drawer's reads can push it
     out past `PROV_CACHE_MAX` — and the page's URLs do not change when it
     does, so an effect that followed only them would leave its line saying
     "reading" with nothing being read. Following the cache too, a missing
     answer is asked for again. It settles: `load` marks a URL as being read
     at once, and a page needs at most `PROV_FILES_MAX` + 2 × `CAS_DAYS_MAX`
     answers, fewer than the cache keeps. */
  $effect(() => {
    const want = pageUrls;
    const have = answers;
    untrack(() => {
      for (const url of want) if (!have.has(url)) load(url);
    });
  });

  /* ── the drawer ─────────────────────────────────────────────────────── */

  /** The row the drawer is open on, kept as the object so paging does not lose it. */
  /** @type {any} */
  let opened = $state.raw(null);
  /** @type {HTMLElement | undefined} */
  let drawerEl = $state();

  /** @param {any} row */
  function openOn(row) {
    opened = row;
    queueMicrotask(() => drawerEl?.focus());
  }

  /** @param {Event} e */
  function pickRow(e) {
    const rk = /** @type {HTMLSelectElement} */ (e.currentTarget).value;
    const row = rows.find((/** @type {any} */ r) => r.rk === rk);
    if (row) opened = row;
  }

  const drawerUrls = $derived.by(() => {
    const row = opened;
    if (!row || !wire) return null;
    const t = Math.floor(row.ts / 1000);
    const at = { feed: wire, instrument: row.instrument, timeframe: row.tf, month: row.month };
    const run = { instrument: row.instrument, tf: row.tf, month: row.month, day: row.tf === '1day' ? '' : row.day };
    return {
      prov: provenanceUrl({ ...at, t }),
      bar: barAtUrl({ ...at, t }),
      cas: casUrls(run, true)
    };
  });

  $effect(() => {
    const u = drawerUrls;
    const have = answers;
    if (!u) return;
    untrack(() => {
      for (const url of [u.prov, u.bar, u.cas?.tail, u.cas?.day]) if (url && !have.has(url)) load(url);
    });
  });

  const drawer = $derived.by(() => {
    const row = opened;
    const u = drawerUrls;
    if (!row || !u) return null;
    const t = Math.floor(row.ts / 1000);
    const barBody = bodyOf(u.bar);
    const bar = Array.isArray(barBody?.bars) ? (barBody.bars.find((/** @type {any} */ b) => b.t === t) ?? null) : null;
    const dayBody = bodyOf(u.cas?.day);
    const dayFacts = Array.isArray(dayBody?.days) ? (dayBody.days.find((/** @type {any} */ d) => d.day === row.day) ?? null) : null;
    const provAnswer = got(u.prov);
    const reading = [u.prov, u.bar, u.cas?.tail, u.cas?.day].filter((url) => url && got(url)?.state === 'loading').length;
    /** @type {string[]} */
    const failed = [];
    for (const url of [u.prov, u.bar, u.cas?.tail, u.cas?.day]) {
      const a = got(url);
      if (a?.state === 'error') failed.push(`${String(url).split('?')[0]}: ${a.why}`);
    }
    if (barBody && !bar) failed.push(`/bars/window.json holds no bar stamped t=${t} in ${row.month}: the bar was not read`);
    return {
      row,
      facts: drawerFacts({
        feed,
        row,
        answer: provAnswer?.state === 'ok' ? provAnswer.body : provAnswer?.state === 'error' ? { error: provAnswer.why } : null,
        bar,
        tail: bodyOf(u.cas?.tail),
        dayFacts,
        // THE DAY BAND'S RECORD OF THE ROW'S DAY, for its bar definition
        // (R3-13 goal 3), and the share's adjusted runs (goal 4).
        band: (db.dayBandCells ?? []).find((/** @type {any} */ c) => c.day === row.day) ?? null,
        basis: basisRuns
      }),
      reading,
      failed,
      raw: u.prov
    };
  });

  /** Forgets every answer the page and the drawer hold, so the next look asks afresh. */
  function readAgain() {
    const drop = new Set(pageUrls);
    const u = drawerUrls;
    if (u) for (const url of [u.prov, u.bar, u.cas?.tail, u.cas?.day]) if (url) drop.add(url);
    // The effects above follow the cache, so each forgotten answer is asked
    // for again as soon as it is gone.
    answers = new Map([...answers].filter(([url]) => !drop.has(url)));
  }

  /**
   * A row's day, and below the day rung its clock: a day bar is stamped at
   * the day's IST midnight and has no time of its own to print.
   * @param {any} r
   */
  const whenOf = (r) => (r.tf === '1day' ? dayLabel(r.ts) : `${dayLabel(r.ts)} ${barTimeLabel(r.ts, TF_SECS.get(r.tf))}`);

  /* ── one run line ───────────────────────────────────────────────────── */

  /** @param {any} run */
  function lineOf(run) {
    const read = fileRead.has(run.file);
    const answer = read ? got(monthUrl(run)) : undefined;
    const src = !read
      ? sourceNotRead(files.files.length + files.over, PROV_FILES_MAX)
      : answer?.state === 'ok'
        ? sourceOfDay(answer.body, run.day)
        : null;
    const segment = segmentOfKey(run.instrument);
    const price = priceBadge(feed, segment, run.tf, src?.pullDay ?? null);
    const close = closeBadge(feed, segment, run.tf);
    // THE ADJUSTED OR QUANTISED RUN THE ROWS' DAY BELONGS TO (R3-13 goal 4).
    const basis = segment === 'CASH' ? basisRunLabel(basisRuns, run.day || istDay(run.first.ts)) : null;
    // EVERY RUN THAT REACHES THE AUCTION IS MARKED from its rows' own phases;
    // the session's last print and the day's record sharpen a GDFL mark where
    // they were read, and a day past the cap says it was not.
    const reaches = reachesAuction(run, feed);
    const c = reaches ? casUrls(run) : null;
    const dayBody = c ? bodyOf(c.day) : null;
    // A RUN'S ROWS NEED NOT BE ADJACENT (another series' rows can sit between
    // them), so its rows are taken by position and each mark carries its row's
    // place on the page.
    const marks = reaches
      ? auctionMarks({
          feed,
          rows: run.rows.map((/** @type {number} */ i) => rows[i]),
          tail: c ? bodyOf(c.tail) : null,
          dayFacts: Array.isArray(dayBody?.days) ? (dayBody.days.find((/** @type {any} */ d) => d.day === run.day) ?? null) : null
        }).map((m) => ({ ...m, at: run.rows[m.index] }))
      : [];
    const casCapped = reaches && c === null && feed?.closing_auction?.handling === 'print_kept_apart';
    const casReading = c ? [c.tail, c.day].some((url) => url && got(url)?.state === 'loading') : false;
    const casFailed = c
      ? [c.tail, c.day]
          .map((url) => (url ? got(url) : undefined))
          .filter((a) => a?.state === 'error')
          .map((a) => a.why)
      : [];
    return {
      read,
      answer,
      src,
      casCapped,
      price,
      close,
      basis,
      marks,
      casReading,
      casFailed,
      window: answer?.state === 'ok' ? (answer.body?.window?.label ?? null) : null
    };
  }

  /**
   * What a run's line SAYS, as one string: two runs that say the same thing
   * are drawn as one line (`foldRuns`). A run with a closing-auction mark or
   * read keeps its own line, because what it says is about its own rows.
   * @param {{ run: any, l: ReturnType<typeof lineOf> }} x
   */
  function lineKey({ run, l }) {
    const said = !l.read
      ? `capped|${l.src?.text}`
      : l.answer?.state === 'ok'
        ? `${l.src?.state}|${l.src?.short}|${l.src?.text}`
        : l.answer?.state === 'error'
          ? `error|${l.answer.why}`
          : 'reading';
    const own = l.marks.length > 0 || l.casReading || l.casCapped || l.casFailed.length > 0 ? run.key : '';
    return [run.tf, run.day, l.price.text, l.close?.short ?? '', l.basis?.short ?? '', l.window ?? '', said, own].join('§');
  }

  /**
   * THE LINES DRAWN: the page's runs, each composed once, folded where they
   * say the same thing. Every row of the page is in exactly one line.
   */
  const lines = $derived.by(() => {
    const made = runs.map((run) => ({ run, l: lineOf(run), rows: run.rows }));
    return foldRuns(made, lineKey).map((f) => {
      const head = f.runs[0];
      const first = rows[f.rows[0]];
      const last = rows[f.rows[f.rows.length - 1]];
      const secs = TF_SECS.get(head.run.tf);
      const day = head.run.day;
      const when = day
        ? `${dayLabel(day)} · ${barTimeLabel(first.ts, secs)}${f.rows.length > 1 && last.ts !== first.ts ? ` – ${barTimeLabel(last.ts, secs)}` : ''}`
        : `${dayLabel(first.ts)}${f.rows.length > 1 && first.day !== last.day ? ` – ${dayLabel(last.ts)}` : ''}`;
      return {
        key: f.key,
        label: rowsLabel(f.rows),
        when,
        files: f.runs.map((x) => x.run),
        l: head.l,
        marks: f.runs.flatMap((x) => x.l.marks),
        first
      };
    });
  });
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === 'Escape' && opened !== null && drawerEl?.contains(/** @type {Node | null} */ (document.activeElement))) opened = null;
  }}
/>

{#if db.view === 'bars' && rows.length > 0}
  <section class="prov" aria-label="Where the bars on this page came from">
    <header class="phead">
      <h3>Source</h3>
      <span class="psub">
        where each row on this page came from, one line per month file and day, and one for files that say the same — /provenance.json{files.over > 0
          ? `; ${fmt(files.over)} more month file(s) on this page are not read (at most ${fmt(files.files.length)} per page)`
          : ''}
      </span>
      <button class="pbtn" type="button" onclick={readAgain} title="Forget these answers and ask the server again — after an import or a re-pull, a month's source can change.">Read again</button>
    </header>
    <ol class="pruns">
      {#each lines as line (line.key)}
        {@const l = line.l}
        <li class="prun">
          <span class="prows">{line.label}</span>
          <!-- A DAY BAR HAS NO TIME OF ITS OWN: its line names the days it
               spans; a bar below the day rung names its day once and the
               clock of its first and last row. -->
          <span class="pwhen">{line.when}</span>
          {#if line.files.length === 1}
            <span class="pfile" title="month file {line.files[0].month} of {line.files[0].instrument} at {line.files[0].tf}, on {display}">{display} · {line.files[0].instrument} · {line.files[0].tf}</span>
          {:else}
            <span class="pfile" title={line.files.map((f) => `${f.instrument} ${f.month}`).join('\n')}>{display} · {fmt(line.files.length)} month files · {line.files[0].tf}</span>
          {/if}
          <span class="bb {l.price.tone}" title={l.price.title}>{l.price.tone === 'adjusted' ? l.price.text : l.price.short}</span>
          {#if l.close}<span class="bb {l.close.tone}" title={l.close.title}>{l.close.short}</span>{/if}
          {#if l.basis}<span class="bb {l.basis.tone}" title={l.basis.text}>{l.basis.short}</span>{/if}
          {#if l.window}<span class="bb warn" title="D-3338: the operator's window">{l.window}</span>{/if}
          <span class="psrc {l.src?.state ?? l.answer?.state ?? 'unread'}">
            {#if !l.read && l.src}
              <b>{l.src.short}</b> · {l.src.text}
            {:else if !l.answer || l.answer.state === 'loading'}
              reading the source…
            {:else if l.answer.state === 'error'}
              source not read: {l.answer.why}
            {:else if l.src}
              <b>{l.src.short}</b> · {l.src.text}
            {/if}
          </span>
          {#each line.marks as m (m.row.rk)}
            <span class="pmark {m.verdict.tone}" title={m.verdict.text}>row {fmt(m.at + 1)} · {secondLabel(m.row.ts)} · {m.verdict.short}</span>
          {/each}
          {#if l.casReading}<span class="pmark">reading the closing-auction session…</span>{/if}
          {#if l.casCapped}<span class="pmark warn" title="At most {fmt(CAS_DAYS_MAX)} share-days of closing-auction session are read per page; {fmt(casDays.over)} more on this page are not. Details reads this day's.">closing-auction session not read: past the page's {fmt(CAS_DAYS_MAX)} days</span>{/if}
          {#each l.casFailed as why, i (i)}<span class="pmark warn">closing-auction session not read: {why}</span>{/each}
          <button class="pbtn" type="button" onclick={() => openOn(line.first)} aria-label="Details of where {line.label} came from">Details</button>
        </li>
      {/each}
    </ol>

    {#if drawer}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <aside class="pdrawer" aria-label="Where one bar came from" tabindex="-1" bind:this={drawerEl}>
        <header class="dhead">
          <h4>Source of one bar</h4>
          <label class="dpick">
            <span>Row</span>
            <select value={drawer.row.rk} onchange={pickRow}>
              {#if !rows.some((/** @type {any} */ r) => r.rk === drawer.row.rk)}
                <option value={drawer.row.rk}>{whenOf(drawer.row)} (no longer on this page)</option>
              {/if}
              {#each rows as r, i (r.rk)}
                <option value={r.rk}>{fmt(i + 1)}. {whenOf(r)} · {r.tf}</option>
              {/each}
            </select>
          </label>
          <a class="draw" href={drawer.raw} target="_blank" rel="noopener" title="The server's whole answer for this bar, as JSON">raw answer</a>
          <button class="pbtn" type="button" onclick={() => (opened = null)}>Close</button>
        </header>
        {#if drawer.reading > 0}<p class="dnote">reading {fmt(drawer.reading)} answer(s)…</p>{/if}
        {#each drawer.failed as why, i (i)}<p class="dnote warn">{why}</p>{/each}
        <dl class="dfacts">
          {#each drawer.facts as f, i (i)}
            <dt>{f.k}</dt>
            <dd class={f.tone ?? ''}>{f.v}</dd>
          {/each}
        </dl>
      </aside>
    {/if}
  </section>
{/if}

<style>
  .prov {
    margin: var(--s3, 10px) 0 0;
    padding: var(--s3, 10px) var(--s4, 14px);
    border: 1px solid var(--line);
    border-radius: var(--r2, 6px);
    background: var(--panel);
    min-width: 0;
  }
  .phead {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--s2, 6px) var(--s3, 10px);
  }
  .phead h3 {
    margin: 0;
    font-size: var(--fs-sm);
    color: var(--ink-hi);
  }
  .psub {
    flex: 1 1 18rem;
    font-size: var(--fs-xs);
    color: var(--dim);
  }
  .pruns {
    list-style: none;
    margin: var(--s2, 6px) 0 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .prun {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px var(--s2, 8px);
    padding: 6px 0;
    border-top: 1px solid var(--line-soft);
    font-size: var(--fs-xs);
    color: var(--ink-2);
    min-width: 0;
  }
  .prows,
  .pwhen,
  .pfile {
    font-family: var(--mono);
    white-space: nowrap;
  }
  .prows {
    color: var(--ink);
    font-weight: var(--w-semi, 600);
  }
  .pfile {
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }
  .psrc {
    flex: 1 1 100%;
    min-width: 0;
    overflow-wrap: anywhere;
    color: var(--ink-2);
  }
  .psrc.error {
    color: var(--down);
  }
  .psrc.capped {
    color: var(--warn);
  }
  .bb,
  .pmark {
    display: inline-block;
    padding: 1px 7px;
    border-radius: var(--r1, 4px);
    border: 1px solid var(--line);
    font-family: var(--mono);
    font-size: var(--fs-micro);
    color: var(--ink-2);
    background: var(--panel-2);
  }
  .bb {
    white-space: nowrap;
    cursor: help;
  }
  .pmark {
    overflow-wrap: anywhere;
  }
  .bb.adjusted {
    color: var(--warn);
    border-color: color-mix(in srgb, var(--warn) 45%, transparent);
    background: color-mix(in srgb, var(--warn) 10%, transparent);
  }
  .bb.official,
  .bb.mixed,
  .pmark.cas {
    color: var(--acc);
    border-color: color-mix(in srgb, var(--acc) 40%, transparent);
  }
  /* THE AUCTION'S PRINT IS NOT A TRADE, and it is the one mark that must read
     apart from the continuous rows around it. */
  .pmark.auction {
    color: var(--on-acc, #fff);
    background: var(--acc);
    border-color: var(--acc);
    font-weight: var(--w-bold, 700);
  }
  .bb.warn,
  .pmark.warn,
  .bb.unverified,
  .bb.unknown {
    color: var(--down);
    border-color: color-mix(in srgb, var(--down) 40%, transparent);
  }
  .pbtn {
    font: inherit;
    font-size: var(--fs-micro);
    padding: 2px 10px;
    border-radius: var(--r1, 4px);
    border: 1px solid var(--line);
    background: var(--bg-2);
    color: var(--ink);
    cursor: pointer;
  }
  .pbtn:hover {
    border-color: var(--acc);
  }
  .pbtn:focus-visible,
  .pdrawer:focus-visible {
    outline: 2px solid var(--focus, var(--acc));
    outline-offset: 2px;
  }
  .pdrawer {
    margin-top: var(--s3, 10px);
    padding: var(--s3, 10px) var(--s4, 14px);
    border: 1px solid var(--acc);
    border-radius: var(--r2, 6px);
    background: var(--bg-2);
    min-width: 0;
  }
  .dhead {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--s2, 6px) var(--s3, 10px);
  }
  .dhead h4 {
    margin: 0;
    font-size: var(--fs-sm);
    color: var(--ink-hi);
    flex: 1 1 auto;
  }
  .dpick {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--fs-xs);
    color: var(--dim);
    min-width: 0;
  }
  .dpick select {
    font: inherit;
    font-family: var(--mono);
    max-width: min(28rem, 70vw);
    background: var(--panel);
    color: var(--ink);
    border: 1px solid var(--line);
    border-radius: var(--r1, 4px);
  }
  .draw {
    font-size: var(--fs-xs);
    color: var(--acc);
  }
  .dnote {
    margin: 6px 0 0;
    font-size: var(--fs-xs);
    color: var(--dim);
  }
  .dnote.warn {
    color: var(--down);
  }
  .dfacts {
    display: grid;
    grid-template-columns: minmax(7rem, max-content) minmax(0, 1fr);
    gap: 4px var(--s3, 10px);
    margin: var(--s2, 8px) 0 0;
    font-size: var(--fs-xs);
  }
  .dfacts dt {
    color: var(--dim);
  }
  .dfacts dd {
    margin: 0;
    color: var(--ink);
    overflow-wrap: anywhere;
  }
  .dfacts dd.adjusted {
    color: var(--warn);
  }
  .dfacts dd.auction,
  .dfacts dd.official {
    color: var(--acc);
  }
  .dfacts dd.warn,
  .dfacts dd.error,
  .dfacts dd.unverified {
    color: var(--down);
  }
  @media (max-width: 640px) {
    .dfacts {
      grid-template-columns: minmax(0, 1fr);
    }
    .dfacts dd {
      margin-bottom: 4px;
    }
  }
</style>

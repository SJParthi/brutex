<script>
  /**
   * THE MAPPING CONSOLE — what the exchange says about the feed's own symbols.
   *
   * # What this closes
   *
   * D-0275 built the join between a vendor's index symbol and the name NSE
   * publishes, and said in its own last paragraph that the module had no
   * caller. D-0276 gave it `/indexmap.json` and said, in ITS last paragraph,
   * that nothing in `web/` fetched it. This is the reader, and the sentence
   * about a surface that exists and cannot be reached being the same as absent
   * does not need writing a third time.
   *
   * # The refusals are the page, not a footnote
   *
   * A symbol that resolves confirms what an operator already assumed. A symbol
   * the exchange does not confirm is one whose bars are being filed under a
   * name no authority backs — and there was no list of those anywhere: not a
   * page, not a log line, not a count. So the refused rows are not filtered
   * out, not collapsed behind a toggle and not sorted to the bottom. They open
   * the table, and the census counts them beside the resolutions.
   *
   * # Two strengths, never merged
   *
   * `published` means the vendor's symbol IS the name NSE prints — proof.
   * `abbreviation` means it shortens exactly one published name — inference,
   * and strong, and still a rule's opinion. The server sends them as separate
   * bases for the reason `CLAUDE.md` §3 rule 1 exists, and a page that painted
   * both the same colour would be undoing that on the last hop. They carry
   * different hues here, from the console's own semantic ramp.
   *
   * # Why the candidates are shown for an ambiguous row
   *
   * "Ambiguous" alone is a dead end; the operator cannot act on it. The names
   * that collided are what makes it actionable — `NIFTY EV` matches nine
   * published indices, and seeing the nine is the difference between a refusal
   * and a decision waiting to be made.
   *
   * # O(1)
   *
   * The filter is a prefix probe into a Map built once per payload — the same
   * `$lib/prefix.js` `/db` and `/backtest` use — never a scan per keystroke and
   * never a request per character. The outcome chips partition a list already
   * in memory. The payload is one request per feed change and nothing polls.
   *
   * # What is deliberately NOT drawn
   *
   * **No suggested name for a refused symbol.** Fifteen of the seventeen
   * absences have an obvious published name beside them — obvious to a reader,
   * not to the rule. Printing a guess in the same column that elsewhere holds a
   * resolution would make inference and proof indistinguishable at a glance,
   * which is the whole thing this surface exists to keep apart. The REASON is
   * printed instead, because a reason is checkable and a guess is not.
   *
   * **No equities, futures or options.** This is the index join only — the rows
   * a feed marks as indices on NSE. Everything else is a different join on a
   * different key and is not claimed by these numbers.
   */
  import { untrack } from 'svelte';
  import { feeds } from '$lib/feeds.svelte.js';
  import { ask } from '$lib/ask.js';
  import * as prefix from '$lib/prefix.js';

  /* ====================================================================
     THE PAYLOAD
     ==================================================================== */

  /** @type {{ phase: 'loading'|'ready'|'failed', body: any, why: string }} */
  let load = $state({ phase: 'loading', body: null, why: '' });

  /* ====================================================================
     THE MASTERS THIS JOIN READS FROM

     This page has always been able to say the join did not run, and never
     able to do anything about it. Its own refusal hint said the exchange
     list "is operator data and is not tracked" -- which was true until
     `pull::masters` made all four files fetchable, and is now a sentence
     telling an operator to go and find a file the server can download.

     `POST /masters/refresh` is that download. It is on this page rather
     than its own because this is where the absence is DISCOVERED: an
     operator reading "the join did not run" is one keystroke from the
     control that fixes it, instead of somewhere else entirely.

     NOTHING RUNS ON LOAD. `refreshMasters` is bound to a press, because
     three of these four requests leave this machine and one of them
     spends the shared vendor credential.
     ==================================================================== */

  /** @type {{ phase: 'idle'|'running'|'done', rows: any[], why: string, restart: boolean, universe: string }} */
  let masters = $state({ phase: 'idle', rows: [], why: '', restart: false, universe: '' });

  /** @type {any[]} */
  let onDisk = $state([]);

  async function readMasters() {
    try {
      const response = await ask('/masters/status.json');
      if (!response.ok) return;
      const body = await response.json();
      onDisk = body.masters ?? [];
    } catch {
      // A STATUS READ THAT FAILS IS NOT THIS PAGE'S SUBJECT. The join's own
      // refusal already says what is missing; a second red banner about the
      // same absence would be noise, and inventing a state for it would be
      // worse than saying nothing.
      onDisk = [];
    }
  }

  async function refreshMasters() {
    if (masters.phase === 'running') return;
    masters = { phase: 'running', rows: [], why: '', restart: false, universe: '' };
    try {
      // A MINUTE AND A HALF, NOT THE DEFAULT FIFTEEN SECONDS. A refused
      // source is retried on a backoff -- 500 ms doubling to a 32 s cap,
      // five attempts per URL -- so a sick host legitimately takes longer
      // than any other request this console makes. Timing out at 15 s would
      // report a working ladder as a wedged server.
      const response = await ask('/masters/refresh', { method: 'POST', ms: 90_000 });
      const body = await response.json();
      masters = {
        phase: 'done',
        rows: body.landed ?? [],
        // A REFUSAL, OR A RELOAD THAT DID NOT HAPPEN. The second is its own
        // failure and used to be invisible: the files can all land and the
        // universe still fail to re-parse, and a page reporting only the
        // downloads would show four green rows over a server still answering
        // from the boot parse.
        why: body.refusal ?? (body.reloaded === false ? body.universe : ''),
        restart: Boolean(body.restart_required),
        universe: body.reloaded ? (body.universe ?? '') : ''
      };
      await readMasters();
      // AND THE JOIN AGAIN, because the whole point of the refresh is that
      // the answer above it changes. Leaving the stale refusal on screen
      // beside four green rows is the contradiction this page exists to
      // avoid drawing.
      await fetchJoin(feeds.active);
    } catch (error) {
      masters = {
        phase: 'done',
        rows: [],
        why: error instanceof Error ? error.message : 'The refresh threw a value that is not an Error.',
        restart: false,
        universe: ''
      };
    }
  }

  $effect(() => {
    readMasters();
  });

  /* ====================================================================
     THE CONSTITUENTS, WHICH ARE NOT THE CATALOGUE

     `nse_indices.csv` is the CATALOGUE -- which indices exist, and 123 of
     them do. Each of those also publishes its own constituents file, and
     NSE lists about 148 across four categories. `pull::nse` decodes them,
     `pull::resolve::crawl` walks them, `POST /universe/resolve` serves the
     result -- and nothing in this console had ever called it, so the whole
     chain was built, tested and unreachable.

     THE FILENAMES CANNOT BE COMPOSED. `ind_niftybanklist.csv` is
     word-joined and `ind_niftytotalmarket_list.csv` is
     underscore-separated; `pull::nse` says so in its own words and refuses
     to guess. They are discovered from the exchange's directory page,
     which is why this is a crawl and not 148 known URLs.

     IT IS EXPENSIVE AND IT IS BOUND TO A PRESS. ~148 requests to
     nseindia.com. Nothing here runs on load, and the timeout is minutes
     rather than seconds because the work legitimately takes them.
     ==================================================================== */

  /** @type {{ phase: 'idle'|'running'|'done', body: any, why: string }} */
  let crawl = $state({ phase: 'idle', body: null, why: '' });

  async function resolveUniverse() {
    if (crawl.phase === 'running') return;
    const feed = feeds.active;
    if (!feed) return;
    crawl = { phase: 'running', body: null, why: '' };
    try {
      // FIVE MINUTES. A crawl of ~148 documents over one polite connection
      // is minutes of real work, and the console's default 15 s would
      // report it as a wedged server every single time.
      const response = await ask('/universe/resolve', {
        method: 'POST',
        ms: 300_000,
        headers: { 'content-type': 'application/x-www-form-urlencoded' },
        body: `feed=${encodeURIComponent(feed)}`
      });
      const body = await response.json();
      crawl = {
        phase: 'done',
        body,
        // `ok:false` CARRIES ITS OWN REASON and the status may still be 200:
        // the route answers a refusal as a document rather than as an HTTP
        // error, so reading the status alone would call a refusal a success.
        why: body?.ok === false ? (body.why ?? 'the crawl refused and gave no reason') : ''
      };
    } catch (error) {
      crawl = {
        phase: 'done',
        body: null,
        why: error instanceof Error ? error.message : 'The crawl threw a value that is not an Error.'
      };
    }
  }

  /** @param {string|null} feed */
  async function fetchJoin(feed) {
    if (!feed) return;
    load = { phase: 'loading', body: null, why: '' };
    try {
      const response = await ask(`/indexmap.json?feed=${encodeURIComponent(feed)}`, {
        cache: 'no-store'
      });
      if (!response.ok) {
        // THE BODY CARRIES THE REASON AND IS READ RATHER THAN DISCARDED.
        // A 500 here is almost always the operator's NSE catalogue file being
        // absent, and the server puts its path in the message. Throwing that
        // away and printing the status code would turn a one-line fix into a
        // search.
        let why = `/indexmap.json answered ${response.status}.`;
        try {
          const body = await response.json();
          if (body?.error) why = String(body.error);
        } catch {
          /* Not JSON. The status line above is all there is to say. */
        }
        load = { phase: 'failed', body: null, why };
        return;
      }
      load = { phase: 'ready', body: await response.json(), why: '' };
    } catch (error) {
      load = {
        phase: 'failed',
        body: null,
        why:
          error instanceof Error
            ? error.message
            : 'The request for /indexmap.json failed and threw a value that is not an Error.'
      };
    }
  }

  $effect(() => {
    const feed = feeds.active;
    untrack(() => fetchJoin(feed));
  });

  /* ====================================================================
     WHAT THE PAYLOAD SAYS
     ==================================================================== */

  const join = $derived(load.body);
  const rows = $derived(join?.rows ?? []);
  const publishedCount = $derived(join?.published ?? 0);
  const listed = $derived(join?.listed ?? 0);
  const resolved = $derived(join?.resolved ?? 0);
  const verbatim = $derived(join?.verbatim ?? 0);
  const aliased = $derived(join?.aliased ?? 0);
  const abbreviated = $derived(join?.abbreviated ?? 0);
  const refused = $derived(join?.refused ?? 0);

  /**
   * One row's outcome as a single token, so the chips, the pill and the row
   * tint all read the same field rather than re-deriving it three ways.
   *
   * @param {any} row
   * @returns {'published'|'aliased'|'abbreviation'|'ambiguous'|'absent'}
   */
  function outcome(row) {
    if (row.basis === 'published') return 'published';
    if (row.basis === 'aliased') return 'aliased';
    if (row.basis === 'abbreviation') return 'abbreviation';
    return row.why === 'ambiguous' ? 'ambiguous' : 'absent';
  }

  const tagged = $derived(rows.map((/** @type {any} */ r) => ({ ...r, kind: outcome(r) })));

  /* ====================================================================
     NARROWING
     ==================================================================== */

  const CHIPS = /** @type {const} */ ([
    { key: 'all', label: 'All', tone: '' },
    { key: 'published', label: 'Confirmed', tone: 'p' },
    { key: 'aliased', label: 'Renamed', tone: 'r' },
    { key: 'abbreviation', label: 'Abbreviated', tone: 'i' },
    { key: 'ambiguous', label: 'Ambiguous', tone: 'a' },
    { key: 'absent', label: 'Absent', tone: 'x' }
  ]);

  let chip = $state('all');
  let query = $state('');

  const byPrefix = $derived(prefix.build(tagged));
  const searched = $derived(prefix.probe(byPrefix, tagged, query));
  const shown = $derived(
    chip === 'all' ? searched : searched.filter((/** @type {any} */ r) => r.kind === chip)
  );

  /** How many rows each chip would show, so a zero is visible before it is clicked. */
  const tally = $derived.by(() => {
    /** @type {Record<string, number>} */
    const n = {
      all: searched.length,
      published: 0,
      aliased: 0,
      abbreviation: 0,
      ambiguous: 0,
      absent: 0
    };
    for (const r of searched) n[r.kind] += 1;
    return n;
  });

  /** @param {any} row */
  function note(row) {
    if (row.kind === 'ambiguous') {
      return `${row.candidates} published names accept this abbreviation. Picking one would be a guess.`;
    }
    if (row.kind === 'absent') {
      return 'No published name accepts it. Either the vendor reordered the words, dropped a number the name carries, or this is not an NSE index at all.';
    }
    if (row.kind === 'abbreviation') return 'Shortens exactly one published name.';
    if (row.kind === 'aliased') {
      return `This engine renamed it. The exchange publishes the vendor's own name for it, and the store keys on ${row.symbol} instead.`;
    }
    return 'The symbol is the published name.';
  }
</script>

<svelte:head><title>Mapping · brutex</title></svelte:head>

<section class="page">
  <header class="head">
    <div>
      <h1>Instrument mapping</h1>
      <p class="sub">
        Every index symbol {feeds.active ?? 'this feed'} lists, joined to the name NSE publishes for
        it. Nothing here was compared against another vendor — two feeds agreeing proves they bought
        the same upstream, not that either is right.
      </p>
    </div>
  </header>

  {#if load.phase === 'loading'}
    <p class="state">Asking the exchange list…</p>
  {:else if load.phase === 'failed'}
    <div class="refusal">
      <span class="rlabel">The join did not run</span>
      <p>{load.why}</p>
      <p class="rhint">
        The exchange list lives beside the vendor masters as <code>nse_indices.csv</code>, one
        <code>index_name,category</code> row per published index. Nothing is guessed when it is
        missing and nothing is served from a stale copy — but it no longer has to be found by hand.
        Press <b>Refresh the masters</b> below.
      </p>
    </div>
  {:else}
    <div class="census">
      <div class="cell">
        <span class="n">{publishedCount}</span><span class="k">Published by NSE</span>
        <span class="s">Index names on the exchange's own list.</span>
      </div>
      <div class="cell">
        <span class="n">{listed}</span><span class="k">Listed by feed</span>
        <span class="s">Index symbols in this feed's master.</span>
      </div>
      <div class="cell p">
        <span class="n">{verbatim}</span><span class="k">Confirmed</span>
        <span class="s">The symbol is the published name. Proof.</span>
      </div>
      <div class="cell r">
        <span class="n">{aliased}</span><span class="k">Renamed</span>
        <span class="s">This engine renamed it; the vendor's name is published.</span>
      </div>
      <div class="cell i">
        <span class="n">{abbreviated}</span><span class="k">Abbreviated</span>
        <span class="s">Shortens exactly one name. Inference.</span>
      </div>
      <div class="cell x">
        <span class="n">{refused}</span><span class="k">Refused</span>
        <span class="s">{resolved} of {listed} resolved. These did not.</span>
      </div>
    </div>

    <div class="controls">
      <div class="chips" role="group" aria-label="Filter by outcome">
        {#each CHIPS as c (c.key)}
          <button
            type="button"
            class="chip {c.tone}"
            class:on={chip === c.key}
            aria-pressed={chip === c.key}
            onclick={() => (chip = c.key)}>{c.label}<span class="cn">{tally[c.key]}</span></button
          >
        {/each}
      </div>
      <input
        class="find"
        type="search"
        placeholder="Filter by symbol…"
        aria-label="Filter by symbol"
        bind:value={query} />
    </div>

    {#if shown.length === 0}
      <p class="state">
        No symbol matches. {#if query}The filter is a prefix on the symbol, not a search of the
          published names.{/if}
      </p>
    {:else}
      <div class="tw">
        <table>
          <thead>
            <tr>
              <th scope="col">Symbol</th>
              <th scope="col">Outcome</th>
              <th scope="col">NSE publishes</th>
              <th scope="col">Why</th>
            </tr>
          </thead>
          <tbody>
            {#each shown as row (row.symbol)}
              <tr class={row.kind}>
                <td class="sym">{row.symbol}</td>
                <td>
                  <span
                    class="pill {row.kind === 'published'
                      ? 'p'
                      : row.kind === 'aliased'
                        ? 'r'
                        : row.kind === 'abbreviation'
                          ? 'i'
                          : row.kind === 'ambiguous'
                            ? 'a'
                            : 'x'}">
                    {row.kind === 'abbreviation' ? 'abbreviated' : row.kind}
                  </span>
                </td>
                <td class="nse">{row.nse ?? '—'}</td>
                <td class="note">{note(row)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      <p class="foot">
        Showing {shown.length} of {listed}. A refused symbol is not a broken feed — it is a name this
        engine will not adopt until the exchange confirms it.
      </p>
    {/if}
  {/if}

  <!-- THE FOUR FILES THIS JOIN READS FROM, and the control that fetches them.
       Placed under the refusal and above the census so it sits where the
       absence is discovered, on both paths: an operator whose join failed
       reads the reason and then the fix, and one whose join worked can still
       see how old the answer's inputs are. -->
  <section class="masters">
    <div class="mhead">
      <div>
        <h2>The four files this join reads</h2>
        <p class="msub">
          Three vendor masters and the exchange's own index list. Fetching them touches no bar and
          spends no bar quota — this is not the ingest pull. One of the four spends the shared
          vendor credential; it is marked.
        </p>
      </div>
      <button class="mgo" onclick={refreshMasters} disabled={masters.phase === 'running'}>
        {masters.phase === 'running' ? 'Asking four hosts…' : 'Refresh the masters'}
      </button>
    </div>

    {#if onDisk.length}
      <div class="mrows">
        {#each onDisk as file (file.file)}
          {@const done = masters.rows.find((r) => r.file === file.file)}
          <div class="mrow">
            <div class="mname">
              <code>{file.file}</code>
              {#if file.needs_token}<span class="tok">shared token</span>{/if}
            </div>
            <div class="mstate">
              {#if file.present}
                <span class="ok">on disk</span>
                <span class="dim">{file.bytes.toLocaleString()} bytes</span>
                {#if file.modified_unix_millis}
                  <span class="dim">{new Date(file.modified_unix_millis).toLocaleString()}</span>
                {/if}
              {:else}
                <span class="no">absent</span>
              {/if}
            </div>
            <div class="mout">
              {#if masters.phase === 'running'}
                <span class="dim">asking…</span>
              {:else if done?.written}
                <span class="ok">{done.changed ? 'updated' : 'unchanged'}</span>
                <span class="dim">{done.bytes.toLocaleString()} bytes</span>
              {:else if done?.skipped}
                <span class="skip">skipped</span><span class="dim">{done.refusal}</span>
              {:else if done}
                <span class="no">refused</span><span class="dim">{done.refusal}</span>
              {:else}
                <span class="dim">—</span>
              {/if}
            </div>
          </div>

          <!-- EVERY STEP, NOT A SUMMARY. Which URL, which attempt, what was
               waited and what the host said. "It failed" is not something an
               operator can act on, and five 503s and one 404 call for
               opposite responses while rendering identically without this. -->
          {#if done?.attempts?.length > 1 || (done && !done.written && done.attempts?.length)}
            <ol class="ladder">
              {#each done.attempts as step, i (i)}
                <li>
                  <span class="lnum">#{step.number}</span>
                  <span class="lgot {step.got}">{step.got.replace(/_/g, ' ')}</span>
                  {#if step.status}<span class="lst">{step.status}</span>{/if}
                  {#if step.waited_ms}<span class="dim">after {step.waited_ms} ms</span>{/if}
                  {#if step.detail}<span class="dim">{step.detail}</span>{/if}
                </li>
              {/each}
            </ol>
          {/if}
        {/each}
      </div>
    {/if}

    {#if masters.why}
      <p class="mwhy">{masters.why}</p>
    {/if}
    {#if masters.restart}
      <p class="mrestart">
        New bytes are on disk and this build could not re-read them in place, so every other page is
        still answering from the boot parse until the server is restarted.
      </p>
    {:else if masters.universe}
      <!-- WHAT THE RE-PARSE ACTUALLY READ, per feed. "Reloaded" alone is a
           claim; `2929 kept, 133802 declined` is the same claim with its
           working shown -- and it is where `dhan: UNAVAILABLE` appears the
           moment a master lands that the reader cannot parse. -->
      <p class="mreload">
        <b>Re-read in place — no restart needed.</b> Every page now answers from these files.
      </p>
      <p class="mnotes">{masters.universe}</p>
    {/if}
  </section>

  <!-- THE CONSTITUENTS. A separate section from the masters above because it
       is a separate act with a separate cost: the masters are four files and
       seconds, this is ~148 documents from one exchange and minutes. Folding
       them into one button would make the cheap operation look expensive and
       be avoided for the wrong reason. -->
  <section class="masters">
    <div class="mhead">
      <div>
        <h2>The index constituents — 148 files, not one</h2>
        <p class="msub">
          The catalogue above says <em>which</em> indices exist. This walks the exchange's own
          directory and reads what is <em>in</em> each of them, then checks every published name
          against this feed's master. ~148 requests to nseindia.com, so it takes minutes and runs
          only when you press it.
        </p>
      </div>
      <button class="mgo" onclick={resolveUniverse} disabled={crawl.phase === 'running'}>
        {crawl.phase === 'running' ? 'Crawling the exchange…' : 'Crawl the constituents'}
      </button>
    </div>

    {#if crawl.phase === 'running'}
      <p class="msub">
        Reading the directory, then one file per index. Nothing is written — the snapshot is
        returned and shown, and publishing it is a separate act.
      </p>
    {:else if crawl.body?.ok}
      <div class="census">
        <div class="cell">
          <span class="n">{crawl.body.published}</span><span class="k">Published names</span>
          <span class="s">Constituents the exchange lists across every index read.</span>
        </div>
        <div class="cell">
          <span class="n">{crawl.body.indices}</span><span class="k">Indices read</span>
          <span class="s">Constituent files walked in this pass.</span>
        </div>
        <div class="cell" class:r={crawl.body.failed > 0}>
          <span class="n">{crawl.body.failed}</span><span class="k">Indices that failed</span>
          <span class="s">Named below. Never dropped from the count.</span>
        </div>
        <div class="cell" class:p={crawl.body.publishable} class:r={!crawl.body.publishable}>
          <span class="n">{crawl.body.publishable ? 'yes' : 'no'}</span
          ><span class="k">Publishable</span>
          <span class="s">Whether this pass is whole enough to be a snapshot.</span>
        </div>
      </div>

      {#if crawl.body.buckets}
        <!-- EVERY BUCKET, INCLUDING THE EMPTY ONES. A bucket shown only when
             non-zero teaches an operator that the absent ones do not exist,
             and the sum is what makes a join defect findable at all. -->
        <div class="mrows">
          {#each Object.entries(crawl.body.buckets) as [name, total] (name)}
            <div class="mrow bucket">
              <div class="mname">{name.replace(/_/g, ' ')}</div>
              <div class="mstate"><span class={total ? 'ok' : 'dim'}>{total}</span></div>
              <div class="mout"></div>
            </div>
          {/each}
        </div>
      {/if}

      {#if crawl.body.failures?.length}
        <ol class="ladder">
          {#each crawl.body.failures as failure, i (i)}
            <li><span class="lst">{failure.at}</span><span class="dim">{failure.why}</span></li>
          {/each}
        </ol>
      {/if}

      <p class="mnotes">
        digest {crawl.body.digest} · key {crawl.body.key} · identity {crawl.body.identity} · day
        {crawl.body.day}
      </p>
    {/if}

    {#if crawl.why}
      <p class="mwhy">{crawl.why}</p>
    {/if}
  </section>
</section>

<style>
  .page {
    padding: var(--s5) var(--s5) var(--s8);
    max-width: 1180px;
    margin: 0 auto;
    /* THIS PAGE OWNS ITS SCROLL, BECAUSE THE SHELL DELIBERATELY DOES NOT.
       `theme.css`'s `.main` is `overflow: hidden` by design — its comment
       says a page whose root is `.split` or `.pane` "keeps the exact height
       it had" — so every page must provide its own scroller. This root is
       `.page`, which is neither, and it provided none.

       Measured at 1440x900 before this line: `.main` had **4,325px clipped**
       with no inner scroller anywhere beneath it. `overflow: hidden` still
       moves under `scrollTop` from script, so the content was in the DOM and
       reachable by code — and a person had no wheel, no scrollbar and no
       keyboard. Nearly the whole page was unreachable.

       `/backtest` had the identical defect at 248px and the identical cause;
       these are the only two routes whose root is `.page`, and both were
       broken. Every route rooted at `.pane` or `.mkt` measured 0.

       `min-height: 0` is load-bearing beside it: without it a flex child
       floors at its content height, so the box never becomes smaller than
       what it holds and `auto` has nothing to scroll. */
    min-height: 0;
    overflow-y: auto;
  }
  .head {
    margin-bottom: var(--s5);
  }
  h1 {
    font: var(--w-semi) var(--fs-xl) / 1.15 var(--sans);
    letter-spacing: -0.015em;
    color: var(--n11);
    margin: 0 0 var(--s2);
  }
  .sub {
    font-size: var(--fs-sm);
    color: var(--n9);
    line-height: 1.55;
    max-width: 78ch;
    margin: 0;
  }
  .state {
    font-size: var(--fs-sm);
    color: var(--n9);
    padding: var(--s5) 0;
  }
  .refusal {
    border: 1px solid var(--n6);
    border-left: 3px solid var(--down);
    background: var(--n3);
    border-radius: var(--r2);
    padding: var(--s4);
  }
  .rlabel {
    display: block;
    font: var(--w-semi) var(--fs-micro) / 1 var(--sans);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--down);
    margin-bottom: var(--s2);
  }
  .refusal p {
    font-size: var(--fs-sm);
    color: var(--n10);
    margin: 0 0 var(--s2);
    max-width: 76ch;
  }
  .rhint {
    color: var(--n9) !important;
    margin-bottom: 0 !important;
  }

  .census {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(158px, 1fr));
    gap: 1px;
    background: var(--n6);
    border: 1px solid var(--n6);
    border-radius: var(--r2);
    overflow: hidden;
    margin-bottom: var(--s4);
  }
  .cell {
    background: var(--n3);
    padding: var(--s3) var(--s4);
    display: flex;
    flex-direction: column;
  }
  .cell .n {
    font: var(--w-semi) var(--fs-data-xl) / 1 var(--num);
    font-variant-numeric: tabular-nums;
    color: var(--n11);
  }
  .cell .k {
    font: var(--w-semi) var(--fs-micro) / 1 var(--sans);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--n8);
    margin-top: var(--s2);
  }
  .cell .s {
    font-size: var(--fs-xs);
    color: var(--n9);
    margin-top: 4px;
    line-height: 1.4;
  }
  .cell.p .n {
    color: var(--up);
  }
  .cell.r .n {
    color: var(--acc);
  }
  .cell.i .n {
    color: var(--info);
  }
  .cell.x .n {
    color: var(--down);
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s3);
    align-items: center;
    margin-bottom: var(--s3);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    font: var(--w-mid) var(--fs-xs) / 1 var(--sans);
    color: var(--n9);
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: var(--r-full);
    padding: 6px 12px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    transition: background var(--d-hover) var(--ease-out);
  }
  .chip:hover {
    background: var(--n4);
  }
  .chip:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  .chip .cn {
    font: var(--w-semi) var(--fs-micro) / 1 var(--num);
    font-variant-numeric: tabular-nums;
    color: var(--n8);
  }
  .chip.on {
    border-color: var(--n10);
    color: var(--n11);
  }
  .chip.p.on {
    border-color: var(--up);
    color: var(--up);
  }
  .chip.r.on {
    border-color: var(--acc);
    color: var(--acc);
  }
  .chip.i.on {
    border-color: var(--info);
    color: var(--info);
  }
  .chip.a.on {
    border-color: var(--warn);
    color: var(--warn);
  }
  .chip.x.on {
    border-color: var(--down);
    color: var(--down);
  }
  .find {
    font: var(--w-reg) var(--fs-sm) / 1 var(--sans);
    color: var(--n11);
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: var(--r1);
    padding: 8px 12px;
    min-width: 220px;
    flex: 0 1 280px;
  }
  .find:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 1px;
  }

  .tw {
    overflow-x: auto;
    border: 1px solid var(--n6);
    border-radius: var(--r2);
    background: var(--n3);
  }
  table {
    border-collapse: collapse;
    width: 100%;
    font-size: var(--fs-xs);
  }
  th {
    position: sticky;
    top: 0;
    background: var(--n2);
    text-align: left;
    font: var(--w-semi) var(--fs-micro) / 1 var(--sans);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    color: var(--n8);
    padding: 9px var(--s3);
    border-bottom: 1px solid var(--n6);
    white-space: nowrap;
  }
  td {
    padding: 7px var(--s3);
    border-bottom: 1px solid var(--n5);
    vertical-align: top;
    color: var(--n10);
  }
  tbody tr:last-child td {
    border-bottom: 0;
  }
  tbody tr:hover td {
    background: var(--n4);
  }
  td.sym {
    font: var(--w-semi) var(--fs-xs) / 1.4 var(--mono);
    color: var(--n11);
    white-space: nowrap;
  }
  td.nse {
    color: var(--n10);
  }
  td.note {
    color: var(--n9);
    max-width: 46ch;
    line-height: 1.45;
  }
  .pill {
    display: inline-block;
    font: var(--w-semi) var(--fs-micro) / 1 var(--sans);
    letter-spacing: var(--track-caps);
    text-transform: uppercase;
    padding: 3px 7px;
    border-radius: var(--r1);
    white-space: nowrap;
  }
  .pill.p {
    background: var(--up-soft);
    color: var(--up);
  }
  .pill.r {
    background: var(--acc-soft);
    color: var(--acc);
  }
  .pill.i {
    background: var(--info-soft);
    color: var(--info);
  }
  .pill.a {
    background: var(--warn-soft);
    color: var(--warn);
  }
  .pill.x {
    background: var(--down-soft);
    color: var(--down);
  }
  .foot {
    font-size: var(--fs-xs);
    color: var(--n9);
    margin: var(--s3) 0 0;
    max-width: 80ch;
  }

  /* ---- the masters this join reads from ---------------------------- */
  .masters {
    margin-top: var(--s8);
    border: 1px solid var(--n6);
    border-radius: var(--r2);
    background: var(--n2);
    padding: var(--s4);
  }
  .mhead {
    display: flex;
    gap: var(--s4);
    align-items: flex-start;
    justify-content: space-between;
    flex-wrap: wrap;
  }
  .masters h2 {
    margin: 0;
    font-size: 1rem;
    letter-spacing: -0.01em;
  }
  .msub {
    margin: var(--s2) 0 0;
    color: var(--n8);
    font-size: 0.82rem;
    max-width: 62ch;
  }
  .mgo {
    font: inherit;
    font-weight: 600;
    font-size: 0.85rem;
    padding: var(--s2) var(--s4);
    border-radius: var(--r1);
    border: 1px solid var(--acc);
    background: transparent;
    color: var(--acc);
    cursor: pointer;
    white-space: nowrap;
  }
  .mgo:hover:not(:disabled) {
    background: var(--n3);
  }
  .mgo:disabled {
    opacity: 0.55;
    cursor: progress;
  }
  .mrows {
    margin-top: var(--s4);
    display: flex;
    flex-direction: column;
    gap: 1px;
    background: var(--n6);
    border: 1px solid var(--n6);
    border-radius: var(--r1);
    overflow: hidden;
  }
  .mrow {
    background: var(--n3);
    padding: var(--s3) var(--s4);
    display: grid;
    grid-template-columns: minmax(180px, 1.2fr) minmax(150px, 1fr) minmax(150px, 1.4fr);
    gap: var(--s4);
    align-items: baseline;
    font-size: 0.82rem;
  }
  .mname code {
    font-family: var(--mono);
    font-size: 0.8rem;
  }
  .tok {
    display: inline-block;
    margin-left: var(--s2);
    font-size: 0.66rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--warn);
  }
  .mstate,
  .mout {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .ok {
    color: var(--up);
    font-weight: 600;
  }
  .no {
    color: var(--down);
    font-weight: 600;
  }
  .skip {
    color: var(--warn);
    font-weight: 600;
  }
  .dim {
    color: var(--n8);
    font-size: 0.76rem;
  }
  /* THE LADDER. One line per network round trip -- which URL, which try,
     what was waited, what the host said. Monospace because the reader is
     comparing statuses down a column. */
  .ladder {
    background: var(--n2);
    margin: 0;
    padding: var(--s2) var(--s4) var(--s3) var(--s8);
    list-style: decimal;
    font-family: var(--mono);
    font-size: 0.72rem;
    color: var(--n8);
  }
  .ladder li {
    padding: 1px 0;
  }
  .lnum {
    color: var(--n9);
  }
  .lgot {
    margin: 0 var(--s2);
    font-weight: 500;
  }
  .lgot.body,
  .lgot.primed {
    color: var(--up);
  }
  .lgot.refused_retrying,
  .lgot.refused_repriming,
  .lgot.prime_refused {
    color: var(--warn);
  }
  .lgot.refused_settled {
    color: var(--down);
  }
  .lst {
    color: var(--down);
    margin-right: var(--s2);
  }
  .mwhy {
    margin: var(--s3) 0 0;
    color: var(--down);
    font-size: 0.82rem;
  }
  .mrestart {
    margin: var(--s3) 0 0;
    padding: var(--s3) var(--s4);
    border-left: 3px solid var(--warn);
    background: var(--n3);
    border-radius: var(--r1);
    color: var(--n9);
    font-size: 0.82rem;
  }
  .mreload {
    margin: var(--s3) 0 0;
    padding: var(--s3) var(--s4);
    border-left: 3px solid var(--up);
    background: var(--n3);
    border-radius: var(--r1);
    color: var(--n9);
    font-size: 0.82rem;
  }
  /* WHAT EACH FEED ACTUALLY PARSED. Monospace because the reader is
     comparing counts, and this is where `dhan: UNAVAILABLE` shows up
     the moment a master lands that the reader cannot read. */
  .mnotes {
    margin: var(--s2) 0 0;
    color: var(--n8);
    font-family: var(--mono);
    font-size: 0.72rem;
    line-height: 1.5;
    word-break: break-word;
  }
  .mrow.bucket {
    grid-template-columns: minmax(180px, 1fr) 80px;
    font-family: var(--mono);
    font-size: 0.76rem;
  }
  .mrow.bucket .mstate { text-align: right; }
</style>

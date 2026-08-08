<script>
  /**
   * TradingView's OWN chart library, not an imitation of it.
   *
   * `lightweight-charts` is the library TradingView publishes under MIT. Using
   * it is how the chart becomes "equal to TradingView" rather than similar to
   * it: same rendering engine, same crosshair, same scale behaviour.
   *
   * PRICES ARRIVE AS PAISA INTEGERS and are divided ONCE, here, at the display
   * boundary. CLAUDE.md section 7: a float has no business anywhere near a price, and
   * the only reason one appears at all is that a canvas needs a number to draw.
   */
  import { onMount } from 'svelte';

  let { instrument, feed } = $props();
  let host = $state(null);
  // `$state`, NOT a plain `let`. `onMount` is async — it awaits the chart
  // library — so the data effect below runs FIRST, bails at `if (!series)`,
  // and with an untracked binding never runs again. The chart drew an empty
  // canvas and said "no bars held", which was true of the series and false of
  // the store: the endpoint had 375 of them the whole time.
  let chart = $state(null);
  let series = $state(null);
  let state = $state({ loading: false, error: null, bars: 0 });

  onMount(async () => {
    const { createChart, CandlestickSeries } = await import('lightweight-charts');
    chart = createChart(host, {
      layout: { background: { color: 'transparent' }, textColor: '#8a95ab', attributionLogo: false },
      grid: { vertLines: { color: '#1b212e' }, horzLines: { color: '#1b212e' } },
      rightPriceScale: { borderColor: '#232a3a' },
      timeScale: { borderColor: '#232a3a', timeVisible: true, secondsVisible: false },
      crosshair: { mode: 0 }
    });
    series = chart.addSeries(CandlestickSeries, {
      upColor: '#26a69a', downColor: '#ef5350',
      wickUpColor: '#26a69a', wickDownColor: '#ef5350', borderVisible: false
    });
    const ro = new ResizeObserver(() => chart.resize(host.clientWidth, host.clientHeight));
    ro.observe(host);
    return () => { ro.disconnect(); chart.remove(); };
  });

  // Refetch when either the instrument OR THE FEED changes. The feed is part of
  // the identity of the data, never a second series drawn beside the first.
  let month = $state('2026-08');

  $effect(() => {
    const key = instrument?.key;
    const f = feed;
    if (!key || !f || !series) return;
    state.loading = true;
    state.error = null;
    // The store is keyed on (feed, exchange, segment, symbol, month) — the
    // path IS the index, so the request names every part rather than a synthetic
    // id the server would have to resolve.
    const q = new URLSearchParams({
      feed: f,
      exchange: instrument.exchange ?? 'NSE',
      segment: instrument.segment ?? 'INDEX',
      symbol: instrument.symbol,
      month: month
    });
    fetch(`/bars.json?${q}`)
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`))))
      .then((body) => {
        // A PARTIAL read answers 206 with {bars, faults}. Both shapes are
        // handled; a faulty record is surfaced, never silently skipped.
        const bars = Array.isArray(body) ? body : (body.bars ?? []);
        if (body.faults) state.error = body.faults;
        // ONE divide per field, at the boundary. Paisa are integers up to here.
        series.setData(
          bars.map((b) => ({
            time: b.t,
            open: b.o / 100, high: b.h / 100, low: b.l / 100, close: b.c / 100
          }))
        );
        chart.timeScale().fitContent();
        state.bars = bars.length;
        state.loading = false;
      })
      .catch((why) => {
        // NAMED, NOT SWALLOWED. An empty chart with no reason is the failure
        // CLAUDE.md section 4 forbids.
        state.error = String(why);
        state.loading = false;
        series.setData([]);
        state.bars = 0;
      });
  });
</script>

<div class="chart" bind:this={host}></div>
{#if state.error}
  <p class="empty" style="color:var(--down)">{instrument.key} on {feed}: {state.error}</p>
{:else if state.loading}
  <p class="empty">Reading bars…</p>
{:else if state.bars === 0}
  <p class="empty">No bars held for {instrument.key} on {feed}. Pull them from Ingest.</p>
{/if}

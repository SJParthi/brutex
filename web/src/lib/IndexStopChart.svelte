<script>
 import {indexStopChartData} from './index-stop-chart.js';
 import {indexStopPoints} from './index-stop-results.js';
 import {candidateTime} from './candidate-trades.js';
 let {candles=/** @type {any[]} */([]),trade=/** @type {any} */(null),direction=/** @type {string|null} */(null)}=$props();
 let host=$state(/** @type {HTMLElement|null} */(null)),hover=$state.raw(/** @type {any} */(null)),why=$state(''),ready=$state(false);
 const clock=new Intl.DateTimeFormat('en-IN',{timeZone:'Asia/Kolkata',day:'2-digit',month:'short',hour:'2-digit',minute:'2-digit',hour12:false});
 $effect(()=>{
  const element=host,rows=candles,selected=trade,side=direction;if(!element)return;
  let disposed=false,chart=/** @type {any} */(null);ready=false;why='';hover=null;
  void(async()=>{
   try{
    const display=indexStopChartData(rows,selected,side),mod=await import('lightweight-charts');if(disposed)return;
    chart=mod.createChart(element,{autoSize:true,layout:{background:{color:'transparent'},textColor:'#677a8d'},grid:{vertLines:{color:'#8f9fad20'},horzLines:{color:'#8f9fad20'}},rightPriceScale:{borderColor:'#8f9fad50'},timeScale:{timeVisible:true,secondsVisible:false,borderColor:'#8f9fad50',tickMarkFormatter:(/** @type {unknown} */ time)=>clock.format(new Date(Number(time)*1000))},localization:{locale:'en-IN',timeFormatter:(/** @type {unknown} */ time)=>`${clock.format(new Date(Number(time)*1000))} IST`}});
    const series=chart.addSeries(mod.CandlestickSeries,{upColor:'#087b85',downColor:'#a84c39',wickUpColor:'#087b85',wickDownColor:'#a84c39',borderVisible:false,priceFormat:{type:'price',precision:2,minMove:.01}});
    series.setData(display.candles);mod.createSeriesMarkers(series,/** @type {any} */(display.markers));
    if(display.stop!==null)series.createPriceLine({price:display.stop,color:'#a84c39',lineWidth:1,lineStyle:2,axisLabelVisible:true,title:'Saved fixed stop'});
    chart.subscribeCrosshairMove((/** @type {{time?:unknown}} */ event)=>{if(!disposed)hover=event.time==null?null:display.byTime.get(Number(event.time))??null;});
    chart.timeScale().fitContent();ready=true;
   }catch(error){if(!disposed){why=error instanceof Error?error.message:String(error);chart?.remove();chart=null;}}
  })();
  return()=>{disposed=true;chart?.remove();};
 });
</script>
<section aria-label="Candles verified against the saved research source">
 <h5>Original one-minute candles</h5>
 <p>The horizontal axis uses IST. An exit marker identifies the recorded candle; it does not invent a tick or an exact stop-touch time.</p>
 <div class="chart" bind:this={host}></div>
 {#if why}<p role="alert">Chart unavailable: {why}</p>{:else if !ready}<p role="status">Drawing this verified candle page…</p>{/if}
 {#if hover}<p class="ohlc"><b>{candidateTime(hover.micros)}</b> · Open {indexStopPoints(hover.open_paisa)} · High {indexStopPoints(hover.high_paisa)} · Low {indexStopPoints(hover.low_paisa)} · Close {indexStopPoints(hover.close_paisa)}</p>{/if}
 <p class="credit">Chart display uses <a href="https://www.tradingview.com/" target="_blank" rel="noreferrer">TradingView Lightweight Charts</a>. The exact saved figures remain available in the evidence table.</p>
</section>
<style>
 section{min-width:0;margin:1rem 0}h5{font-size:.9rem;margin:.6rem 0}p{font-size:.78rem;line-height:1.6}.chart{height:340px;width:100%;min-width:0;position:relative;border:1px solid var(--line,#cbd6df);border-radius:6px;overflow:hidden}.ohlc{font-variant-numeric:tabular-nums}.credit{color:var(--n8,#5c7287);font-size:.72rem}a{color:var(--acc,#287e96)}[role=alert]{color:var(--down,#ad4944)}@media(max-width:520px){.chart{height:260px}}
</style>

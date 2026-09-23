// Generated chart-boundary fixtures; never used as historical market evidence.
import test from 'node:test';
import assert from 'node:assert/strict';
import {indexStopChartData,indexStopChartPrice,indexStopChartTime} from '../src/lib/index-stop-chart.js';
const start=1704167400000000n;
const bar=(n=0)=>({micros:String(start+60000000n*BigInt(n)),open_paisa:'2000050',high_paisa:'2000200',low_paisa:'1999900',close_paisa:'2000100'});
const trade={entry_micros:String(start),exit_bar_micros:String(start+60000000n),stop_paisa:'1999900'};

test('the chart keeps exact saved integers in its legend and distinct long and short markers',()=>{
 const rows=[bar(),bar(1)];
 for(const direction of ['long','short']){
  const result=indexStopChartData(rows,trade,direction);
  assert.equal(result.candles[0].open,20000.5);
  assert.equal(result.byTime.get(Number(start/1000000n)),rows[0]);
  assert.equal(result.markers[0].shape,direction==='long'?'arrowUp':'arrowDown');
  assert.equal(result.markers[1].text,'Exit candle');
  assert.equal(result.stop,19999);
 }
 assert.throws(()=>indexStopChartData(rows,trade),/direction/);
});

test('markers outside the displayed real candles are not placed on a nearby bar',()=>{
 const result=indexStopChartData([bar(2),bar(3)],trade,'long');
 assert.deepEqual(result.markers,[]);
 const partial=indexStopChartData([bar(1)],trade,'short');
 assert.equal(partial.markers.length,1);
 assert.equal(partial.markers[0].text,'Exit candle');
});

test('the plotted range refuses unsafe price conversion and invalid minute timestamps',()=>{
 for(const price of ['9007199254740992','9223372036854775807','-1','0','01','1.5','NaN'])assert.throws(()=>indexStopChartPrice(price));
 assert.equal(indexStopChartTime('0'),0);
 for(const stamp of ['1',String(start+1n),'8640000000060000000','9223372036854775807'])assert.throws(()=>indexStopChartTime(stamp));
});

test('duplicate, reversed, missing, oversized or invalid OHLC pages refuse without sorting or padding',()=>{
 for(const rows of [[],[bar(),bar()],[bar(1),bar()],[{...bar(),low_paisa:'2000101'}],[{...bar(),high_paisa:'2000000'}],[{...bar(),close_paisa:'-2'}],Array(4097).fill(bar())])assert.throws(()=>indexStopChartData(rows));
 assert.throws(()=>indexStopChartData([bar(),bar(1)],{...trade,entry_micros:trade.exit_bar_micros,exit_bar_micros:trade.entry_micros},'long'),/before/);
 const gap=indexStopChartData([bar(),bar(5)]);
 assert.equal(gap.candles.length,2);
 assert.equal(gap.candles[1].time-gap.candles[0].time,300);
});

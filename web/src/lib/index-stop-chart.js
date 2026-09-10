/** Presentation only. Callers must authenticate the saved source before using
 * this adapter. Exact integer prices remain in the evidence table; decimal
 * numbers are created only for the chart library's display boundary. */
const MAX=BigInt(Number.MAX_SAFE_INTEGER),MINUTE=60000000n;
/** @param {any} value @param {string} label */
function integer(value,label){
 if(typeof value!=='string'||! /^(0|-?[1-9]\d*)$/.test(value)||value.length>20)throw new Error(`The saved ${label} is not an exact integer.`);
 return BigInt(value);
}
/** @param {any} value */
export function indexStopChartPrice(value){
 const price=integer(value,'price');
 if(price<=0n||price>MAX)throw new Error('This price exceeds the chart’s exact integer display range. Use the saved price table.');
 return Number(price)/100;
}
/** @param {any} value */
export function indexStopChartTime(value){
 const stamp=integer(value,'candle time');
 if(stamp%MINUTE!==0n||stamp/1000n>8640000000000000n||stamp/1000n< -8640000000000000n)throw new Error('The saved candle time is outside the chart’s supported minute range.');
 return Number(stamp/1000000n);
}
/** Input keys are a UI adapter contract, not a second storage format.
 * @param {any[]} rows @param {any} [trade] @param {string|null} [direction] */
export function indexStopChartData(rows,trade=null,direction=null){
 if(!Array.isArray(rows)||rows.length===0||rows.length>4096)throw new Error('A bounded page of authenticated candles is required to draw this chart.');
 const byTime=new Map();let previous=-Infinity;
 const candles=rows.map(row=>{
  const time=indexStopChartTime(row.micros);
  if(time<=previous)throw new Error('Saved chart candles repeat or move backwards.');
  const open=indexStopChartPrice(row.open_paisa),high=indexStopChartPrice(row.high_paisa),low=indexStopChartPrice(row.low_paisa),close=indexStopChartPrice(row.close_paisa);
  if(BigInt(row.low_paisa)>BigInt(row.high_paisa)||[row.open_paisa,row.close_paisa].some(value=>BigInt(value)<BigInt(row.low_paisa)||BigInt(value)>BigInt(row.high_paisa)))throw new Error('Saved chart prices violate their OHLC bounds.');
  previous=time;byTime.set(time,row);return {time,open,high,low,close};
 });
 const markers=[];let stop=null;
 if(trade){
  if(!['long','short'].includes(direction??''))throw new Error('The saved trade direction is required for chart markers.');
  const entry=indexStopChartTime(trade.entry_micros),exit=indexStopChartTime(trade.exit_bar_micros);
  if(entry>exit)throw new Error('The saved trade exits before its entry.');
  stop=indexStopChartPrice(trade.stop_paisa);
  if(byTime.has(entry))markers.push({time:entry,position:direction==='long'?'belowBar':'aboveBar',shape:direction==='long'?'arrowUp':'arrowDown',color:'#087b85',text:`${direction==='long'?'Long':'Short'} entry open`});
  if(byTime.has(exit))markers.push({time:exit,position:'aboveBar',shape:'circle',color:'#a84c39',text:'Exit candle'});
 }
 return {candles,byTime,markers,stop};
}

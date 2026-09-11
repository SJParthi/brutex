// Generated transport fixtures only; no market or performance observations.
export const vixHex=(/** @type {number} */ n)=>n.toString(16).padStart(2,'0').repeat(32);
export const VIX_ID=vixHex(1),VIX_PIN=vixHex(2);
export const VIX_START=20000n*86400000000n+555n*60000000n-19800000000n;
const metrics={trades:'1',wins:'0',optimistic_paisa:'-100',pessimistic_paisa:'-150',drawdown_paisa:'150',worst_trade_paisa:'-150',adverse_paisa:'150',favourable_paisa:'50',holding_minutes:'2',unreachable:'0',too_late:'0',gap_invalid:'0',while_open:'0',entry_refused:'0',path_refused:'0',closing_refused:'0',stopped:'1',forced:'0',stop_gaps:'0'};
export const vixSelected={index:'0',program_index:'0',run_id:vixHex(3),source_id:vixHex(4),evaluation_digest:vixHex(5),instrument:'NSE-NIFTY',direction:'long',timeframe:'1min',first_day:'20000',last_day:'20001',expression:'0 | !0',truth:{evaluated:'4',hits:'1',misses:'2',unknown:'1'},metrics,events_count:'1',trades_count:'1',days_count:'2',feed:null};
export const vixTrade={index:'0',signal_bar:'0',entry_bar:'1',exit_bar:'2',holding_minutes:'2',signal_micros:String(VIX_START),signal_close_micros:String(VIX_START+60000000n),entry_micros:String(VIX_START+60000000n),exit_bar_micros:String(VIX_START+120000000n),exit_from_micros:String(VIX_START+120000000n),exit_until_micros:String(VIX_START+180000000n),stop_paisa:'9900',entry_paisa:'10000',optimistic_exit_paisa:'9900',pessimistic_exit_paisa:'9850',optimistic_paisa:'-100',pessimistic_paisa:'-150',adverse_paisa:'150',favourable_paisa:'50',exit_reason:'signal_candle_stop',gapped:false};
export const vixSelection=()=>structuredClone({identity:VIX_ID,completion:VIX_PIN,selected:vixSelected,trade:vixTrade});
/** @param {string} micros */
export const vixCandle=micros=>({micros,open_paisa:'1500',high_paisa:'1550',low_paisa:'1490',close_paisa:'1530',volume:'0',open_interest:'-9223372036854775808'});
export function vixBody(){
 const monthDate=new Date(Number(20000n*86400000n));
 return structuredClone({schema_version:1,status:'saved',model:'index-stop-vix',provenance_status:'saved_reference_snapshot',catalog_identity:VIX_ID,catalog_completion:VIX_PIN,
  reference:{identity:vixHex(6),publication_id:vixHex(7),completion:vixHex(8)},selected:vixSelected,feed:'fixture-feed',reference_symbol:'NSE-INDIAVIX',reference_timeframe:'1min',reference_only:true,entry_basis:'entry_minute',exit_basis:'exit_minute_candle',policy:'Generated fixture; immutable reference only.',
  summary:{settings:'2',trades:'1',exact_stamps:'2',absent_stamps:'0',unavailable_stamps:'0'},total:'1',offset:'0',limit:1,next:null,
  rows:[{trade_index:'0',run_id:vixSelected.run_id,original_trade_digest:vixHex(9),entry_micros:vixTrade.entry_micros,exit_bar_micros:vixTrade.exit_bar_micros,exit_from_micros:vixTrade.exit_from_micros,exit_until_micros:vixTrade.exit_until_micros,
   month:{index:'0',year:monthDate.getUTCFullYear(),month:monthDate.getUTCMonth()+1,records:'2',snapshot_digest:vixHex(10),unavailable_code:null,unavailable_reason:null},entry:{state:'exact',candle:vixCandle(vixTrade.entry_micros)},exit:{state:'exact',candle:vixCandle(vixTrade.exit_bar_micros)},original_trade:vixTrade}],
  admitted_bytes:'65536',observation_byte_limit:'131072'});
}

// Generated transport-only examples. No row is market evidence or a trading result.
import {assessment,sync,hex} from './index-consistency-fixture.js';
export const SEARCH=hex(900),PIN=hex(901);
/** @returns {any} */
export function setting(batch=0,index=0,later=false,score='1000'){
 return {index:String(index),program_index:String(Math.floor(index/2)),run_id:hex(100+batch*100+index+(later?30:0)),source_id:hex(later?7:6),evaluation_digest:hex(1000+batch*100+index+(later?30:0)),instrument:'NSE-NIFTY',direction:index%2===0?'long':'short',timeframe:'1min',first_day:later?'11':'4',last_day:later?'15':'8',expression:'0 | !0',truth:{evaluated:'20',hits:'10',misses:'5',unknown:'5'},metrics:{trades:'10',wins:'6',optimistic_paisa:String(BigInt(score)+200n),pessimistic_paisa:score,drawdown_paisa:'200',worst_trade_paisa:'-100',adverse_paisa:'200',favourable_paisa:'2000',holding_minutes:'20',unreachable:'0',too_late:'0',gap_invalid:'0',while_open:'0',entry_refused:'0',path_refused:'0',closing_refused:'0',stopped:'0',forced:'10',stop_gaps:'0'},events_count:'10',trades_count:'10',days_count:'5',feed:null};
}
/** @returns {any} */
export function row(batch=0,index=0,score='1000',status='admitted',localRank=1){
 const qualification={identity:hex(10+batch*10),completion:hex(11+batch*10)},original=setting(batch,index),later=setting(batch,index,true,score),daily=assessment();
 daily.qualification=qualification;daily.setting_index=String(index);daily.periods.later.summary.pessimistic_paisa=score;daily.periods.full.summary.pessimistic_paisa=String(1000n+BigInt(score));sync(daily,status);
 const state=status==='admitted'?'passed':status==='rejected'?'failed':status;
 return {index:String(index),rank:'1',global_rank:'1',batch:String(batch),qualification_rank:String(localRank),qualification_total:'2',qualification,original_source:{identity:hex(50+batch*10),completion:hex(51+batch*10)},later_source:{identity:hex(52+batch*10),completion:hex(53+batch*10)},original,later,institutional:{index:String(index),source_index:String(index),identity:later.run_id,status,failed:status==='rejected'?'1':'0',unmeasured:status==='unmeasured'?'1':'0',refused:status==='refused'?'1':'0',checks:Array.from({length:44},(_,i)=>({index:String(i),name:`fixture_check_${i}`,state:i===0?state:'passed'})),values:Array.from({length:44},(_,i)=>({name:`fixture_value_${i}`,state:'measured',value:'0'}))},index_consistency:daily};
}
/** @param {string} filter @param {string} offset @param {number} limit @returns {any} */
export function page(filter='qualified',offset='0',limit=16){
 const all=[row(1,1,'1200','admitted',1),row(0,0,'1100','admitted',1),row(0,1,'1100','rejected',2),row(1,0,'1000','refused',2)];
 all.forEach((r,index)=>r.global_rank=String(index+1));
 const shown=all.filter(r=>filter==='all'||r.index_consistency.combined_qualifies);shown.forEach((r,index)=>r.rank=String(index+1));
 return {schema_version:1,status:'saved',model:'index-stop-ranking',identity:SEARCH,timeframe:'1min',rung:'0',checkpoint:{sequence:'5',completion:PIN},scope:{completed_batches:'2',completed_programs:'2',completed_work:'16',included_families:'2',first_batch:'0',last_batch:'1',pending_next_batch:false,exhausted:false,interrupted_checkpoints:'0',writer_observed_at_admission:false,instrument:'NSE-NIFTY',feed:'zerodha',training_first_day:'4',training_last_day:'8',later_first_day:'11',later_last_day:'15'},ranking:'later_pessimistic_descending_then_batch_then_setting',filter,total:String(shown.length),all_total:'4',offset,limit,summary:{institutional_admitted:'2',rejected:'1',refused:'1',unmeasured:'0',combined_qualified:'2'},read_work:{retained_serialized_bytes:'5000',charged_grammar_nodes:'400',charged_required_bootstrap_work:'100000',charged_required_split_work:'100000',cold:'complete_acknowledged_prefix_then_global_sort',warm:'ancestor_generation_checks_then_bounded_indexed_page'},rows:shown.slice(Number(offset),Number(offset)+limit),refusal:null};
}

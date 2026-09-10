// Generated wire fixtures only. These are not market data or research results.
export const hex = (/** @type {any} */ n) => Number(n).toString(16).padStart(64, '0');
export const reasons = ['winning_day_ratio_below_three_fifths','complete_week_has_fewer_than_three_wins','complete_week_has_more_than_two_losses','losing_day_streak_exceeds_two','no_complete_five_session_week','no_eligible_session','missing_expected_session','unmeasured_calendar','invalid_requested_span','invalid_session_order_or_span','session_on_closed_or_excluded_day','invalid_session_count_or_return','exact_arithmetic_overflow','week_output_allocation_refused'];
export const context = () => ({ qualification:{identity:hex(1),completion:hex(2)},setting_index:'480',instrument:'NSE-NIFTY',institutional:'admitted' });
export const policy = () => ({schema_version:1,policy_digest:hex(3),instruments:['NSE-NIFTY','NSE-BANKNIFTY'],minimum_winning_day_numerator:'3',minimum_winning_day_denominator:'5',minimum_week_winning_days:'3',maximum_week_losing_days:'2',maximum_losing_day_streak:'2',pnl_basis:'pessimistic_gross_paisa',zero_days_reset_streak:false,costs_included:false,evaluated_scope:'training_and_later_with_calendar_gap_check'});
/** @returns {any} */
export function summary() { return {calendar_days:'5',eligible_days:'5',observed_days:'5',winning_days:'3',losing_days:'2',zero_days:'0',no_trade_days:'0',missing_days:'0',unmeasured_days:'0',excluded_days:'0',closed_days:'0',weekend_sessions:'0',trades:'10',pessimistic_paisa:'1000',longest_losing_streak:'2',complete_weeks:'1',passing_weeks:'1',failing_weeks:'0',short_weeks:'0',partial_weeks:'0',unmeasured_weeks:'0'}; }
/** @returns {any} */
export function period(first='4',last='8') { return {state:'passed',first_day:first,last_day:last,reasons_bits:'0',reasons:[],first_issue_day:null,days_count:'5',weeks_count:'1',summary:summary()}; }
/** @param {any} value @returns {any} */
export function sync(value, institutional='admitted') {
  const full=value.periods.full;
  Object.assign(value,{state:['refused','unmeasured','failed','passed','not_applicable'].find((/** @type {any} */ state) =>Object.values(value.periods).some((/** @type {any} */ p) =>p.state===state)),days_count:full.days_count,weeks_count:full.weeks_count});
  const {state,days_count,weeks_count,...facts}=full;
  value.evaluation={instrument:value.evaluation?.instrument??'NSE-NIFTY',calendar_digest:hex(6),sessions_digest:hex(7),weeks_digest:hex(8),...structuredClone(facts)};
  value.combined_qualifies=institutional==='admitted'&&['passed','not_applicable'].includes(value.state);
  return value;
}
export function assessment() {
  const full=period('4','15');full.days_count='10';full.weeks_count='2';
  Object.assign(full.summary,{calendar_days:'12',eligible_days:'10',observed_days:'10',winning_days:'6',losing_days:'4',closed_days:'2',trades:'20',pessimistic_paisa:'2000',complete_weeks:'2',passing_weeks:'2'});
  return sync({schema_version:1,state:'passed',policy_digest:hex(3),receipt:{identity:hex(4),completion:hex(5)},qualification:context().qualification,setting_index:'480',combined_qualifies:true,evaluation:null,days_count:'10',weeks_count:'2',evaluated_scope:'training_and_later_with_calendar_gap_check',periods:{training:period(),later:period('11','15'),full}});
}
export function legacy() { return {schema_version:1,state:'not_assessed',policy_digest:null,receipt:null,qualification:context().qualification,setting_index:'480',combined_qualifies:null,evaluation:null,days_count:'0',weeks_count:'0',evaluated_scope:null,periods:null}; }
/** @param {any} period @param {number} bits @param {string} state */
export function reason(period,bits,state) { Object.assign(period,{state,reasons_bits:String(bits),reasons:reasons.filter((_,index)=>BigInt(bits)&(1n<<BigInt(index)))}); }
export function cash() {
  const value=assessment();value.evaluation.instrument='NSE-RELIANCE';
  for(const p of Object.values(value.periods)) { p.state='not_applicable';p.days_count='0';p.weeks_count='0';Object.keys(p.summary).forEach((/** @type {any} */ k) =>p.summary[k]='0'); }
  return sync(value);
}
export function dayRows(period='full') { return (period==='training'?[4,5,6,7,8]:period==='later'?[11,12,13,14,15]:[4,5,6,7,8,11,12,13,14,15]).map((day,index)=>({index:String(index),day:String(day),pessimistic_paisa:[7,8,14,15].includes(day)?'-100':'400',trades:'2'})); }
export function week(index=0,monday='4') { return {index:String(index),monday,kind:'complete',state:'passed',weekday_days:'5',eligible_days:'5',observed_days:'5',winning_days:'3',losing_days:'2',zero_days:'0',no_trade_days:'0',missing_days:'0',unmeasured_days:'0',excluded_days:'0',closed_weekdays:'0',weekend_sessions:'0',trades:'10',pessimistic_paisa:'1000'}; }
/** @param {any} value @returns {any} */
export function page(value,kind='index-weeks',period='full',offset='0',limit=16) {
  const rows=kind==='index-days'?dayRows(period):period==='full'?[week(),week(1,'11')]:[week(0,period==='later'?'11':'4')];
  return {schema_version:1,status:'saved',model:'qualification',kind,identity:value.qualification.identity,completion:value.qualification.completion,setting:value.setting_index,receipt:structuredClone(value.receipt),period,total:String(rows.length),offset,limit,rows:rows.slice(Number(offset),Number(offset)+limit),refusal:null};
}

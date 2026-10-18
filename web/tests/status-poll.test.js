import {test} from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createPageRequests} from '../src/lib/page-requests.js';
import {createStatusPoll,statusAnswer,BASE_MS,CEILING_MS,STOP_AFTER} from '../src/lib/status-poll.js';

// p99-7 (AF-06): the /backtest status poll backs off, pauses and stops out loud.
const flush=()=>new Promise(resolve=>setImmediate(resolve));
function clock(){
 let id=0;
 /** @type {Map<number,{work:()=>void,delay:number}>} */ const timers=new Map();
 return {timers,
  /** @param {()=>void} work @param {number} delay */
  schedule(work,delay){timers.set(++id,{work,delay});return id;},
  /** @param {number} key */ cancel(key){timers.delete(key);},
  delays(){return [...timers.values()].map(timer=>timer.delay);},
  async tick(){const next=timers.entries().next().value;assert.ok(next,'a check must be armed');timers.delete(next[0]);next[1].work();await flush();}
 };
}

/** A finished CLI sweep: settled, so an unchanged copy of it is not news. */
const SETTLED=Object.freeze({phase:'done',run:{where:'cli',status:'completed',in_flight:false,run:7,message:'command finished',at_unix_millis:1,age_millis:0,report:'The external command recorded successful completion.'},why:''});
/**
 * A browser-launched sweep in flight, exactly as `Progress::to_json` answers:
 * every field stays the same until the run ends, so the answer key never
 * changes while rungs load (the AF-06 r1 blocking finding).
 */
const BROWSER_RUNNING=Object.freeze({phase:'running',run:{kind:'sweep',where:'browser',feed:'zerodha',underlying:'NSE-NIFTY',from_year:2018,from_month:1,to_year:2026,to_month:9,support_ppm:5000,started_micros:1760500000000000,attempt:3,attempt_key:'3',in_flight:true},why:''});

/**
 * A page in miniature: `answer` is what the server says, `poll` is the page's
 * own re-poll (it asks, then schedules itself at 2 s exactly as `pollSweep` does).
 * @param {{answer?:any,visible?:boolean,stopAfter?:number}} [options]
 */
function page(options={}){
 const timer=clock();
 /** @type {{answer:any,visible:boolean,gets:number,reports:any[],wake:(()=>void)|null,unlistened:boolean}} */
 const world={answer:options.answer??SETTLED,visible:options.visible??true,gets:0,reports:[],wake:null,unlistened:false};
 /** @type {any} */ let sweep={phase:'idle',run:null,why:''};
 const admission={available:false,why:'A sweep is running.'};
 /** @param {import('../src/lib/page-requests.js').ReadTicket} ticket */
 async function poll(ticket){
  world.gets+=1;await flush();if(!ticket.current())return;
  sweep=structuredClone(world.answer);
  status.schedule(poll,2000);
 }
 const status=createStatusPoll({
  requests:createPageRequests(timer),
  isPoll:work=>work===poll,
  answer:()=>statusAnswer(sweep,admission),
  visible:()=>world.visible,
  listen:wake=>{world.wake=wake;return ()=>{world.unlistened=true;};},
  report:state=>world.reports.push(state),
  stopAfter:options.stopAfter
 });
 return {timer,world,status,poll};
}

test('backs_off_while_unchanged',async()=>{
 const {timer,world,status,poll}=page();
 await status.run(poll);
 /** @type {number[]} */ const delays=[];
 for(let i=0;i<8;i+=1){delays.push(...timer.delays());await timer.tick();}
 assert.deepEqual(delays,[2000,4000,8000,16000,32000,60000,60000,60000]);
 assert.equal(world.gets,9);
 assert.equal(status.state().stopped,'','a known answer never stops the poll; it only slows to the ceiling');
 // The volatile age the server recomputes on every read is not a change.
 world.answer={...world.answer,run:{...world.answer.run,age_millis:123456}};
 await timer.tick();assert.deepEqual(timer.delays(),[CEILING_MS]);
});

test('returns_to_two_seconds_on_change',async()=>{
 const {timer,world,status,poll}=page();
 await status.run(poll);
 for(let i=0;i<4;i+=1)await timer.tick();
 assert.deepEqual(timer.delays(),[32000]);
 world.answer={...world.answer,run:{...world.answer.run,run:8,message:'command finished',at_unix_millis:2}};
 await timer.tick();
 assert.deepEqual(timer.delays(),[BASE_MS],'a new finished run is a change, so the next check is 2 s away again');
 await timer.tick();assert.deepEqual(timer.delays(),[4000]);
 // Moving between running and unknown is a change too.
 world.answer={phase:'unknown',run:null,why:'the log is silent'};
 await timer.tick();assert.deepEqual(timer.delays(),[BASE_MS]);
 // A launch cancels: the next check goes at the page's own delay, not an inherited back-off.
 await timer.tick();assert.deepEqual(timer.delays(),[4000]);
 status.cancel();assert.equal(timer.timers.size,0);
 status.schedule(poll,1200);assert.deepEqual(timer.delays(),[1200]);
});

test('pauses_while_hidden',async()=>{
 const {timer,world,status,poll}=page();
 await status.run(poll);
 assert.equal(world.gets,1);
 world.visible=false;world.wake?.();
 assert.equal(world.reports.at(-1).hidden,true);
 await timer.tick();
 assert.equal(world.gets,1,'a timer that fires while the tab is hidden sends no GET');
 assert.equal(timer.timers.size,0,'and arms nothing: a hidden tab costs the store nothing');
 assert.equal(status.state().parked,true);
 world.visible=true;world.wake?.();await flush();await flush();
 assert.equal(world.gets,2,'showing the tab checks at once');
 assert.equal(status.state().parked,false);
 assert.equal(timer.timers.size,1);
 // Shown while a long back-off is still waiting: check now rather than up to a minute later.
 for(let i=0;i<4;i+=1)await timer.tick();
 assert.deepEqual(timer.delays(),[CEILING_MS]);
 const before=world.gets;world.visible=false;world.wake?.();world.visible=true;world.wake?.();await flush();await flush();
 assert.equal(world.gets,before+1);
 // A page that starts hidden asks nothing until it is shown.
 const hidden=page({visible:false});
 await hidden.status.run(hidden.poll);
 assert.equal(hidden.world.gets,0);assert.equal(hidden.status.state().parked,true);
 hidden.world.visible=true;hidden.world.wake?.();await flush();await flush();
 assert.equal(hidden.world.gets,1);
});

test('stops_after_n_unknown_and_says_why',async()=>{
 const why='external sweep log is incomplete: scan cap true.';
 const {timer,world,status,poll}=page({answer:{phase:'unknown',run:{where:'cli',status:'unknown'},why}});
 await status.run(poll);
 let ticks=0;
 while(timer.timers.size>0){
  // The reason text may differ between unknown answers; it is still one answer.
  world.answer={...world.answer,why:`${why} (${ticks})`};
  await timer.tick();ticks+=1;
  assert.ok(ticks<STOP_AFTER+2,'the poll must stop');
 }
 assert.equal(world.gets,STOP_AFTER,`it stops on the ${STOP_AFTER}th unchanged unknown answer`);
 const stopped=status.state().stopped;
 assert.match(stopped,new RegExp(`after ${STOP_AFTER} "unknown" answers in a row`));
 assert.match(stopped,/scan cap true/,'the reason the server gave is in the sentence');
 assert.match(stopped,/Press Refresh/);
 assert.equal(world.reports.at(-1).stopped,stopped,'the stop is reported to the page, never silent');
 // Refresh checks at once and starts again from 2 s.
 status.refresh();await flush();await flush();
 assert.equal(world.gets,STOP_AFTER+1);
 assert.equal(status.state().stopped,'');
 assert.deepEqual(timer.delays(),[BASE_MS]);
 // Showing the tab never restarts a stopped poll on its own: only Refresh does.
 const again=page({answer:{phase:'unknown',run:null,why},stopAfter:2});
 await again.status.run(again.poll);await again.timer.tick();
 assert.notEqual(again.status.state().stopped,'');
 again.world.wake?.();await flush();
 assert.equal(again.world.gets,2);assert.equal(again.timer.timers.size,0);
});

test('keeps_two_seconds_while_a_sweep_is_in_flight',async()=>{
 // THE AF-06 r1 BLOCKING FINDING. A browser sweep's status body is constant
 // until it ends, and the rung progress, the best rows so far and the finish
 // are all read on this tick. Backing off here made the live view up to 60 s
 // old and a finished sweep look running for up to 60 s.
 const {timer,world,status,poll}=page({answer:BROWSER_RUNNING});
 await status.run(poll);
 /** @type {number[]} */ const delays=[];
 for(let i=0;i<9;i+=1){delays.push(...timer.delays());assert.equal(status.state().backedOff,false,'an in-flight sweep is never backed off');await timer.tick();}
 assert.deepEqual(delays,Array(9).fill(BASE_MS),'every check of an in-flight sweep is 2 s after the last');
 assert.equal(world.gets,10);
 assert.equal(status.state().stopped,'');
 // The finish is seen on the next 2 s check, and only THEN does the poll slow.
 world.answer={phase:'done',run:{...BROWSER_RUNNING.run,in_flight:false,report:'swept'},why:''};
 await timer.tick();assert.deepEqual(timer.delays(),[BASE_MS]);
 await timer.tick();assert.deepEqual(timer.delays(),[4000]);assert.equal(status.state().backedOff,true);
 // A CLI sweep in flight keeps 2 s as well, even across two identical answers.
 const cli=page({answer:{phase:'running',run:{where:'cli',status:'running',in_flight:true,run:61,message:'rung 1min loaded',at_unix_millis:5,age_millis:0},why:''}});
 await cli.status.run(cli.poll);await cli.timer.tick();await cli.timer.tick();
 assert.deepEqual(cli.timer.delays(),[BASE_MS]);
 // A launch accepted but not yet reported (run null) is live too.
 assert.equal(statusAnswer({phase:'running',run:null,why:''},null).live,true);
 // A contradictory body (running, in_flight false) is not treated as live.
 assert.equal(statusAnswer({phase:'running',run:{in_flight:false}},null).live,false);
 assert.equal(statusAnswer({phase:'unknown',run:{in_flight:true},why:'x'},null).live,false);
 // Only the poll that read the run keeps the pace: a lease re-check made
 // while the page still shows an earlier `running` backs off as before.
 const lease=clock();
 /** @param {import('../src/lib/page-requests.js').ReadTicket} _ticket */
 async function recheck(_ticket){leasePoll.schedule(recheck,2000);}
 const leasePoll=createStatusPoll({requests:createPageRequests(lease),isPoll:()=>true,followsRun:work=>work!==recheck,
  answer:()=>statusAnswer(BROWSER_RUNNING,{available:false,why:'held'})});
 await leasePoll.run(recheck);
 /** @type {number[]} */ const leaseDelays=[];
 for(let i=0;i<4;i+=1){leaseDelays.push(...lease.delays());await lease.tick();}
 assert.deepEqual(leaseDelays,[BASE_MS,4000,8000,16000]);
 // A hidden tab still sends nothing while a sweep runs.
 world.answer=BROWSER_RUNNING;const hidden=page({answer:BROWSER_RUNNING});
 await hidden.status.run(hidden.poll);hidden.world.visible=false;hidden.world.wake?.();await hidden.timer.tick();
 assert.equal(hidden.world.gets,1);assert.equal(hidden.status.state().parked,true);
});

test('an_unknown_answer_whose_lease_alternates_still_stops',async()=>{
 // Flipping the lease is a change, so the wait goes back to 2 s, but it is
 // still an unknown answer, so it still counts towards the stop.
 const {timer,world,status}=page();
 const why='the external command has no usable terminal receipt';
 let free=false;
 /** @param {import('../src/lib/page-requests.js').ReadTicket} ticket */
 async function flipping(ticket){
  world.gets+=1;await flush();if(!ticket.current())return;
  free=!free;
  flips.schedule(flipping,2000);
 }
 const flips=createStatusPoll({
  requests:createPageRequests(timer),
  isPoll:work=>work===flipping,
  answer:()=>statusAnswer({phase:'unknown',run:null,why},{available:free,why:''}),
  report:state=>world.reports.push(state)
 });
 status.dispose();
 const before=world.gets;
 await flips.run(flipping);
 let ticks=0;
 while(timer.timers.size>0){await timer.tick();ticks+=1;assert.ok(ticks<STOP_AFTER+2,'an alternating lease must not keep the poll at 2 s for ever');}
 assert.equal(world.gets-before,STOP_AFTER);
 assert.match(flips.state().stopped,new RegExp(`after ${STOP_AFTER} "unknown" answers in a row`));
 assert.match(flips.state().stopped,/no usable terminal receipt/);
 // A known answer in the middle starts the count again.
 const mixed=page({answer:{phase:'unknown',run:null,why},stopAfter:3});
 await mixed.status.run(mixed.poll);await mixed.timer.tick();
 mixed.world.answer=SETTLED;await mixed.timer.tick();
 mixed.world.answer={phase:'unknown',run:null,why};await mixed.timer.tick();await mixed.timer.tick();
 assert.equal(mixed.status.state().stopped,'','two unknown answers after a known one are not three in a row');
 await mixed.timer.tick();assert.notEqual(mixed.status.state().stopped,'');
});

test('extremes: a non-poll is never slowed, dispose unlistens, and the answer key ignores only the volatile age',async()=>{
 const {timer,world,status}=page();
 const other=async()=>{};
 status.schedule(other,0);status.schedule(other,0);
 assert.deepEqual(timer.delays(),[0],'an immediate adoption read keeps its own delay');
 status.dispose();
 assert.equal(world.unlistened,true);assert.equal(timer.timers.size,0);
 status.schedule(other,0);assert.equal(timer.timers.size,0,'a disposed poll arms nothing');
 const a=statusAnswer({phase:'running',run:{run:1,at_unix_millis:5,age_millis:1}},{available:false,why:'x'});
 const b=statusAnswer({phase:'running',run:{run:1,at_unix_millis:5,age_millis:9999}},{available:false,why:'x'});
 const c=statusAnswer({phase:'running',run:{run:1,at_unix_millis:6,age_millis:1}},{available:false,why:'x'});
 assert.equal(a.key,b.key);assert.notEqual(a.key,c.key);
 assert.notEqual(statusAnswer({phase:'unknown',why:'a'},{available:false}).key,statusAnswer({phase:'unknown',why:'a'},{available:true}).key,
  'the lease becoming free is a change even while history is unknown');
 assert.equal(statusAnswer({phase:'unknown',why:''},null).why,'Execution state is unknown.');
 assert.equal(statusAnswer(null,null).unknown,false);
 // Garbage options are bounded rather than trusted.
 const tiny=createStatusPoll({requests:createPageRequests(clock()),isPoll:()=>true,answer:()=>({key:'k',unknown:true,live:false,why:'w'}),stopAfter:0,ceiling:1});
 tiny.schedule(async()=>{},2000);assert.equal(tiny.state().stopped,'');
 tiny.schedule(async()=>{},2000);assert.notEqual(tiny.state().stopped,'','stopAfter below one is one');
});

test('the actual /backtest page polls through the helper and renders its stop with a Refresh control',()=>{
 const source=readFileSync(new URL('../src/routes/backtest/+page.svelte',import.meta.url),'utf8');
 assert.match(source,/import \{ BASE_MS, createStatusPoll, statusAnswer \} from '\$lib\/status-poll\.js';/);
 assert.match(source,/const statusRequests = createStatusPoll\(\{\s*requests: createPageRequests\(\),/);
 assert.match(source,/isPoll: \(work\) => work === pollSweep \|\| work === refreshCurrentAdmission,/);
 assert.match(source,/followsRun: \(work\) => work === pollSweep,/,'only the poll that reads the run keeps 2 s while it runs');
 assert.match(source,/answer: \(\) => statusAnswer\(sweep, launchAdmission\),/);
 assert.match(source,/document\.addEventListener\('visibilitychange', wake\)/);
 assert.match(source,/document\.removeEventListener\('visibilitychange', wake\)/);
 assert.match(source,/\{:else if statusPoll\.backedOff \|\| sweep\.phase === 'unknown'\}/,'the "has not changed" note follows the poll, not the phase');
 assert.doesNotMatch(source,/sweep\.phase === 'running' && statusPoll\.delay > BASE_MS/);
 assert.match(source,/\{#if statusPoll\.stopped\}[\s\S]{0,400}\{statusPoll\.stopped\}[\s\S]{0,400}onclick=\{\(\) => statusRequests\.refresh\(adoptRunning\)\}/);
 assert.doesNotMatch(source,/The monitor will retry\./,'the page no longer promises an endless retry');
 assert.doesNotMatch(source,/setInterval\(/);
});

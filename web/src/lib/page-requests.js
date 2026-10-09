/** @typedef {{signal:AbortSignal,current:()=>boolean}} ReadTicket */
/** @typedef {(ticket:ReadTicket)=>Promise<void>} Work */

/** One in-flight read and one pending timer owned by a mounted page.
 * Cancellation revokes late responses even when a mock/transport ignores abort.
 *
 * A TIMER THAT COMES DUE WHILE THE PAGE IS HIDDEN ISSUES NO READ (conc17-2,
 * D-2567). With `visible` and `listen` given, a scheduled read whose timer
 * fires while `visible()` is false is PARKED instead of run: no request leaves
 * the browser and nothing reschedules, so a background tab stops polling. The
 * first `listen` wake that finds the page visible runs the parked read at once
 * -- one immediate poll on return, not a wait for the next interval. A newer
 * `schedule`, a direct `run`, `cancel` and `dispose` each discard a parked read,
 * exactly as they discard a pending timer. Without the two options nothing
 * parks and the lifecycle is the one it always was. `run` is never gated: a
 * read the operator or the mount asked for directly is not a poll.
 *
 * Why here and not `watchVisible`: the backtest status poll schedules several
 * different reads (`pollSweep`, `adoptRunning`, `refreshCurrentAdmission`) from
 * many call sites through one owner, which `watchVisible`'s single repeating
 * `work` cannot express. Each of its `/backtest/run.json` and `/live.json`
 * reads is one audited invocation on the server, so the hidden poll was an
 * unbounded on-disk writer; this cuts the rate, it does not bound the journal.
 * @param {{schedule?:(work:()=>void,delay:number)=>any,cancel?:(timer:any)=>void,onError?:(error:unknown)=>void,visible?:()=>boolean,listen?:(wake:()=>void)=>()=>void}} [options] */
export function createPageRequests(options={}) {
  const schedule=options.schedule??setTimeout,cancel=options.cancel??clearTimeout;
  const visible=options.visible??(()=>true);
  let disposed=false,generation=0,active=/** @type {AbortController|null} */(null);
  let timer=/** @type {any} */(null),pending=/** @type {{work:Work,delay:number}|null} */(null);
  let parked=/** @type {Work|null} */(null);
  const clear=()=>{if(timer!==null)cancel(timer);timer=null;};
  function arm(){
    if(disposed||active||!pending)return;
    const next=pending,mine=generation;pending=null;
    timer=schedule(()=>{
      if(disposed||mine!==generation)return;
      timer=null;
      if(!visible()){parked=next.work;return;}
      void run(next.work).catch(options.onError??console.error);
    },next.delay);
  }
  /** @param {Work} work */
  async function run(work){
    if(disposed)return;
    clear();parked=null;
    if(active){pending={work,delay:0};return;}
    pending=null;
    const mine=generation,controller=new AbortController();active=controller;
    try{await work({signal:controller.signal,current:()=>!disposed&&mine===generation&&!controller.signal.aborted});}
    finally{if(active===controller)active=null;arm();}
  }
  function revoke(){generation+=1;pending=null;parked=null;clear();active?.abort();}
  const wake=()=>{
    if(disposed||parked===null||!visible())return;
    const work=parked;parked=null;
    void run(work).catch(options.onError??console.error);
  };
  const unlisten=options.listen?options.listen(wake):null;
  return {
    run,
    /** @param {Work} work @param {number} delay */
    schedule(work,delay){if(disposed)return;clear();parked=null;pending={work,delay};arm();},
    cancel:revoke,
    dispose(){if(disposed)return;disposed=true;revoke();unlisten?.();}
  };
}

/** Poll only while visible, using the same single-flight lifecycle as page reads.
 * @param {Work} work @param {number} interval
 * @param {{visible:()=>boolean,listen:(wake:()=>void)=>()=>void,schedule?:(work:()=>void,delay:number)=>any,cancel?:(timer:any)=>void}} options */
export function watchVisible(work,interval,options){
  // ONLY THE CLOCK IS PASSED DOWN. `watchVisible` owns its own visibility
  // listener below; handing `visible`/`listen` to `createPageRequests` as well
  // would register a second listener and a second gate (conc17-2, D-2567).
  const reads=createPageRequests({schedule:options.schedule,cancel:options.cancel});
  /** @param {ReadTicket} ticket */
  async function poll(ticket){
    try{await work(ticket);}
    finally{if(ticket.current()&&options.visible())reads.schedule(poll,interval);}
  }
  const wake=()=>{reads.cancel();if(options.visible())void reads.run(poll);};
  const unlisten=options.listen(wake);wake();
  return Object.assign(()=>{unlisten();reads.dispose();},{refresh:wake});
}

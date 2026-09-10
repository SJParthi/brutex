/** @typedef {{signal:AbortSignal,current:()=>boolean}} ReadTicket */
/** @typedef {(ticket:ReadTicket)=>Promise<void>} Work */

/** One in-flight read and one pending timer owned by a mounted page.
 * Cancellation revokes late responses even when a mock/transport ignores abort.
 * @param {{schedule?:(work:()=>void,delay:number)=>any,cancel?:(timer:any)=>void,onError?:(error:unknown)=>void}} [options] */
export function createPageRequests(options={}) {
  const schedule=options.schedule??setTimeout,cancel=options.cancel??clearTimeout;
  let disposed=false,generation=0,active=/** @type {AbortController|null} */(null);
  let timer=/** @type {any} */(null),pending=/** @type {{work:Work,delay:number}|null} */(null);
  const clear=()=>{if(timer!==null)cancel(timer);timer=null;};
  function arm(){
    if(disposed||active||!pending)return;
    const next=pending,mine=generation;pending=null;
    timer=schedule(()=>{if(disposed||mine!==generation)return;timer=null;void run(next.work).catch(options.onError??console.error);},next.delay);
  }
  /** @param {Work} work */
  async function run(work){
    if(disposed)return;
    clear();
    if(active){pending={work,delay:0};return;}
    pending=null;
    const mine=generation,controller=new AbortController();active=controller;
    try{await work({signal:controller.signal,current:()=>!disposed&&mine===generation&&!controller.signal.aborted});}
    finally{if(active===controller)active=null;arm();}
  }
  function revoke(){generation+=1;pending=null;clear();active?.abort();}
  return {
    run,
    /** @param {Work} work @param {number} delay */
    schedule(work,delay){if(disposed)return;clear();pending={work,delay};arm();},
    cancel:revoke,
    dispose(){disposed=true;revoke();}
  };
}

/** Poll only while visible, using the same single-flight lifecycle as page reads.
 * @param {Work} work @param {number} interval
 * @param {{visible:()=>boolean,listen:(wake:()=>void)=>()=>void,schedule?:(work:()=>void,delay:number)=>any,cancel?:(timer:any)=>void}} options */
export function watchVisible(work,interval,options){
  const reads=createPageRequests(options);
  /** @param {ReadTicket} ticket */
  async function poll(ticket){
    try{await work(ticket);}
    finally{if(ticket.current()&&options.visible())reads.schedule(poll,interval);}
  }
  const wake=()=>{reads.cancel();if(options.visible())void reads.run(poll);};
  const unlisten=options.listen(wake);wake();
  return Object.assign(()=>{unlisten();reads.dispose();},{refresh:wake});
}

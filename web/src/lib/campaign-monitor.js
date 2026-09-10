import { campaignSelection } from './boolean-campaign.js';

/** One sequential request, one timer, exact-identity cancellation and bounded retries.
 * Success follows newer snapshots only while waiting/started. Paused or terminal
 * campaigns stop. A watcher is an observer, never a claim of worker liveness.
 * @param {(selection:any,signal:AbortSignal)=>Promise<any>} request
 * @param {(state:any)=>void} publish
 * @param {{intervalMs?:number,schedule?:(work:()=>void,delay:number)=>any,cancel?:(timer:any)=>void}} options */
export function createCampaignMonitor(request,publish,options={}) {
  const interval=options.intervalMs??5000, schedule=options.schedule??setTimeout, cancel=options.cancel??clearTimeout;
  let generation=0, active=/** @type {any} */ (null), timer=/** @type {any} */ (null), abort=/** @type {AbortController|null} */ (null);
  let running=false, latest=/** @type {any} */ (null), failures=0;
  const clear=()=>{if(timer!==null)cancel(timer);timer=null;};
  async function run(){
    if(running||!active)return;
    const ticket=active;running=true;const controller=new AbortController();abort=controller;
    let delay=/** @type {number|null} */ (null);
    try{
      const body=await request(ticket.selection,controller.signal);
      if(active!==ticket)return;
      if(latest && (BigInt(body.sequence)<BigInt(latest.sequence) || body.sequence===latest.sequence&&body.pin!==latest.pin))throw new Error('Campaign snapshot regressed or changed at the same sequence.');
      latest=body;failures=0;
      const watching=ticket.follow&&['waiting','running'].includes(body.state);
      publish({phase:'ready',body,why:'',watching,failures:0});
      if(watching){ticket.selection={identity:body.identity,pin:null};delay=interval;}
    }catch(why){
      if(active!==ticket)return;
      failures+=1;const watching=ticket.follow&&failures<3;
      publish({phase:latest?'ready':'failed',body:latest,why:why instanceof Error?why.message:String(why),watching,failures});
      if(watching)delay=interval*failures;
    }finally{
      running=false;if(abort===controller)abort=null;
      if(active!==ticket){if(active)void run();}
      else if(delay!==null)timer=schedule(()=>{timer=null;void run();},delay);
    }
  }
  return {
    /** Start a fresh observer; an old unsettled request drains before this one.
     * @param {any} selection @param {boolean} [follow] */
    start(selection,follow=true){
      clear();active=null;abort?.abort();generation+=1;const asked=campaignSelection(selection);
      active={generation,selection:asked,follow};latest=null;failures=0;
      publish({phase:'loading',body:null,why:'',watching:follow,failures:0});void run();
    },
    stop(){clear();active=null;generation+=1;abort?.abort();}
  };
}

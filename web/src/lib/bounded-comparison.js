/** Preserve per-group successes/refusals while limiting active full-frontier reads.
 * No queued group starts after cancellation; callers publish only their current generation.
 * @template T,R
 * @param {T[]} items @param {number} concurrency
 * @param {(item:T,index:number)=>Promise<R>} read @param {AbortSignal} signal
 * @returns {Promise<PromiseSettledResult<R>[]>} */
export async function settleComparison(items,concurrency,read,signal){
  if(!Number.isSafeInteger(concurrency)||concurrency<1||concurrency>8)throw new Error('Comparison concurrency must be from one through eight.');
  let next=0;
  /** @type {PromiseSettledResult<R>[]} */
  const results=new Array(items.length);
  async function worker(){
    while(next<items.length){
      signal.throwIfAborted();const index=next++;
      try{results[index]={status:'fulfilled',value:await read(items[index],index)};}
      catch(reason){results[index]={status:'rejected',reason};}
    }
  }
  await Promise.all(Array.from({length:Math.min(concurrency,items.length)},worker));
  signal.throwIfAborted();return results;
}

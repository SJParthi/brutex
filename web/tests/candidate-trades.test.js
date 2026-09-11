import test from 'node:test';
import assert from 'node:assert/strict';
import { fetchCandidatePage, candidateMoney, candidateTotal, candidatePricingDetails, candidateConditions } from '../src/lib/candidate-trades.js';
const identity = 'ab'.repeat(32), digest = 'cd'.repeat(32), tradeId = 'ef'.repeat(32), attempt = '9007199254740993';
const request = (/** @type {any} */ body) => async () => ({ ok: true, status: 200, json: async () => body });
const rule = { max_mae_ppm:'0', min_rr_bp:'0', min_win_rate_bp:'0', min_trades:'0', min_assurance_bp:'0', min_weakest_bp:'0', min_ret_over_dd_bp:'0', require_protective_exits:false, min_fill_headroom_bp:'0', min_avg_rr_bp:'0', top:'2' };
const tier = { index:'0', eligible:'200', evaluated:'129', horizon:'3', rungs:'1',step_ppm:'100',forced_ppm:null,ratios:true,stops_ppm:['100'],rules:rule };
function candidate(/** @type {number} */ slot) {
  return { tier:'0', rank:String(Math.floor(slot/2)+1),direction:slot%2===0?'long':'short',mask_words:['1','0','0','0','0','0'],trade_identity:tradeId,cell_rules_pass:false,signals:'12',refused_paths:'0',stops_ppm:['100'],targets_ppm:['300'],trails_ppm:['100'],cell:{stop:'0',target:'0',tsl:null,ttp:null,trades:'257',wins:'100',pessimistic:'-150',optimistic:'200',max_drawdown:'160',worst_trade:'-160'} };
}
function page(offset=0) {
  return {model:"and-mask",expression:null,schema_version:1,status:'saved',identity,attempt,catalog_digest:digest,execution_digest:identity,capture:'sealed-pricing-evidence',audit_completion:'refused',kind:'candidates',tier_count:'1',candidate_side_count:'258',tier,offset:String(offset),limit:256,total_count:'258',next_offset:offset===0?'256':null,page_complete:true,selected:null,rows:Array.from({length:offset===0?256:2},(_,i)=>candidate(offset+i)),refusal:null};
}
function tradePage(offset=0) {
  const selected=candidate(1);
  const rows=Array.from({length:offset===0?256:1},(_,i)=>({identity:tradeId,seq:String(offset+i),direction:'short',signal_bar:'9007199254740993',entry_bar:'9007199254740994',exit_bar:'9007199254740995',best:'200',worst:'-150',entry_micros:'1788680000000000',exit_micros:'1788680000000001',adverse_ppm:'100',adverse_paisa:'150',favourable_ppm:'200',favourable_paisa:'200'}));
  return {...page(),kind:'trades',offset:String(offset),total_count:'257',next_offset:offset===0?'256':null,selected,rows};
}

test('candidate paging pins exact attempt/catalog and preserves no institutional claim',async()=>{
  const first=await fetchCandidatePage({identity,attempt},request(page()));
  assert.equal(first.rows.length,256);assert.equal(first.audit_completion,'refused');
  let url='';
  const second=await fetchCandidatePage({identity,attempt,digest:first.catalog_digest,offset:'256'},async (value)=>{url=value;return {ok:true,json:async()=>page(256)};});
  assert.equal(second.rows.length,2);assert.ok(url.includes('attempt=9007199254740993'));assert.ok(url.includes('digest='+digest));
});
test('exact selected candidate-side trade pages keep large indices and money exact',async()=>{
  const selection={identity,attempt,digest,rank:'1',direction:'short'};
  const first=await fetchCandidatePage(selection,request(tradePage()));assert.equal(first.rows[0].entry_bar,'9007199254740994');
  const second=await fetchCandidatePage({...selection,offset:'256'},request(tradePage(256)));assert.equal(second.rows[0].seq,'256');
  assert.equal(candidateMoney('9223372036854775807'),'₹92233720368547758.07');assert.equal(candidateMoney('-9223372036854775808'),'−₹92233720368547758.08');
});
test('candidate and trade mutation matrix refuses mixed pages and winner substitution',async()=>{
  for(const change of [
    (/** @type {any} */body)=>{body.catalog_digest=identity;},
    (/** @type {any} */body)=>{body.attempt='2';},
    (/** @type {any} */body)=>{body.rows.pop();},
    (/** @type {any} */body)=>{body.next_offset=null;},
    (/** @type {any} */body)=>{body.rows[0].rank='2';},
    (/** @type {any} */body)=>{body.rows[0].direction='short';},
    (/** @type {any} */body)=>{body.rows[0].mask_words[0]=1;},
    (/** @type {any} */body)=>{body.rows[0].cell.stop='1';},
    (/** @type {any} */body)=>{body.tier.evaluated='201';},
    (/** @type {any} */body)=>{body.page_complete=false;}
  ]){const body=structuredClone(page());change(body);await assert.rejects(fetchCandidatePage({identity,attempt,digest},request(body)));}
  for(const change of [
    (/** @type {any} */body)=>{body.selected.rank='2';},
    (/** @type {any} */body)=>{body.rows[0].identity=identity;},
    (/** @type {any} */body)=>{body.rows[0].direction='long';},
    (/** @type {any} */body)=>{body.rows[0].seq='1';},
    (/** @type {any} */body)=>{body.rows[0].worst='201';},
    (/** @type {any} */body)=>{body.rows[0].entry_bar='9007199254740996';}
  ]){const body=tradePage();change(body);await assert.rejects(fetchCandidatePage({identity,attempt,digest,rank:'1',direction:'short'},request(body)));}
});
test('missing captures and no-cell candidates remain explicit and paginated requests cannot fall back',async()=>{
  const missing={model:"and-mask",expression:null,schema_version:1,status:'missing',identity,attempt,rows:[],refusal:null,why:'No sealed capture.'};
  assert.equal((await fetchCandidatePage({identity,attempt},request(missing))).status,'missing');
  await assert.rejects(fetchCandidatePage({identity,attempt,digest},request(missing)));
  const body=page();body.rows[0].cell=/** @type {any} */(null);
  assert.equal((await fetchCandidatePage({identity,attempt},request(body))).rows[0].cell,null);
  body.rows[0].cell_rules_pass=true;await assert.rejects(fetchCandidatePage({identity,attempt},request(body)));
  await assert.rejects(fetchCandidatePage({identity,attempt,offset:'256'},request(page(256))));
  await assert.rejects(fetchCandidatePage({identity,attempt},async()=>({ok:false,status:503})),/503/);
});
test('condition names use supplied canonical vocabulary and otherwise show explicit bit positions',()=>{
  assert.equal(candidateConditions(candidate(0),{bits:new Map([[0,{name:'canonical_name'}]])}),'canonical_name');
  assert.equal(candidateConditions(candidate(0),null),'Condition bit 0');
});
test('expression pages require explicit model and complete descriptor, never an AND replacement', async () => {
  const body = page();
  body.model = 'expression';
  body.expression = /** @type {any} */ ({ version: 1, encoded_hex: '01000100010000', source: '0', referenced_mask_words: ['1','0','0','0','0','0'] });
  const selected = { identity, attempt, model: 'expression' };
  assert.equal((await fetchCandidatePage(selected, request(body))).model, 'expression');
  await assert.rejects(fetchCandidatePage({ identity, attempt }, request(body)));
  await assert.rejects(fetchCandidatePage(selected, request(page())));
  const absent = structuredClone(body); absent.expression = null;
  await assert.rejects(fetchCandidatePage(selected, request(absent)));
  const changed = structuredClone(body); changed.rows[0].mask_words[0] = '2';
  await assert.rejects(fetchCandidatePage(selected, request(changed)));
});

test('a priced row with no selected cell keeps its saved signals and refusal counts without inventing totals', async () => {
  const body = page();
  body.rows[0].cell = /** @type {any} */ (null);
  body.rows[0].signals = '9007199254740993';
  body.rows[0].refused_paths = '9007199254740992';
  const saved = (await fetchCandidatePage({ identity, attempt }, request(body))).rows[0];
  assert.equal(candidateTotal(saved, 'pessimistic'), 'No selected cell');
  assert.equal(candidateTotal(saved, 'optimistic'), 'No selected cell');
  const detail = candidatePricingDetails(saved);
  assert.deepEqual(detail.slice(0, 3), [
    { label: 'Observed signals', value: '9007199254740993' },
    { label: 'Refused execution paths', value: '9007199254740992' },
    { label: 'Pricing outcome', value: 'No selected cell' }
  ]);
  assert.ok(detail.some((row) => row.label === 'Saved stop choices'));
  assert.ok(detail.every((row) => !row.label.startsWith('Selected ')));
});

test('selected exit indices map to their own exact saved ladders including trailing profit', () => {
  const saved = candidate(0);
  saved.stops_ppm = ['100', '9007199254740993'];
  saved.targets_ppm = ['300', '700'];
  saved.trails_ppm = ['50', '125'];
  saved.cell.stop = '1';
  saved.cell.target = '1';
  saved.cell.tsl = /** @type {any} */ ('1');
  saved.cell.ttp = /** @type {any} */ ({ arm: '0', trail: '0' });
  const rows = candidatePricingDetails(saved);
  const field = (/** @type {string} */ label) => rows.find((row) => row.label === label)?.value;
  assert.equal(field('Observed signals'), '12');
  assert.equal(field('Refused execution paths'), '0');
  assert.equal(field('Selected stop'), 'Choice 1 · 9007199254740993 ppm');
  assert.equal(field('Selected target'), 'Choice 1 · 700 ppm');
  assert.equal(field('Selected trailing stop'), 'Choice 1 · 125 ppm');
  assert.equal(field('Trailing profit activation'), 'Choice 0 · 300 ppm');
  assert.equal(field('Trailing profit distance'), 'Choice 0 · 50 ppm');
  assert.equal(field('Saved stop choices'), '0: 100 ppm · 1: 9007199254740993 ppm');
  assert.equal(candidateTotal(saved, 'pessimistic'), '−₹1.50');
});

test('saved zero, empty ladders and explicit unselected settings remain distinct from missing fields', () => {
  assert.deepEqual(candidatePricingDetails(null), []);
  assert.deepEqual(candidatePricingDetails({}), []);
  assert.deepEqual(candidatePricingDetails({ signals: '0', stops_ppm: [] }), [
    { label: 'Observed signals', value: '0' }, { label: 'Saved stop choices', value: 'Empty saved ladder' }
  ]);
  assert.deepEqual(candidatePricingDetails({ cell: { stop: null, ttp: null } }), [
    { label: 'Selected stop', value: 'No setting selected' },
    { label: 'Selected trailing profit', value: 'No setting selected' }
  ]);
  assert.deepEqual(candidatePricingDetails({ cell: { stop: '1' }, stops_ppm: ['100'] }), [
    { label: 'Saved stop choices', value: '0: 100 ppm' }
  ]);
  assert.equal(candidateTotal({}, 'pessimistic'), 'unavailable');
});

import assert from 'node:assert/strict';
import {chromium} from 'playwright';

const base=process.env.CONNECTORS_WEBSITE_URL??'http://127.0.0.1:3100';
const browser=await chromium.launch({headless:true});
const page=await browser.newPage({viewport:{width:1440,height:1000}});
const errors=[];
page.on('pageerror',error=>errors.push(String(error)));
async function visit(route){await page.goto(base+route,{waitUntil:'networkidle'});assert.equal(await page.locator('#webpack-dev-server-client-overlay').count(),0,'development compiler overlay');}
async function click(label){await page.getByRole('button',{name:label,exact:true}).click();}
async function guided(count){for(let i=0;i<count;i++)await page.getByRole('button',{name:/Next guided step/}).click();}
function state(label){return page.locator('.lab-state').filter({has:page.locator('span',{hasText:label})}).locator('strong');}
try{
  await visit('/');
  assert.match(await page.locator('h1').innerText(),/Independent integrations/);
  for(const label of ['Home / Introduction','Contracts','Adapters'])assert.equal(await page.locator('.navbar').getByRole('link',{name:label,exact:true}).count(),1);
  // With system preference enabled the switch cycles system → light → dark.
  for(let i=0;i<3&&await page.locator('html').getAttribute('data-theme')!=='dark';i++)await page.getByRole('button',{name:/Switch between dark and light mode/}).click();
  assert.equal(await page.locator('html').getAttribute('data-theme'),'dark');
  await page.getByRole('button',{name:/Switch between dark and light mode/}).click();
  await visit('/contracts/auth/connection#41-connection-viability-and-operation-eligibility');
  assert.equal(await page.locator('[id="41-connection-viability-and-operation-eligibility"]').count(),1);
  assert.match(await page.locator('.canonical-contract').innerText(),/Unknown and unavailable never become positive evidence/);
  assert.doesNotMatch(await page.locator('.reference-article').innerText(),/\/home\/|\.local\/|\.engineering\//);
  await visit('/contracts/model/domains/connectors-mutations');
  const link=page.getByRole('link',{name:'Connection',exact:true});
  if(await link.count())assert.match(await link.first().getAttribute('href'),/connectors-auth_bindings#connection$/);
  const diagram=page.locator('.model-diagram').first();
  if(await diagram.count()){
    if(await diagram.getAttribute('open')===null)await diagram.locator('summary').first().click();
    await diagram.locator('svg').first().waitFor({state:'visible'});
  }
  await visit('/introduction/examples');
  await page.getByRole('button',{name:'Play',exact:true}).waitFor();
  const progress=()=>page.locator('.journey-progress').innerText();
  const heading=()=>page.locator('.journey-explanation h2').innerText();
  const next=()=>page.getByRole('button',{name:'Next',exact:true});
  async function finishJourney(){for(let i=0;i<65;i++){const read=page.getByRole('button',{name:'Read issues',exact:true});if(await read.count()){await read.click();continue;}if(await next().isDisabled())return;await next().click();}throw new Error('Journey did not terminate');}
  await page.clock.install();
  await click('Play');assert.match(await progress(),/Step 1 /);
  await page.clock.fastForward(6100);assert.match(await progress(),/Step 2 /);
  await click('Pause');assert.equal(await page.locator('.wire-active').evaluate(el=>getComputedStyle(el).animationPlayState),'paused');const paused=await progress();await page.clock.fastForward(10000);assert.equal(await progress(),paused);
  await click('Back');assert.match(await heading(),/asks what it can call/);
  await click('Restart');assert.equal(await progress(),'Ready to start');
  await finishJourney();assert.equal(await page.locator('.journey-result tbody tr').count(),3);
  assert.match(await page.locator('.journey-result').innerText(),/Closed/);
  for(const [situation,outcome] of [['gateway_denied','unauthorized'],['adapter_unreachable','unavailable'],['provider_rejected','unauthorized']]){
    await page.getByLabel('Explore a situation').selectOption(situation);await finishJourney();
    assert.equal(await page.locator('.journey-result').count(),0);
    assert.match(await page.locator('.journey-details').textContent(),new RegExp(outcome));
    await page.locator('.wire-error').waitFor({state:'attached'});
  }
  await page.getByRole('button',{name:/User authorization/}).click();
  assert.equal(await page.locator('.laptop-zone [data-node="identity"]').count(),0,'identity service must not be depicted inside the laptop');
  for(let i=0;i<60;i++){if(await page.getByRole('button',{name:'Read issues',exact:true}).count())break;await next().click();}
  assert.match(await heading(),/read has not run/);
  assert(await page.getByRole('button',{name:'Play',exact:true}).isDisabled());
  assert.equal(await page.locator('.journey-result').count(),0);
  const connected=await progress();await page.clock.fastForward(30000);assert.equal(await progress(),connected);
  await click('Read issues');await finishJourney();assert.equal(await page.locator('.journey-result tbody tr').count(),3);
  for(const [situation,outcome] of [['consent_declined','refused_by_provider'],['connection_denied','not_granted'],['custody_unavailable','connection_not_ready']]){
    await page.getByLabel('Explore a situation').selectOption(situation);await finishJourney();
    assert.equal(await page.locator('.journey-result').count(),0);assert.match(await page.locator('.journey-details').textContent(),new RegExp(outcome));
  }
  await page.getByRole('button',{name:/Configured access/}).click();
  await page.emulateMedia({reducedMotion:'reduce'});await next().click();
  assert.equal(await page.locator('.wire-active').evaluate(el=>getComputedStyle(el).animationName),'none');
  assert(await page.locator('.journey-explanation>p').evaluate(el=>parseFloat(getComputedStyle(el).fontSize)>=16));
  assert(await page.locator('[data-node="client"]>strong').evaluate(el=>parseFloat(getComputedStyle(el).fontSize)>=14));
  await page.emulateMedia({reducedMotion:'no-preference'});
  await visit('/contracts/examples');
  await page.getByRole('button',{name:/Next guided step/}).waitFor({state:'visible'});
  await guided(8);
  assert.equal(await state('OPERATION ELIGIBILITY').innerText(),'eligible');
  assert.equal(await page.locator('.lab-binding code').last().innerText(),'database-a.example');
  assert.match(await page.getByRole('log').innerText(),/new_connection_required/);
  await click('02 · Readiness');await guided(9);
  assert.equal(await state('GLOBAL VIABILITY').innerText(),'revoked');
  await click('03 · Uncertain outcomes');await guided(8);
  assert.equal(await state('ATTEMPT STATE').innerText(),'Indeterminate');
  assert.equal(await state('PROVIDER SENDS').innerText(),'1');
  assert.equal(await state('APPROVAL REDEMPTIONS').innerText(),'1');
  await click('Try conflicting input');assert.match(await page.getByRole('log').innerText(),/idempotency_conflict/);
  await click('Reset example');
  for(const label of ['Supply approval','Prepare attempt','Dispatch once','Record success','Retry same request'])await click(label);
  assert.match(await page.getByRole('log').innerText(),/result_replayed/);
  await click('Toggle host admission');await click('Retry same request');
  assert.match(await page.locator('.lab-event').first().innerText(),/not_granted/);
  await click('Reset example');
  await page.getByRole('button',{name:'Supply approval',exact:true}).focus();
  await page.keyboard.press('Enter');
  assert.match(await page.locator('.lab-event').first().innerText(),/approval_supplied/);
  await page.setViewportSize({width:390,height:844});await visit('/');
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth+1),'home horizontal overflow');
  await page.getByRole('button',{name:/Toggle navigation bar/}).click();
  await page.locator('.navbar-sidebar').getByRole('link',{name:'Contracts',exact:true}).click();
  await page.waitForURL(base+'/contracts');
  await visit('/contracts/examples');
  await page.getByRole('button',{name:/Next guided step/}).waitFor({state:'visible'});
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth+1),'lab horizontal overflow');
  await page.route('**/examples/contracts.wasm',route=>route.fulfill({status:404,body:'Missing module'}));
  await visit('/contracts/examples');
  await page.getByRole('alert').filter({hasText:'could not be loaded'}).waitFor();
  await click('02 · Readiness');
  assert.match(await page.getByRole('alert').innerText(),/could not be loaded/);
  assert.equal(await page.getByRole('button',{name:/Next guided step/}).count(),0,'unavailable module must not fabricate execution');
  await visit('/introduction/examples');
  await page.getByRole('alert').filter({hasText:'walkthrough could not be loaded'}).waitFor();
  assert.equal(await page.getByRole('button',{name:'Play',exact:true}).count(),0);
  await page.unroute('**/examples/contracts.wasm');
  await visit('/introduction/examples');await page.getByRole('button',{name:'Play',exact:true}).waitFor();
  assert(await page.evaluate(()=>document.documentElement.scrollWidth<=window.innerWidth+1),'walkthrough horizontal overflow');
  await page.getByRole('button',{name:'Next',exact:true}).focus();await page.keyboard.press('Enter');assert.match(await page.locator('.journey-progress').innerText(),/Step 1 /);
  assert.deepEqual(errors,[],'browser runtime errors');
  console.log('Browser smoke: both practical walkthroughs, six failures, deliberate post-consent read, playback/navigation, credential boundaries, advanced labs, reference/diagram, keyboard, themes, reduced motion, mobile layout and unavailable modules passed.');
}finally{await browser.close();}
